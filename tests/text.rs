use std::env;
use std::ffi::OsString;
use std::iter;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn test() {
    run(&["-vvv"]);
}

fn run(args: &[&str]) {
    let r = Command::new(cargo())
        .arg("llvmcov")
        .args(args)
        .current_dir("tests/example")
        .env("PATH", path())
        .status()
        .unwrap();
    assert!(r.success());
}

fn cargo() -> PathBuf {
    let cargo = env::var("CARGO").unwrap_or("cargo".to_owned());
    cargo.into()
}

fn path() -> OsString {
    let target = PathBuf::from(env!("CARGO_BIN_EXE_cargo-llvmcov"));
    let target = target.parent().unwrap().to_path_buf();

    let path = env::var("PATH").unwrap_or_default();
    let path = env::split_paths(&path);

    env::join_paths(iter::once(target).chain(path)).unwrap()
}
