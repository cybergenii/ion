# Ion registry — publish API (specification)

This document defines an **HTTP API** for uploading package versions to the Ion registry. It is a **design specification**: the registry service that implements it may live outside this repository. Client support (for example `ion publish`) can follow this contract once the service is available.

**Protocol:** HTTPS only.  
**Base URL (example):** `https://registry.ion-cpp.dev`  
**API prefix:** `/api/v1`

---

## 1. General conventions

### 1.1 Versioning

Breaking changes to the API surface require a new prefix (`/api/v2`). Non-breaking additions (optional fields, new endpoints) remain under `/api/v1`.

### 1.2 Content types

| Usage | `Content-Type` |
|-------|----------------|
| JSON request/response | `application/json; charset=utf-8` |
| Tarball upload | `application/gzip` or `application/x-tar` with `Content-Encoding: gzip` as negotiated per endpoint |

### 1.3 Authentication

All **mutating** endpoints require authentication.

```
Authorization: Bearer <token>
```

Tokens are **API keys** issued by the registry (scoped, revocable). Optional prefix for clarity: `ion_pat_…`.

**Scopes (examples):**

| Scope | Allows |
|-------|--------|
| `package:publish` | Upload new versions for packages owned by the key |
| `package:yank` | Yank versions |
| `package:read` | Read private package metadata (future) |

Requests without sufficient scope return `403 Forbidden`.

### 1.4 Idempotency

`POST` requests that create resources accept an optional header:

```
Idempotency-Key: <opaque string>
```

Repeating the same key with the same body within a server-defined window (for example 24 hours) returns the **same** response as the first success, without creating a duplicate version.

### 1.5 Rate limiting

Responses may include:

```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: <unix timestamp>
```

`429 Too Many Requests` with a `Retry-After` header when exceeded.

### 1.6 Error model

Errors use **JSON** with a stable shape (aligned with [RFC 7807 Problem Details](https://datatracker.ietf.org/doc/html/rfc7807) where practical):

```json
{
  "type": "https://registry.ion-cpp.dev/errors/validation-failed",
  "title": "Validation failed",
  "status": 422,
  "detail": "checksum does not match tarball bytes",
  "instance": "/api/v1/packages/acme-lib/versions",
  "errors": [
    { "field": "checksum", "message": "expected sha256:…" }
  ]
}
```

| Field | Meaning |
|-------|---------|
| `type` | URI identifying the error kind (stable, documentation link) |
| `title` | Short human-readable summary |
| `status` | HTTP status code |
| `detail` | Specific explanation |
| `instance` | Request path or correlation id |
| `errors` | Optional list of field-level issues |

---

## 2. Endpoints

### 2.1 Health

**`GET /api/v1/health`**

**Auth:** none.

**Response `200 OK`:**

```json
{
  "status": "ok",
  "version": "1.2.0"
}
```

Use for load balancers and CI smoke checks.

---

### 2.2 Publish a package version

**`POST /api/v1/packages/{name}/versions`**

**Auth:** `package:publish` for `{name}`.

**Path:**

| Parameter | Description |
|-----------|-------------|
| `name` | Package name (lowercase, `[a-z0-9][a-z0-9-]*` or documented namespaced form `scope/name`) |

**Request headers:**

| Header | Required | Description |
|--------|----------|-------------|
| `Authorization` | Yes | Bearer token |
| `Content-Type` | Yes | `multipart/form-data` **or** `application/json` (see below) |
| `Idempotency-Key` | No | Idempotency token |

**Option A — `multipart/form-data` (recommended for large tarballs)**

| Part | Type | Description |
|------|------|-------------|
| `metadata` | JSON string | See **Metadata object** |
| `artifact` | file | Gzip-compressed tarball (`.tar.gz`) |

**Option B — `application/json` (metadata only + pre-signed upload)**

Response `202 Accepted` with upload instructions (S3 POST fields or `PUT` URL). Client uploads the blob, then calls **complete** (below). Use when tarballs are large and should not pass through the API process.

**Metadata object** (same for multipart JSON part or Option B body):

```json
{
  "version": "1.4.2",
  "checksum": "sha256:abcdef…",
  "checksum_algorithm": "sha256",
  "manifest": { },
  "cmake_targets": ["acme::acme"],
  "dependencies": [
    { "name": "fmt", "version_req": "^10.0", "optional": false }
  ],
  "features": ["async"],
  "description": "Short summary",
  "homepage": "https://example.com/acme-lib",
  "license": "MIT",
  "repository": "https://github.com/acme/acme-lib",
  "commit": "abc123…"
}
```

| Field | Required | Description |
|-------|----------|-------------|
| `version` | Yes | Semver `MAJOR.MINOR.PATCH` (+ optional prerelease/build per registry policy) |
| `checksum` | Yes | Hex digest with `sha256:` prefix or algorithm field |
| `checksum_algorithm` | No | Default `sha256` |
| `manifest` | No | Parsed `ion.toml` subset or full object for audit |
| `cmake_targets` | Yes | Targets consumers will link against |
| `dependencies` | No | Ion registry deps; must resolve against existing index |
| `features` | No | Feature flags exposed by this version |
| `description`, `homepage`, `license` | Recommended | Shown in index and search |
| `repository`, `commit` | No | Provenance |

**Server validation (non-exhaustive):**

1. `version` is newer than existing non-yanked versions (or registry allows prerelease channels).
2. Tarball bytes match `checksum`.
3. Tarball unpacks to a tree containing **`ion.toml`** at root and passes structural checks.
4. Optional: run **policy hooks** (license allowlist, virus scan, size limits).

**Response `201 Created`:**

```json
{
  "package": "acme-lib",
  "version": "1.4.2",
  "index_url": "https://registry.ion-cpp.dev/index/acme-lib.json",
  "tarball_url": "https://…",
  "published_at": "2026-04-02T12:00:00Z"
}
```

**Response `409 Conflict`:** version already exists (non-idempotent duplicate).

**Response `422 Unprocessable Entity`:** validation failed (see error model).

---

### 2.3 Complete multipart upload (optional flow)

**`POST /api/v1/packages/{name}/versions/{version}/complete`**

**Auth:** `package:publish`.

Used after **Option B** pre-signed upload to confirm the blob landed and trigger validation.

**Body:**

```json
{
  "upload_id": "…",
  "checksum": "sha256:…"
}
```

**Response `201 Created`:** same as publish success.

---

### 2.4 Yank a version

**`DELETE /api/v1/packages/{name}/versions/{version}`**

**Auth:** `package:yank`.

Marks the version as **yanked** in the index (`yanked: true`). Does **not** delete the tarball blob (immutability for reproducibility).

**Response `204 No Content`**

**Response `404 Not Found`:** unknown package or version.

---

### 2.5 List versions (maintainer)

**`GET /api/v1/packages/{name}/versions`**

**Auth:** `package:read` or `package:publish` (policy-dependent).

**Response `200 OK`:**

```json
{
  "package": "acme-lib",
  "versions": [
    {
      "version": "1.4.2",
      "yanked": false,
      "published_at": "2026-04-02T12:00:00Z"
    }
  ]
}
```

---

## 3. CI integration

CI systems should **not** embed long-lived tokens in workflow YAML. Recommended:

1. Store `ION_REGISTRY_TOKEN` (or `REGISTRY_TOKEN`) in **GitHub Actions secrets** / GitLab CI variables.
2. Call `POST …/packages/{name}/versions` from a **release** job that runs only on protected tags (`v*`) or `workflow_dispatch`.
3. Use **`Idempotency-Key`** derived from `${{ github.repository }}-${{ github.sha }}-${{ github.run_id }}` to avoid duplicate publishes on retries.

See [templates/github-actions/README.md](templates/github-actions/README.md) for a workflow that builds with Ion and optionally invokes this API via `curl`.

---

## 4. Security considerations

- **TLS:** all traffic encrypted; pin registry hostname in CI.
- **Token rotation:** support multiple active keys per maintainer; revoke on leak.
- **Audit log:** server records who published what, when, and from which IP / CI job id.
- **Supply chain:** encourage **signed commits** and **SLSA**-style attestations as a later phase.

---

## 5. Changelog

| Date | Change |
|------|--------|
| 2026-04 | Initial specification (design-only; implementation may differ) |
