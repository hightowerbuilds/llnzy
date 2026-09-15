+++
title = "Scalar Types & Control Flow"
chapter = 3
concepts = ["integer and float types", "bool and char", "if as an expression", "loop, while, and for over ranges"]

[[exercise]]
prompt = "Implement the loop body so the program prints each number from 1 to 15, but prints rust for multiples of 3, ace for multiples of 5, and rustace when both divide evenly. Numbers divisible by neither print themselves. Check the combined case first, or 15 will print the wrong word."
[exercise.check]
command = ["cargo", "run"]
expected = "1\n2\nrust\n4\nace\nrust\n7\n8\nrust\nace\n11\nrust\n13\n14\nrustace\n"
mode = "exact"
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
    for n in 1..=15 {
        // n % 3 and n % 5 decide which of the four lines to print.
    }
}
"""
solution = """fn main() {
    for n in 1..=15 {
        if n % 3 == 0 && n % 5 == 0 {
            println!("rustace");
        } else if n % 3 == 0 {
            println!("rust");
        } else if n % 5 == 0 {
            println!("ace");
        } else {
            println!("{}", n);
        }
    }
}
"""
+++
# Scalar Types & Control Flow

Still chapter 3, now the scalar types.

## The scalars

Integers are sized and signed per name: `i32`, `u8`, `u64`. Then two float types, `bool`, and `char`, which is a four-byte Unicode scalar rather than a byte.

## Control flow is deliberately plain

`if` does not require parentheses around its condition and is an expression: you can assign its result when its arms have compatible types.

Loops come in three shapes: bare `loop` until you `break`, `while` on a condition, and `for` over an iterator such as `1..=15`. Iterating directly over values avoids the indexing mistakes a manually managed index can introduce.

The classic classroom task here is divisibility printing, and arm order is the whole exercise.

## Practice

Choose **Open practice** in the exercise card to create or reopen this lesson’s files. Edit the implementation, save your changes, then choose **Check work**. Your practice folder is reused when you return; opening it again keeps your edits. No source checkout or Python is needed.

You can also run this command in the practice folder’s terminal:

```bash
cargo run
```

Extend the range to 30 and confirm your arm ordering holds beyond the first coincidence.

## Hint before a solution

Trace 3, 5, and 15 separately. A branch for multiples of 3 alone would capture 15 before the combined case.

Try one focused change and check again. Before comparing with a reference solution, explain the failing case in your own words. A passing check covers the supplied examples; also try a new input and explain why your implementation handles it.
