//! End-to-end pass through the student practice loop without the GUI:
//! prepare starters, check (fail), apply solution, check (pass), record
//! verified progress and the resume location, persist, reload.
use llnzy::academy::practice::{prepare_exercise, run_check, CheckStatus};
use llnzy::academy::CourseLibrary;
use llnzy::academy_progress::{exercise_id, AcademyProgress};
use std::path::Path;

fn temp(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("llnzy-loop-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn has(program: &str) -> bool {
    std::process::Command::new(program)
        .arg("--version")
        .output()
        .is_ok()
}

fn walk(course_id: &str, lessons: &[&str]) {
    let library =
        CourseLibrary::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/academy/courses"))
            .unwrap();
    let course = library.course(course_id).unwrap();
    let base = temp(course_id);
    let progress_file = base.join("progress.json");
    let mut progress = AcademyProgress::default();
    for lesson_id in lessons {
        let lesson = &course.lessons[*lesson_id];
        let ids = lesson
            .meta
            .exercises
            .iter()
            .map(exercise_id)
            .collect::<Vec<_>>();
        let required = ids.iter().map(String::as_str).collect::<Vec<_>>();
        for (index, exercise) in lesson.meta.exercises.iter().enumerate() {
            let prepared = prepare_exercise(
                &base.join("practice"),
                course_id,
                lesson_id,
                index,
                exercise,
            )
            .unwrap();
            let started = std::time::Instant::now();
            let starter = run_check(&prepared.directory, &exercise.check);
            assert_eq!(
                starter.status,
                CheckStatus::Failed,
                "{course_id}/{lesson_id} starter: {starter:?}"
            );
            assert!(!progress.is_exercise_passed(course_id, lesson_id, &ids[index]));
            for file in &exercise.files {
                std::fs::write(prepared.directory.join(&file.path), &file.solution).unwrap();
            }
            // Re-preparing must keep the student's edits.
            let again = prepare_exercise(
                &base.join("practice"),
                course_id,
                lesson_id,
                index,
                exercise,
            )
            .unwrap();
            assert_eq!(again.created, 0);
            let solved = run_check(&prepared.directory, &exercise.check);
            assert_eq!(
                solved.status,
                CheckStatus::Passed,
                "{course_id}/{lesson_id} solution: {solved:?}"
            );
            eprintln!(
                "{course_id}/{lesson_id} exercise {} ok in {:.1}s",
                index + 1,
                started.elapsed().as_secs_f32()
            );
            progress.record_exercise_passed(course_id, lesson_id, &ids[index], &required);
            progress.record_location(course_id, lesson_id, Some(prepared.directory.clone()));
            progress.save_to(&progress_file).unwrap();
        }
        assert!(progress.is_lesson_complete(course_id, lesson_id));
        progress.record_lesson_read(course_id, lesson_id);
        progress.save_to(&progress_file).unwrap();
    }
    let reloaded = AcademyProgress::load_from(&progress_file);
    for lesson_id in lessons {
        assert!(
            reloaded.is_lesson_complete(course_id, lesson_id),
            "{lesson_id} lost"
        );
        assert!(reloaded.is_lesson_read(course_id, lesson_id));
    }
    let location = reloaded.last_location().unwrap();
    assert_eq!(location.lesson_id, *lessons.last().unwrap());
    assert!(location.practice_path.as_ref().unwrap().is_dir());
    let all = course.lesson_ids_in_order();
    assert_eq!(reloaded.completed_lessons(course_id, &all), lessons.len());
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn rust_course_full_loop() {
    if !has("cargo") {
        return;
    }
    walk("rust", &["L00", "L01", "L02", "L03", "L04", "L05", "L06"]);
}

#[test]
fn typescript_course_full_loop() {
    if !has("tsc") {
        eprintln!("skipping: tsc missing");
        return;
    }
    walk(
        "typescript",
        &[
            "L00", "L01", "L02", "L03", "L04", "L05", "L06", "L07", "L08", "L09",
        ],
    );
}
