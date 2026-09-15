+++
title = "Pattern Matching & Tagged Results"
concepts = ["pattern matching", "tuples", "case", "pin operator"]

[[exercise]]
prompt = "Implement Result.label/1: {:ok, name} becomes \"Welcome, NAME\"; {:error, reason} becomes \"Error: REASON\" (reason is a string); all other values become \"Unknown result\"."
[exercise.check]
command = ["elixir", "check.exs"]
expected = "0"
mode = "exit_code"
timeout_secs = 30

[[exercise.files]]
path = "lesson.exs"
starter = '''
defmodule Result do
  def label(_result), do: "TODO"
end
'''
solution = '''
defmodule Result do
  def label(result) do
    case result do
      {:ok, name} -> "Welcome, #{name}"
      {:error, reason} -> "Error: #{reason}"
      _ -> "Unknown result"
    end
  end
end
'''

[[exercise.files]]
path = "check.exs"
starter = '''
Code.require_file("lesson.exs", __DIR__)
ExUnit.start()
defmodule LessonTest do
  use ExUnit.Case
  test "public contract" do
    assert Result.label({:ok, "Ada"}) == "Welcome, Ada"
    assert Result.label({:error, "offline"}) == "Error: offline"
    assert Result.label(:ok) == "Unknown result"
    assert Result.label({:ok, "Ada", 3}) == "Unknown result"
  end
end
'''
solution = '''
Code.require_file("lesson.exs", __DIR__)
ExUnit.start()
defmodule LessonTest do
  use ExUnit.Case
  test "public contract" do
    assert Result.label({:ok, "Ada"}) == "Welcome, Ada"
    assert Result.label({:error, "offline"}) == "Error: offline"
    assert Result.label(:ok) == "Unknown result"
    assert Result.label({:ok, "Ada", 3}) == "Unknown result"
  end
end
'''
+++
# Pattern Matching & Tagged Results

The equals sign matches a pattern against a value. Matching can bind variables and require a particular shape at the same time. Tuples collect a fixed number of values, which makes `{:ok, value}` and `{:error, reason}` useful result formats: callers can tell success from failure without guessing what the payload means.

```elixir
{:ok, name} = {:ok, "Ada"}
name # "Ada"
expected = :ok
{^expected, value} = {:ok, 42}
```

The pin operator `^` requires a variable's existing value instead of rebinding it. A failed match with `=` raises MatchError. Use `case` when multiple shapes are expected: clauses are tried in order, and the first match supplies the result. `_` matches anything without binding a usable variable.

```elixir
case {:error, "offline"} do
  {:ok, value} -> value
  {:error, message} -> "Try again: #{message}"
end
```

A tuple's size is part of its shape. `{:ok, value}` does not match `{:ok, value, metadata}`. Put your catch-all clause last so specific clauses can run. In this lesson errors are ordinary data; the caller chooses how to display them. Later lessons preserve these tags across parsing steps and process calls.

After the exercise, explain why an unconditional match on `{:ok, value}` would be inappropriate for an operation that can fail.

## Practice

Implement Result.label/1: {:ok, name} becomes "Welcome, NAME"; {:error, reason} becomes "Error: REASON" (reason is a string); all other values become "Unknown result".

Choose **Open practice** in the exercise card to create or reopen this lesson’s files. Edit the implementation, save your changes, then choose **Check work**. Your practice folder is reused when you return; opening it again keeps your edits. No source checkout or Python is needed.

The command shown in the exercise card runs from that folder. The starter intentionally fails. Leave check.exs unchanged and implement lesson.exs. A successful check exits with status zero; read assertion failures to find the input that needs attention.

## Hint before a solution

Match the tuple tag and size before using its payload. Where must the catch-all clause go?

Try one focused change and check again. Before comparing with a reference solution, explain the failing case in your own words. A passing check covers the supplied examples; also try a new input and explain why your implementation handles it.
