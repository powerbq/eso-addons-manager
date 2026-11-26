mod apply;
mod cleanup;
mod plan;

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use crate::cache::Archive;
use crate::deps;

pub use apply::apply;
pub use cleanup::{Pruned, clean_archives, clean_cache, clean_config, record, record_archives};
pub(crate) use plan::custom_archives;
pub use plan::{Input, Source, plan};

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("invalid exclusion: {0}")]
    Exclusion(Arc<regex::Error>),
    #[error("cannot read the AddOns folder: {0}")]
    Target(Arc<io::Error>),
    #[error("cannot read {path}: {error}")]
    Archive { path: PathBuf, error: deps::Error },
    #[error("cannot update {path}: {error}")]
    Apply {
        path: PathBuf,
        error: Arc<io::Error>,
    },
}

#[derive(Debug, Clone)]
pub enum Event {
    Downloading { name: String },
    Comparing,
    Applying { done: usize, total: usize },
}

#[derive(Debug, Clone, Default)]
pub struct Plan {
    pub sources: Vec<Source>,
    pub actions: Vec<Action>,
    pub archives: BTreeMap<String, Archive>,
    pub used: BTreeSet<String>,
    pub selected: BTreeSet<String>,
    pub unknown: Vec<String>,
    pub missing: Vec<String>,
    pub failed: Vec<Failure>,
    pub choices: BTreeMap<String, Vec<String>>,
    pub recommended: BTreeMap<String, Vec<String>>,
    // Addon UID -> the UIDs of installed addons that pull it in through `DependsOn`.
    pub dependents: BTreeMap<String, BTreeSet<String>>,
    pub superseded: BTreeSet<(usize, String)>,
    pub ttc: Option<crate::ttc::Outcome>,
}

impl Plan {
    pub fn is_applicable(&self) -> bool {
        self.failed.is_empty() && self.choices.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    RemoveFile(String),
    RemoveDirectory(String),
    CreateDirectory(String),
    WriteFile {
        path: String,
        source: usize,
        entry: usize,
        replace: bool,
    },
}

#[derive(Debug, Clone)]
pub struct Failure {
    pub uid: String,
    pub name: String,
    pub error: String,
}
