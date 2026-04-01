use crate::linter::diagnostic::{Diagnostic, Severity};
use anyhow::Result;
use colored::Colorize;
use serde_json::json;
use std::fs;

#[derive(Clone, Copy)]
pub enum OutputFormat {
    Text,
    Json,
    Sarif,
}

pub fn report(diagnostics: &[Diagnostic], format: OutputFormat, color: bool) -> Result<()> {
    match format {
        OutputFormat::Text => report_text(diagnostics, color),
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(diagnostics)?);
            Ok(())
        }
        OutputFormat::Sarif => {
            println!("{}", serde_json::to_string_pretty(&to_sarif(diagnostics))?);
            Ok(())
        }
    }
}

fn report_text(diagnostics: &[Diagnostic], color: bool) -> Result<()> {
    for d in diagnostics {
        let sev = match d.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
            Severity::Hint => "hint",
        };
        let sev = if color {
            match d.severity {
                Severity::Error => sev.red().to_string(),
                Severity::Warning => sev.yellow().to_string(),
                Severity::Info => sev.blue().to_string(),
                Severity::Hint => sev.cyan().to_string(),
            }
        } else {
            sev.to_string()
        };
        println!(
            "{}:{}:{}: {}[{}]: {}",
            d.file.display(),
            d.line,
            d.column,
            sev,
            d.rule,
            d.message
        );
        if let Ok(content) = fs::read_to_string(&d.file) {
            if let Some(line) = content.lines().nth(d.line.saturating_sub(1) as usize) {
                println!("   |");
                println!("{:>3} | {}", d.line, line);
            }
        }
        if let Some(s) = &d.suggestion {
            println!("   = suggestion: {s}");
        }
        println!();
    }
    Ok(())
}

fn to_sarif(diagnostics: &[Diagnostic]) -> serde_json::Value {
    let results = diagnostics
        .iter()
        .map(|d| {
            json!({
                "ruleId": d.rule,
                "level": match d.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    Severity::Info => "note",
                    Severity::Hint => "note",
                },
                "message": { "text": d.message },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": d.file.to_string_lossy() },
                        "region": { "startLine": d.line, "startColumn": d.column }
                    }
                }]
            })
        })
        .collect::<Vec<_>>();
    json!({
        "version": "2.1.0",
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "runs": [{
            "tool": { "driver": { "name": "ion", "informationUri": "https://github.com/cybergenii/ion" }},
            "results": results
        }]
    })
}
