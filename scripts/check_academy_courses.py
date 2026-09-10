#!/usr/bin/env python3
"""Run bundled exercise fixtures (Python 3.11+ and course runtimes on PATH).

Only run against trusted course content: check commands execute local code.
The Rust loader tests separately validate the full authoring schema.
"""

import argparse
import os
from pathlib import Path
import shutil
import shlex
import subprocess
import tempfile
import tomllib


COURSES = {"javascript": ("node",), "typescript": ("node", "tsc"), "elixir": ("elixir", "mix")}


ROOT = Path(__file__).resolve().parents[1] / "assets/academy/courses"


def read_lesson(course_id, lesson_id):
    course = ROOT / course_id
    manifest = tomllib.loads((course / "course.toml").read_text())
    ids = [item for module in manifest["modules"] for item in module["lessons"]]
    if lesson_id not in ids:
        raise ValueError(f"Unknown lesson {lesson_id!r} in {course_id}")
    path = course / "lessons" / lesson_id / "lesson.md"
    delimiters = path.read_text().split("+++", 2)
    if len(delimiters) != 3 or delimiters[0].strip():
        raise ValueError(f"Invalid frontmatter: {path}")
    return tomllib.loads(delimiters[1])


def write_files(root, exercise, variant):
    for file in exercise["files"]:
        target = root / file["path"]
        if not target.resolve().is_relative_to(root.resolve()):
            raise ValueError(f"Unsafe exercise path: {file['path']}")
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(file[variant])


def matches(result, check):
    expected = check["expected"]
    if check["mode"] == "exit_code":
        return result.returncode == int(expected)
    if result.returncode != 0:
        return False
    if check["mode"] == "exact":
        return result.stdout == expected
    if check["mode"] == "contains":
        return expected in result.stdout
    raise ValueError(f"Unknown match mode: {check['mode']}")


def run_fixture(exercise, variant):
    with tempfile.TemporaryDirectory(prefix="llnzy-course-") as directory:
        root = Path(directory)
        write_files(root, exercise, variant)
        check = exercise["check"]
        return subprocess.run(
            check["command"],
            cwd=root,
            env={**os.environ, "TZ": "UTC", "LC_ALL": "C", "NO_COLOR": "1",
                 "npm_config_offline": "true", "npm_config_yes": "false"},
            capture_output=True,
            text=True,
            timeout=check.get("timeout_secs", 60),
            check=False,
        )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--export", nargs=3, metavar=("COURSE", "LESSON", "DESTINATION"),
                        help="export starter files into a new directory instead of validating")
    parser.add_argument("--course", choices=COURSES, help="validate only this course")
    args = parser.parse_args()
    if args.export and args.course:
        parser.error("--course and --export cannot be combined")
    if args.export:
        course_id, lesson_id, destination = args.export
        if course_id not in COURSES:
            parser.error("COURSE must be " + ", ".join(COURSES))
        lesson = read_lesson(course_id, lesson_id)
        root = Path(destination).resolve()
        try:
            root.mkdir(parents=True, exist_ok=False)
        except FileExistsError:
            parser.error(f"Destination already exists: {root}; choose a new directory")
        for index, exercise in enumerate(lesson["exercise"]):
            target = root if len(lesson["exercise"]) == 1 else root / f"exercise-{index + 1}"
            write_files(target, exercise, "starter")
            print(f"Starter files: {target}")
            print(f"Check command: {shlex.join(exercise['check']['command'])}")
        return
    selected = [args.course] if args.course else list(COURSES)
    for executable in sorted({exe for course in selected for exe in COURSES[course]}):
        if not shutil.which(executable):
            raise SystemExit(f"Missing {executable} on PATH; see assets/academy/courses/README.md")
    count = 0
    for course_id in selected:
        course = ROOT / course_id
        manifest = tomllib.loads((course / "course.toml").read_text())
        for module in manifest["modules"]:
            for lesson_id in module["lessons"]:
                lesson = read_lesson(course_id, lesson_id)
                for index, exercise in enumerate(lesson["exercise"]):
                    label = f"{course_id}/{lesson_id} exercise {index + 1}"
                    for variant in ("solution", "starter"):
                        result = run_fixture(exercise, variant)
                        passed = matches(result, exercise["check"])
                        if passed != (variant == "solution"):
                            raise AssertionError(
                                f"{label}: {variant} unexpectedly {'passed' if passed else 'failed'}\n"
                                f"exit={result.returncode}\nstdout={result.stdout!r}\n"
                                f"stderr={result.stderr}\nexpected={exercise['check']['expected']!r}"
                            )
                    count += 1
                    print(f"PASS {label}: solution passes, starter fails", flush=True)
    print(f"Validated {count} exercises across {len(selected)} courses.")


if __name__ == "__main__":
    main()
