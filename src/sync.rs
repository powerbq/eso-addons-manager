use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use eso_addons_manager_core::sync::{self, Action, Event, Plan};
use eso_addons_manager_core::{Addon, Catalogue, Config, api, cache};
use iced::Task;
use iced::futures::channel::mpsc;

use crate::i18n::fl;
use crate::report::{Line, Report};

pub const CUSTOM_DIRECTORY: &str = "custom";

#[derive(Debug, Clone)]
pub enum Update {
    Refreshed(Catalogue),
    Progress(Event),
    Finished(Result<Plan, String>),
}

pub fn start(
    client: api::Client,
    catalogue: Catalogue,
    config: Config,
    refresh: bool,
) -> Task<Update> {
    let (sender, receiver) = mpsc::unbounded();

    let job = async move {
        let catalogue = if refresh {
            match client.file_list().await {
                Ok(entries) => {
                    let catalogue = catalogue.with_addons(entries);
                    let _ = sender.unbounded_send(Update::Refreshed(catalogue.clone()));
                    catalogue
                }
                Err(error) => {
                    tracing::error!("cannot refresh the catalogue: {error}");
                    let _ = sender.unbounded_send(Update::Finished(Err(error.to_string())));
                    return;
                }
            }
        } else {
            catalogue
        };

        let handle = tokio::runtime::Handle::current();
        let progress = sender.clone();

        let result = tokio::task::spawn_blocking(move || {
            run(&handle, &client, &catalogue, &config, |event| {
                let _ = progress.unbounded_send(Update::Progress(event));
            })
        })
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result);

        let _ = sender.unbounded_send(Update::Finished(result));
    };

    Task::batch([Task::stream(receiver), Task::future(job).discard()])
}

pub struct Summary {
    applied: bool,
    changes: Vec<Change>,
    failed: Vec<(String, String)>,
    missing: Vec<String>,
    unknown: Vec<(String, String)>,
    written: usize,
    deleted: usize,
}

struct Change {
    name: String,
    addon: bool,
    kind: ChangeKind,
}

enum ChangeKind {
    Installed { version: String },
    Updated { from: String, to: String },
    Removed,
}

pub fn summarise(
    plan: &Plan,
    config: &Config,
    catalogue: &Catalogue,
    synced_addons: &BTreeSet<String>,
) -> Summary {
    let addons: HashMap<&str, &Addon> = catalogue
        .addons()
        .iter()
        .map(|addon| (addon.uid.as_str(), addon))
        .collect();

    let name = |uid: &str| {
        addons
            .get(uid)
            .map(|addon| addon.name.clone())
            .or_else(|| {
                config
                    .addons
                    .get(uid)
                    .filter(|name| !name.is_empty())
                    .cloned()
            })
            .unwrap_or_else(|| uid.to_owned())
    };
    let change = |uid: &str, kind: ChangeKind| Change {
        name: name(uid),
        addon: config.addons.contains_key(uid) || synced_addons.contains(uid),
        kind,
    };

    let applied = plan.is_applicable();
    let mut changes = Vec::new();

    if applied {
        for uid in &plan.used {
            if !config.archives.contains_key(uid) {
                let version = plan
                    .archives
                    .get(uid)
                    .map(|archive| archive.version.clone())
                    .unwrap_or_default();
                changes.push(change(uid, ChangeKind::Installed { version }));
            }
        }
        for (uid, archive) in &plan.archives {
            if let Some(previous) = config.archives.get(uid) {
                changes.push(change(
                    uid,
                    ChangeKind::Updated {
                        from: previous.version.clone(),
                        to: archive.version.clone(),
                    },
                ));
            }
        }
        for uid in config.archives.keys() {
            if !plan.used.contains(uid) {
                changes.push(change(uid, ChangeKind::Removed));
            }
        }
    }

    let (written, deleted) = if applied { file_counts(plan) } else { (0, 0) };

    Summary {
        applied,
        changes,
        failed: plan
            .failed
            .iter()
            .map(|failure| (name(&failure.uid), failure.error.clone()))
            .collect(),
        missing: plan.missing.clone(),
        unknown: plan
            .unknown
            .iter()
            .map(|uid| (name(uid), uid.clone()))
            .collect(),
        written,
        deleted,
    }
}

pub fn report(summary: &Summary) -> Report {
    let mut report = Report::default();

    let lines = |addon: bool, matches: fn(&ChangeKind) -> bool| -> Vec<Line> {
        summary
            .changes
            .iter()
            .filter(|change| change.addon == addon && matches(&change.kind))
            .map(|change| Line {
                name: change.name.clone(),
                detail: match &change.kind {
                    ChangeKind::Installed { version } => version.clone(),
                    ChangeKind::Updated { from, to } if from == to => {
                        fl!("sync-downloaded-again", version = to.clone())
                    }
                    ChangeKind::Updated { from, to } => format!("{from} → {to}"),
                    ChangeKind::Removed => String::new(),
                },
            })
            .collect()
    };
    let installed = |kind: &ChangeKind| matches!(kind, ChangeKind::Installed { .. });
    let updated = |kind: &ChangeKind| matches!(kind, ChangeKind::Updated { .. });
    let removed = |kind: &ChangeKind| matches!(kind, ChangeKind::Removed);

    for (title, lines) in [
        (fl!("sync-installed-addons"), lines(true, installed)),
        (fl!("sync-updated-addons"), lines(true, updated)),
        (fl!("sync-removed-addons"), lines(true, removed)),
        (fl!("sync-installed-dependencies"), lines(false, installed)),
        (fl!("sync-updated-dependencies"), lines(false, updated)),
        (fl!("sync-removed-dependencies"), lines(false, removed)),
    ] {
        report.push_section(title, lines);
    }

    if !summary.failed.is_empty() {
        report.problems.push(fl!("sync-not-applied"));
    }
    for (name, error) in &summary.failed {
        report.problems.push(fl!(
            "sync-download-failed",
            name = name.clone(),
            error = error.clone()
        ));
    }
    for directory in &summary.missing {
        report.problems.push(fl!(
            "sync-missing-dependency",
            directory = directory.clone()
        ));
    }
    for (name, uid) in &summary.unknown {
        report.problems.push(fl!(
            "sync-unknown-addon",
            name = name.clone(),
            uid = uid.clone()
        ));
    }

    if report.sections.is_empty() && report.problems.is_empty() {
        report.note = Some(fl!("sync-up-to-date"));
    }

    report
}

pub fn log(plan: &Plan, summary: &Summary) {
    for change in &summary.changes {
        let kind = if change.addon { "addon" } else { "dependency" };
        let name = &change.name;
        match &change.kind {
            ChangeKind::Installed { version } => {
                tracing::info!("installed {kind} {name} {version}");
            }
            ChangeKind::Updated { from, to } if from == to => {
                tracing::info!("downloaded {kind} {name} {to} again");
            }
            ChangeKind::Updated { from, to } => {
                tracing::info!("updated {kind} {name}: {from} → {to}");
            }
            ChangeKind::Removed => tracing::info!("removed {kind} {name}"),
        }
    }

    if !summary.failed.is_empty() {
        tracing::warn!("the AddOns folder was not changed because some downloads failed");
    } else if !summary.applied {
        tracing::info!("the AddOns folder was not changed: a provider has to be chosen");
    }
    for (name, error) in &summary.failed {
        tracing::warn!("download failed: {name}: {error}");
    }
    for directory in &summary.missing {
        tracing::warn!("missing dependency: {directory}");
    }
    for (name, uid) in &summary.unknown {
        tracing::warn!("not in the catalogue: {name} ({uid})");
    }

    if summary.applied {
        for action in &plan.actions {
            tracing::debug!("{action:?}");
        }
    }
    tracing::info!(
        "sync finished: {} files written, {} removed",
        summary.written,
        summary.deleted
    );
}

fn file_counts(plan: &Plan) -> (usize, usize) {
    let written = plan
        .actions
        .iter()
        .filter(|action| matches!(action, Action::WriteFile { .. }))
        .count();
    let deleted = plan
        .actions
        .iter()
        .filter(|action| matches!(action, Action::RemoveFile(_)))
        .count();

    (written, deleted)
}

fn run(
    handle: &tokio::runtime::Handle,
    client: &api::Client,
    catalogue: &Catalogue,
    config: &Config,
    mut on_event: impl FnMut(Event),
) -> Result<Plan, String> {
    let target = PathBuf::from(&config.target_directory);
    let cache_directory = Path::new(cache::DIRECTORY);
    let input = sync::Input {
        catalogue,
        config,
        cache: cache_directory,
        custom: Path::new(CUSTOM_DIRECTORY),
        target: &target,
    };

    let plan = sync::plan(
        &input,
        |uid, path| handle.block_on(cache::fetch(client, uid, path)),
        &mut on_event,
    )
    .map_err(|error| error.to_string())?;

    if plan.is_applicable() {
        sync::apply(&plan, &target, &mut on_event).map_err(|error| error.to_string())?;

        if let Err(error) = sync::clean_cache(&plan, cache_directory) {
            tracing::warn!("cannot clean the download cache: {error}");
        }
    }

    Ok(plan)
}
