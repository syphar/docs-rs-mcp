use crate::{client::get_source::fetch_cargo_toml, config::Config};
use anyhow::Result;
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
    pub(crate) name: String,
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
    let Some(cargo) = fetch_cargo_toml(config, krate, version).await? else {
        return Ok(None);
    };

    let mut out = Vec::new();
    collect_section(
        &cargo,
        "dependencies",
        DependencyKind::Normal,
        None,
        &mut out,
    );
    collect_section(
        &cargo,
        "dev-dependencies",
        DependencyKind::Dev,
        None,
        &mut out,
    );
    collect_section(
        &cargo,
        "build-dependencies",
        DependencyKind::Build,
        None,
        &mut out,
    );

    // Target-specific deps: [target.'cfg(...)'.dependencies] etc.
    if let Some(targets) = cargo.get("target").and_then(|v| v.as_table()) {
        for (target_cfg, t) in targets {
            collect_section(
                t,
                "dependencies",
                DependencyKind::Normal,
                Some(target_cfg.clone()),
                &mut out,
            );
            collect_section(
                t,
                "dev-dependencies",
                DependencyKind::Dev,
                Some(target_cfg.clone()),
                &mut out,
            );
            collect_section(
                t,
                "build-dependencies",
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
    root: &toml::Value,
    key: &str,
    kind: DependencyKind,
    target: Option<String>,
    out: &mut Vec<Dependency>,
) {
    let Some(table) = root.get(key).and_then(|v| v.as_table()) else {
        return;
    };
    for (name, spec) in table {
        let dep = match spec {
            toml::Value::String(req) => Dependency {
                name: name.clone(),
                kind,
                req: Some(req.clone()),
                optional: false,
                default_features: true,
                features: Vec::new(),
                target: target.clone(),
            },
            toml::Value::Table(t) => {
                let req = t
                    .get("version")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
                let optional = t.get("optional").and_then(|v| v.as_bool()).unwrap_or(false);
                let default_features = t
                    .get("default-features")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let features = t
                    .get("features")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                // If the table uses `package = "..."`, the dep name in the
                // graph is the package name, not the key; emit both via name
                // (we keep the key as the display name).
                Dependency {
                    name: name.clone(),
                    kind,
                    req,
                    optional,
                    default_features,
                    features,
                    target: target.clone(),
                }
            }
            _ => continue,
        };
        out.push(dep);
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
        // axum should depend on tokio, hyper, tower, etc.
        assert!(names.iter().any(|n| n.contains("axum-core")));
        assert!(names.contains(&"tower"));
        // Some are dev/build deps.
        assert!(deps.iter().any(|d| d.kind == DependencyKind::Dev));

        Ok(())
    }
}
