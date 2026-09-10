+++
title = "Toolchain Check"
chapter = 1
concepts = ["rustup and the stable toolchain", "rustc vs cargo", "why checks never assert on version text", "a first compiling program"]

[[exercise]]
prompt = "First run rustc --version and cargo --version in your terminal (free play; nothing asserts on their output). Then edit src/main.rs so the program prints the single line toolchain ready."
[exercise.check]
command = ["cargo", "run"]
expected = "toolchain ready"
mode = "contains"
timeout_secs = 120
[[exercise.files]]
path = "Cargo.toml"
starter = """[package]
name = "lesson"
version = "0.1.0"
edition = "2021"

[dependencies]
"""
solution = """[package]
name = "lesson"
version = "0.1.0"
edition = "2021"

[dependencies]
"""
[[exercise.files]]
path = "src/main.rs"
starter = """fn main() {
    // This program should print one line once the toolchain is working.
}
"""
solution = """fn main() {
    println!("toolchain ready");
}
"""
+++
# Toolchain Check

This lesson pairs with chapter 1 of *The Rust Programming Language* (3rd ed.), the getting-started chapter.

## What rustup owns

Rust installs through `rustup`, which owns a toolchain directory and can hold several at once. Ask it which one is active:

```bash
rustup show
```

Underneath sit two programs: `rustc`, the compiler, and `cargo`, the build tool and package manager that almost everyone drives instead.

## Check your versions first

Versions are the first thing to check when something behaves oddly, which is why the prompt asks you to run these yourself:

```bash
rustc --version
cargo --version
```

We deliberately never assert on that text in a check. Version strings drift across machines and releases, and graded output must stay deterministic.

When your program prints a fixed line instead, the toolchain is proven.

## Run it

```bash
cargo run
```

Try the same thing in your own terminal and watch the line appear there too.
