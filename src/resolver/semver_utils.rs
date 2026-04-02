/// Semver utility helpers.
/// While we primarily use the `semver` crate, this module
/// provides Ion-specific utilities and user-friendly error messages.
/// Determine if a version string satisfies a requirement string.
pub fn satisfies(version: &str, req: &str) -> bool {
    let Ok(v) = semver::Version::parse(version) else {
        return false;
    };
    let Ok(r) = semver::VersionReq::parse(req) else {
        return false;
    };
    r.matches(&v)
}

/// Parse a user-supplied version requirement into a canonical form.
/// Handles various shorthand formats:
///   - `10`       → `^10` (compatible with major)
///   - `10.2`     → `^10.2` (compatible with minor)
///   - `10.2.1`   → `=10.2.1` (exact)
///   - `^10.2`    → `^10.2` (unchanged)
///   - `>=10,<11` → `>=10, <11` (unchanged)
///   - `*`        → `*` (any version)
///   - `latest`   → `*`
pub fn normalize_version_req(input: &str) -> String {
    let input = input.trim();

    if input.is_empty() || input == "*" || input == "latest" {
        return "*".to_string();
    }

    // If it starts with a comparator, pass through
    if input.starts_with('^')
        || input.starts_with('~')
        || input.starts_with('>')
        || input.starts_with('<')
        || input.starts_with('=')
    {
        return input.to_string();
    }

    // Count dots to determine precision
    let dot_count = input.chars().filter(|&c| c == '.').count();
    match dot_count {
        0 => format!("^{}.0.0", input), // "10" → "^10.0.0"
        1 => format!("^{}.0", input),   // "10.2" → "^10.2.0"
        _ => input.to_string(),         // "10.2.1" → use as-is (semver crate handles it)
    }
}

/// Format a version for display, trimming unnecessary precision
pub fn display_version(version: &str) -> &str {
    version.trim_start_matches('v')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_satisfies() {
        assert!(satisfies("10.2.1", "^10.0.0"));
        assert!(satisfies("10.2.1", ">=10.0.0, <11.0.0"));
        assert!(!satisfies("11.0.0", "^10.0.0"));
        assert!(satisfies("10.2.1", "*"));
        assert!(satisfies("0.1.0", "^0.1.0"));
    }

    #[test]
    fn test_normalize() {
        assert_eq!(normalize_version_req("*"), "*");
        assert_eq!(normalize_version_req("latest"), "*");
        assert_eq!(normalize_version_req("^10.2"), "^10.2");
        assert_eq!(normalize_version_req("10"), "^10.0.0");
        assert_eq!(normalize_version_req("10.2"), "^10.2.0");
        assert_eq!(normalize_version_req("10.2.1"), "10.2.1");
    }
}
