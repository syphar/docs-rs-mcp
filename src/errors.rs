use anyhow::anyhow;
use reqwest::Url;
use rmcp::ErrorData;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("crate or version not found on {0}")]
    VersionNotFound(Url),

    #[error("missing source file in crate: {0}")]
    MissingSourceFile(String),

    #[error("missing metadata in crate: {0}")]
    MissingMetadata(String),

    #[error("resource not found")]
    ResourceNotFound(String),

    #[error("unsupported rustdoc json version: {0}")]
    UnsupportedRustdocJsonVersion(u32),

    #[error("item not found: {0}")]
    ItemNotFound(String),

    #[error("http error")]
    Http(#[from] reqwest::Error),

    #[error("i/o error")]
    Io(#[from] std::io::Error),

    #[error("other error: {0}")]
    Other(anyhow::Error),
}

impl Error {
    pub(crate) fn item_not_found<S, I>(path: I) -> Self
    where
        S: Into<String>,
        I: IntoIterator<Item = S>,
    {
        let path: Vec<_> = path.into_iter().map(Into::into).collect();
        Error::ItemNotFound(path.join("::"))
    }
}

impl From<Error> for ErrorData {
    fn from(value: Error) -> Self {
        match value {
            Error::VersionNotFound(_) => ErrorData::resource_not_found(value.to_string(), None),
            Error::MissingSourceFile(_) => ErrorData::resource_not_found(value.to_string(), None),
            Error::MissingMetadata(_) => ErrorData::resource_not_found(value.to_string(), None),
            Error::ResourceNotFound(_) => ErrorData::resource_not_found(value.to_string(), None),
            Error::ItemNotFound(_) => ErrorData::resource_not_found(value.to_string(), None),
            Error::UnsupportedRustdocJsonVersion(_) => {
                ErrorData::resource_not_found(value.to_string(), None)
            }
            Error::Io(error) => ErrorData::internal_error(error.to_string(), None),
            Error::Http(error) => ErrorData::internal_error(error.to_string(), None),
            Error::Other(error) => ErrorData::internal_error(error.to_string(), None),
        }
    }
}

// for moka cache api
impl From<Arc<Error>> for Error {
    fn from(value: Arc<Error>) -> Self {
        match &*value {
            Error::VersionNotFound(url) => Self::VersionNotFound(url.clone()),
            Error::MissingSourceFile(filename) => Self::MissingSourceFile(filename.clone()),
            Error::MissingMetadata(metadata) => Self::MissingMetadata(metadata.clone()),
            Error::ResourceNotFound(name) => Self::ResourceNotFound(name.clone()),
            Error::ItemNotFound(path) => Self::ItemNotFound(path.clone()),
            Error::UnsupportedRustdocJsonVersion(v) => Self::UnsupportedRustdocJsonVersion(*v),
            // FIXME: how to keep the original error variant?
            Error::Http(error) => Self::Other(anyhow!(error.to_string())),
            Error::Io(error) => Self::Other(anyhow!(error.to_string())),
            Error::Other(error) => Self::Other(anyhow!(error.to_string())),
        }
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        match err.downcast::<Error>() {
            Ok(our_err) => our_err,
            Err(err) => Self::Other(err),
        }
    }
}
