# Ion Phase 3/4 Status

## Implemented

- Phase 3 linting framework: file discovery, diagnostics, reporting, rule execution, fixing, and watch mode.
- `ion check` command with:
  - `--fix`
  - `--watch`
  - `--format text|json|sarif`
  - `--rule <id>`
  - `--no-color`
- Phase 4 analysis modules:
  - CFG scaffolding via `petgraph`
  - lightweight forward-style dataflow checks for leaks and use-after-free
- Phase 4 LSP:
  - `ion lsp` command
  - Tower LSP server with open/save/close diagnostics
  - quick-fix code actions for `Fix::Replace`
  - hover details for flagged spans

## Runtime behavior with and without libclang

- If `libclang` is available:
  - semantic checks run via `src/linter/engine.rs`
  - dataflow checks are added into the same diagnostic pipeline
- If `libclang` is not available:
  - tree-sitter based checks still run
  - Ion prints:
    - `[ion] warning: libclang not found — semantic checks disabled`

## Known limitations

- Semantic rules currently use conservative heuristics and may report false positives.
- CFG builder is intentionally minimal and currently acts as a foundation for deeper path-sensitive analysis.
- Some modern checks are pattern-based and do not yet perform full semantic validation.
- Watch mode performs direct re-analysis of changed files; batching/debouncing can be improved.
