use crate::{
    client::{
        crate_metadata::{self, CrateMetadata},
        get_docs::{TargetResolution, get_docs},
        inspect_feature_flags::{self, Feature},
        list_module,
        manifest_dependencies::{self, Dependency, DependencyKind},
        readme,
    },
    context::Context,
    errors::Error,
    tools::render_response,
    types::semver::Version,
};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};
use serde::Serialize;

const HOST_TARGET: &str = env!("BUILD_TARGET");

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct CrateOverviewArgs {
    pub(crate) krate: String,
    pub(crate) version: Version,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) target: Option<String>,
}

#[derive(Debug, Serialize)]
struct CrateOverviewResult {
    #[serde(flatten)]
    target: TargetResolution,
    metadata: CrateMetadata,
    default_features: Vec<Feature>,
    direct_dependencies: Vec<Dependency>,
    public_modules: Vec<list_module::Entry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    readme: Option<readme::Readme>,
}

pub(crate) async fn handle(
    context: &Context,
    args: CrateOverviewArgs,
) -> Result<CallToolResult, McpError> {
    let target = args.target.as_deref().unwrap_or(HOST_TARGET);
    let (docs, metadata, features, dependencies) = tokio::try_join!(
        get_docs(context, &args.krate, args.version.as_ref(), Some(target)),
        crate_metadata::crate_metadata(context, &args.krate, args.version.as_ref()),
        inspect_feature_flags::inspect_feature_flags(context, &args.krate, args.version.as_ref()),
        manifest_dependencies::manifest_dependencies(context, &args.krate, args.version.as_ref()),
    )?;
    let readme = match readme::readme(context, &args.krate, args.version.as_ref()).await {
        Ok(readme) => readme.map(|mut readme| {
            readme.content = readme.content.chars().take(4_000).collect();
            readme
        }),
        Err(error)
            if matches!(
                error.downcast_ref::<Error>(),
                Some(Error::MissingSourceFile(_))
            ) =>
        {
            None
        }
        Err(error) => return Err(McpError::internal_error(error.to_string(), None)),
    };
    let listing = list_module::list_module(&docs, None)
        .ok_or_else(|| McpError::resource_not_found("crate root module not found", None))?;

    render_response(CrateOverviewResult {
        target: docs.target_resolution(),
        metadata,
        default_features: features
            .into_iter()
            .filter(|feature| feature.default || feature.name == "default")
            .collect(),
        direct_dependencies: dependencies
            .into_iter()
            .filter(|dependency| {
                dependency.kind == DependencyKind::Normal && dependency.target.is_none()
            })
            .collect(),
        public_modules: listing
            .entries
            .into_iter()
            .filter(|entry| entry.kind == crate::types::rustdoc_types::ItemKind::Module)
            .collect(),
        readme,
    })
}
