use crate::{
    client::{dir_for_crate, download},
    context::{Context, DocsKey},
    errors::Error,
};
use anyhow::{Context as _, Result};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{fs, task::spawn_blocking};
use tracing::{debug, instrument};

/// read the format version from a rustdoc JSON file.
#[instrument(skip_all, fields(path=%path.as_ref().display()))]
async fn read_format_version_from_rustdoc_json(path: impl AsRef<Path>) -> Result<u32> {
    #[derive(Deserialize)]
    struct RustdocJson {
        format_version: u32,
    }

    let path = path.as_ref().to_path_buf();
    spawn_blocking(move || {
        use std::{fs, io};

        let file = fs::File::open(&path)?;
        let reader = io::BufReader::new(file);
        let decoder = zstd::stream::read::Decoder::new(reader)?;

        let rustdoc_json: RustdocJson = serde_json::from_reader(decoder)?;

        Ok(rustdoc_json.format_version)
    })
    .await?
}

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

#[instrument(skip(context))]
async fn fetch_rustdoc_json(
    context: &Context,
    krate: &str,
    version: &semver::Version,
    target: Option<&str>,
) -> Result<PathBuf, Error> {
    let version = version.to_string();

    let target_dir = dir_for_crate(&context.config().cache_dir, krate, &version);
    let target_path = target_dir
        .join(target.unwrap_or("default_target"))
        .with_extension("json.zst");

    if fs::try_exists(&target_path).await? {
        debug!(target_path = %target_path.display(), "found rustdoc json");
        return Ok(target_path);
    }

    fs::create_dir_all(&target_dir).await?;
    let url = context
        .config()
        .docs_rs_server
        .join(&build_download_url(krate, &version, target))
        .context("can't build download url")?;

    debug!(%url, target_path=%target_path.display(), "downloading rustdoc json");

    download(url, &target_path).await?;

    Ok(target_path)
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
#[instrument(skip(context))]
pub(crate) async fn get_docs(
    context: &Context,
    krate: &str,
    version: &semver::Version,
    target: Option<&str>,
) -> Result<Arc<rustdoc_types::Crate>, Error> {
    let key = DocsKey {
        name: krate.to_string(),
        version: version.to_owned(),
        target: target.map(|t| t.to_string()),
    };

    Ok(context
        .rustdoc_json_cache
        .entry(key)
        .or_try_insert_with::<_, Error>(async move {
            let path = match fetch_rustdoc_json(context, krate, version, target).await {
                Ok(p) => p,
                Err(err) if matches!(err, Error::VersionNotFound(_)) && target.is_some() => {
                    fetch_rustdoc_json(context, krate, version, None).await?
                }
                Err(err) => return Err(err),
            };

            // Parse the file in a single pass and read `format_version` off the
            // result. `format_version` is the last field of the rustdoc JSON
            // object, so reading it separately would require decompressing and
            // tokenizing the whole file a second time. If parsing fails we fall
            // back to reading just the version, which lets us report an
            // unsupported-version error precisely instead of a raw parse error.
            match parse_rustdoc_json(&path).await {
                Ok(krate) if krate.format_version == rustdoc_types::FORMAT_VERSION => {
                    Ok(Arc::new(krate))
                }
                Ok(krate) => Err(Error::UnsupportedRustdocJsonVersion(krate.format_version)),
                Err(parse_err) => {
                    let format_version = read_format_version_from_rustdoc_json(&path).await?;
                    if format_version != rustdoc_types::FORMAT_VERSION {
                        Err(Error::UnsupportedRustdocJsonVersion(format_version))
                    } else {
                        Err(parse_err.into())
                    }
                }
            }
        })
        .await?
        .into_value())
}

#[instrument(skip_all, fields(path=%path.as_ref().display()))]
pub(crate) async fn parse_rustdoc_json(path: impl AsRef<Path>) -> Result<rustdoc_types::Crate> {
    let path = path.as_ref().to_path_buf();
    spawn_blocking(move || {
        use std::{fs, io};

        let file = fs::File::open(&path)?;
        let reader = io::BufReader::new(file);
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

        let docs = get_docs(env.context(), "axum", &version, target).await?;
        assert_eq!(docs.crate_version, Some(version.to_string()));

        let root = &docs.paths[&docs.root];
        assert_eq!(root.path, vec!["axum"]);
        assert_eq!(root.kind, rustdoc_types::ItemKind::Module);

        Ok(())
    }
}
