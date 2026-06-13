use rmcp::schemars::{self, JsonSchema};
use std::ops::Deref;

#[derive(
    Clone, Debug, Eq, PartialEq, Ord, PartialOrd, serde::Deserialize, serde::Serialize, JsonSchema,
)]
#[serde(transparent)]
#[schemars(transparent)]
pub(crate) struct Version(#[schemars(with = "String")] semver::Version);

impl AsRef<semver::Version> for Version {
    fn as_ref(&self) -> &semver::Version {
        &self.0
    }
}

impl Deref for Version {
    type Target = semver::Version;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Version> for semver::Version {
    fn from(version: Version) -> Self {
        version.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
pub(crate) struct VersionReq(#[schemars(with = "String")] semver::VersionReq);

/// Custom deserializer that matches docs.rs URL semantics instead of Cargo's.
///
/// Cargo treats a bare `"1.2.3"` as `^1.2.3` (latest 1.x ≥ 1.2.3), which is
/// surprising for AIs and users who say "axum 0.8.9" meaning *that exact
/// version*. We diverge: any input that parses as a fully-qualified
/// `semver::Version` (`MAJOR.MINOR.PATCH[-pre][+build]`) is normalized to
/// `=<input>`, an exact-match requirement. Everything else (`"1.2"`, `"^1.0"`,
/// `"~1.2"`, `">=1, <2"`, `"*"`) is parsed as a Cargo-style requirement
/// unchanged.
impl<'de> serde::Deserialize<'de> for VersionReq {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        let trimmed = raw.trim();
        // Friendly aliases for "latest published" — case-insensitive.
        if trimmed.eq_ignore_ascii_case("latest") || trimmed.eq_ignore_ascii_case("newest") {
            return Ok(Self(semver::VersionReq::STAR));
        }
        let parsed = if semver::Version::parse(trimmed).is_ok() {
            semver::VersionReq::parse(&format!("={trimmed}"))
        } else {
            semver::VersionReq::parse(trimmed)
        };
        parsed.map(Self).map_err(serde::de::Error::custom)
    }
}

impl Default for VersionReq {
    fn default() -> Self {
        Self(semver::VersionReq::STAR)
    }
}

impl AsRef<semver::VersionReq> for VersionReq {
    fn as_ref(&self) -> &semver::VersionReq {
        &self.0
    }
}

impl Deref for VersionReq {
    type Target = semver::VersionReq;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<VersionReq> for semver::VersionReq {
    fn from(req: VersionReq) -> Self {
        req.0
    }
}

impl From<semver::VersionReq> for VersionReq {
    fn from(value: semver::VersionReq) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    fn parse(s: &str) -> VersionReq {
        serde_json::from_str(&format!("\"{s}\"")).unwrap()
    }

    // Diverges from Cargo: bare `MAJOR.MINOR.PATCH[-pre][+build]` → exact match.
    #[test_case("1.2.3",         "=1.2.3"        ; "bare triplet")]
    #[test_case("0.8.9",         "=0.8.9"        ; "the case that bit us")]
    #[test_case("1.2.3-alpha.1", "=1.2.3-alpha.1"; "pre-release")]
    #[test_case("1.2.3+meta",    "=1.2.3+meta"   ; "build metadata")]
    fn bare_full_version_becomes_exact(input: &str, expected: &str) {
        assert_eq!(parse(input).as_ref(), &semver::VersionReq::parse(expected).unwrap());
    }

    // Partial versions and explicit Cargo-style requirements are parsed unchanged.
    #[test_case("1.2",        "1.2"       ; "partial minor")]
    #[test_case("1",          "1"         ; "partial major")]
    #[test_case("^1.2.3",     "^1.2.3"    ; "caret")]
    #[test_case("=1.2.3",     "=1.2.3"    ; "explicit equals")]
    #[test_case("~1.2",       "~1.2"      ; "tilde")]
    #[test_case(">=1.0, <2",  ">=1.0, <2" ; "range")]
    fn cargo_style_requirements_pass_through(input: &str, expected: &str) {
        assert_eq!(parse(input).as_ref(), &semver::VersionReq::parse(expected).unwrap());
    }

    // `*`, `latest`, `newest` (case-insensitive, whitespace-tolerant) → STAR.
    #[test_case("*"        ; "star")]
    #[test_case("latest"   ; "latest lowercase")]
    #[test_case("Latest"   ; "latest mixed case")]
    #[test_case("LATEST"   ; "latest uppercase")]
    #[test_case("newest"   ; "newest lowercase")]
    #[test_case("Newest"   ; "newest mixed case")]
    #[test_case(" latest " ; "whitespace padded")]
    fn aliases_for_latest_resolve_to_star(input: &str) {
        assert_eq!(parse(input).as_ref(), &semver::VersionReq::STAR);
    }
}
