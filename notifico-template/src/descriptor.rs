use serde::Deserialize;
use std::path::PathBuf;
use toml::Table;

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TemplateSourceSelector {
    Content(String),
    File(PathBuf),
}

#[derive(Deserialize)]
pub(crate) struct Descriptor {
    pub template: Table,
}
