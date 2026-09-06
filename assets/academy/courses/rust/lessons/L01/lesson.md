+++
title = "Hello, Cargo"
chapter = 1
concepts = ["cargo new conventions", "Cargo.toml anatomy", "cargo run vs cargo build", "src/main.rs as the crate root"]

[[exercise]]
prompt = "Change src/main.rs so the program prints Hello, Cargo! exactly. Leave Cargo.toml alone — it is already in the shape cargo new produces."
[exercise.check]
command = ["cargo", "run"]
expected = "Hello, Cargo!"
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
}
"""
solution = """fn main() {
    println!("Hello, Cargo!");
}
"""
+++
# Hello, Cargo

Still chapter 1, now on the project conventions you will use for the whole course. `cargo new` scaffolds a directory with a manifest and one source file: `Cargo.toml` at the root describing the package, and `src/main.rs` as the binary's entry point. The manifest's `[package]` table carries the name, version, and the language edition; `[dependencies]` starts empty and stays empty here, because this course is std-only and no check ever touches the network. Two commands matter today. `cargo run` compiles and then executes in one step; `cargo build` only compiles, leaving a binary under `target/debug/` that you can run directly. We pin `edition = "2021"` in every starter so a lesson behaves the same regardless of what your local cargo defaults to.

In the terminal, run `cargo build`, then execute `./target/debug/lesson` yourself and compare the two workflows.
