+++
title = "Mix Projects & ExUnit"
concepts = ["Mix", "ExUnit", "assertions", "project layout"]

[[exercise]]
prompt = "Implement Basket.total/1 for a list of %{price: integer_cents, quantity: integer} maps with nonnegative values. Sum price times quantity. Add a test for a zero-quantity item to test/basket_test.exs."
[exercise.check]
command = ["mix", "test"]
expected = "0"
mode = "exit_code"
timeout_secs = 30

[[exercise.files]]
path = "lib/basket.ex"
starter = '''
defmodule Basket do
  def total(_items), do: 0
end
'''
solution = '''
defmodule Basket do
  def total(items) do
    Enum.reduce(items, 0, fn item, total -> total + item.price * item.quantity end)
  end
end
'''

[[exercise.files]]
path = "test/basket_test.exs"
starter = '''
defmodule LessonTest do
  use ExUnit.Case
  test "public contract" do
    assert Basket.total([]) == 0
    assert Basket.total([%{price: 125, quantity: 2}, %{price: 90, quantity: 3}]) == 520
    assert Basket.total([%{price: 999, quantity: 0}]) == 0
  end
end
'''
solution = '''
defmodule LessonTest do
  use ExUnit.Case
  test "public contract" do
    assert Basket.total([]) == 0
    assert Basket.total([%{price: 125, quantity: 2}, %{price: 90, quantity: 3}]) == 520
    assert Basket.total([%{price: 999, quantity: 0}]) == 0
  end
end
'''

[[exercise.files]]
path = "mix.exs"
starter = '''
defmodule Basket.MixProject do
  use Mix.Project
  def project, do: [app: :basket, version: "0.1.0", elixir: "~> 1.19", deps: []]
  def application, do: []
end
'''
solution = '''
defmodule Basket.MixProject do
  use Mix.Project
  def project, do: [app: :basket, version: "0.1.0", elixir: "~> 1.19", deps: []]
  def application, do: []
end
'''

[[exercise.files]]
path = "test/test_helper.exs"
starter = '''
ExUnit.start()
'''
solution = '''
ExUnit.start()
'''
+++
# Mix Projects & ExUnit

Mix builds projects, compiles modules, and runs tests. In a scratch directory, `mix new sample` creates `mix.exs`, source under `lib/`, and tests under `test/`. The practice folder for this lesson already contains a minimal project, so run its commands there without generating another nested project.

```sh
mix compile
mix test
iex -S mix
```

The project function in mix.exs declares application metadata and dependencies. This course uses only Elixir's standard tools and needs no Hex dependencies. `test/test_helper.exs` starts ExUnit; files ending in `_test.exs` define test cases.

`use ExUnit.Case` provides `test` and `assert` so failures show the expression and values involved.

```elixir
test "integer cents preserve exact totals" do
  assert Basket.total([%{price: 125, quantity: 2}]) == 250
end
```

Tests describe behavior through a public function. Check an empty input, multiple items, and a boundary such as zero quantity. Integer cents avoid floating-point rounding for this deliberately small pricing model. Validation belongs at the input boundary; this function assumes the documented nonnegative integer fields.

Implement the reduction, run `mix test`, and inspect any failed assertion before editing. Then add your own zero-quantity case even though the provided contract also covers it. Use `mix format lib/basket.ex test/basket_test.exs` to format your work.

The earlier lessons used ExUnit from scripts; Mix now handles source compilation and test discovery for you.

## Practice

Implement Basket.total/1 for a list of %{price: integer_cents, quantity: integer} maps with nonnegative values. Sum price times quantity. Add a test for a zero-quantity item to test/basket_test.exs.

Choose **Open practice** in the exercise card to create or reopen this lesson’s files. Edit the implementation, save your changes, then choose **Check work**. Your practice folder is reused when you return; opening it again keeps your edits. No source checkout or Python is needed.

The command shown in the exercise card runs from that folder. The starter intentionally fails. Add the requested test without removing the supplied assertions. A successful check exits with status zero; read assertion failures to find the input that needs attention.

## Hint before a solution

Start the reduction at zero. Compute one item’s contribution before adding it to the accumulated cents.

Try one focused change and check again. Before comparing with a reference solution, explain the failing case in your own words. A passing check covers the supplied examples; also try a new input and explain why your implementation handles it.
