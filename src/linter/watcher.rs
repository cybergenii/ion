use crate::linter::Linter;
use anyhow::Result;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::SystemTime;

pub fn watch_src(linter: &Linter) -> Result<()> {
    let (tx, rx) = channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
    watcher.watch(Path::new("src"), RecursiveMode::Recursive)?;
    println!("[ion] Watching src/ for changes — Ctrl+C to stop");
    loop {
        let event = rx.recv()?;
        if matches!(event.kind, EventKind::Modify(_)) {
            for p in event.paths {
                if is_cpp_file(&p) {
                    print!("\x1B[2J\x1B[H");
                    println!("[ion] {}", humantime(SystemTime::now()));
                    let diagnostics = linter.run_on_files(&[p], None)?;
                    crate::linter::reporter::report(
                        &diagnostics,
                        crate::linter::reporter::OutputFormat::Text,
                        true,
                    )?;
                }
            }
        }
    }
}

fn is_cpp_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("cpp" | "cc" | "cxx" | "h" | "hpp")
    )
}

fn humantime(ts: SystemTime) -> String {
    let dt: chrono::DateTime<chrono::Local> = ts.into();
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}
