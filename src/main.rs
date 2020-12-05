use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command as StdCommand, Stdio, Output, Child, ExitStatus};

use cargo_binutils::Tool;
use cargo_metadata::Metadata;
use clap::Clap;

#[derive(Debug, serde::Deserialize)]
struct BuildTarget {
    test: bool,
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "reason")]
enum BuildMessage {
    #[serde(rename = "compiler-message")]
    CompilerMessage {},

    #[serde(rename = "compiler-artifact")]
    CompilerArtifact {
        target: BuildTarget,
        executable: Option<PathBuf>,
    },

    #[serde(rename = "build-script-executed")]
    BuildScriptExecuted {},

    #[serde(rename = "build-finished")]
    BuildFinished { success: bool },
}

#[derive(Debug)]
struct Command {
    commands: Vec<String>,
    inner: StdCommand,
}

impl Command {
    fn log(&self) {
        log::debug!("call {}", self.commands.join(" "));
    }

    fn new(program: impl AsRef<OsStr>) -> Self {
        let program = program.as_ref();
        Self {
            commands: vec![program.to_string_lossy().to_string()],
            inner: StdCommand::new(program),
        }
    }

    fn arg(&mut self, arg: impl AsRef<OsStr>) -> &mut Self {
        let arg = arg.as_ref();
        self.commands.push(arg.to_string_lossy().to_string());
        self.inner.arg(arg);
        self
    }

    fn args(&mut self, args: impl IntoIterator<Item=impl AsRef<OsStr>>) -> &mut Self {
        for arg in args {
            self.arg(arg);
        }
        self
    }

    fn env(&mut self, key: impl AsRef<OsStr>, val: impl AsRef<OsStr>) -> &mut Self {
        self.inner.env(key, val);
        self
    }

    fn stdout(&mut self, cfg: impl Into<Stdio>) -> &mut Self {
        self.inner.stdout(cfg);
        self
    }

    fn output(&mut self) -> io::Result<Output> {
        self.log();
        self.inner.output()
    }

    fn spawn(&mut self) -> io::Result<Child> {
        self.log();
        self.inner.spawn()
    }

    fn status(&mut self) -> io::Result<ExitStatus> {
        self.log();
        self.inner.status()
    }
}

fn target_dir(cargo: &Path) -> anyhow::Result<PathBuf> {
    let metadata = Command::new(cargo).arg("metadata").output()?;
    let metadata = serde_json::from_slice::<Metadata>(&metadata.stdout)?;
    Ok(metadata.target_directory)
}

fn build(
    cargo: &Path,
    target: &Path,
    profraw: &Path,
) -> anyhow::Result<Vec<PathBuf>> {
    let mut build_proc = Command::new(cargo)
        .arg("build")
        .arg("--message-format")
        .arg("json")
        .arg("--tests")
        .arg("--target-dir")
        .arg(target)
        .env("RUSTC_BOOTSTRAP", "1")
        .env("RUSTFLAGS", "-Zinstrument-coverage")
        .env("LLVM_PROFILE_FILE", &profraw)
        .stdout(Stdio::piped())
        .spawn()?;

    let mut executables = vec![];

    let stdout = build_proc.stdout.take().unwrap();
    let stdout = BufReader::new(stdout);
    for line in stdout.lines() {
        let line = line?;
        let line = serde_json::from_str::<BuildMessage>(&line)?;
        if let BuildMessage::CompilerArtifact {
            executable: Some(exe),
            target:
                BuildTarget {
                    test: true,
                },
        } = line
        {
            executables.push(exe);
        }
    }

    let r = build_proc.wait()?;
    if !r.success() {
        anyhow::bail!("failed to run cargo build.");
    }

    if executables.is_empty() {
        anyhow::bail!("no executable found.")
    }
    Ok(executables)
}

fn merge_profdata(profraw: impl AsRef<Path>, profdata: impl AsRef<Path>) -> anyhow::Result<()> {
    let llvm_profdata = Tool::Profdata.path()?;
    let result = Command::new(llvm_profdata)
        .arg("merge")
        .arg("-sparse")
        .arg(profraw.as_ref())
        .arg("-o")
        .arg(profdata.as_ref())
        .status()?;
    if !result.success() {
        anyhow::bail!("failed to run llvm-profdata");
    }
    Ok(())
}

fn to_obj_args<'a>(executables: &'a [PathBuf]) -> Vec<&'a OsStr> {
    let mut r = vec![];
    let mut iter = executables.iter();
    if let Some(exe) = iter.next() {
        r.push(exe.as_ref());
    }
    for exe in iter {
        r.push("-object".as_ref());
        r.push(exe.as_ref());
    }
    r
}

fn llvm_cov_show(
    profdata: &Path,
    executables: &[PathBuf],
    html_output: Option<&Path>,
    ignore: &str,
) -> anyhow::Result<()> {
    let output = if let Some(path) = html_output {
        vec![format!("-output-dir={}", path.to_string_lossy())]
    } else {
        vec![]
    };

    let llvm_cov = Tool::Cov.path()?;
    let result = Command::new(llvm_cov)
        .arg("show")
        .arg("-Xdemangler=rustfilt") // TODO
        .args(to_obj_args(executables))
        .arg(format!("-instr-profile={}", profdata.to_string_lossy()))
        .arg(format!(
            "-format={}",
            if html_output.is_some() {
                "html"
            } else {
                "text"
            }
        ))
        .args(output)
        .arg(format!("-ignore-filename-regex={}", ignore))
        .arg("-show-instantiations=false")
        .status()?;
    if !result.success() {
        anyhow::bail!("failed to run llvm-cov");
    }
    Ok(())
}

fn llvm_cov_export(
    profdata: impl AsRef<Path>,
    executables: &[PathBuf],
    output: impl AsRef<Path>,
    ignore: &str,
) -> anyhow::Result<()> {
    let llvm_cov = Tool::Cov.path()?;
    let result = Command::new(llvm_cov)
        .arg("export")
        .arg("-Xdemangler=rustfilt") // TODO
        .args(to_obj_args(executables))
        .arg(format!(
            "-instr-profile={}",
            profdata.as_ref().to_string_lossy()
        ))
        .arg("-format=lcov")
        .arg(format!("-ignore-filename-regex={}", ignore))
        .arg("-show-instantiations=false")
        .stdout(
            fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open(output)?,
        )
        .status()?;
    if !result.success() {
        anyhow::bail!("failed to run llvm-cov");
    }
    Ok(())
}

#[derive(Debug, Clap)]
pub struct Opts {
    #[clap(short = 'l', long, conflicts_with_all = &["html", "lcov-output"])]
    lcov: bool,

    #[clap(short = 'L', long)]
    lcov_output: Option<PathBuf>,

    #[clap(short = 'h', long, conflicts_with_all = &["lcov", "lcov-output"])]
    html: bool,

    #[clap(short = 'o', long, requires = "html")]
    open: bool,

    #[clap(short = 'k', long)]
    keep: bool,

    #[clap(short = 'v', long, parse(from_occurrences))]
    verbose: usize,
}

fn main() -> anyhow::Result<()> {
    let opts = Opts::parse_from(env::args().skip(1));

    stderrlog::new().verbosity(opts.verbose).init()?;

    let cargo = env::var("CARGO").unwrap_or("cargo".to_owned());
    let cargo = PathBuf::from(cargo);
    log::debug!("cargo binary: {:?}", cargo);

    let target = target_dir(&cargo)?;
    let target = target.join("cov");
    log::debug!("output directory: {:?}", target);

    let profraw = target.join("default.profraw");
    log::debug!("LLVM_PROFILE_FILE: {:?}", profraw);
    let executables = build(&cargo, &target, &profraw)?;
    log::debug!("executables: {:?}", executables);

    for executable in &executables {
        if !Command::new(&executable)
            .env("LLVM_PROFILE_FILE", &profraw)
            .status()?
            .success()
        {
            anyhow::bail!("failed to run executable.");
        }
    }

    let profdata = target.join("default.profdata");

    merge_profdata(&profraw, &profdata)?;

    log::debug!("generating report..");
    let cargo_home = env::var("CARGO_HOME").unwrap_or_default();
    match opts {
        Opts { lcov: true, .. } => {
            llvm_cov_export(&profdata, &executables, &target.join("cov.info"), &cargo_home)?;
        }
        Opts {
            lcov_output: Some(lcov),
            ..
        } => {
            llvm_cov_export(&profdata, &executables, &lcov, &cargo_home)?;
        }
        Opts { html: true, .. } => {
            llvm_cov_show(&profdata, &executables, Some(&target.join("html")), &cargo_home)?;
        }
        _ => {
            llvm_cov_show(&profdata, &executables, None, &cargo_home)?;
        }
    }

    if !opts.keep {
        log::debug!("remove profraw & profdata");
        fs::remove_file(&profraw)?;
        fs::remove_file(&profdata)?;
    }

    if opts.open {
        opener::open(&target.join("html/index.html"))?;
    }

    Ok(())
}
