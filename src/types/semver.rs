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
        let parsed = if semver::Version::parse(&raw).is_ok() {
            semver::VersionReq::parse(&format!("={raw}"))
        } else {
            semver::VersionReq::parse(&raw)
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

    fn parse(s: &str) -> VersionReq {
        serde_json::from_str(&format!("\"{s}\"")).unwrap()
    }

    #[test]
    fn bare_full_version_becomes_exact() {
        // Diverges from Cargo: "1.2.3" → "=1.2.3", not "^1.2.3".
        assert_eq!(parse("1.2.3").as_ref(), &semver::VersionReq::parse("=1.2.3").unwrap());
        assert_eq!(
            parse("0.8.9").as_ref(),
            &semver::VersionReq::parse("=0.8.9").unwrap(),
        );
    }

    #[test]
    fn pre_release_and_build_metadata_also_exact() {
        assert_eq!(
            parse("1.2.3-alpha.1").as_ref(),
            &semver::VersionReq::parse("=1.2.3-alpha.1").unwrap(),
        );
        assert_eq!(
            parse("1.2.3+meta").as_ref(),
            &semver::VersionReq::parse("=1.2.3+meta").unwrap(),
        );
    }

    #[test]
    fn partial_versions_stay_as_caret_requirements() {
        // These aren't valid `Version`s, so the fallback parse runs unchanged.
        assert_eq!(parse("1.2").as_ref(), &semver::VersionReq::parse("1.2").unwrap());
        assert_eq!(parse("1").as_ref(), &semver::VersionReq::parse("1").unwrap());
    }

    #[test]
    fn explicit_requirements_pass_through() {
        assert_eq!(parse("^1.2.3").as_ref(), &semver::VersionReq::parse("^1.2.3").unwrap());
        assert_eq!(parse("=1.2.3").as_ref(), &semver::VersionReq::parse("=1.2.3").unwrap());
        assert_eq!(parse("~1.2").as_ref(), &semver::VersionReq::parse("~1.2").unwrap());
        assert_eq!(
            parse(">=1.0, <2").as_ref(),
            &semver::VersionReq::parse(">=1.0, <2").unwrap(),
        );
        assert_eq!(parse("*").as_ref(), &semver::VersionReq::STAR);
    }
}
