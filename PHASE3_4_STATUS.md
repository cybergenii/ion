# Ion Phase 3/4 Status

Internal snapshot of lint (`ion check`) and editor (`ion lsp`) capabilities. **End users** should rely on [README.md](README.md) (features, roadmap, prerequisites). This file is for contributors tracing what shipped in which area.

## Implemented

- Phase 3 linting framework: file discovery, diagnostics, reporting, rule execution, fixing, and watch mode.
- `ion check` command with:
  - `--fix`
  - `--watch`
  - `--format text|json|sarif`
  - `--rule <id[,id...]>`
  - `--list-rules`
  - `--no-color`
- Semantic analysis context (`SemanticContext`) for libclang rules: enclosing function name, full file source for cross-checks.
- Textual analysis always runs (even without libclang): tree-sitter modern checks, **function-scoped** dataflow (leak / use-after-free with reassignment clearing), smart-pointer heuristics.
- Smart-pointer / ownership heuristics (`src/analysis/smart_ptr.rs`): `memory/smart-get`, `memory/raw-from-smart`, `memory/move-after-use`, `memory/shared-cycle-hint`.
- Refined semantic rules: double-free only when duplicate free/delete matches the same variable in-file; `null/deref` skips `*this`; resource leak skips `std::`/`filesystem::` call patterns; memory leak skips `make_unique`/`make_shared`.
- Auto-fix: `modern/c-cast` suggests `static_cast` with machine-applicable `Fix::Replace` when a simple `(Type)expr` is detected.
- Phase 4 LSP:
  - `ion lsp` command
  - Full document sync with `did_open` / `did_change` / `did_save` / `did_close`
  - In-memory buffers for unsaved files (same pipeline as CLI via `analyze_file_with_source`)
  - Diagnostics include `code` and `code_description`; notes map to `related_information` when a file URL resolves
  - Quick-fix code actions filtered by request range and linked to diagnostics
  - Hover for diagnostic spans
  - Go-to-definition (`textDocument/definition`): resolves symbol at cursor via libclang (`get_reference` → `get_definition`), same compile args as `ion check`; LSP UTF-16 columns ↔ libclang 1-based UTF-8 byte columns within each line; definition ranges load other files from disk when needed

## Runtime behavior with and without libclang

- If `libclang` is available:
  - semantic checks run via `src/linter/engine.rs` (with unsaved buffers when used from LSP)
- If `libclang` is not available:
  - tree-sitter + dataflow + smart-pointer heuristics still run
  - LSP go-to-definition is unavailable (returns no result)
  - Ion prints:
    - `[ion] warning: libclang not found — semantic checks disabled`

## Known limitations

- Semantic rules still use heuristics; some findings may be false positives or negatives.
- CFG builder remains a foundation for deeper path-sensitive analysis.
- Smart-pointer checks are pattern-based, not full ownership/CFG analysis.
- `memory/shared-cycle-hint` may trigger on benign multi-`shared_ptr` designs; treat as informational.
