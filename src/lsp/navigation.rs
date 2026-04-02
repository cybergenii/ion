//! Go-to-definition using libclang (same parse flags as `ion check` semantic engine).
//!
//! LSP positions use UTF-16 code units per line; libclang `clang_getLocation` / spelling columns use
//! 1-based UTF-8 byte offsets within the line. We convert between the two.

use crate::linter::engine::compile_args_for;
use clang::source::SourceRange;
use clang::{Clang, EntityKind, Index, Unsaved};
use std::borrow::Cow;
use std::path::Path;
use tower_lsp::lsp_types::{Location, Position, Range, Url};

fn entity_kind_is_invalid(kind: EntityKind) -> bool {
    matches!(
        kind,
        EntityKind::InvalidFile | EntityKind::InvalidDecl | EntityKind::InvalidCode
    )
}

/// UTF-16 code units occupied by `c` in UTF-16 (BMP = 1, supplementary = 2).
fn utf16_len_char(c: char) -> u32 {
    if (c as u32) <= 0xFFFF {
        1
    } else {
        2
    }
}

/// Total UTF-16 length of `s`.
fn utf16_len_str(s: &str) -> u32 {
    s.chars().map(utf16_len_char).sum()
}

/// LSP `character` (UTF-16 offset from line start) → byte offset within the line for libclang.
fn utf16_offset_to_byte_offset(line: &str, utf16_off: u32) -> Option<u32> {
    let mut seen = 0u32;
    let mut byte_idx = 0usize;
    for c in line.chars() {
        let w = utf16_len_char(c);
        if seen == utf16_off {
            return Some(byte_idx as u32);
        }
        if seen + w > utf16_off {
            return Some(byte_idx as u32);
        }
        seen += w;
        byte_idx += c.len_utf8();
    }
    if seen == utf16_off {
        return Some(line.len() as u32);
    }
    None
}

fn line_at(source: &str, line0: u32) -> Option<&str> {
    source.lines().nth(line0 as usize)
}

/// libclang 1-based column (UTF-8 byte index from line start + 1) → LSP `character` (UTF-16).
fn clang_column_to_utf16(line: &str, col_1based: u32) -> u32 {
    let byte0 = col_1based.saturating_sub(1) as usize;
    let byte_end = byte0.min(line.len());
    utf16_len_str(line.get(..byte_end).unwrap_or(""))
}

fn spelling_to_position(loc: &clang::source::Location, file_text: &str) -> Option<Position> {
    if loc.line == 0 {
        return None;
    }
    let line0 = loc.line.saturating_sub(1);
    let character = line_at(file_text, line0)
        .map(|l| clang_column_to_utf16(l, loc.column))
        .unwrap_or_else(|| loc.column.saturating_sub(1));
    Some(Position {
        line: line0,
        character,
    })
}

/// If end equals start (clang sometimes does), nudge end one UTF-16 unit for a non-empty range.
fn nudge_end_if_empty(start: Position, mut end: Position) -> Position {
    if start.line == end.line && start.character == end.character {
        end.character = end.character.saturating_add(1);
    }
    end
}

fn file_text_for<'a>(path: &Path, primary: &Path, primary_src: &'a str) -> Cow<'a, str> {
    if path == primary {
        Cow::Borrowed(primary_src)
    } else {
        match std::fs::read_to_string(path) {
            Ok(s) => Cow::Owned(s),
            Err(_) => Cow::Owned(String::new()),
        }
    }
}

/// Resolve the definition of the symbol at `line0` / `character0` (LSP 0-based).
pub fn goto_definition(file: &Path, source: &str, line0: u32, character0: u32) -> Option<Location> {
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
    let line_str = line_at(source, line0)?;
    let byte_off =
        utf16_offset_to_byte_offset(line_str, character0).unwrap_or(line_str.len() as u32);
    let line = line0.saturating_add(1);
    let col = byte_off.saturating_add(1);
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
    let start_path = start_sl.file.as_ref()?.get_path();
    let end_path = end_sl.file.as_ref()?.get_path();
    let start_text = file_text_for(&start_path, file, source);
    let end_text = file_text_for(&end_path, file, source);
    let start = spelling_to_position(&start_sl, start_text.as_ref())?;
    let end_raw = spelling_to_position(&end_sl, end_text.as_ref())?;
    let end = nudge_end_if_empty(start, end_raw);
    Some(Location {
        uri,
        range: Range { start, end },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_byte_roundtrip_ascii() {
        let line = "hello";
        assert_eq!(utf16_offset_to_byte_offset(line, 0), Some(0));
        assert_eq!(utf16_offset_to_byte_offset(line, 5), Some(5));
        assert_eq!(clang_column_to_utf16(line, 6), 5);
    }

    #[test]
    fn utf16_emoji_line() {
        let line = "a😀b";
        assert_eq!(utf16_offset_to_byte_offset(line, 1), Some(1));
        assert_eq!(utf16_offset_to_byte_offset(line, 3), Some(5));
        assert_eq!(clang_column_to_utf16(line, 2), 1);
        // libclang column = 1-based byte offset; col 6 → bytes [0..5) = "a😀" → 3 UTF-16 units; col 7 → full line → 4
        assert_eq!(clang_column_to_utf16(line, 6), 3);
        assert_eq!(clang_column_to_utf16(line, 7), 4);
    }
}
