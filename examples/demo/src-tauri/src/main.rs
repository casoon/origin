// The windows_subsystem attribute keeps a console window from appearing next to the
// GUI in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // An AI client starts this binary as a child process and talks to it over stdio.
    // The branch happens before anything Tauri-related, so no window is created and no
    // desktop session is required.
    if std::env::args().any(|argument| argument == "--mcp") {
        if let Err(error) = origin_demo::run_mcp() {
            eprintln!("mcp server failed: {error}");
            std::process::exit(1);
        }
        return;
    }

    origin_demo::run();
}
