+++
title = "Values, Rebinding & Maps"
concepts = ["immutability", "atoms", "maps", "truthiness", "rebinding"]

[[exercise]]
prompt = "Implement Values.bump/1, which returns a map with its integer :count increased by one and preserves other keys. Implement default/1, which substitutes \"missing\" only for nil and false."
[exercise.check]
command = ["elixir", "check.exs"]
expected = "0"
mode = "exit_code"
timeout_secs = 30

[[exercise.files]]
path = "lesson.exs"
starter = '''
defmodule Values do
  def bump(data), do: data
  def default(_value), do: "missing"
end
'''
solution = '''
defmodule Values do
  def bump(data), do: %{data | count: data.count + 1}
  def default(value), do: value || "missing"
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
    original = %{count: 2, label: "jobs"}
    assert Values.bump(original) == %{count: 3, label: "jobs"}
    assert original.count == 2
    assert Values.bump(%{count: -1}) == %{count: 0}
    for value <- [0, "", [], :ready], do: assert(Values.default(value) == value)
    assert Values.default(nil) == "missing"
    assert Values.default(false) == "missing"
  end
end
'''
solution = '''
Code.require_file("lesson.exs", __DIR__)
ExUnit.start()
defmodule LessonTest do
  use ExUnit.Case
  test "public contract" do
    original = %{count: 2, label: "jobs"}
    assert Values.bump(original) == %{count: 3, label: "jobs"}
    assert original.count == 2
    assert Values.bump(%{count: -1}) == %{count: 0}
    for value <- [0, "", [], :ready], do: assert(Values.default(value) == value)
    assert Values.default(nil) == "missing"
    assert Values.default(false) == "missing"
  end
end
'''
+++
# Values, Rebinding & Maps

Elixir values are immutable. A map update produces a new map; code holding the original value still sees the original. Variables can be rebound to new values, but rebinding does not mutate an earlier value. This makes functions easier to reason about: a caller's data does not change behind its back.

```elixir
before = %{count: 4, label: "jobs"}
after_update = %{before | count: 5}
{before.count, after_update.count} # {4, 5}
```

Maps associate keys with values. `%{count: 4}` uses an atom key, equivalent to `%{:count => 4}`. `%{"count" => 4}` uses a string key and is a different map. The update syntax above requires the key to exist; `Map.put/3` also inserts absent keys. Atoms such as `:ok` represent named values. Keep externally supplied names as strings instead of creating arbitrary atoms.

Only `false` and `nil` are falsy. Zero, empty strings, and empty lists are truthy. `value || fallback` therefore preserves zero, unlike JavaScript's corresponding expression. Integers and floats are distinct number types; `/` returns a float, while `div/2` performs integer division. Try `7 / 2` and `div(7, 2)` in IEx.

The exercise assumes a map with an integer count. Preserve its other fields and use a new map as the result. After passing, explain why rebinding `original = Values.bump(original)` would still leave another variable bound to the old map unchanged.

## Practice

Implement Values.bump/1, which returns a map with its integer :count increased by one and preserves other keys. Implement default/1, which substitutes "missing" only for nil and false.

Export from the repository root with `python3 scripts/check_academy_courses.py --export elixir L01 /tmp/llnzy-elixir-l01` (Python 3.11+). Choose a destination that does not already exist. Open the exported files in LLNZY and run `elixir check.exs` from that directory. The starter intentionally fails. Leave check.exs unchanged and implement lesson.exs. A successful check exits with status zero; read assertion failures to find the input that needs attention.
