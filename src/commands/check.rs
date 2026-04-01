use crate::linter::diagnostic::Severity;
use crate::linter::fixer::Fixer;
use crate::linter::reporter::{report, OutputFormat};
use crate::linter::rules::{describe_rule, is_known_rule_id, KNOWN_RULE_IDS};
use crate::linter::watcher;
use crate::linter::Linter;
use anyhow::{bail, Result};

pub async fn execute(
    fix: bool,
    watch: bool,
    format: &str,
    rule: Option<String>,
    list_rules: bool,
    no_color: bool,
) -> Result<()> {
    if list_rules {
        println!("[ion] Known lint rule ids:");
        for id in KNOWN_RULE_IDS {
            println!("  {:<24} {}", id, describe_rule(id));
        }
        return Ok(());
    }

    let rule_filter: Option<Vec<String>> = match &rule {
        None => None,
        Some(s) => {
            let ids: Vec<String> = s
                .split(',')
                .map(str::trim)
                .filter(|x| !x.is_empty())
                .map(String::from)
                .collect();
            if ids.is_empty() {
                None
            } else {
                for id in &ids {
                    if !is_known_rule_id(id) {
                        bail!(
                            "[ion] error: unknown rule id `{id}`\n\
                             Run `ion check --list-rules` for valid ids."
                        );
                    }
                }
                Some(ids)
            }
        }
    };

    let filter_slice = rule_filter.as_deref();

    let linter = Linter::new();
    if !linter.semantic_available() {
        println!("[ion] warning: libclang not found — semantic checks disabled");
    }
    if watch {
        watcher::watch_src(&linter, filter_slice)?;
        return Ok(());
    }
    let (diagnostics, summary) = linter.run(filter_slice)?;
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
