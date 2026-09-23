//! Profile-aware log routing — multiplex one process's queued file logs
//! across several Hermes homes.
//!
//! PARITY: hermes_logging.py `_known_log_homes` (172–185),
//! `_adopt_secondary_home` (188–200), `_ProfileRoutingFileHandler`
//! (423–476), `enable_profile_log_routing` (583–626).

use crate::queue::with_queue_state;
use crate::record::{Level, LogRecord, LogTarget};
use crate::rotating::{ComponentFilter, RotatingHandler};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Python `Path.expanduser()` analog via Hermes' real-home contract.
///
/// PARITY: `Path(entry).expanduser()` — Python consults `$HOME`/passwd;
/// `get_real_home` walks `HERMES_REAL_HOME` → `HOME` → passwd →
/// `USERPROFILE` (the same effective user home on this platform).
fn expanduser(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    if s == "~" {
        return PathBuf::from(hermes_constants::get_real_home(None));
    }
    if let Some(rest) = s.strip_prefix("~/") {
        return PathBuf::from(hermes_constants::get_real_home(None)).join(rest);
    }
    p.to_path_buf()
}

/// Python `Path(home).expanduser().resolve()` for routing keys.
fn resolve_home(p: &Path) -> PathBuf {
    hermes_constants::resolve_tolerant(&expanduser(p))
}

struct RouterInner {
    homes: HashSet<PathBuf>,
    handlers: HashMap<PathBuf, Arc<RotatingHandler>>,
}

/// Route queued records to the log file for their Hermes home.
///
/// PARITY: `_ProfileRoutingFileHandler` (423–476). Used only behind the
/// queue listener, so its routing lock never blocks an emitting thread.
/// Per-home handlers keep rotation and component filtering at the router
/// level (upstream copies the original's filters onto the router; each
/// per-home handler is created WITHOUT them — `_handler_for_home` →
/// `_new_file_handler` has no `log_filter` parameter).
pub struct ProfileRouter {
    /// Resolved path of the handler this router took over.
    routed_path: PathBuf,
    default_home: PathBuf,
    filename: String,
    level: Level,
    max_bytes: u64,
    backup_count: usize,
    component: Option<ComponentFilter>,
    formatter: Option<crate::rotating::Formatter>,
    inner: Mutex<RouterInner>,
}

impl ProfileRouter {
    /// Take over `existing`'s path, level, rotation and component filter for
    /// every home in `profile_homes` (PARITY: `__init__` 430–445).
    pub(crate) fn new(existing: &RotatingHandler, profile_homes: &[PathBuf]) -> Self {
        let routed_path = hermes_constants::resolve_tolerant(&existing.path);
        let default_home = routed_path
            .parent()
            .and_then(|p| p.parent())
            .map(hermes_constants::resolve_tolerant)
            .unwrap_or_else(|| PathBuf::from("/"));
        let filename = routed_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let homes: HashSet<PathBuf> = profile_homes.iter().map(|h| resolve_home(h)).collect();
        ProfileRouter {
            routed_path,
            default_home,
            filename,
            level: existing.level,
            max_bytes: existing.max_bytes,
            backup_count: existing.backup_count,
            component: existing.component.clone(),
            formatter: existing.formatter().cloned(),
            inner: Mutex::new(RouterInner {
                homes,
                handlers: HashMap::new(),
            }),
        }
    }

    pub fn routed_path(&self) -> &Path {
        &self.routed_path
    }

    pub fn default_home(&self) -> &Path {
        &self.default_home
    }

    pub fn filename(&self) -> &str {
        &self.filename
    }

    pub fn profile_homes(&self) -> Vec<PathBuf> {
        self.inner
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .homes
            .iter()
            .cloned()
            .collect()
    }

    pub fn homes_contains(&self, home: &Path) -> bool {
        self.inner
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .homes
            .contains(home)
    }

    /// Widen the routed set — PARITY: `handler._profile_homes =
    /// handler._profile_homes.union(homes)` (608–611).
    pub(crate) fn widen_homes(&self, homes: impl IntoIterator<Item = PathBuf>) {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        inner.homes.extend(homes);
    }

    /// PARITY: `_home_for_record` (447–453). An empty stamp behaves like
    /// Python `Path("").resolve()` → cwd, which falls through to the default
    /// unless cwd happens to be a routed home.
    fn home_for_record(&self, record: &LogRecord) -> PathBuf {
        let raw = record.hermes_home.as_str();
        let candidate = if raw.is_empty() {
            std::env::current_dir().unwrap_or_else(|_| self.default_home.clone())
        } else {
            resolve_home(Path::new(raw))
        };
        if self.homes_contains(&candidate) {
            candidate
        } else {
            self.default_home.clone()
        }
    }

    /// PARITY: `_handler_for_home` (455–462) — lazy per-home handler at
    /// `<home>/logs/<filename>` with the router's level/rotation/formatter
    /// and NO component filter (the router already applied it via `accepts`).
    fn handler_for_home(&self, home: &Path) -> std::io::Result<Arc<RotatingHandler>> {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(h) = inner.handlers.get(home) {
            return Ok(h.clone());
        }
        let mut handler = RotatingHandler::new(
            home.join("logs").join(&self.filename),
            self.level,
            self.max_bytes,
            self.backup_count,
            None,
        )?;
        if let Some(formatter) = &self.formatter {
            handler.set_formatter(formatter.clone());
        }
        let handler = Arc::new(handler);
        inner.handlers.insert(home.to_path_buf(), handler.clone());
        Ok(handler)
    }

    /// Level + component gate the worker applies to this router (the
    /// original handler's level/filters, PARITY: Handler(level=existing.level)
    /// + copied filters).
    pub fn accepts_record(&self, record: &LogRecord) -> bool {
        if record.level < self.level {
            return false;
        }
        if let Some(filter) = &self.component {
            if !filter.matches(&record.target) {
                return false;
            }
        }
        true
    }
}

impl LogTarget for ProfileRouter {
    fn accepts(&self, record: &LogRecord) -> bool {
        self.accepts_record(record)
    }

    fn emit(&self, record: &LogRecord) {
        // PARITY: emit (464–468) — route then handle; errors go to
        // handleError (traceback-once-per-record for non-EIO failures).
        let home = self.home_for_record(record);
        match self.handler_for_home(&home) {
            Ok(handler) => handler.emit_record(record),
            Err(e) => eprintln!(
                "--- Logging error ---\n{}: {}",
                self.routed_path.display(),
                e
            ),
        }
    }
}

/// Homes the queued file handlers already serve: bare handlers by their
/// file path, routers by default home + every routed profile home.
///
/// PARITY: `_known_log_homes` (172–185). Caller convention: results are
/// read under the queue lock via `with_queue_state` (upstream holds
/// `_queue_state_lock` around the helper).
fn known_log_homes() -> HashSet<PathBuf> {
    with_queue_state(|state| {
        let mut homes = HashSet::new();
        for router in &state.routers {
            homes.insert(router.default_home().to_path_buf());
            homes.extend(router.profile_homes());
        }
        for handler in &state.file_handlers {
            // PARITY: `Path(handler.baseFilename).resolve().parent.parent`
            // with the TypeError/ValueError/OSError `continue` — resolve-
            // tolerant never raises; a missing parent chain just yields no
            // insert.
            let resolved = hermes_constants::resolve_tolerant(&handler.path);
            if let Some(parent) = resolved.parent().and_then(|p| p.parent()) {
                homes.insert(parent.to_path_buf());
            }
        }
        homes
    })
}

/// Enable (or widen) profile routing for `profile_homes`.
///
/// PARITY: `enable_profile_log_routing` (583–626). Returns true when routing
/// is (or already was) enabled; a single-profile caller or an empty queue is
/// left untouched.
///
/// Note: upstream accepts `(name, home)` tuples (entry[1]); no in-tree caller
/// passes them — the port takes plain paths (PORT SEAMS).
pub fn enable_profile_log_routing(profile_homes: &[PathBuf]) -> bool {
    let mut homes: Vec<PathBuf> = Vec::new();
    for entry in profile_homes {
        let resolved = resolve_home(entry);
        if !homes.contains(&resolved) {
            homes.push(resolved);
        }
    }
    if homes.len() < 2 {
        return false;
    }

    with_queue_state(|state| {
        // PARITY: `if not _queued_file_handlers: return False` — the typed
        // bare + router sets are the upstream list (verbose stderr targets
        // live only in `handlers`, like upstream's root-only StreamHandler).
        if state.file_handlers.is_empty() && state.routers.is_empty() {
            return false;
        }
        if !state.routers.is_empty() {
            for router in &state.routers {
                router.widen_homes(homes.iter().cloned());
            }
            return true;
        }

        // Replace every bare handler with a router; non-file targets (the
        // verbose stderr handler) pass through untouched — PARITY:
        // `replacement` loop (616–623).
        let bare = std::mem::take(&mut state.file_handlers);
        let bare_ptrs: Vec<*const ()> = bare.iter().map(|h| Arc::as_ptr(h) as *const ()).collect();
        let old_handlers = std::mem::take(&mut state.handlers);
        let mut new_handlers: Vec<Arc<dyn LogTarget>> = Vec::new();
        let mut new_routers: Vec<Arc<ProfileRouter>> = Vec::new();
        for h in old_handlers {
            let p = Arc::as_ptr(&h) as *const ();
            if let Some(idx) = bare_ptrs.iter().position(|&bp| bp == p) {
                let router = Arc::new(ProfileRouter::new(&bare[idx], &homes));
                new_handlers.push(router.clone());
                new_routers.push(router);
                // `bare[idx]`'s Arc drops with `bare` at the end of this
                // closure — PARITY: `_quietly(existing.close)` (620).
            } else {
                new_handlers.push(h);
            }
        }
        state.handlers = new_handlers;
        state.routers = new_routers;
        // Listener restart — PARITY: `_start_queue_listener_locked` (624–625).
        crate::queue::restart_listener(state);
        true
    })
}

/// Route *home*'s records to its own files when this process already logs
/// for another home — PARITY: `_adopt_secondary_home` (188–200).
///
/// Enables profile routing for the union of known homes (or widens the live
/// routers); false when *home* is the first home seen or is already served.
pub(crate) fn adopt_secondary_home(home: &Path) -> bool {
    let resolved = resolve_home(home);
    let known = known_log_homes();
    if known.is_empty() || known.contains(&resolved) {
        return false;
    }
    let mut list: Vec<PathBuf> = known.into_iter().collect();
    list.sort(); // PARITY: `[*sorted(known), resolved]`
    list.push(resolved);
    enable_profile_log_routing(&list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expanduser_and_resolve_home() {
        let td = tempfile::TempDir::new().unwrap();
        assert_eq!(
            resolve_home(td.path()),
            hermes_constants::resolve_tolerant(td.path())
        );
        let home = PathBuf::from(hermes_constants::get_real_home(None));
        assert_eq!(expanduser(Path::new("~")), home);
        assert_eq!(expanduser(Path::new("~/x")), home.join("x"));
        assert_eq!(expanduser(Path::new("/abs")), PathBuf::from("/abs"));
    }

    #[test]
    fn single_home_routing_disabled() {
        // PARITY: `len(homes) < 2 → False`.
        let td = tempfile::TempDir::new().unwrap();
        assert!(!enable_profile_log_routing(&[td.path().to_path_buf()]));
        assert!(!enable_profile_log_routing(&[]));
    }
}
