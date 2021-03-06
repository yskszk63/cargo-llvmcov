use super::*;
use capture_logger::{begin_capture, pop_captured};

#[test]
fn test_build_message() {
    let m = r#"{"reason":"compiler-message","package_id":"cargo-llvmcov 0.1.0 (path+file:///home/ysk/work/cargo-llvmcov)","target":{"kind":["bin"],"crate_types":["bin"],"name":"cargo-llvmcov","src_path":"/home/ysk/work/cargo-llvmcov/src/main.rs","edition":"2018","doctest":false,"test":true},"message":{"rendered":"warning: unused variable: `a`\n  --> src/main.rs:50:13\n   |\n50 |         let a = 1;\n   |             ^ help: if this is intentional, prefix it with an underscore: `_a`\n   |\n   = note: `#[warn(unused_variables)]` on by default\n\n","children":[{"children":[],"code":null,"level":"note","message":"`#[warn(unused_variables)]` on by default","rendered":null,"spans":[]},{"children":[],"code":null,"level":"help","message":"if this is intentional, prefix it with an underscore","rendered":null,"spans":[{"byte_end":1044,"byte_start":1043,"column_end":14,"column_start":13,"expansion":null,"file_name":"src/main.rs","is_primary":true,"label":null,"line_end":50,"line_start":50,"suggested_replacement":"_a","suggestion_applicability":"MachineApplicable","text":[{"highlight_end":14,"highlight_start":13,"text":"        let a = 1;"}]}]}],"code":{"code":"unused_variables","explanation":null},"level":"warning","message":"unused variable: `a`","spans":[{"byte_end":1044,"byte_start":1043,"column_end":14,"column_start":13,"expansion":null,"file_name":"src/main.rs","is_primary":true,"label":null,"line_end":50,"line_start":50,"suggested_replacement":null,"suggestion_applicability":null,"text":[{"highlight_end":14,"highlight_start":13,"text":"        let a = 1;"}]}]}}"#;
    let m = serde_json::from_str(m).unwrap();
    assert_eq!(BuildMessage::CompilerMessage {}, m);

    let m = r#"{"reason":"compiler-artifact","package_id":"cargo-llvmcov 0.1.0 (path+file:///home/ysk/work/cargo-llvmcov)","target":{"kind":["bin"],"crate_types":["bin"],"name":"cargo-llvmcov","src_path":"/home/ysk/work/cargo-llvmcov/src/main.rs","edition":"2018","doctest":false,"test":true},"profile":{"opt_level":"0","debuginfo":2,"debug_assertions":true,"overflow_checks":true,"test":false},"features":[],"filenames":["/home/ysk/work/cargo-llvmcov/target/debug/cargo-llvmcov"],"executable":"/home/ysk/work/cargo-llvmcov/target/debug/cargo-llvmcov","fresh":true}"#;
    let m = serde_json::from_str(m).unwrap();
    assert_eq!(
        BuildMessage::CompilerArtifact {
            target: BuildTarget { test: true },
            profile: BuildProfile { test: false },
            executable: Some("/home/ysk/work/cargo-llvmcov/target/debug/cargo-llvmcov".into())
        },
        m
    );

    let m = r#"{"reason":"build-script-executed","package_id":"indexmap 1.6.0 (registry+https://github.com/rust-lang/crates.io-index)","linked_libs":[],"linked_paths":[],"cfgs":["has_std"],"env":[],"out_dir":"/home/ysk/work/cargo-llvmcov/target/debug/build/indexmap-df2d449462c3d567/out"}"#;
    let m = serde_json::from_str(m).unwrap();
    assert_eq!(BuildMessage::BuildScriptExecuted {}, m);

    let m = r#"{"reason":"build-finished","success":true}"#;
    let m = serde_json::from_str(m).unwrap();
    assert_eq!(BuildMessage::BuildFinished { success: true }, m);
}

#[test]
fn test_command() {
    begin_capture();
    Command::new("cargo")
        .args(&["--version"])
        .env("RUST_BACKTRACE", "1")
        .stdout(Stdio::null())
        .status()
        .unwrap();

    assert_eq!(pop_captured().unwrap().message(), "CALL cargo --version");

    Command::new("cargo")
        .args(&["--version"])
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
    Command::new("cargo").args(&["--version"]).output().unwrap();
}

#[test]
fn test_cargo() {
    let key = "CARGO";
    let old = env::var(key);
    env::set_var(key, "x");
    assert_eq!(PathBuf::from("x"), cargo());

    env::remove_var(key);
    assert_eq!(PathBuf::from("cargo"), cargo());

    if let Ok(old) = old {
        env::set_var(key, old);
    }
}

#[test]
fn test_cargo_home() {
    let key = "CARGO_HOME";
    let old = env::var(key);
    env::set_var(key, "x");
    assert_eq!("x", cargo_home());

    env::remove_var(key);
    assert_eq!("", cargo_home());

    if let Ok(old) = old {
        env::set_var(key, old);
    }
}

#[test]
fn test_rustup_home() {
    let key = "RUSTUP_HOME";
    let old = env::var(key);
    env::set_var(key, "x");
    assert_eq!("x", rustup_home());

    env::remove_var(key);
    assert_eq!("", rustup_home());

    if let Ok(old) = old {
        env::set_var(key, old);
    }
}

#[test]
fn test_target_dir() {
    MOCK_RESULT.with(|o| o.borrow_mut().replace((br#"{"packages":[{"name":"x","version":"0.1.0","id":"x 0.1.0 (path+file:///tmp/x)","license":null,"license_file":null,"description":null,"source":null,"dependencies":[],"targets":[{"kind":["bin"],"crate_types":["bin"],"name":"x","src_path":"/tmp/x/src/main.rs","edition":"2018","doctest":false,"test":true}],"features":{},"manifest_path":"/tmp/x/Cargo.toml","metadata":null,"publish":null,"authors":["yskszk63 <yskszk63@gmail.com>"],"categories":[],"keywords":[],"readme":null,"repository":null,"edition":"2018","links":null}],"workspace_members":["x 0.1.0 (path+file:///tmp/x)"],"resolve":{"nodes":[{"id":"x 0.1.0 (path+file:///tmp/x)","dependencies":[],"deps":[],"features":[]}],"root":"x 0.1.0 (path+file:///tmp/x)"},"target_directory":"/tmp/x/target","version":1,"workspace_root":"/tmp/x","metadata":null}"#, true)));

    let target_dir = target_dir(&cargo()).unwrap();
    assert_eq!(PathBuf::from("/tmp/x/target"), target_dir);
}

#[test]
fn test_build() {
    MOCK_RESULT.with(|o| o.borrow_mut().replace((br#"{"reason":"compiler-artifact","package_id":"cargo-binutils 0.3.3 (registry+https://github.com/rust-lang/crates.io-index)","target":{"kind":["lib"],"crate_types":["lib"],"name":"cargo-binutils","src_path":"/home/ysk/.cargo/registry/src/github.com-1ecc6299db9ec823/cargo-binutils-0.3.3/src/lib.rs","edition":"2018","doctest":true,"test":true},"profile":{"opt_level":"0","debuginfo":2,"debug_assertions":true,"overflow_checks":true,"test":false},"features":[],"filenames":["/home/ysk/work/cargo-llvmcov/target/debug/deps/libcargo_binutils-2869e11bf8c84ac2.rlib","/home/ysk/work/cargo-llvmcov/target/debug/deps/libcargo_binutils-2869e11bf8c84ac2.rmeta"],"executable":null,"fresh":true}
{"reason":"compiler-artifact","package_id":"cargo-llvmcov 0.1.0 (path+file:///home/ysk/work/cargo-llvmcov)","target":{"kind":["test"],"crate_types":["bin"],"name":"text","src_path":"/home/ysk/work/cargo-llvmcov/tests/text.rs","edition":"2018","doctest":false,"test":true},"profile":{"opt_level":"0","debuginfo":2,"debug_assertions":true,"overflow_checks":true,"test":true},"features":[],"filenames":["/home/ysk/work/cargo-llvmcov/target/debug/deps/text-1ed1826ee82efe68"],"executable":"/home/ysk/work/cargo-llvmcov/target/debug/deps/text-1ed1826ee82efe68","fresh":true}
{"reason":"compiler-artifact","package_id":"cargo-llvmcov 0.1.0 (path+file:///home/ysk/work/cargo-llvmcov)","target":{"kind":["bin"],"crate_types":["bin"],"name":"cargo-llvmcov","src_path":"/home/ysk/work/cargo-llvmcov/src/main.rs","edition":"2018","doctest":false,"test":true},"profile":{"opt_level":"0","debuginfo":2,"debug_assertions":true,"overflow_checks":true,"test":true},"features":[],"filenames":["/home/ysk/work/cargo-llvmcov/target/debug/deps/cargo_llvmcov-24ed17e95a11ece8"],"executable":"/home/ysk/work/cargo-llvmcov/target/debug/deps/cargo_llvmcov-24ed17e95a11ece8","fresh":false}
{"reason":"compiler-artifact","package_id":"cargo-llvmcov 0.1.0 (path+file:///home/ysk/work/cargo-llvmcov)","target":{"kind":["bin"],"crate_types":["bin"],"name":"cargo-llvmcov","src_path":"/home/ysk/work/cargo-llvmcov/src/main.rs","edition":"2018","doctest":false,"test":true},"profile":{"opt_level":"0","debuginfo":2,"debug_assertions":true,"overflow_checks":true,"test":false},"features":[],"filenames":["/home/ysk/work/cargo-llvmcov/target/debug/cargo-llvmcov"],"executable":"/home/ysk/work/cargo-llvmcov/target/debug/cargo-llvmcov","fresh":false}
{"reason":"build-finished","success":true}
"#, true)));

    let tmpdir = mktemp::Temp::new_dir().unwrap();
    let profenv = Profenv::new(&tmpdir).unwrap();

    begin_capture();
    build(&PathBuf::from("cargo"), &PathBuf::from("target"), &profenv).unwrap();
    assert_eq!(
        pop_captured().unwrap().message(),
        "CALL cargo build --message-format json --tests --target-dir target"
    );
}

#[test]
fn test_build_failed() {
    MOCK_RESULT.with(|o| o.borrow_mut().replace((br#""#, false)));

    let tmpdir = mktemp::Temp::new_dir().unwrap();
    let profenv = Profenv::new(&tmpdir).unwrap();

    let r = build(&PathBuf::from("cargo"), &PathBuf::from("target"), &profenv).unwrap_err();
    assert_eq!(&r.to_string(), "failed to run cargo build.");
}

#[test]
fn test_build_no_executable_found() {
    MOCK_RESULT.with(|o| o.borrow_mut().replace((br#"{"reason":"compiler-artifact","package_id":"cargo-binutils 0.3.3 (registry+https://github.com/rust-lang/crates.io-index)","target":{"kind":["lib"],"crate_types":["lib"],"name":"cargo-binutils","src_path":"/home/ysk/.cargo/registry/src/github.com-1ecc6299db9ec823/cargo-binutils-0.3.3/src/lib.rs","edition":"2018","doctest":true,"test":true},"profile":{"opt_level":"0","debuginfo":2,"debug_assertions":true,"overflow_checks":true,"test":false},"features":[],"filenames":["/home/ysk/work/cargo-llvmcov/target/debug/deps/libcargo_binutils-2869e11bf8c84ac2.rlib","/home/ysk/work/cargo-llvmcov/target/debug/deps/libcargo_binutils-2869e11bf8c84ac2.rmeta"],"executable":null,"fresh":true}
{"reason":"build-finished","success":true}
"#, true)));

    let tmpdir = mktemp::Temp::new_dir().unwrap();
    let profenv = Profenv::new(&tmpdir).unwrap();

    let r = build(&PathBuf::from("cargo"), &PathBuf::from("target"), &profenv).unwrap_err();
    assert_eq!(&r.to_string(), "no executable found.");
}

#[test]
fn test_run_test() {
    MOCK_RESULT.with(|o| o.borrow_mut().replace((br#""#, true)));

    let tmpdir = mktemp::Temp::new_dir().unwrap();
    let profenv = Profenv::new(&tmpdir).unwrap();

    begin_capture();
    run_test(&PathBuf::from("program"), &profenv).unwrap();
    assert_eq!(
        pop_captured().unwrap().message(),
        "CALL program --nocapture"
    );
}

#[test]
fn test_run_test_failed() {
    MOCK_RESULT.with(|o| o.borrow_mut().replace((br#""#, false)));

    let tmpdir = mktemp::Temp::new_dir().unwrap();
    let profenv = Profenv::new(&tmpdir).unwrap();

    let r = run_test(&PathBuf::from("program"), &profenv).unwrap_err();
    assert_eq!(&r.to_string(), "failed to run executable.");
}

#[test]
fn test_merge_profdata() {
    MOCK_RESULT.with(|o| o.borrow_mut().replace((br#""#, true)));

    let tmpdir = mktemp::Temp::new_dir().unwrap();
    let profenv = Profenv::new(&tmpdir).unwrap();

    begin_capture();
    merge_profdata(&PathBuf::from("llvm-profdata"), &profenv).unwrap();
    assert_eq!(
        pop_captured().unwrap().message(),
        format!(
            "CALL llvm-profdata merge -sparse -o {}",
            tmpdir.join("default.profdata").to_string_lossy()
        ),
    );
}

#[test]
fn test_merge_profdata_failed() {
    MOCK_RESULT.with(|o| o.borrow_mut().replace((br#""#, false)));

    let tmpdir = mktemp::Temp::new_dir().unwrap();
    let profenv = Profenv::new(&tmpdir).unwrap();

    let r = merge_profdata(&PathBuf::from("llvm-profdata"), &profenv).unwrap_err();
    assert_eq!(&r.to_string(), "failed to run llvm-profdata.");
}

#[test]
fn test_to_obj_args() {
    let r = to_obj_args(&[]);
    assert_eq!(r, &[] as &[PathBuf]);

    let v = vec![PathBuf::from("a")];
    let r = to_obj_args(&v);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0], v[0]);

    let v = vec![PathBuf::from("a"), PathBuf::from("b")];
    let r = to_obj_args(&v);
    assert_eq!(r.len(), 3);
    assert_eq!(r[0], v[0]);
    assert_eq!(r[1].to_str().unwrap(), "-object");
    assert_eq!(r[2], v[1]);
}

#[test]
fn test_llvm_cov_show() {
    MOCK_RESULT.with(|o| o.borrow_mut().replace((br#""#, true)));

    let tmpdir = mktemp::Temp::new_dir().unwrap();
    let profenv = Profenv::new(&tmpdir).unwrap();

    begin_capture();
    llvm_cov_show(
        &PathBuf::from("llvm-cov"),
        &PathBuf::from("rustfilt"),
        &profenv,
        &[PathBuf::from("exe")],
        None,
        "ignore",
    )
    .unwrap();
    assert_eq!(
        pop_captured().unwrap().message(),
        format!("CALL llvm-cov show -Xdemangler=rustfilt exe -instr-profile={} -format=text -ignore-filename-regex=ignore -show-instantiations=false", tmpdir.join("default.profdata").to_string_lossy()));
}

#[test]
fn test_llvm_cov_show_html() {
    MOCK_RESULT.with(|o| o.borrow_mut().replace((br#""#, true)));

    let tmpdir = mktemp::Temp::new_dir().unwrap();
    let profenv = Profenv::new(&tmpdir).unwrap();

    begin_capture();
    llvm_cov_show(
        &PathBuf::from("llvm-cov"),
        &PathBuf::from("rustfilt"),
        &profenv,
        &[PathBuf::from("exe")],
        Some(&PathBuf::from("output")),
        "ignore",
    )
    .unwrap();
    assert_eq!(pop_captured().unwrap().message(),
        format!("CALL llvm-cov show -Xdemangler=rustfilt exe -instr-profile={} -format=html -output-dir=output -ignore-filename-regex=ignore -show-instantiations=false", tmpdir.join("default.profdata").to_string_lossy()));
}

#[test]
fn test_llvm_cov_show_failed() {
    MOCK_RESULT.with(|o| o.borrow_mut().replace((br#""#, false)));

    let tmpdir = mktemp::Temp::new_dir().unwrap();
    let profenv = Profenv::new(&tmpdir).unwrap();

    let r = llvm_cov_show(
        &PathBuf::from("llvm-cov"),
        &PathBuf::from("rustfilt"),
        &profenv,
        &[PathBuf::from("exe")],
        None,
        "ignore",
    )
    .unwrap_err();
    assert_eq!(&r.to_string(), "failed to run llvm-cov.");
}

#[test]
fn test_llvm_cov_export() {
    let output = mktemp::Temp::new_file().unwrap();

    MOCK_RESULT.with(|o| o.borrow_mut().replace((br#""#, true)));

    let tmpdir = mktemp::Temp::new_dir().unwrap();
    let profenv = Profenv::new(&tmpdir).unwrap();

    begin_capture();
    llvm_cov_export(
        &PathBuf::from("llvm-cov"),
        &PathBuf::from("rustfilt"),
        &profenv,
        &[PathBuf::from("exe")],
        &output,
        "ignore",
    )
    .unwrap();
    assert_eq!(pop_captured().unwrap().message(),
        format!("CALL llvm-cov export -Xdemangler=rustfilt exe -instr-profile={} -format=lcov -ignore-filename-regex=ignore -show-instantiations=false", tmpdir.join("default.profdata").to_string_lossy()));
}

#[test]
fn test_llvm_cov_export_failed() {
    let output = mktemp::Temp::new_file().unwrap();

    MOCK_RESULT.with(|o| o.borrow_mut().replace((br#""#, false)));

    let tmpdir = mktemp::Temp::new_dir().unwrap();
    let profenv = Profenv::new(&tmpdir).unwrap();

    let r = llvm_cov_export(
        &PathBuf::from("llvm-cov"),
        &PathBuf::from("rustfilt"),
        &profenv,
        &[PathBuf::from("exe")],
        &output,
        "ignore",
    )
    .unwrap_err();
    assert_eq!(&r.to_string(), "failed to run llvm-cov.");
}
