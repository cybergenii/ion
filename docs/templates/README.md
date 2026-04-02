# Ion documentation templates

Templates for **library and application maintainers** who use Ion (`ion.toml`) in their own repositories. They are maintained alongside the Ion CLI and registry design documents.

## Contents

| Path | Purpose |
|------|---------|
| [github-actions/ion-library-ci.yml](github-actions/ion-library-ci.yml) | GitHub Actions workflow: install Ion, `ion install`, build, test, `ion check`, optional publish hook |
| [github-actions/README.md](github-actions/README.md) | How to copy, customize, and secure the workflow |

## Related specifications

- [Publish API](../PUBLISH_API.md) — HTTP API for uploading versions (registry service)
- [Ecosystem architecture](../ECOSYSTEM_ARCHITECTURE.md) — how catalog scale fits together

## Copying into your project

1. Copy `github-actions/ion-library-ci.yml` to `.github/workflows/ion-ci.yml` (or merge jobs into your existing CI).
2. Set `env.ION_VERSION` to a released Ion version and ensure tag `v…` exists on GitHub (or change the install step to use crates.io — see template README).
3. Enable **semantic** `ion check` where `libclang` is installed (Ubuntu job includes `libclang-dev`).
4. For automated publishing, configure secrets only after your registry implements [PUBLISH_API.md](../PUBLISH_API.md).

Do not commit API tokens. Use GitHub **encrypted secrets** (`Settings → Secrets and variables → Actions`).
