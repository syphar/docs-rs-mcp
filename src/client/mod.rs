pub(crate) mod get_docs;
pub(crate) mod get_item;
pub(crate) mod list_methods;
pub(crate) mod list_module;
pub(crate) mod search_items;
pub(crate) mod status;

use std::sync::LazyLock;

const APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION"));

pub(crate) static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(APP_USER_AGENT)
        .build()
        .expect("can't create request client & connection pool")
});
