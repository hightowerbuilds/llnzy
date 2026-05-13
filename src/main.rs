fn main() {
    env_logger::init();

    // Headless CLI dispatch must run before any GUI setup so
    // `llnzy stacker ...` / `llnzy prompt ...` can be used by agents and scripts.
    let argv: Vec<String> = std::env::args().collect();
    if matches!(argv.get(1).map(String::as_str), Some("stacker" | "prompt")) {
        std::process::exit(llnzy::stacker::cli::run_from_env());
    }

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = format!("llnzy panic: {}\n", info);
        let _ = llnzy::diagnostics::write_diagnostic("crash.log", &msg);
        default_hook(info);
    }));

    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
    }

    llnzy::gpui_workspace::run_workspace_prototype();
}
