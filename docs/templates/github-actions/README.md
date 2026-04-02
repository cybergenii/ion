# GitHub Actions template for Ion-based C++ projects

## What this is

[`ion-library-ci.yml`](ion-library-ci.yml) is a **reference workflow** you copy into **your** repository. It is not executed by the Ion repository itself; it documents a production-style pipeline for projects that use `ion.toml`.

## Quick start

1. Ensure your repo has **`ion.toml`** at the root and builds with `ion install` / `ion build` locally.
2. Copy the file:

   ```text
   cp docs/templates/github-actions/ion-library-ci.yml  .github/workflows/ion-ci.yml
   ```

   (If you are not developing Ion itself, copy from the Ion release or raw GitHub URL instead of a local path.)

3. Commit `.github/workflows/ion-ci.yml` and open a pull request to verify CI passes.

## Customization

| Item | Suggestion |
|------|------------|
| **Ion version** | Set `env.ION_VERSION` (e.g. `0.3.0`); the workflow installs from Git tag `v${ION_VERSION}` via `cargo install --git …`. Ensure that tag exists on [cybergenii/ion](https://github.com/cybergenii/ion/releases). Alternatively switch the install step to `cargo install ionx --version …` from [crates.io/crates/ionx](https://crates.io/crates/ionx) if the published version meets your needs. |
| **Runners** | Default: Ubuntu + macOS for build/test/lint; Windows in a separate smoke job. Add `windows-latest` to the main matrix if you need full parity. |
| **`ion check`** | On Ubuntu, `libclang-dev` is installed for semantic rules. On macOS, install LLVM/libclang if you require the same depth (optional follow-up step). |
| **Publish job** | Leave disabled until [PUBLISH_API.md](../../PUBLISH_API.md) is implemented. Then set `ION_REGISTRY_TOKEN` and optionally `ION_REGISTRY_URL`, and uncomment the `curl` placeholder in the workflow. |

## Secrets (publish only)

| Secret | Meaning |
|--------|---------|
| `ION_REGISTRY_TOKEN` | Bearer token with `package:publish` scope |
| `ION_REGISTRY_URL` | Optional; defaults to `https://registry.ion-cpp.dev` in the example script |

Never echo secrets in logs. Use environment-scoped or repository secrets with **restricted** branches/tags for production publishes.

## Idempotency

When the publish API is live, set `Idempotency-Key` to a value derived from the GitHub run (for example `${{ github.run_id }}-${{ github.sha }}`) so retries do not create duplicate versions.

## Support

Issues with **Ion itself**: [github.com/cybergenii/ion/issues](https://github.com/cybergenii/ion/issues).  
Issues with **your** workflow after copying: adjust matrix, paths, and secrets in your repo.
