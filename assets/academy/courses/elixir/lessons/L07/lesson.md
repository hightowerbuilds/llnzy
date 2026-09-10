+++
title = "Processes, Messages & Tasks"
concepts = ["spawn", "send", "receive", "references", "Task"]

[[exercise]]
prompt = "Implement Worker.reply/2: send caller the tuple {reference, value * 2}, where the second argument is {reference, value}. Implement Worker.double_all/1 using Task.async and Task.await, preserving input order."
[exercise.check]
command = ["elixir", "check.exs"]
expected = "0"
mode = "exit_code"
timeout_secs = 30

[[exercise.files]]
path = "lesson.exs"
starter = '''
defmodule Worker do
  def reply(_caller, _request), do: :ok
  def double_all(_values), do: []
end
'''
solution = '''
defmodule Worker do
  def reply(caller, {reference, value}), do: send(caller, {reference, value * 2})
  def double_all(values) do
    values
    |> Enum.map(fn value -> Task.async(fn -> value * 2 end) end)
    |> Enum.map(&Task.await/1)
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
    caller = self()
    reference = make_ref()
    spawn(fn -> Worker.reply(caller, {reference, 7}) end)
    assert_receive {^reference, 14}, 1000
    assert Worker.double_all([3, 0, -2]) == [6, 0, -4]
    assert Worker.double_all([]) == []
  end
end
'''
solution = '''
Code.require_file("lesson.exs", __DIR__)
ExUnit.start()
defmodule LessonTest do
  use ExUnit.Case
  test "public contract" do
    caller = self()
    reference = make_ref()
    spawn(fn -> Worker.reply(caller, {reference, 7}) end)
    assert_receive {^reference, 14}, 1000
    assert Worker.double_all([3, 0, -2]) == [6, 0, -4]
    assert Worker.double_all([]) == []
  end
end
'''
+++
# Processes, Messages & Tasks

BEAM processes have separate state and exchange messages. `self/0` returns the current process identifier; `spawn/1` starts a process running a function. `send/2` places a message in the recipient's mailbox. `receive` selects a message by pattern. A reference from `make_ref/0` lets you correlate a reply with the request that produced it.

```elixir
caller = self()
ref = make_ref()
spawn(fn -> send(caller, {ref, :ready}) end)
receive do
  {^ref, result} -> result
after
  1000 -> :timeout
end
```

Capture caller before spawning: `self()` inside the spawned function identifies the child instead. Pin the reference when receiving so an unrelated reply cannot satisfy the pattern. A timeout bounds waiting; sleeping and hoping a worker finished does not establish a reliable protocol.

Task packages a short-lived computation and its result. `Task.async/1` starts a linked task; `Task.await/1` waits for its result and can exit if the task fails or times out. Launch tasks first, then await them in input order. Awaiting immediately inside the creation loop would serialize this example. For large collections, consider Task.async_stream with bounded concurrency instead of starting an unbounded number of tasks.

The arithmetic here is too small to benefit from concurrency; it makes the message contract easy to inspect. After passing, explain why computation completion order need not match the returned list order, and why linked task failure can affect its caller.

## Practice

Implement Worker.reply/2: send caller the tuple {reference, value * 2}, where the second argument is {reference, value}. Implement Worker.double_all/1 using Task.async and Task.await, preserving input order.

Export from the repository root with `python3 scripts/check_academy_courses.py --export elixir L07 /tmp/llnzy-elixir-l07` (Python 3.11+). Choose a destination that does not already exist. Open the exported files in LLNZY and run `elixir check.exs` from that directory. The starter intentionally fails. Leave check.exs unchanged and implement lesson.exs. A successful check exits with status zero; read assertion failures to find the input that needs attention.
