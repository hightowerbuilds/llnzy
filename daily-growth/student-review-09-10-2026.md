**LLNZY from a student's perspective — review draft, September 10, 2026**

LLNZY offers a compelling place to learn programming with real tools, but the current student experience leaves too much of the connection between lessons, practice, and progress to the learner.

This is a code-informed product review, not a testimonial from a student who has used the app. It considers a beginning programming student who can create files and navigate a terminal, matching the JavaScript course's stated prerequisites. The review covers the current working tree, including existing uncommitted changes. Runtime performance, visual comfort, accessibility, and reliability were not tested interactively.

**Student-voice draft**

“What appeals to me about LLNZY is having my lessons close to the tools I am learning to use. I can read about programming, edit real files, and run commands in a real terminal. That makes the work feel connected to building my own projects. Having a notepad in the same app is useful too: I want somewhere to write down a confusing error or explain a concept in my own words without first organizing another set of files.

“The courses give me a direction. JavaScript leads toward an expense-report command-line app, TypeScript toward a modular ledger report, and Elixir toward a supervised task tracker. Those are concrete outcomes I can understand. The lessons also ask me to explain behavior and change examples, which gives me something to think about beyond copying code.

“The difficult part is getting from an exercise description to actually doing the exercise. I can see the files an exercise needs and copy its check command, but the app does not prepare those files or run the check for me. The documented practice workflow sends me to a Python script in the source repository. As a student, I would expect opening an exercise to bring up the starter files in a working project. Instead, I have another setup task before I can start learning the concept.

“I also want the app to recognize what I have done. There are lesson counts and progress bars, but I cannot find a way in the current interface to mark a lesson complete or have a passing exercise update them. That would leave me unsure whether I missed a step. Returning to a course should also take me back to where I stopped.

“I could see myself using LLNZY with a teacher or as a student who already knows how to troubleshoot development tools. As a beginner studying alone, I would need more help with setup, understanding failed checks, and knowing what to do next. The feature I most want is a complete path from opening a lesson to solving its exercise and seeing my progress saved.”

**What supports that assessment**

| Area | Current evidence | Student consequence |
| --- | --- | --- |
| A real coding workspace | Terminal, editor, project navigation, and joined panes are implemented; the editor includes completion, diagnostics, navigation, and formatting integrations. | Skills and files can carry into independent projects. Editor assistance depends on separately available language servers. |
| Structured curriculum | Course manifests contain JavaScript: 11 lessons; TypeScript: 10; Rust: 7; Elixir: 10. JS, TS, and Elixir culminate in practical projects. | There is a clear sequence, but the courses differ in coverage. Rust currently covers chapters 1–3; students should not mistake book alignment for a complete Rust curriculum. |
| Exercise handoff | `academy_exercise` renders the prompt, file names, check command, and copy button. It does not expose starter-file creation or a check runner. The course README documents repository-based export. | Students must bridge the gap themselves. Copying a command is insufficient when its required files have not been created. |
| Feedback | JS/TS/Elixir fixtures include executable checks and solutions, and an authoring validation script checks passing solutions and failing starters. | Useful assessment material exists, but learners currently receive terminal output rather than an integrated explanation or next step. Authoring checks are not evidence of an operational in-app grader. |
| Completion | The completion store and progress displays exist. A source-wide search found `record_lesson_complete` called only by tests. Lesson navigation only changes the selected course and lesson. | Normal use has no connected path to advance the displayed completion count. This is a missing workflow, not merely unclear wording. |
| Returning to study | Home course cards open the course list. Course and lesson selection initialize to `None`; workspace recovery does not store those selections. | There is no implemented persistent “continue this lesson” path in the reviewed code. |
| Notes | Notes save after a short idle interval, flush at shutdown, and are shared across app windows. Load failures block editing; save failures offer retry. | Capturing questions takes little setup, with explicit handling for persistence failures. Notes are general-purpose rather than attached to a particular lesson. |
| Reading and copying | Lessons have previous/next navigation, 16-pixel body text, code-block copy buttons, and a whole-lesson copy button. Static prose does not support ordinary text selection in this implementation. | Copying commands is convenient; extracting a sentence for notes is less direct. |
| Laptop layout | Home has a 760-pixel minimum content width and the Academy uses 720-pixel panels. | Narrow windows and joined study panes deserve direct testing for overflow and comfortable reading. This is a layout concern inferred from code, not a reproduced visual defect. |
| Setup and reach | macOS is the active supported release target. Practice requires language tools; the documented export path also requires Python and the repository. Elixir has neither a bundled editor grammar nor a built-in language-server entry. | A student with a prepared Mac has a more straightforward path than someone on another platform or an unconfigured machine. Course availability does not imply equal editor assistance. |

**Changes with the greatest student value**

1. Connect the first exercise completely: create a safe, persistent practice folder, open its starter files, use that folder in the terminal, run the check, and show the outcome. Make this work in the packaged app without requiring the source repository.
2. Make progress meaningful: connect passing checks to completion, distinguish reading from verified practice, and persist the last lesson and practice workspace for a “Continue” action.
3. Explain readiness in student language: show whether the selected course's runtime is available and separately whether editor assistance is available. Provide actionable setup guidance and a way to recheck.
4. Help students recover from mistakes: distinguish an incorrect answer from a missing tool or wrong working directory; show expected versus actual results where appropriate and offer a hint before a solution.
5. Clarify course scope and study ergonomics: identify the Rust course as its current introductory portion, make prerequisites visible, and test lesson/editor/terminal layouts at ordinary laptop sizes. Add lesson-linked notes or a shortcut that captures the lesson title with a question.

**Evidence map**

- Product scope and platform status: [README](../README.md).
- Home and course entry: [home.rs](../src/gpui_workspace/home.rs), [menu_actions.rs](../src/gpui_workspace/menu_actions.rs).
- Reader, exercises, copying, and progress display: [academy.rs](../src/gpui_workspace/academy.rs).
- Completion model: [academy_progress.rs](../src/academy_progress.rs).
- Selection initialization and recovery: [gpui_workspace.rs](../src/gpui_workspace.rs), [recovery.rs](../src/gpui_workspace/recovery.rs).
- Notes and save behavior: [notepad.rs](../src/gpui_workspace/notepad.rs).
- Editor tool dependencies: [registry.rs](../src/lsp/registry.rs).
- Curriculum, setup, and practice: [course documentation](../assets/academy/courses/README.md), course manifests and lesson files beneath that directory, and [exercise validation/export script](../scripts/check_academy_courses.py).

The next useful validation is to observe a student starting with the packaged app: open JavaScript L00, prepare practice, fix the intentionally failing starter, check the result, save a question, quit, and return. Record where they need help and whether they can explain the failure. That would test the central learning workflow more directly than a feature rating.
