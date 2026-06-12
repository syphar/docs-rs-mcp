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

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
pub(crate) struct VersionReq(#[schemars(with = "String")] semver::VersionReq);

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
