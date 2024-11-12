use serde::Deserialize;
use std::path::PathBuf;
use toml::Table;

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum TemplateSourceSelector {
    Content(String),
    File(PathBuf),
}

#[derive(Deserialize)]
pub(super) struct Descriptor {
    pub template: Table,
}
