use crate::{client::get_source::fetch_cargo_manifest, config::Config};
use anyhow::Result;
use cargo_manifest::{Dependency as ManifestDep, DepsSet};
use serde::Serialize;

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DependencyKind {
    Normal,
    Dev,
    Build,
}

#[derive(Debug, Serialize)]
pub(crate) struct Dependency {
    /// Manifest key for the dep — note this can differ from the actual crate
    /// name when the dep uses `package = "..."` to rename it.
    pub(crate) name: String,
    /// Set to the real crate name when the manifest renames it via
    /// `package = "..."`; otherwise omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) package: Option<String>,
    pub(crate) kind: DependencyKind,
    /// Version requirement string from Cargo.toml, e.g. `"^1.0"`, `"=0.4.2"`,
    /// `"*"`. `None` for path/git-only deps with no version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) req: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub(crate) optional: bool,
    pub(crate) default_features: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) features: Vec<String>,
    /// `target` cfg expression when the dep is target-gated
    /// (`[target.'cfg(unix)'.dependencies]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) target: Option<String>,
}

/// One-level dep list (not a recursive tree — that would need fetching every
/// transitive crate). For each direct dep returns kind + version req + the
/// flags that affect its build.
pub(crate) async fn dependency_tree(
    config: &Config,
    krate: &str,
    version: &semver::Version,
) -> Result<Option<Vec<Dependency>>> {
    let Some(manifest) = fetch_cargo_manifest(config, krate, version).await? else {
        return Ok(None);
    };

    let mut out = Vec::new();
    collect_section(manifest.dependencies.as_ref(), DependencyKind::Normal, None, &mut out);
    collect_section(manifest.dev_dependencies.as_ref(), DependencyKind::Dev, None, &mut out);
    collect_section(
        manifest.build_dependencies.as_ref(),
        DependencyKind::Build,
        None,
        &mut out,
    );

    if let Some(targets) = manifest.target.as_ref() {
        for (target_cfg, t) in targets {
            collect_section(
                Some(&t.dependencies),
                DependencyKind::Normal,
                Some(target_cfg.clone()),
                &mut out,
            );
            collect_section(
                Some(&t.dev_dependencies),
                DependencyKind::Dev,
                Some(target_cfg.clone()),
                &mut out,
            );
            collect_section(
                Some(&t.build_dependencies),
                DependencyKind::Build,
                Some(target_cfg.clone()),
                &mut out,
            );
        }
    }

    out.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| format!("{:?}", a.kind).cmp(&format!("{:?}", b.kind)))
    });
    Ok(Some(out))
}

fn collect_section(
    section: Option<&DepsSet>,
    kind: DependencyKind,
    target: Option<String>,
    out: &mut Vec<Dependency>,
) {
    let Some(section) = section else { return };
    for (name, dep) in section {
        let (req, optional, default_features, features, package) = match dep {
            ManifestDep::Simple(req) => (Some(req.clone()), false, true, Vec::new(), None),
            ManifestDep::Detailed(d) => (
                d.version.clone(),
                d.optional.unwrap_or(false),
                d.default_features.unwrap_or(true),
                d.features.clone().unwrap_or_default(),
                d.package.clone(),
            ),
            // Inherited from workspace — we only have the local crate's
            // Cargo.toml, so we can't resolve the inherited fields. Surface
            // the dep with what little we know.
            ManifestDep::Inherited(i) => (
                None,
                i.optional.unwrap_or(false),
                true,
                i.features.clone().unwrap_or_default(),
                None,
            ),
        };
        out.push(Dependency {
            name: name.clone(),
            package,
            kind,
            req,
            optional,
            default_features,
            features,
            target: target.clone(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_env;

    #[tokio::test]
    async fn test_axum_deps() -> Result<()> {
        let mut env = test_env().await?;
        let version = semver::Version::new(0, 8, 9);
        let fixture = crate::test_utils::fixture("axum-0.8.9.crate")?;
        let _mock = env
            .server
            .mock("GET", "/crates/axum/axum-0.8.9.crate")
            .with_status(200)
            .with_body_from_file(&fixture)
            .create();

        let deps = dependency_tree(env.config(), "axum", &version)
            .await?
            .expect("deps present");

        let names: Vec<&str> = deps.iter().map(|d| d.name.as_str()).collect();
        assert!(names.iter().any(|n| n.contains("axum-core")));
        assert!(names.contains(&"tower"));
        assert!(deps.iter().any(|d| d.kind == DependencyKind::Dev));

        Ok(())
    }
}
