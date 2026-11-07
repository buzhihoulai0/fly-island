//! DeepSeek Harness has no hook system, so its activity is inferred from the
//! session store the app keeps rewriting under `~/.dsh`.

use std::fs;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

/// Writing within this window means a turn is in flight.
const RUNNING_WINDOW_MS: u128 = 45_000;
/// After the writes stop the panel keeps a short `completed` state.
const DONE_WINDOW_MS: u128 = 180_000;
/// The island polls the snapshot every 200 ms; scanning the store every time would be wasteful.
const CACHE_TTL: Duration = Duration::from_millis(2_000);

static CACHE: Mutex<Option<(Instant, String)>> = Mutex::new(None);

/// `running` while DSH is still writing session data, `completed` right after it
/// stops, `idle` once the store has been quiet for a few minutes.
pub(super) fn dsh_phase(home_dir: &Path) -> String {
    let Ok(mut cache) = CACHE.lock() else {
        return compute_phase(home_dir);
    };

    if let Some((stamp, phase)) = cache.as_ref() {
        if stamp.elapsed() < CACHE_TTL {
            return phase.clone();
        }
    }

    let phase = compute_phase(home_dir);
    *cache = Some((Instant::now(), phase.clone()));
    phase
}

fn compute_phase(home_dir: &Path) -> String {
    match latest_activity_age_ms(home_dir) {
        Some(age) if age <= RUNNING_WINDOW_MS => "running".to_string(),
        Some(age) if age <= DONE_WINDOW_MS => "completed".to_string(),
        _ => "idle".to_string(),
    }
}

fn latest_activity_age_ms(home_dir: &Path) -> Option<u128> {
    let root = home_dir.join(".dsh");
    let mut newest: Option<SystemTime> = None;

    fn consider(newest: &mut Option<SystemTime>, path: &Path) {
        let Ok(metadata) = fs::metadata(path) else {
            return;
        };
        if !metadata.is_file() {
            return;
        }
        let Ok(modified) = metadata.modified() else {
            return;
        };
        if newest.map_or(true, |current| modified > current) {
            *newest = Some(modified);
        }
    }

    consider(&mut newest, &root.join(".dshw-turn.json"));

    // `~/.dsh/sessions/<workspace>/<session>/<files>`
    if let Ok(workspaces) = fs::read_dir(root.join("sessions")) {
        for workspace in workspaces.flatten() {
            let Ok(sessions) = fs::read_dir(workspace.path()) else {
                continue;
            };
            for session in sessions.flatten() {
                let Ok(files) = fs::read_dir(session.path()) else {
                    continue;
                };
                for file in files.flatten() {
                    consider(&mut newest, &file.path());
                }
            }
        }
    }

    // `~/.dsh/storages/session_projcache/sessions/*.json`
    if let Ok(files) = fs::read_dir(root.join("storages").join("session_projcache").join("sessions")) {
        for file in files.flatten() {
            consider(&mut newest, &file.path());
        }
    }

    let newest = newest?;
    SystemTime::now()
        .duration_since(newest)
        .ok()
        .map(|elapsed| elapsed.as_millis())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_home(name: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!("focusd-dsh-activity-{name}"));
        fs::remove_dir_all(&base).ok();
        fs::create_dir_all(base.join(".dsh").join("sessions")).unwrap();
        base
    }

    #[test]
    fn idle_without_any_session_store() {
        let home = temp_home("idle");
        assert_eq!(compute_phase(&home), "idle");
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_fresh_session_write_counts_as_running() {
        let home = temp_home("running");
        let session = home.join(".dsh").join("sessions").join("ws").join("s1");
        fs::create_dir_all(&session).unwrap();
        fs::write(session.join("session.v4.jsonl.zstd"), b"x").unwrap();
        assert_eq!(compute_phase(&home), "running");
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn an_old_session_write_falls_back_to_idle() {
        let home = temp_home("stale");
        let session = home.join(".dsh").join("sessions").join("ws").join("s1");
        fs::create_dir_all(&session).unwrap();
        let file = session.join("session.v4.jsonl.zstd");
        fs::write(&file, b"x").unwrap();
        let old = SystemTime::now() - Duration::from_secs(3_600);
        fs::File::options()
            .write(true)
            .open(&file)
            .unwrap()
            .set_modified(old)
            .unwrap();
        assert_eq!(compute_phase(&home), "idle");
        fs::remove_dir_all(&home).ok();
    }
}
