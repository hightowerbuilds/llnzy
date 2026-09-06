fn main() {
    llnzy::error_log::install_logger();

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = format!("llnzy panic: {}\n", info);
        let _ = llnzy::diagnostics::write_diagnostic("crash.log", &msg);
        default_hook(info);
    }));

    #[cfg(unix)]
    unsafe {
        // SAFETY: Installing SIG_IGN for SIGPIPE is a process-wide Unix signal
        // disposition change. The handler does not dereference pointers or call
        // into Rust; it asks libc to ignore SIGPIPE so closed pipe writes report
        // normal I/O errors instead of terminating the app.
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
    }

    llnzy::gpui_workspace::run_workspace_prototype();
}
