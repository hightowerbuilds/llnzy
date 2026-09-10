+++
title = "The Secret Number"
chapter = 2
concepts = ["cmp and Ordering", "match arms with break", "a deterministic LCG instead of rand", "loop control in a game"]

[[exercise]]
prompt = "Implement the round loop. For each guess, generate a secret with next_secret, print secret: N, then match guess.cmp(&secret): print N is too small on Less, N is too big on Greater, and You win! N was right on Equal, breaking out of the loop. You will need to import Ordering."
[exercise.check]
command = ["cargo", "run"]
expected = """secret: 79
50 is too small
secret: 32
You win! 32 was right"""
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
starter = """fn next_secret(state: &mut u64) -> u32 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*state >> 33) as u32) % 100 + 1
}

fn main() {
    let mut state: u64 = 7;
    let guesses = [50, 32, 12];

    for guess in guesses {
        let secret = next_secret(&mut state);
        // Compare guess against secret and print the outcome.
    }
}
"""
solution = """use std::cmp::Ordering;

fn next_secret(state: &mut u64) -> u32 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*state >> 33) as u32) % 100 + 1
}

fn main() {
    let mut state: u64 = 7;
    let guesses = [50, 32, 12];

    for guess in guesses {
        let secret = next_secret(&mut state);
        println!("secret: {}", secret);
        match guess.cmp(&secret) {
            Ordering::Less => println!("{} is too small", guess),
            Ordering::Greater => println!("{} is too big", guess),
            Ordering::Equal => {
                println!("You win! {} was right", guess);
                break;
            }
        }
    }
}
"""
+++
# The Secret Number

Chapter 2's game compares a guess against a secret with `guess.cmp(&secret)` and matches on `Ordering` — `Less`, `Greater`, `Equal` — breaking out when the guess lands.

## No rand crate here

The book reaches for the `rand` crate at this point, which is right for real projects. This course stays std-only and offline, so `next_secret` is a tiny linear congruential generator: multiply, add, shift, wrap.

Seeded at 7 it always produces 79, then 32. Deterministic — which is exactly what a graded check needs, and exactly what a real game must avoid.

## Two mutable threads at once

Notice what runs together here: `state` mutates through the generator, while the loop itself stays immutable in its inputs.

## Run it

```bash
cargo run
```

Change the seed or the guesses, and predict the output before you run it.
