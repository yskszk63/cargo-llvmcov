use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::mem;
use std::path::{Path, PathBuf};
use std::process::{
    self, Child as StdChild, ChildStdout as StdChildStdout, Command as StdCommand,
    Output as StdOutput, Stdio,
};

use cargo_binutils::Tool;
use cargo_metadata::Metadata;
use clap::Clap;

#[cfg(test)]
mod tests;

#[derive(Debug, PartialEq, Eq, serde::Deserialize)]
struct BuildTarget {
    test: bool,
}

#[derive(Debug, PartialEq, Eq, serde::Deserialize)]
struct BuildProfile {
    test: bool,
}

#[derive(Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(tag = "reason")]
enum BuildMessage {
    #[serde(rename = "compiler-message")]
    CompilerMessage {},

    #[serde(rename = "compiler-artifact")]
    CompilerArtifact {
        target: BuildTarget,
        profile: BuildProfile,
        executable: Option<PathBuf>,
    },

    #[serde(rename = "build-script-executed")]
    BuildScriptExecuted {},

    #[serde(rename = "build-finished")]
    BuildFinished { success: bool },
}

#[cfg(test)]
std::thread_local! {
    static MOCK_RESULT: std::cell::RefCell<Option<(&'static [u8], bool)>> = std::cell::RefCell::new(None);
}

#[derive(Debug)]
enum Output {
    Actual(StdOutput),
    #[cfg(test)]
    Mock(Vec<u8>, bool),
}

impl Output {
    fn stdout(&self) -> &[u8] {
        match self {
            Self::Actual(output) => &output.stdout,
            #[cfg(test)]
            Self::Mock(output, _) => &output,
        }
    }
}

#[derive(Debug)]
struct ExitStatus(bool);

impl ExitStatus {
    fn success(&self) -> bool {
        self.0
    }
}

#[derive(Debug)]
enum Child {
    Actual(StdChild),
    #[cfg(test)]
    Mock(Vec<u8>, bool),
}

impl Child {
    fn wait(&mut self) -> io::Result<ExitStatus> {
        match self {
            Self::Actual(child) => Ok(ExitStatus(child.wait()?.success())),
            #[cfg(test)]
            Self::Mock(_, r) => Ok(ExitStatus(*r)),
        }
    }

    fn take_stdout(&mut self) -> ChildStdout {
        match self {
            Self::Actual(child) => ChildStdout::Actual(child.stdout.take().unwrap()),
            #[cfg(test)]
            Self::Mock(v, _) => ChildStdout::Mock(io::Cursor::new(v.clone())),
        }
    }
}

#[derive(Debug)]
enum ChildStdout {
    Actual(StdChildStdout),
    #[cfg(test)]
    Mock(std::io::Cursor<Vec<u8>>),
}

impl io::Read for ChildStdout {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Actual(stdout) => stdout.read(buf),
            #[cfg(test)]
            Self::Mock(v) => v.read(buf),
        }
    }
}

#[derive(Debug)]
struct Command {
    commands: Vec<String>,
    inner: StdCommand,
}

impl Command {
    fn log(&self) {
        log::debug!("CALL {}", self.commands.join(" "));
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

    fn args(&mut self, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> &mut Self {
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
        #[cfg(test)]
        {
            if let Some((v, r)) = MOCK_RESULT.with(|o| o.borrow_mut().take()) {
                return Ok(Output::Mock(v.to_vec(), r));
            }
        }
        Ok(Output::Actual(self.inner.output()?))
    }

    fn spawn(&mut self) -> io::Result<Child> {
        self.log();
        #[cfg(test)]
        {
            if let Some((v, r)) = MOCK_RESULT.with(|o| o.borrow_mut().take()) {
                return Ok(Child::Mock(v.to_vec(), r));
            }
        }
        Ok(Child::Actual(self.inner.spawn()?))
    }

    fn status(&mut self) -> io::Result<ExitStatus> {
        self.log();
        #[cfg(test)]
        {
            if let Some((_, r)) = MOCK_RESULT.with(|o| o.borrow_mut().take()) {
                return Ok(ExitStatus(r));
            }
        }
        Ok(ExitStatus(self.inner.status()?.success()))
    }
}

#[derive(Debug)]
struct Profenv {
    profraw_dir: PathBuf,
    profdata: PathBuf,
}

impl Profenv {
    fn new(basedir: &Path) -> io::Result<Self> {
        let profraw_dir = basedir.join(format!("profraw-{}", process::id()));
        let profdata = basedir.join("default.profdata");
        fs::create_dir(&profraw_dir)?;
        Ok(Self {
            profraw_dir,
            profdata,
        })
    }

    fn profraw(&self) -> PathBuf {
        self.profraw_dir.join("%p.profraw")
    }

    fn profraw_files(&self) -> anyhow::Result<Vec<PathBuf>> {
        let pattern = self.profraw_dir.join("*.profraw");
        let result = glob::glob(pattern.to_string_lossy().as_ref())?.collect::<Result<_, _>>()?;
        Ok(result)
    }
}

impl Drop for Profenv {
    fn drop(&mut self) {
        log::debug!("remove profraw & profdata");
        if let Err(e) = fs::remove_dir_all(&self.profraw_dir) {
            log::warn!("failed to remove dir {}", e);
        }
        if let Err(e) = fs::remove_file(&self.profdata) {
            log::warn!("failed to remove dir {}", e);
        }
    }
}

fn cargo() -> PathBuf {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    cargo.into()
}

fn cargo_home() -> String {
    env::var("CARGO_HOME").unwrap_or_default()
}

fn rustup_home() -> String {
    env::var("RUSTUP_HOME").unwrap_or_default()
}

fn target_dir(cargo: &Path) -> anyhow::Result<PathBuf> {
    let metadata = Command::new(cargo).arg("metadata").output()?;
    let metadata = serde_json::from_slice::<Metadata>(metadata.stdout())?;
    Ok(metadata.target_directory)
}

fn build(cargo: &Path, target: &Path, profenv: &Profenv) -> anyhow::Result<Vec<PathBuf>> {
    let mut build_proc = Command::new(cargo)
        .arg("build")
        .arg("--message-format")
        .arg("json")
        .arg("--tests")
        .arg("--target-dir")
        .arg(target)
        .env("RUSTC_BOOTSTRAP", "1")
        .env("RUSTFLAGS", "-Zinstrument-coverage")
        .env("LLVM_PROFILE_FILE", &profenv.profraw())
        .stdout(Stdio::piped())
        .spawn()?;

    let mut executables = vec![];

    let stdout = build_proc.take_stdout();
    let stdout = BufReader::new(stdout);
    for line in stdout.lines() {
        let line = line?;
        let line = serde_json::from_str::<BuildMessage>(&line)?;
        if let BuildMessage::CompilerArtifact {
            executable: Some(exe),
            profile: BuildProfile { test: true },
            target: BuildTarget { test: true },
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

fn run_test(prog: &Path, profenv: &Profenv) -> anyhow::Result<()> {
    let r = Command::new(&prog)
        .arg("--nocapture")
        .env("LLVM_PROFILE_FILE", profenv.profraw())
        .status()?;
    if !r.success() {
        anyhow::bail!("failed to run executable.");
    }
    Ok(())
}

fn merge_profdata(llvm_profdata: &Path, profenv: &Profenv) -> anyhow::Result<()> {
    let result = Command::new(llvm_profdata)
        .arg("merge")
        .arg("-sparse")
        .args(profenv.profraw_files()?)
        .arg("-o")
        .arg(&profenv.profdata)
        .status()?;
    if !result.success() {
        anyhow::bail!("failed to run llvm-profdata.");
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
    llvm_cov: &Path,
    rustfilt: &Path,
    profenv: &Profenv,
    executables: &[PathBuf],
    html_output: Option<&Path>,
    ignore: &str,
) -> anyhow::Result<()> {
    let output = if let Some(path) = html_output {
        vec![format!("-output-dir={}", path.to_string_lossy())]
    } else {
        vec![]
    };

    let result = Command::new(llvm_cov)
        .arg("show")
        .arg(format!(
            "-Xdemangler={}",
            rustfilt.to_string_lossy().as_ref()
        ))
        .args(to_obj_args(executables))
        .arg(format!(
            "-instr-profile={}",
            profenv.profdata.to_string_lossy()
        ))
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
        anyhow::bail!("failed to run llvm-cov.");
    }
    Ok(())
}

fn llvm_cov_export(
    llvm_cov: &Path,
    rustfilt: &Path,
    profenv: &Profenv,
    executables: &[PathBuf],
    output: &Path,
    ignore: &str,
) -> anyhow::Result<()> {
    let result = Command::new(llvm_cov)
        .arg("export")
        .arg(format!(
            "-Xdemangler={}",
            rustfilt.to_string_lossy().as_ref()
        ))
        .args(to_obj_args(executables))
        .arg(format!(
            "-instr-profile={}",
            profenv.profdata.to_string_lossy()
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
        anyhow::bail!("failed to run llvm-cov.");
    }
    Ok(())
}

#[derive(Debug, Clap)]
#[clap(bin_name = "cargo", version = env!("CARGO_PKG_VERSION"), after_long_help = option_env!("RUSTFLAGS").unwrap_or_default())]
pub enum SubCommand {
    Llvmcov(Opts),
}

#[derive(Debug, Clap)]
#[clap(version = env!("CARGO_PKG_VERSION"))]
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
    let opts = SubCommand::parse();
    let SubCommand::Llvmcov(opts) = opts;

    stderrlog::new().verbosity(opts.verbose).init()?;

    let llvm_profdata = Tool::Profdata.path()?;
    let llvm_cov = Tool::Cov.path()?;
    let rustfilt = which::which("rustfilt")?;

    let cargo = cargo();
    let target = target_dir(&cargo)?.join("cov");
    fs::create_dir_all(&target)?;
    let profenv = Profenv::new(&target)?;
    let executables = build(&cargo, &target, &profenv)?;

    log::debug!("cargo binary: {:?}", cargo);
    log::debug!("output directory: {:?}", target);
    log::debug!("llvm-profdata: {:?}", llvm_profdata);
    log::debug!("executables: {:?}", executables);
    log::debug!("LLVM_PROFILE_FILE: {:?}", profenv.profraw());

    for executable in &executables {
        run_test(&executable, &profenv)?;
    }

    merge_profdata(&llvm_profdata, &profenv)?;

    let cargo_home = cargo_home();
    let rustup_home = rustup_home();
    let ignore = format!("{}|{}", cargo_home, rustup_home);

    log::debug!("generating report..");
    match opts {
        Opts { lcov: true, .. } => {
            llvm_cov_export(
                &llvm_cov,
                &rustfilt,
                &profenv,
                &executables,
                &target.join("cov.info"),
                &cargo_home,
            )?;
        }
        Opts {
            lcov_output: Some(lcov),
            ..
        } => {
            llvm_cov_export(&llvm_cov, &rustfilt, &profenv, &executables, &lcov, &ignore)?;
        }
        Opts { html: true, .. } => {
            llvm_cov_show(
                &llvm_cov,
                &rustfilt,
                &profenv,
                &executables,
                Some(&target.join("html")),
                &ignore,
            )?;
        }
        _ => {
            llvm_cov_show(&llvm_cov, &rustfilt, &profenv, &executables, None, &ignore)?;
        }
    }

    if opts.keep {
        mem::forget(profenv);
    }

    if opts.open {
        opener::open(&target.join("html/index.html"))?;
    }

    Ok(())
}
