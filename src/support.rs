use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use eso_addons_manager_core::api;
use zip::CompressionMethod;
use zip::write::{SimpleFileOptions, ZipWriter};

use crate::log::LOG_SUFFIX;
use crate::paths::Paths;

const DSN: Option<&str> = option_env!("SENTRY_DSN");
const CONTENT_TYPE: &str = "application/x-sentry-envelope";

pub fn is_enabled() -> bool {
    DSN.is_some_and(|dsn| !dsn.trim().is_empty())
}

#[derive(Debug, Clone, Copy)]
pub struct Selection {
    pub config: bool,
    pub config_backups: bool,
    pub log: bool,
    pub log_backups: bool,
}

#[derive(Debug, Clone)]
pub enum Outcome {
    Sent,
    Failed { error: String, bundle: Vec<u8> },
}

pub fn bundle_filename() -> String {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis()) as u64;
    let date = crate::format::date(now_ms);
    let seconds = (now_ms / 1000) % 86_400;
    let (h, m, s) = (seconds / 3600, (seconds % 3600) / 60, seconds % 60);
    format!(
        "eso-addons-manager-report-{}-{date}_{h:02}-{m:02}-{s:02}.zip",
        env!("CARGO_PKG_VERSION")
    )
}

pub async fn send(
    paths: Paths,
    selection: Selection,
    description: String,
    contact: String,
    log: String,
) -> Outcome {
    let Some(dsn) = DSN else {
        return Outcome::Failed {
            error: "problem reporting is not enabled in this build".to_owned(),
            bundle: Vec::new(),
        };
    };
    let Some((url, key)) = parse_dsn(dsn) else {
        return Outcome::Failed {
            error: "the Sentry DSN is malformed".to_owned(),
            bundle: Vec::new(),
        };
    };

    let files = {
        let paths = paths.clone();
        let description = description.clone();
        let contact = contact.clone();
        match tokio::task::spawn_blocking(move || {
            collect_files(&paths, &selection, &description, &contact, &log)
        })
        .await
        {
            Ok(files) => files,
            Err(error) => {
                return Outcome::Failed {
                    error: error.to_string(),
                    bundle: Vec::new(),
                };
            }
        }
    };

    let envelope = build_envelope(&description, &contact, &files);
    let endpoint = format!("{url}?sentry_key={key}");

    let client = match api::Client::new() {
        Ok(client) => client,
        Err(error) => {
            return Outcome::Failed {
                error: error.to_string(),
                bundle: build_zip(&files).unwrap_or_default(),
            };
        }
    };

    match client.post(&endpoint, CONTENT_TYPE, envelope).await {
        Ok(()) => {
            tracing::info!("problem report sent ({} file(s))", files.len());
            Outcome::Sent
        }
        Err(error) => {
            tracing::warn!("could not send the problem report: {error}");
            Outcome::Failed {
                error: error.to_string(),
                bundle: build_zip(&files).unwrap_or_default(),
            }
        }
    }
}

pub async fn bundle(
    paths: Paths,
    selection: Selection,
    description: String,
    contact: String,
    log: String,
) -> Vec<u8> {
    tokio::task::spawn_blocking(move || {
        let files = collect_files(&paths, &selection, &description, &contact, &log);
        build_zip(&files).unwrap_or_default()
    })
    .await
    .unwrap_or_default()
}

fn collect_files(
    paths: &Paths,
    selection: &Selection,
    description: &str,
    contact: &str,
    log: &str,
) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<(String, Vec<u8>)> = vec![(
        "report.txt".to_owned(),
        report_text(description, contact).into_bytes(),
    )];

    if selection.log && !log.trim().is_empty() {
        files.push((format!("live{LOG_SUFFIX}"), log.as_bytes().to_vec()));
    }

    let mut sources: Vec<(String, PathBuf)> = Vec::new();

    let config = &paths.config_file;
    if selection.config {
        push(&mut sources, config.clone());
    }
    if selection.config_backups
        && let Some(dir) = config.parent()
        && let Some(stem) = config.file_stem().and_then(|s| s.to_str())
    {
        let suffix = config
            .extension()
            .and_then(|e| e.to_str())
            .map_or_else(String::new, |e| format!(".{e}"));
        push_numbered(&mut sources, dir, stem, &suffix);
    }
    if selection.log_backups
        && let Ok(entries) = std::fs::read_dir(&paths.log_dir)
    {
        let mut logs: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| crate::log::is_log(path))
            .collect();
        logs.sort();
        for path in logs {
            push(&mut sources, path);
        }
    }

    for (name, path) in sources {
        match std::fs::read(&path) {
            Ok(bytes) => files.push((name, bytes)),
            Err(error) => {
                tracing::warn!("leaving {} out of the report: {error}", path.display());
            }
        }
    }
    files
}

fn report_text(description: &str, contact: &str) -> String {
    let contact = contact.trim();
    let contact = if contact.is_empty() {
        "(none)"
    } else {
        contact
    };
    let description = description.trim();
    let description = if description.is_empty() {
        "(no description)"
    } else {
        description
    };
    format!(
        "ESO Addons Manager — problem report\n\
         Version: {}\n\
         OS: {} ({})\n\
         Date: {} UTC\n\
         Contact: {contact}\n\n\
         {description}\n",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        timestamp(),
    )
}

fn timestamp() -> String {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis()) as u64;
    let date = crate::format::date(now_ms);
    let seconds = (now_ms / 1000) % 86_400;
    let (h, m, s) = (seconds / 3600, (seconds % 3600) / 60, seconds % 60);
    format!("{date} {h:02}:{m:02}:{s:02}")
}

fn push(files: &mut Vec<(String, PathBuf)>, path: PathBuf) {
    if path.is_file()
        && let Some(name) = path.file_name().and_then(|n| n.to_str())
    {
        files.push((name.to_owned(), path.clone()));
    }
}

fn push_numbered(files: &mut Vec<(String, PathBuf)>, dir: &Path, name: &str, suffix: &str) {
    for n in 1..10 {
        push(files, dir.join(format!("{name}.{n}{suffix}")));
    }
}

fn build_zip(files: &[(String, Vec<u8>)]) -> std::io::Result<Vec<u8>> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, bytes) in files {
            zip.start_file(name.as_str(), options)?;
            zip.write_all(bytes)?;
        }
        zip.finish()?;
    }
    Ok(cursor.into_inner())
}

fn build_envelope(description: &str, contact: &str, files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let id = event_id();
    let message = if description.trim().is_empty() {
        "User problem report".to_owned()
    } else {
        description.trim().to_owned()
    };

    let mut tags = serde_json::Map::new();
    if !contact.trim().is_empty() {
        tags.insert("contact".to_owned(), contact.trim().into());
    }

    let event = serde_json::json!({
        "event_id": id,
        "timestamp": unix_seconds(),
        "platform": "native",
        "level": "info",
        "logger": "report-a-problem",
        "release": concat!("eso-addons-manager@", env!("CARGO_PKG_VERSION")),
        "message": message,
        "contexts": { "os": { "name": std::env::consts::OS, "kernel_version": std::env::consts::ARCH } },
        "tags": tags,
        "fingerprint": [id],
    });

    let mut out = Vec::new();
    write_line(&mut out, &serde_json::json!({ "event_id": id }));
    write_line(&mut out, &serde_json::json!({ "type": "event" }));
    write_line(&mut out, &event);
    for (name, bytes) in files {
        write_line(
            &mut out,
            &serde_json::json!({
                "type": "attachment",
                "length": bytes.len(),
                "filename": name,
                "attachment_type": "event.attachment",
                "content_type": "text/plain",
            }),
        );
        out.extend_from_slice(bytes);
        out.push(b'\n');
    }
    out
}

fn write_line(out: &mut Vec<u8>, value: &serde_json::Value) {
    out.extend_from_slice(&serde_json::to_vec(value).unwrap_or_default());
    out.push(b'\n');
}

fn parse_dsn(dsn: &str) -> Option<(String, String)> {
    let rest = dsn.strip_prefix("https://")?;
    let (key, rest) = rest.split_once('@')?;
    let key = key.split(':').next().unwrap_or(key);
    let (host, path) = rest.split_once('/')?;
    let project = path.trim_end_matches('/').rsplit('/').next()?;
    if key.is_empty() || host.is_empty() || project.is_empty() {
        return None;
    }
    Some((
        format!("https://{host}/api/{project}/envelope/"),
        key.to_owned(),
    ))
}

fn event_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
