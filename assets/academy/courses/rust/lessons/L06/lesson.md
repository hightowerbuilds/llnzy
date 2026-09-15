+++
title = "Functions & Compound Types Intro"
chapter = 3
concepts = ["parameters and return types", "expression bodies without semicolons", "tuples and destructuring", "fixed-size arrays"]

[[exercise]]
prompt = "Two one-line fixes. area should return the product of its arguments, not their sum, using an expression body — no semicolon, no return keyword. swap should return the tuple with its elements reversed, so flipped prints (9, 3)."
[exercise.check]
command = ["cargo", "run"]
expected = """area: 24
flipped: (9, 3)
days summed: 15"""
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
starter = """fn area(width: u32, height: u32) -> u32 {
    width + height
}

fn swap(pair: (i32, i32)) -> (i32, i32) {
    pair
}

fn main() {
    println!("area: {}", area(6, 4));

    let point = (3, 9);
    let flipped = swap(point);
    println!("flipped: ({}, {})", flipped.0, flipped.1);

    let week = [1, 2, 3, 4, 5];
    let mut sum = 0;
    for day in week {
        sum += day;
    }
    println!("days summed: {}", sum);
}
"""
solution = """fn area(width: u32, height: u32) -> u32 {
    width * height
}

fn swap(pair: (i32, i32)) -> (i32, i32) {
    (pair.1, pair.0)
}

fn main() {
    println!("area: {}", area(6, 4));

    let point = (3, 9);
    let flipped = swap(point);
    println!("flipped: ({}, {})", flipped.0, flipped.1);

    let week = [1, 2, 3, 4, 5];
    let mut sum = 0;
    for day in week {
        sum += day;
    }
    println!("days summed: {}", sum);
}
"""
+++
# Functions & Compound Types Intro

Chapter 3 closes with functions and its compound types.

## Functions return their last expression

Every parameter is typed, and the return type sits after `->`.

A body that ends in an expression *without* a semicolon is the value. That is why `width * height` returns, while `width * height;` would not.

## Tuples and arrays

Tuples group mixed types of fixed arity. Reach elements by position like `pair.1`, or destructure with `let (a, b) = pair;`.

Arrays hold one type with a length fixed at compile time. Both have fixed size; where they are stored depends on how the surrounding program uses them.

You have reached the end of the seven shipped Rust lessons. Continue with ownership and borrowing in chapter 4 of the book, then structs in chapter 5. Those topics and a Rust capstone are not yet part of this app’s course.

## Practice

Choose **Open practice** in the exercise card to create or reopen this lesson’s files. Edit the implementation, save your changes, then choose **Check work**. Your practice folder is reused when you return; opening it again keeps your edits. No source checkout or Python is needed.

You can also run this command in the practice folder’s terminal:

```bash
cargo run
```

Rewrite swap to destructure instead of indexing, then add a three-element tuple to see where fixed arity starts to chafe.

## Hint before a solution

The last expression supplies a function’s return value. A tuple’s numbered fields let you build a new tuple in a different order.

Try one focused change and check again. Before comparing with a reference solution, explain the failing case in your own words. A passing check covers the supplied examples; also try a new input and explain why your implementation handles it.
