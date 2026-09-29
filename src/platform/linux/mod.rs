use crate::IconData;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

/// Upper bound on cached desktop-file lookups; the key space (process names)
/// is unbounded over a long monitor session, so the map is cleared and
/// rebuilt past this size. Eviction only costs a re-scan, never correctness.
const MAX_CACHED_LOOKUPS: usize = 1024;

/// `Icons::new()` walks every icon theme directory on the system. It is pure
/// filesystem discovery, so one instance is shared for the process lifetime
/// instead of re-scanning per icon request (hundreds per icon batch).
static ICONS: LazyLock<Mutex<icon::Icons>> = LazyLock::new(|| Mutex::new(icon::Icons::new()));

/// Maps process names to the `Icon=` value of their `.desktop` file
/// (`None` = looked up, no match). Without this, every icon request re-read
/// every `.desktop` file on the system.
static DESKTOP_ICON_CACHE: LazyLock<Mutex<HashMap<String, Option<String>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) fn get_icon(name: String) -> Option<IconData> {
    let icon_name_opt = cached_icon_name(&name);
    let icon_name = icon_name_opt.unwrap_or(name);

    // first check if the icon name is an absolute path to an icon file
    let icon_path = Path::new(&icon_name);
    if icon_path.is_absolute() {
        return icon_data(icon_path);
    }

    // try to find the icon using the shared icon-theme index
    let icon_opt = ICONS.lock().ok()?.find_default_icon(&icon_name, 64, 1);

    if let Some(icon) = icon_opt {
        let path = icon.path();
        return icon_data(path);
    }

    None
}

/// Memoized [`find_icon_name`]: the `.desktop` scan runs once per unique
/// process name instead of once per icon request.
fn cached_icon_name(name: &str) -> Option<String> {
    if let Ok(cache) = DESKTOP_ICON_CACHE.lock()
        && let Some(cached) = cache.get(name)
    {
        return cached.clone();
    }
    let found = find_icon_name(name);
    if let Ok(mut cache) = DESKTOP_ICON_CACHE.lock() {
        if cache.len() >= MAX_CACHED_LOOKUPS && !cache.contains_key(name) {
            cache.clear();
        }
        cache.insert(name.to_string(), found.clone());
    }
    found
}

fn find_icon_name(name: &str) -> Option<String> {
    let mut dirs = Vec::new();
    if let Some(home_dir) = dirs::home_dir().map(|p| p.to_string_lossy().into_owned()) {
        dirs.push(PathBuf::from(&format!(
            "{home_dir}/.local/share/applications"
        )));
    }
    dirs.push(PathBuf::from("/usr/share/applications"));
    dirs.push(PathBuf::from("/usr/local/share/applications"));

    let entries = dirs
        .into_iter()
        .filter_map(|dir| fs::read_dir(dir).ok())
        .flat_map(|rd| rd.filter_map(Result::ok));

    let mut ret_val = None;
    let mut found = false;

    for entry in entries {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("desktop")
            && let Ok(content) = fs::read_to_string(&path)
        {
            for line in content.lines() {
                if let Some(icon_name) = line
                    .strip_prefix("Icon=")
                    .map(|s| s.replace(['\"', '\\'], "").trim().to_string())
                    && !icon_name.is_empty()
                {
                    ret_val = Some(icon_name);
                }
                if let Some(exec_cmd) = line
                    .strip_prefix("Exec=")
                    .map(|s| s.replace(['\"', '\\'], "").trim().to_string())
                {
                    let parts: Vec<&str> = exec_cmd.split_whitespace().collect();

                    if parts.iter().any(|part| {
                        let part_name = Path::new(part)
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("");
                        if name.len() < 15 {
                            part_name == name
                        } else {
                            part_name.contains(name)
                        }
                    }) {
                        found = true;
                    }
                }
            }
        }
        if found && ret_val.is_some() {
            return ret_val;
        }
        found = false;
        ret_val = None;
    }
    None
}

fn icon_data(path: &Path) -> Option<IconData> {
    if path.extension().and_then(|e| e.to_str()) == Some("png") {
        let img = image::open(path).ok()?.into_rgba8();
        let (width, height) = img.dimensions();
        Some(IconData {
            width,
            height,
            rgba: img.into_raw(),
        })
    } else {
        None
    }
}
