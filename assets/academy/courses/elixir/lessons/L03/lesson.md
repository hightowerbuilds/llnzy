+++
title = "Function Clauses, Guards & Recursion"
concepts = ["function clauses", "guards", "list patterns", "recursion"]

[[exercise]]
prompt = "Implement Totals.sum_nonnegative/1 for lists of integers: sum values greater than or equal to zero, skipping negatives. Use an empty-list base case and recursive head/tail clauses."
[exercise.check]
command = ["elixir", "check.exs"]
expected = "0"
mode = "exit_code"
timeout_secs = 30

[[exercise.files]]
path = "lesson.exs"
starter = '''
defmodule Totals do
  def sum_nonnegative(_values), do: 0
end
'''
solution = '''
defmodule Totals do
  def sum_nonnegative([]), do: 0
  def sum_nonnegative([head | tail]) when head >= 0 do
    head + sum_nonnegative(tail)
  end
  def sum_nonnegative([_head | tail]), do: sum_nonnegative(tail)
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
    assert Totals.sum_nonnegative([]) == 0
    assert Totals.sum_nonnegative([3, -2, 0, 7]) == 10
    assert Totals.sum_nonnegative([-9, -1]) == 0
    assert Totals.sum_nonnegative([5]) == 5
  end
end
'''
solution = '''
Code.require_file("lesson.exs", __DIR__)
ExUnit.start()
defmodule LessonTest do
  use ExUnit.Case
  test "public contract" do
    assert Totals.sum_nonnegative([]) == 0
    assert Totals.sum_nonnegative([3, -2, 0, 7]) == 10
    assert Totals.sum_nonnegative([-9, -1]) == 0
    assert Totals.sum_nonnegative([5]) == 5
  end
end
'''
+++
# Function Clauses, Guards & Recursion

A named function can have multiple clauses. Elixir selects the first clause whose argument patterns and guard match. Keep clauses of the same function together, with specific cases before broad fallbacks. Guards are restricted expressions such as type checks and comparisons, not places to call arbitrary application functions.

Lists are linked structures. `[head | tail]` separates the first item from the remaining list; `[]` is the empty list. Recursion handles one item and asks the same function to handle the smaller remainder. A base case prevents the recursion from continuing forever.

```elixir
defmodule Count do
  def size([]), do: 0
  def size([_head | tail]), do: 1 + size(tail)
end
Count.size([:a, :b]) # 2
```

Trace that example: two nonempty clauses lead to the empty clause returning zero, then the pending additions produce two. The exercise adds a guard to distinguish included values from skipped values. Every recursive branch must pass the tail, so the input gets smaller.

This direct style leaves pending additions. An accumulator can express a tail-recursive version, and Enum offers standard collection operations so you usually do not need a custom traversal. We write one here to understand the mechanism. After passing, trace `[2, -1, 4]` by hand, naming the selected clause and remaining list at every call.

## Practice

Implement Totals.sum_nonnegative/1 for lists of integers: sum values greater than or equal to zero, skipping negatives. Use an empty-list base case and recursive head/tail clauses.

Export from the repository root with `python3 scripts/check_academy_courses.py --export elixir L03 /tmp/llnzy-elixir-l03` (Python 3.11+). Choose a destination that does not already exist. Open the exported files in LLNZY and run `elixir check.exs` from that directory. The starter intentionally fails. Leave check.exs unchanged and implement lesson.exs. A successful check exits with status zero; read assertion failures to find the input that needs attention.
