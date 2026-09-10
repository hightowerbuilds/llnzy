+++
title = "State with GenServer"
concepts = ["GenServer", "callbacks", "state", "synchronous calls"]

[[exercise]]
prompt = "Implement Counter.init/1 and handle_call/3: start with the supplied integer, :value returns it unchanged, and {:add, amount} returns and stores the incremented value. Keep the supplied client API."
[exercise.check]
command = ["elixir", "check.exs"]
expected = "0"
mode = "exit_code"
timeout_secs = 30

[[exercise.files]]
path = "lesson.exs"
starter = '''
defmodule Counter do
  use GenServer
  def start_link(initial), do: GenServer.start_link(__MODULE__, initial)
  def value(pid), do: GenServer.call(pid, :value)
  def add(pid, amount), do: GenServer.call(pid, {:add, amount})
  @impl true
  def init(_initial), do: {:ok, 0}
  @impl true
  def handle_call(_request, _from, state), do: {:reply, :todo, state}
end
'''
solution = '''
defmodule Counter do
  use GenServer
  def start_link(initial), do: GenServer.start_link(__MODULE__, initial)
  def value(pid), do: GenServer.call(pid, :value)
  def add(pid, amount), do: GenServer.call(pid, {:add, amount})
  @impl true
  def init(initial), do: {:ok, initial}
  @impl true
  def handle_call(:value, _from, state), do: {:reply, state, state}
  def handle_call({:add, amount}, _from, state) do
    updated = state + amount
    {:reply, updated, updated}
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
    pid = start_supervised!({Counter, 10})
    assert Counter.value(pid) == 10
    assert Counter.add(pid, 5) == 15
    assert Counter.add(pid, -2) == 13
    assert Counter.value(pid) == 13
    {:ok, other} = Counter.start_link(0)
    try do
      assert Counter.value(other) == 0
      assert Counter.value(pid) == 13
     after
      GenServer.stop(other)
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
    pid = start_supervised!({Counter, 10})
    assert Counter.value(pid) == 10
    assert Counter.add(pid, 5) == 15
    assert Counter.add(pid, -2) == 13
    assert Counter.value(pid) == 13
    {:ok, other} = Counter.start_link(0)
    try do
      assert Counter.value(other) == 0
      assert Counter.value(pid) == 13
     after
      GenServer.stop(other)
    end
  end
end
'''
+++
# State with GenServer

A long-lived process can hold state by passing a new value to its next iteration. GenServer supplies the message loop and lifecycle conventions, leaving your module to implement callbacks. The public client functions execute in the caller; init and handle_call execute in the server process.

```elixir
# Callback return shape:
{:reply, reply_for_caller, next_state}
```

`init/1` returns `{:ok, initial_state}`. A synchronous `GenServer.call/2` waits for a reply from `handle_call/3`. The callback receives the request, caller information, and current state. Returning updated state does not mutate an integer; GenServer uses that returned value for the next message. `@impl true` marks callbacks so the compiler can check their relationship to the behaviour.

Our counter's :value request replies without changing state. An add request computes one updated value and uses it both as reply and next state. Each server handles its messages serially, so this read-modify-write sequence stays inside a single process. Two separately started counters have independent state.

`GenServer.cast/2` does not wait for a reply. Prefer a call when the caller needs confirmation or a returned value. A GenServer is useful for coordinating state, but wrapping every pure function in a process adds unnecessary serialization. Keep calculations as ordinary functions whenever no process ownership is needed.

The check starts the counter under ExUnit supervision for cleanup. After passing, deliberately return the old state from the add callback in a scratch copy: the first reply may look right, but the subsequent value check reveals the lost update.

## Practice

Implement Counter.init/1 and handle_call/3: start with the supplied integer, :value returns it unchanged, and {:add, amount} returns and stores the incremented value. Keep the supplied client API.

Export from the repository root with `python3 scripts/check_academy_courses.py --export elixir L08 /tmp/llnzy-elixir-l08` (Python 3.11+). Choose a destination that does not already exist. Open the exported files in LLNZY and run `elixir check.exs` from that directory. The starter intentionally fails. Leave check.exs unchanged and implement lesson.exs. A successful check exits with status zero; read assertion failures to find the input that needs attention.
