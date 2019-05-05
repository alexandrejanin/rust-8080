# rust-8080
A WIP Intel 8080 emulator using Rust.

Not feature-complete yet, but can run space invaders.

Build with feature `logging` to enable step-by-step logging (very slow, should be built in release mode).

Build with feature `cpu_compare` to use a [modified version](https://github.com/alexandrejanin/i8080) of [i8080](https://github.com/XAMPPRocky/i8080) as a CPU reference, panicking on register/flag mismatch.
