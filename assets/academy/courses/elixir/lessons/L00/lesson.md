+++
title = "Your First Elixir Program"
concepts = ["BEAM", "IEx", "scripts", "modules", "strings"]

[[exercise]]
prompt = "Implement Hello.greet/1: trim the supplied name and return \"Hello, NAME!\". An empty or whitespace-only name becomes \"friend\". Return the string; do not print inside the function."
[exercise.check]
command = ["elixir", "check.exs"]
expected = "0"
mode = "exit_code"
timeout_secs = 30

[[exercise.files]]
path = "lesson.exs"
starter = '''
defmodule Hello do
  def greet(_name), do: "TODO"
end
'''
solution = '''
defmodule Hello do
  def greet(name) do
    cleaned = String.trim(name)
    label = if cleaned == "", do: "friend", else: cleaned
    "Hello, #{label}!"
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
    assert Hello.greet(" Ada ") == "Hello, Ada!"
    assert Hello.greet("") == "Hello, friend!"
    assert Hello.greet("  ") == "Hello, friend!"
    assert Hello.greet("José") == "Hello, José!"
  end
end
'''
solution = '''
Code.require_file("lesson.exs", __DIR__)
ExUnit.start()
defmodule LessonTest do
  use ExUnit.Case
  test "public contract" do
    assert Hello.greet(" Ada ") == "Hello, Ada!"
    assert Hello.greet("") == "Hello, friend!"
    assert Hello.greet("  ") == "Hello, friend!"
    assert Hello.greet("José") == "Hello, José!"
  end
end
'''
+++
# Your First Elixir Program

Elixir runs on the Erlang virtual machine, commonly called the BEAM. Start this course with basic programming familiarity; no Rust, JavaScript, or Erlang knowledge is required. Each lesson gives you a small contract to implement, with checks you can run locally.

The final project combines data transformations with a supervised process.

Install Elixir and a compatible Erlang/OTP release using the [official installation guide](https://elixir-lang.org/install.html). This course is validated with Elixir 1.19.5 and OTP 28. Confirm `elixir --version` and `mix --version`, then launch `iex`. Type expressions without copying the `iex>` prompt:

```elixir
40 + 2
String.trim("  Ada  ")
"Hello, #{String.upcase("Ada")}!"
```

IEx prints an expression's result. `IO.puts/1` prints text as a side effect and returns `:ok`; returning a greeting from a function is a different operation. The notation `String.trim/1` means a function named trim in the String module taking one argument.

A module groups named functions; `def` defines a public function, and the last expression is its return value.

Save `IO.puts("Hello")` in a scratch `hello.exs` file and run `elixir hello.exs`. Scripts use `.exs`; compiled project source normally uses `.ex`. Exit IEx with Ctrl+C twice. In the practice function, use `String.trim/1`, an `if` expression, and interpolation.

Predict the result for a blank name before running the check.

## Practice

Implement Hello.greet/1: trim the supplied name and return "Hello, NAME!". An empty or whitespace-only name becomes "friend". Return the string; do not print inside the function.

Choose **Open practice** in the exercise card to create or reopen this lesson’s files. Edit the implementation, save your changes, then choose **Check work**. Your practice folder is reused when you return; opening it again keeps your edits. No source checkout or Python is needed.

The command shown in the exercise card runs from that folder. The starter intentionally fails. Leave check.exs unchanged and implement lesson.exs. A successful check exits with status zero; read assertion failures to find the input that needs attention.

## Hint before a solution

Trim before deciding whether the name is blank. The last expression should be the greeting string; IO.puts would return :ok.

Try one focused change and check again. Before comparing with a reference solution, explain the failing case in your own words. A passing check covers the supplied examples; also try a new input and explain why your implementation handles it.
