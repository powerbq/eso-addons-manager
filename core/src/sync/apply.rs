use std::fs::{self, File};
use std::io;
use std::path::Path;
use std::sync::Arc;

use zip::ZipArchive;

use super::{Action, Error, Event, Plan};
use crate::deps;

pub fn apply(plan: &Plan, target: &Path, mut on_event: impl FnMut(Event)) -> Result<(), Error> {
    fs::create_dir_all(target).map_err(failed(target))?;

    let mut archives = plan
        .sources
        .iter()
        .map(|path| {
            File::open(path)
                .map_err(deps::Error::from)
                .and_then(|file| ZipArchive::new(file).map_err(deps::Error::from))
                .map_err(|error| Error::Archive {
                    path: path.clone(),
                    error,
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let total = plan.actions.len();
    for (done, action) in plan.actions.iter().enumerate() {
        on_event(Event::Applying { done, total });

        match action {
            Action::RemoveFile(path) => {
                let path = target.join(path);
                ignore_missing(fs::remove_file(&path)).map_err(failed(&path))?;
            }
            Action::RemoveDirectory(path) => {
                let path = target.join(path);
                ignore_missing(fs::remove_dir(&path)).map_err(failed(&path))?;
            }
            Action::CreateDirectory(path) => {
                let path = target.join(path);
                fs::create_dir_all(&path).map_err(failed(&path))?;
            }
            Action::WriteFile {
                path,
                source,
                entry,
                ..
            } => {
                let mut file =
                    archives[*source]
                        .by_index(*entry)
                        .map_err(|error| Error::Archive {
                            path: plan.sources[*source].clone(),
                            error: error.into(),
                        })?;
                let path = target.join(path);
                let mut output = File::create(&path).map_err(failed(&path))?;
                io::copy(&mut file, &mut output).map_err(failed(&path))?;
            }
        }
    }

    on_event(Event::Applying { done: total, total });
    Ok(())
}

fn failed(path: &Path) -> impl FnOnce(io::Error) -> Error {
    let path = path.to_owned();
    move |error| Error::Apply {
        path,
        error: Arc::new(error),
    }
}

fn ignore_missing(result: io::Result<()>) -> io::Result<()> {
    match result {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}
