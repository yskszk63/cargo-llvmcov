cargo-llvmcov
=============

A utility for report LLVM Source-based code coverage.

Example
-------

```
cargo +nightly llvmcov --html --open
```

Installation
------------

```
rustup component add --toolchain nightly llvm-tools-preview
cargo install rustfilt
cargo install --git https://github.com/yskszk63/cargo-llvmcov --branch main cargo-llvmcov
```

Usage
-----

```
cargo-llvmcov 0.1.0
A utility for report LLVM Source-based code coverage

USAGE:
    cargo llvmcov [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -H, --html       Generate HTML report
    -k, --keep       Keep default.profdata & *.profraw
    -l, --lcov       Generate lcov report
    -o, --open       Open HTML report when done
    -v, --verbose    Verbose output
    -V, --version    Prints version information

OPTIONS:
    -L, --lcov-output <lcov-output>    Lcov output file name
```

License
-------

Apache-2.0/MIT
