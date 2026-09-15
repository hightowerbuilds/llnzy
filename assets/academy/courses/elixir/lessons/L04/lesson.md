+++
title = "Enum & the Pipe Operator"
concepts = ["Enum", "pipes", "anonymous functions", "capture syntax"]

[[exercise]]
prompt = "Implement Pipeline.labels/1: take maps with :name and :active, keep active == true, trim and uppercase names, discard blank names, remove duplicates, and sort ascending."
[exercise.check]
command = ["elixir", "check.exs"]
expected = "0"
mode = "exit_code"
timeout_secs = 30

[[exercise.files]]
path = "lesson.exs"
starter = '''
defmodule Pipeline do
  def labels(_people), do: []
end
'''
solution = '''
defmodule Pipeline do
  def labels(people) do
    people
    |> Enum.filter(&(&1.active == true))
    |> Enum.map(fn person -> person.name |> String.trim() |> String.upcase() end)
    |> Enum.reject(&(&1 == ""))
    |> Enum.uniq()
    |> Enum.sort()
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
    assert Pipeline.labels([]) == []
    assert Pipeline.labels([
      %{name: " zoe ", active: true}, %{name: "Ada", active: false},
      %{name: "ada", active: true}, %{name: " ADA ", active: true},
      %{name: " ", active: true}
    ]) == ["ADA", "ZOE"]
  end
end
'''
solution = '''
Code.require_file("lesson.exs", __DIR__)
ExUnit.start()
defmodule LessonTest do
  use ExUnit.Case
  test "public contract" do
    assert Pipeline.labels([]) == []
    assert Pipeline.labels([
      %{name: " zoe ", active: true}, %{name: "Ada", active: false},
      %{name: "ada", active: true}, %{name: " ADA ", active: true},
      %{name: " ", active: true}
    ]) == ["ADA", "ZOE"]
  end
end
'''
+++
# Enum & the Pipe Operator

Enum provides eager collection operations: map transforms each element, filter keeps matching elements, and reduce combines elements into an accumulator. The pipe operator passes the expression on its left as the first argument of the call on its right. A pipeline reads in the same order as the transformations happen.

```elixir
[1, 2, 3, 4]
|> Enum.filter(fn n -> rem(n, 2) == 0 end)
|> Enum.map(fn n -> n * 10 end)
# [20, 40]
```

Anonymous functions use `fn ... -> ... end`. `&(&1 * 10)` is a shorter function using its first argument, and `&String.upcase/1` captures an existing named function. Use whichever makes a transformation easiest to read. To call a function stored in a variable, use the dot: `double.(3)`.

Order changes the meaning of a pipeline. For the exercise, normalize names before removing duplicates, otherwise `"ada"` and `" ADA "` survive as distinct inputs. Drop blanks after trimming. Sort at the end so the returned order does not depend on incoming order.

Each operation returns a new collection and leaves the original intact.

Enum computes intermediate results immediately. Stream can defer transformations until enumeration, which is useful for larger inputs, but a short in-memory list needs no such machinery. After passing, move `Enum.uniq/1` before normalization in a scratch copy and identify the input that reveals the difference.

## Practice

Implement Pipeline.labels/1: take maps with :name and :active, keep active == true, trim and uppercase names, discard blank names, remove duplicates, and sort ascending.

Choose **Open practice** in the exercise card to create or reopen this lesson’s files. Edit the implementation, save your changes, then choose **Check work**. Your practice folder is reused when you return; opening it again keeps your edits. No source checkout or Python is needed.

The command shown in the exercise card runs from that folder. The starter intentionally fails. Leave check.exs unchanged and implement lesson.exs. A successful check exits with status zero; read assertion failures to find the input that needs attention.

## Hint before a solution

If duplicates survive, inspect the order of normalization and Enum.uniq. What happens to a name containing only spaces?

Try one focused change and check again. Before comparing with a reference solution, explain the failing case in your own words. A passing check covers the supplied examples; also try a new input and explain why your implementation handles it.
