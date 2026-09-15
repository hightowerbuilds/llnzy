//! Connect lessons to persistent practice folders, checks, and student progress.
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use gpui::{Context, Window};

use super::{WorkspacePrototype, WorkspaceSurface};
use crate::academy::practice::{self, CheckResult, CheckStatus, ToolReadiness};

#[derive(Clone, Default)]
pub(super) struct AcademyPracticeState {
    pub busy: bool,
    pub checking_setup: BTreeSet<String>,
    pub notice: Option<String>,
    pub directories: BTreeMap<String, PathBuf>,
    terminal_tabs: BTreeMap<PathBuf, u64>,
    pub results: BTreeMap<String, CheckResult>,
    pub readiness: BTreeMap<String, Vec<ToolReadiness>>,
    pub revealed: BTreeSet<String>,
}

pub(super) fn exercise_key(course: &str, lesson: &str, index: usize) -> String {
    format!("{course}/{lesson}/{index}")
}

impl WorkspacePrototype {
    pub(super) fn persist_academy_progress(&mut self) {
        if let Err(error) = self.academy_progress.save() {
            self.academy_practice.notice = Some(format!(
                "Your progress is in memory but could not be saved: {error}. Use Retry saving progress."
            ));
        } else {
            self.academy_progress = crate::academy_progress::AcademyProgress::load();
        }
    }

    fn academy_open_terminal(
        &mut self,
        directory: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .academy_practice
            .terminal_tabs
            .get(&directory)
            .and_then(|id| self.terminals.get(id))
            .is_some_and(|terminal| terminal.read(cx).is_running_in(&directory))
        {
            return;
        }
        self.open_new_terminal_tab(window, cx);
        self.academy_practice
            .terminal_tabs
            .insert(directory, self.active_tab_id.0);
    }

    pub(super) fn continue_academy_learning(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(location) = self.academy_progress.last_location().cloned() else {
            return;
        };
        let valid = self
            .academy_library
            .as_ref()
            .and_then(|library| library.course(&location.course_id))
            .is_some_and(|course| course.lessons.contains_key(&location.lesson_id));
        if !valid {
            self.academy_practice.notice =
                Some("That lesson is no longer in this catalog. Choose another course.".into());
            self.open_academy_from_home(window, cx);
            return;
        }
        self.select_academy_lesson(location.course_id.clone(), location.lesson_id.clone(), cx);
        if let (Some(directory), Some(paths)) = (
            location.practice_path,
            crate::platform::paths::current_paths(),
        ) {
            let count = self
                .academy_library
                .as_ref()
                .and_then(|library| library.course(&location.course_id))
                .and_then(|course| course.lessons.get(&location.lesson_id))
                .map_or(0, |lesson| lesson.meta.exercises.len());
            let base = paths
                .data_dir
                .join("academy/practice")
                .join(&location.course_id)
                .join(&location.lesson_id);
            if let Some(index) =
                (0..count).find(|index| directory == base.join(format!("exercise-{}", index + 1)))
            {
                self.academy_prepare(location.course_id, location.lesson_id, index, window, cx);
            } else {
                self.academy_practice.notice = Some("Your saved practice location has changed. Choose Open practice to reopen the lesson's files.".into());
            }
        }
        self.open_or_activate_surface(WorkspaceSurface::Academy, window, cx);
    }

    pub(super) fn academy_check_readiness(&mut self, language: String, cx: &mut Context<Self>) {
        if !self
            .academy_practice
            .checking_setup
            .insert(language.clone())
        {
            return;
        }
        cx.notify();
        cx.spawn(
            move |workspace: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    let probe_language = language.clone();
                    let readiness = cx
                        .background_executor()
                        .spawn(async move { practice::probe_readiness(&probe_language) })
                        .await;
                    let _ = workspace.update(&mut cx, |this, cx| {
                        this.academy_practice.checking_setup.remove(&language);
                        this.academy_practice.readiness.insert(language, readiness);
                        cx.notify();
                    });
                }
            },
        )
        .detach();
    }

    pub(super) fn academy_prepare(
        &mut self,
        course: String,
        lesson: String,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.academy_practice.busy {
            self.academy_practice.notice =
                Some("An exercise operation is still running. Please wait for its result.".into());
            cx.notify();
            return;
        }
        let Some(exercise) = self
            .academy_library
            .as_ref()
            .and_then(|library| library.course(&course))
            .and_then(|course| course.lessons.get(&lesson))
            .and_then(|lesson| lesson.meta.exercises.get(index))
            .cloned()
        else {
            return;
        };
        let Some(paths) = crate::platform::paths::current_paths() else {
            self.academy_practice.notice =
                Some("Practice storage is unavailable on this device.".into());
            cx.notify();
            return;
        };
        let primary_file = exercise
            .files
            .iter()
            .find(|file| file.starter != file.solution)
            .or_else(|| exercise.files.first())
            .map(|file| file.path.clone());
        self.academy_practice.busy = true;
        self.academy_practice.notice = Some("Preparing your practice files…".into());
        let handle = window.window_handle();
        cx.notify();
        cx.spawn(move |workspace: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let c = course.clone(); let l = lesson.clone();
                let result = cx.background_executor().spawn(async move {
                    practice::prepare_exercise(&paths.data_dir.join("academy/practice"), &c, &l, index, &exercise)
                }).await;
                let _ = handle.update(&mut cx, |_, window, cx| {
                    let _ = workspace.update(cx, |this, cx| {
                        this.academy_practice.busy = false;
                        match result {
                            Ok(prepared) => {
                                let directory = prepared.directory;
                                this.academy_practice.directories.insert(exercise_key(&course, &lesson, index), directory.clone());
                                if this.academy_course.as_ref() != Some(&course) || this.academy_lesson.as_ref() != Some(&lesson) {
                                    cx.notify();
                                    return;
                                }
                                this.academy_progress.record_location(&course, &lesson, Some(directory.clone()));
                                this.academy_practice.notice = Some(format!("Practice ready: {}. Edit your files, save, then return here to Check work.", directory.display()));
                                this.persist_academy_progress();
                                if this.workspace_root.as_ref() != Some(&directory) {
                                    this.open_project(directory.clone(), cx);
                                }
                                if this.workspace_root.as_ref() != Some(&directory) {
                                    this.academy_practice.notice = Some("Practice files are ready. Save or close your modified files, then choose Open practice again to switch projects.".into());
                                } else {
                                    this.academy_open_terminal(directory.clone(), window, cx);
                                    let primary = primary_file.as_ref().map(|path| directory.join(path));
                                    for path in prepared.files.iter().filter(|path| Some(*path) != primary.as_ref()) {
                                        this.open_sidebar_file(path.clone(), window, cx);
                                    }
                                    if let Some(path) = primary { this.open_sidebar_file(path, window, cx); }
                                    this.open_or_activate_surface(WorkspaceSurface::Academy, window, cx);
                                }
                            }
                            Err(error) => this.academy_practice.notice = Some(error),
                        }
                        cx.notify();
                    });
                });
            }
        }).detach();
    }

    pub(super) fn academy_run_check(
        &mut self,
        course: String,
        lesson: String,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        if self.academy_practice.busy {
            self.academy_practice.notice =
                Some("An exercise operation is still running. Please wait for its result.".into());
            cx.notify();
            return;
        }
        let key = exercise_key(&course, &lesson, index);
        let Some(directory) = self.academy_practice.directories.get(&key).cloned() else {
            self.academy_practice.notice =
                Some("Choose Open practice first to prepare your files.".into());
            cx.notify();
            return;
        };
        if let Some(message) = self
            .editor_entities()
            .iter()
            .find_map(|editor| editor.read(cx).modified_practice_file(&directory))
        {
            self.academy_practice.notice = Some(format!(
                "Save {message} before checking so the result includes your latest edits."
            ));
            cx.notify();
            return;
        }
        let Some(lesson_data) = self
            .academy_library
            .as_ref()
            .and_then(|library| library.course(&course))
            .and_then(|course| course.lessons.get(&lesson))
        else {
            return;
        };
        let Some(exercise) = lesson_data.meta.exercises.get(index) else {
            return;
        };
        let check = exercise.check.clone();
        let ids = lesson_data
            .meta
            .exercises
            .iter()
            .map(crate::academy_progress::exercise_id)
            .collect::<Vec<_>>();
        let exercise_id = ids[index].clone();
        self.academy_practice.busy = true;
        self.academy_practice.notice = Some("Checking your saved work…".into());
        cx.notify();
        cx.spawn(
            move |workspace: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    let result = cx
                        .background_executor()
                        .spawn(async move { practice::run_check(&directory, &check) })
                        .await;
                    let _ = workspace.update(&mut cx, |this, cx| {
                        this.academy_practice.busy = false;
                        this.academy_practice.notice = None;
                        if result.status == CheckStatus::Passed {
                            let required = ids.iter().map(String::as_str).collect::<Vec<_>>();
                            this.academy_progress.record_exercise_passed(
                                &course,
                                &lesson,
                                &exercise_id,
                                &required,
                            );
                            this.persist_academy_progress();
                        }
                        this.academy_practice.results.insert(key, result);
                        cx.notify();
                    });
                }
            },
        )
        .detach();
    }
}
