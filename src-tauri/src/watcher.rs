use notify::event::{CreateKind, ModifyKind};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub enum CsvKind {
    Amount,
    Cost,
}

pub struct CsvEvent {
    pub kind: CsvKind,
    pub path: PathBuf,
}

pub fn start_watching(
    downloads_dir: PathBuf,
) -> Result<(RecommendedWatcher, mpsc::Receiver<CsvEvent>), String> {
    let (tx, rx) = mpsc::channel();

    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        if let Ok(event) = res {
            match event.kind {
                EventKind::Create(CreateKind::File)
                | EventKind::Modify(ModifyKind::Data(_))
                | EventKind::Modify(ModifyKind::Name(_)) => {}
                _ => return,
            }

            for path in &event.paths {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                if let Some(csv_kind) = match_csv(name) {
                    let _ = tx.send(CsvEvent {
                        kind: csv_kind,
                        path: path.clone(),
                    });
                }
            }
        }
    })
    .map_err(|e| format!("Create watcher: {e}"))?;

    watcher
        .watch(&downloads_dir, RecursiveMode::NonRecursive)
        .map_err(|e| format!("Watch dir: {e}"))?;

    Ok((watcher, rx))
}

fn match_csv(filename: &str) -> Option<CsvKind> {
    // amount-2026-6.csv or amount-2026-06.csv
    if filename.starts_with("amount-") && filename.ends_with(".csv") {
        Some(CsvKind::Amount)
    } else if filename.starts_with("cost-") && filename.ends_with(".csv") {
        Some(CsvKind::Cost)
    } else {
        None
    }
}

pub fn wait_stable(rx: &mpsc::Receiver<CsvEvent>, timeout: Duration) -> Option<CsvEvent> {
    let first = rx.recv().ok()?;
    let mut last_path = first.path.clone();
    let mut last_kind = match first.kind {
        CsvKind::Amount => CsvKind::Amount,
        CsvKind::Cost => CsvKind::Cost,
    };
    let deadline = Instant::now() + timeout;

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Some(CsvEvent {
                kind: last_kind,
                path: last_path,
            });
        }
        match rx.recv_timeout(remaining) {
            Ok(e) => {
                last_path = e.path;
                last_kind = match e.kind {
                    CsvKind::Amount => CsvKind::Amount,
                    CsvKind::Cost => CsvKind::Cost,
                };
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                return Some(CsvEvent {
                    kind: last_kind,
                    path: last_path,
                });
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return None,
        }
    }
}
