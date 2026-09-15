+++
title = "Capstone: A Supervised Task Tracker"
concepts = ["Supervisor", "restart strategy", "GenServer", "validation", "capstone"]

[[exercise]]
prompt = "Complete Board: init stores %{}; {:add, title} trims a binary title, rejects blanks with {:error, :blank}, rejects duplicate normalized titles with {:error, :duplicate}, otherwise stores :open and replies :ok. {:finish, title} marks an exact existing title :done or returns {:error, :missing}. :list returns sorted {title, status} pairs. Run under the supplied one_for_one supervisor; restarted state is empty."
[exercise.check]
command = ["elixir", "check.exs"]
expected = "0"
mode = "exit_code"
timeout_secs = 30

[[exercise.files]]
path = "lesson.exs"
starter = '''
defmodule Board do
  use GenServer
  def start_link(options), do: GenServer.start_link(__MODULE__, :empty, options)
  def add(server, title), do: GenServer.call(server, {:add, title})
  def finish(server, title), do: GenServer.call(server, {:finish, title})
  def list(server), do: GenServer.call(server, :list)
  @impl true
  def init(:empty), do: {:ok, %{}}
  @impl true
  def handle_call(_request, _from, state), do: {:reply, :todo, state}
end
'''
solution = '''
defmodule Board do
  use GenServer
  def start_link(options), do: GenServer.start_link(__MODULE__, :empty, options)
  def add(server, title), do: GenServer.call(server, {:add, title})
  def finish(server, title), do: GenServer.call(server, {:finish, title})
  def list(server), do: GenServer.call(server, :list)
  @impl true
  def init(:empty), do: {:ok, %{}}
  @impl true
  def handle_call({:add, title}, _from, state) do
    title = String.trim(title)
    cond do
      title == "" -> {:reply, {:error, :blank}, state}
      Map.has_key?(state, title) -> {:reply, {:error, :duplicate}, state}
      true -> {:reply, :ok, Map.put(state, title, :open)}
    end
  end
  def handle_call({:finish, title}, _from, state) do
    if Map.has_key?(state, title) do
      {:reply, :ok, Map.put(state, title, :done)}
    else
      {:reply, {:error, :missing}, state}
    end
  end
  def handle_call(:list, _from, state), do: {:reply, Enum.sort(state), state}
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
    supervisor = start_supervised!(%{id: :board_supervisor, type: :supervisor, start: {Supervisor, :start_link, [[{Board, []}], [strategy: :one_for_one]]}})
    [{Board, pid, :worker, _}] = Supervisor.which_children(supervisor)
    assert Board.list(pid) == []
    assert Board.add(pid, "  Ship  ") == :ok
    assert Board.add(pid, "Read") == :ok
    assert Board.add(pid, "Ship") == {:error, :duplicate}
    assert Board.add(pid, " ") == {:error, :blank}
    assert Board.finish(pid, "Missing") == {:error, :missing}
    assert Board.finish(pid, "Read") == :ok
    assert Board.finish(pid, "Read") == :ok
    assert Board.list(pid) == [{"Read", :done}, {"Ship", :open}]
    reference = Process.monitor(pid)
    Process.exit(pid, :kill)
    assert_receive {:DOWN, ^reference, :process, ^pid, :killed}, 1000
    replacement = await_replacement(supervisor, pid, System.monotonic_time(:millisecond) + 2000)
    assert replacement != pid
    assert Board.list(replacement) == []
    assert Board.add(replacement, "Restarted") == :ok
    assert Board.list(replacement) == [{"Restarted", :open}]
  end

  defp await_replacement(supervisor, old_pid, deadline) do
    case Supervisor.which_children(supervisor) do
      [{Board, pid, :worker, _}] when is_pid(pid) and pid != old_pid -> pid
      _ ->
        if System.monotonic_time(:millisecond) >= deadline, do: flunk("worker did not restart")
        Process.sleep(10)
        await_replacement(supervisor, old_pid, deadline)
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
    supervisor = start_supervised!(%{id: :board_supervisor, type: :supervisor, start: {Supervisor, :start_link, [[{Board, []}], [strategy: :one_for_one]]}})
    [{Board, pid, :worker, _}] = Supervisor.which_children(supervisor)
    assert Board.list(pid) == []
    assert Board.add(pid, "  Ship  ") == :ok
    assert Board.add(pid, "Read") == :ok
    assert Board.add(pid, "Ship") == {:error, :duplicate}
    assert Board.add(pid, " ") == {:error, :blank}
    assert Board.finish(pid, "Missing") == {:error, :missing}
    assert Board.finish(pid, "Read") == :ok
    assert Board.finish(pid, "Read") == :ok
    assert Board.list(pid) == [{"Read", :done}, {"Ship", :open}]
    reference = Process.monitor(pid)
    Process.exit(pid, :kill)
    assert_receive {:DOWN, ^reference, :process, ^pid, :killed}, 1000
    replacement = await_replacement(supervisor, pid, System.monotonic_time(:millisecond) + 2000)
    assert replacement != pid
    assert Board.list(replacement) == []
    assert Board.add(replacement, "Restarted") == :ok
    assert Board.list(replacement) == [{"Restarted", :open}]
  end

  defp await_replacement(supervisor, old_pid, deadline) do
    case Supervisor.which_children(supervisor) do
      [{Board, pid, :worker, _}] when is_pid(pid) and pid != old_pid -> pid
      _ ->
        if System.monotonic_time(:millisecond) >= deadline, do: flunk("worker did not restart")
        Process.sleep(10)
        await_replacement(supervisor, old_pid, deadline)
    end
  end
end
'''
+++
# Capstone: A Supervised Task Tracker

Build a small service that owns a map of task titles to statuses. You now have the tools to normalize input, preserve error tuples, transform immutable data, and coordinate state through GenServer. This capstone combines them behind add, finish, and list operations.

Titles are case-sensitive; finish uses the exact stored title, and finishing an already completed task succeeds again.

Use `cond` when choosing among Boolean conditions such as blank or duplicate titles. The first truthy branch wins; a final `true` branch is the fallback. Use `Map.has_key?/2` to distinguish an existing title before updating the map, and `Enum.sort/1` to make list output predictable.

Expected validation failures must preserve state.

```elixir
children = [{Board, []}]
{:ok, supervisor} = Supervisor.start_link(children, strategy: :one_for_one)
```

A supervisor starts children and applies their restart policies. `use GenServer` supplies a child specification whose default restart policy is permanent. With one_for_one, a failing child is restarted individually.

The provided test kills the worker, monitors its exit, then waits with a deadline for a different PID. It checks actual recovery instead of assuming a fixed sleep was enough.

Restarting restores availability, not data. This board deliberately stores tasks only in memory, so init produces an empty map after a crash. Durable tasks require a separate persistence design.

A real application normally places its supervisor under an Application callback; the check uses an ExUnit-managed supervisor to keep the exercise self-contained.

Implement and validate one operation at a time, then run the recovery check. The test helper's short bounded polling handles supervisor scheduling; it is not application logic. After passing, explain what a caller holding the old PID would need to do after a restart.

As an extension, use a registered name and discuss how concurrent callers should discover the service. Persistence and Phoenix interfaces are future extensions beyond this foundation course.

## Practice

Complete Board: init stores %{}; {:add, title} trims a binary title, rejects blanks with {:error, :blank}, rejects duplicate normalized titles with {:error, :duplicate}, otherwise stores :open and replies :ok. {:finish, title} marks an exact existing title :done or returns {:error, :missing}. :list returns sorted {title, status} pairs.

Run under the supplied one_for_one supervisor; restarted state is empty.

Choose **Open practice** in the exercise card to create or reopen this lesson’s files. Edit the implementation, save your changes, then choose **Check work**. Your practice folder is reused when you return; opening it again keeps your edits. No source checkout or Python is needed.

The command shown in the exercise card runs from that folder. The starter intentionally fails. Leave check.exs unchanged and implement lesson.exs. A successful check exits with status zero; read assertion failures to find the input that needs attention.

## Hint before a solution

Keep task updates inside the server callbacks. After a worker restart, should the task list come from memory or durable storage in this project?

Try one focused change and check again. Before comparing with a reference solution, explain the failing case in your own words. A passing check covers the supplied examples; also try a new input and explain why your implementation handles it.
