use crate::{
    client::{dir_for_crate, download},
    context::{Context, DocsKey},
};
use anyhow::{Context as _, Result, anyhow};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{fs, task::spawn_blocking};
use tracing::debug;

/// build a rustdoc json download url.
///
/// We don't know the default target of the crate, so without specific target,
/// we leave it empty.
pub(crate) fn build_download_url(krate: &str, version: &str, target: Option<&str>) -> String {
    if let Some(target) = target {
        format!("/crate/{krate}/{version}/{target}/json.zst")
    } else {
        format!("/crate/{krate}/{version}/json.zst")
    }
}

async fn fetch_rustdoc_json(
    context: &Context,
    krate: &str,
    version: &semver::Version,
    target: Option<&str>,
) -> Result<Option<PathBuf>> {
    let version = version.to_string();

    let target_dir = dir_for_crate(&context.config().cache_dir, krate, &version);
    let target_path = target_dir
        .join(target.unwrap_or("default_target"))
        .with_extension("json.zst");

    if fs::try_exists(&target_path).await? {
        debug!(target_path = %target_path.display(), "found rustdoc json");
        return Ok(Some(target_path));
    }

    fs::create_dir_all(&target_dir).await?;
    let url = context
        .config()
        .docs_rs_server
        .join(&build_download_url(krate, &version, target))
        .context("can't build download url")?;

    debug!(%url, target_path=%target_path.display(), "downloading rustdoc json");

    if !download(url, &target_path).await? {
        return Ok(None);
    }

    Ok(Some(target_path))
}

/// Fetch rustdoc JSON for `(krate, version, target)`. On a 404 for the
/// requested `target`, transparently retries with `target = None`, which
/// resolves to whichever target the crate author marked as default in their
/// docs.rs metadata (served by docs.rs at `/crate/<k>/<v>/json.zst` without
/// a platform segment). Assumes the crate's API is the same across targets,
/// which holds for crates that don't gate items on `#[cfg(target_os = ...)]`.
///
/// Returns `Ok(None)` only when even the crate's default build doesn't exist
/// (typically: unknown crate or version).
pub(crate) async fn get_docs(
    context: &Context,
    krate: &str,
    version: &semver::Version,
    target: Option<&str>,
) -> Result<Option<Arc<rustdoc_types::Crate>>> {
    let key = DocsKey {
        name: krate.to_string(),
        version: version.to_owned(),
        target: target.map(|t| t.to_string()),
    };

    let docs = context
        .rustdoc_json_cache
        .entry(key)
        .or_try_insert_with::<_, anyhow::Error>(async move {
            let path = match fetch_rustdoc_json(context, krate, version, target).await? {
                Some(p) => p,
                None if target.is_some() => {
                    let Some(p) = fetch_rustdoc_json(context, krate, version, None).await? else {
                        return Ok(None);
                    };
                    p
                }
                None => return Ok(None),
            };

            Ok(Some(Arc::new(parse_rustdoc_json(&path).await?)))
        })
        .await
        .map_err(|err| anyhow!(err))?
        .into_value();

    Ok(docs)
}

pub(crate) async fn parse_rustdoc_json(path: impl AsRef<Path>) -> Result<rustdoc_types::Crate> {
    let path = path.as_ref().to_path_buf();
    spawn_blocking(move || {
        let file = std::fs::File::open(&path)?;
        let reader = std::io::BufReader::new(file);
        let decoder = zstd::stream::read::Decoder::new(reader)?;

        Ok(serde_json::from_reader(decoder)?)
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{fixture, test_env};
    use test_case::test_case;

    #[tokio::test]
    #[test_case(Some("test-target"))]
    #[test_case(None)]
    async fn test_success(target: Option<&str>) -> Result<()> {
        let mut env = test_env().await?;

        let version = semver::Version::new(0, 8, 9);
        let fixure_path = fixture("axum_0.8.9.json.zst")?;

        let _mock = env
            .server
            .mock(
                "GET",
                build_download_url("axum", &version.to_string(), target).as_str(),
            )
            .with_status(200)
            .with_body_from_file(&fixure_path)
            .create();

        let docs = get_docs(env.context(), "axum", &version, target)
            .await?
            .expect("expected docs to be present");
        assert_eq!(docs.crate_version, Some(version.to_string()));

        let root = &docs.paths[&docs.root];
        assert_eq!(root.path, vec!["axum"]);
        assert_eq!(root.kind, rustdoc_types::ItemKind::Module);

        Ok(())
    }
}
