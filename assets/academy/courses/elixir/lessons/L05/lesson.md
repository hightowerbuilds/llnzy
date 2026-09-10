+++
title = "Parsing at the Boundary"
concepts = ["with", "Integer.parse", "validation", "error tuples"]

[[exercise]]
prompt = "Implement Input.quantity/1: trim a string, accept only a fully parsed integer greater than zero, and return {:ok, n}; otherwise return {:error, :invalid_quantity}. Non-string inputs also return that error."
[exercise.check]
command = ["elixir", "check.exs"]
expected = "0"
mode = "exit_code"
timeout_secs = 30

[[exercise.files]]
path = "lesson.exs"
starter = '''
defmodule Input do
  def quantity(_value), do: {:error, :invalid_quantity}
end
'''
solution = '''
defmodule Input do
  def quantity(value) when is_binary(value) do
    with {number, ""} <- Integer.parse(String.trim(value)),
         true <- number > 0 do
      {:ok, number}
    else
      _ -> {:error, :invalid_quantity}
    end
  end
  def quantity(_value), do: {:error, :invalid_quantity}
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
    assert Input.quantity(" 12 ") == {:ok, 12}
    assert Input.quantity("+3") == {:ok, 3}
    for value <- ["0", "-1", "12x", "1.5", "", "abc", nil, 12] do
      assert Input.quantity(value) == {:error, :invalid_quantity}
    end
  end
end
'''
solution = '''
Code.require_file("lesson.exs", __DIR__)
ExUnit.start()
defmodule LessonTest do
  use ExUnit.Case
  test "public contract" do
    assert Input.quantity(" 12 ") == {:ok, 12}
    assert Input.quantity("+3") == {:ok, 3}
    for value <- ["0", "-1", "12x", "1.5", "", "abc", nil, 12] do
      assert Input.quantity(value) == {:error, :invalid_quantity}
    end
  end
end
'''
+++
# Parsing at the Boundary

External text needs validation before it becomes trusted domain data. `Integer.parse/1` returns either `{integer, remaining_text}` or `:error`. Parsing a numeric prefix is useful in some programs, but our quantity contract requires the entire trimmed string to be an integer.

```elixir
Integer.parse("12")  # {12, ""}
Integer.parse("12x") # {12, "x"}
Integer.parse("x12") # :error
```

`with` connects steps that must match expected shapes. In `pattern <- expression`, a match continues to the next step; a mismatch stops the chain. An `else` block can translate mismatches into the public error format. Without else, the unmatched value becomes the result. This is useful for short validation sequences without deeply nested cases.

For the exercise, first guard on a binary (Elixir strings are UTF-8 binaries). Trim it, require `{number, ""}`, then require a positive number. Return the number wrapped in `{:ok, number}`. All expected validation failures share one error atom, keeping the caller's decision simple. This policy accepts a leading plus sign because Integer.parse accepts it.

Do not rescue every exception to make the check pass. Bad user input is expected and can be handled explicitly; unrelated programming mistakes should stay visible. After passing, consider a different product policy that distinguishes malformed text from nonpositive values. Which patterns would you preserve rather than folding into one error?

## Practice

Implement Input.quantity/1: trim a string, accept only a fully parsed integer greater than zero, and return {:ok, n}; otherwise return {:error, :invalid_quantity}. Non-string inputs also return that error.

Export from the repository root with `python3 scripts/check_academy_courses.py --export elixir L05 /tmp/llnzy-elixir-l05` (Python 3.11+). Choose a destination that does not already exist. Open the exported files in LLNZY and run `elixir check.exs` from that directory. The starter intentionally fails. Leave check.exs unchanged and implement lesson.exs. A successful check exits with status zero; read assertion failures to find the input that needs attention.
