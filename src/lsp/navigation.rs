//! Go-to-definition using libclang (same parse flags as `ion check` semantic engine).
//!
//! LSP positions are UTF-16 code units; we map line + character as for ASCII/UTF-8 file bytes
//! (good enough for common C++ sources).

use crate::linter::engine::compile_args_for;
use clang::source::SourceRange;
use clang::{Clang, EntityKind, Index, Unsaved};
use std::path::Path;
use tower_lsp::lsp_types::{Location, Position, Range, Url};

fn entity_kind_is_invalid(kind: EntityKind) -> bool {
    matches!(
        kind,
        EntityKind::InvalidFile | EntityKind::InvalidDecl | EntityKind::InvalidCode
    )
}

/// Resolve the definition of the symbol at `line0` / `character0` (LSP 0-based).
pub fn goto_definition(
    file: &Path,
    source: &str,
    line0: u32,
    character0: u32,
) -> Option<Location> {
    let clang = Clang::new().ok()?;
    let index = Index::new(&clang, false, false);
    let mut parser = index.parser(file);
    let args = compile_args_for(file);
    if args.is_empty() {
        parser.arguments(&["-x", "c++", "-std=c++20"]);
    } else {
        let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
        parser.arguments(&refs);
    }
    parser.unsaved(&[Unsaved::new(file, source)]);
    let tu = parser.parse().ok()?;
    let cfile = tu.get_file(file)?;
    let line = line0.saturating_add(1);
    let col = character0.saturating_add(1);
    let sl = cfile.get_location(line, col);
    let entity = sl.get_entity()?;
    if entity_kind_is_invalid(entity.get_kind()) {
        return None;
    }
    let mut cur = entity;
    if let Some(r) = cur.get_reference() {
        if !entity_kind_is_invalid(r.get_kind()) {
            cur = r;
        }
    }
    let def = cur.get_definition().unwrap_or(cur);
    if entity_kind_is_invalid(def.get_kind()) {
        return None;
    }
    let extent = def.get_range().or_else(|| {
        let loc = def.get_location()?;
        Some(SourceRange::new(loc, loc))
    })?;
    let start_sl = extent.get_start().get_spelling_location();
    let end_sl = extent.get_end().get_spelling_location();
    let path = start_sl.file.as_ref()?.get_path();
    let uri = Url::from_file_path(&path).ok()?;
    Some(Location {
        uri,
        range: Range {
            start: clang_location_to_position(&start_sl)?,
            end: clang_location_to_position_end(&end_sl, &start_sl)?,
        },
    })
}

fn clang_location_to_position(loc: &clang::source::Location) -> Option<Position> {
    if loc.line == 0 {
        return None;
    }
    Some(Position {
        line: loc.line.saturating_sub(1),
        character: loc.column.saturating_sub(1),
    })
}

/// If end equals start (clang sometimes does), nudge end one character for a non-empty range.
fn clang_location_to_position_end(
    end: &clang::source::Location,
    start: &clang::source::Location,
) -> Option<Position> {
    let mut p = clang_location_to_position(end)?;
    let ps = clang_location_to_position(start)?;
    if p.line == ps.line && p.character == ps.character {
        p.character = p.character.saturating_add(1);
    }
    Some(p)
}
