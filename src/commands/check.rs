use crate::linter::diagnostic::Severity;
use crate::linter::fixer::Fixer;
use crate::linter::reporter::{report, OutputFormat};
use crate::linter::watcher;
use crate::linter::Linter;
use anyhow::Result;

pub async fn execute(
    fix: bool,
    watch: bool,
    format: &str,
    rule: Option<&str>,
    no_color: bool,
) -> Result<()> {
    let linter = Linter::new();
    if !linter.semantic_available() {
        println!("[ion] warning: libclang not found — semantic checks disabled");
    }
    if watch {
        watcher::watch_src(&linter)?;
        return Ok(());
    }
    let (diagnostics, summary) = linter.run(rule)?;
    if fix {
        let fixer = Fixer;
        fixer.apply_all(&diagnostics)?;
    }
    report(
        &diagnostics,
        match format {
            "json" => OutputFormat::Json,
            "sarif" => OutputFormat::Sarif,
            _ => OutputFormat::Text,
        },
        !no_color,
    )?;
    println!(
        "[ion] {} errors, {} warnings in {} files ({:.1}s)",
        summary.errors, summary.warnings, summary.files, summary.elapsed_secs
    );
    if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        std::process::exit(1);
    }
    Ok(())
}
