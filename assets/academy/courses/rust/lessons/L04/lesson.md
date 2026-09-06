+++
title = "Variables & Mutability"
chapter = 3
concepts = ["let vs let mut", "shadowing a binding", "const vs immutable let", "compiler-enforced mutation discipline"]

[[exercise]]
prompt = "Two fixes in src/main.rs. First, print hours: 35 by shadowing hours with a new let that multiplies the old value by the constant — the binding stays immutable. Second, print total: 6 by making exactly one change to the total declaration so the loop body can add to it."
[exercise.check]
command = ["cargo", "run"]
expected = """hours: 35
total: 6"""
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
starter = """const DAYS_PER_WEEK: u32 = 7;

fn main() {
    let hours = 5;
    println!("hours: {}", hours);

    let total: u32 = 0;
    for day in 1..=3 {
        // total is immutable, so this addition is refused.
        // The fix is one keyword on the declaration above.
        // total += day;
    }
    println!("total: {}", total);
}
"""
solution = """const DAYS_PER_WEEK: u32 = 7;

fn main() {
    let hours = 5;
    let hours = hours * DAYS_PER_WEEK;
    println!("hours: {}", hours);

    let mut total: u32 = 0;
    for day in 1..=3 {
        total += day;
    }
    println!("total: {}", total);
}
"""
+++
# Variables & Mutability

Chapter 3 opens with the rule that surprises everyone: `let` bindings are immutable by default, and opting in costs one word — `let mut`. That default is load-bearing, because a reader can trust that a value named without `mut` never changes under them. Shadowing is the other move: `let hours = hours * 7;` builds a *new* binding that can even change type, while the old one quietly goes out of scope — different from mutation, which reuses storage and must be declared. `const` is a third thing again: named in `SCREAMING_SNAKE_CASE`, typed explicitly, set at compile time, and never `mut`.

In the terminal, drop the `mut` fix back in and read the exact compiler wording — that message will be a colleague for years.
