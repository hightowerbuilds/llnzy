+++
title = "Processing a Guess"
chapter = 2
concepts = ["stdin and read_line returning Result", "trim before parse", "parse::<u32>() with match", "a pure helper for deterministic checks"]

[[exercise]]
prompt = "The book reads a guess with io::stdin().read_line, which needs a human typing. To keep the check deterministic we factor the logic into a pure function instead. Implement process_guess(raw) so it returns You guessed: N for input that trims to a valid u32, and Please type a number! for anything else (including numbers too large for u32). Keep the three demo calls in main as they are."
[exercise.check]
command = ["cargo", "run"]
expected = """You guessed: 42
Please type a number!
Please type a number!"""
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
starter = """fn process_guess(raw: &str) -> String {
    // Echoing the raw text is not good enough: it must be trimmed
    // and parsed as a u32 before we can trust it.
    format!("You guessed: {}", raw)
}

fn main() {
    for raw in ["  42\\n", "banana", "9999999999"] {
        println!("{}", process_guess(raw));
    }
}
"""
solution = """fn process_guess(raw: &str) -> String {
    match raw.trim().parse::<u32>() {
        Ok(number) => format!("You guessed: {}", number),
        Err(_) => String::from("Please type a number!"),
    }
}

fn main() {
    for raw in ["  42\\n", "banana", "9999999999"] {
        println!("{}", process_guess(raw));
    }
}
"""
+++
# Processing a Guess

Chapter 2 builds a guessing game, and its first lesson is that input is untrusted text.

## Reading a line

The book calls `io::stdin().read_line(&mut guess)`, which returns a `Result` because reading can fail, then leans on `.expect()` for now.

A line read normally includes its trailing newline (the final line at end-of-file may not). So `trim()` comes before `parse::<u32>()` — and `parse` itself returns a `Result`, because "banana" and a number past `u32`'s range both land in the error arm.

## Why this lesson looks different from the book

Graded checks cannot pipe stdin. So we pull the logic out into a pure function that takes `&str` and returns `String`, and `main` just demonstrates it on fixed samples.

That refactor — isolate the decision, print at the edge — is a habit worth keeping well past this lesson.

## Practice

Choose **Open practice** in the exercise card to create or reopen this lesson’s files. Edit the implementation, save your changes, then choose **Check work**. Your practice folder is reused when you return; opening it again keeps your edits. No source checkout or Python is needed.

You can also run this command in the practice folder’s terminal:

```bash
cargo run
```

Add your own demo strings to the loop and watch which arm they hit.

## Hint before a solution

Trim first, then match the Result returned by parse::<u32>(). Which branch handles both text and an out-of-range integer?

Try one focused change and check again. Before comparing with a reference solution, explain the failing case in your own words. A passing check covers the supplied examples; also try a new input and explain why your implementation handles it.
