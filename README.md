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
cargo-llvmcov

USAGE:
    llvmcov [FLAGS] [OPTIONS]

FLAGS:
        --help       Prints help information
    -h, --html
    -k, --keep
    -l, --lcov
    -o, --open
    -v, --verbose
    -V, --version    Prints version information

OPTIONS:
    -L, --lcov-output <lcov-output>
```

License
-------

Apache-2.0/MIT
