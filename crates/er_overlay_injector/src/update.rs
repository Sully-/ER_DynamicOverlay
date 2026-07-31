//! Self-update: check the latest GitHub release, prompt, download and apply.
//!
//! The injector embeds its own version (`CARGO_PKG_VERSION`). On startup it asks
//! GitHub for recent releases, and if a newer tag exists it prints the changelog
//! for every release since the installed version, then offers to download the
//! latest zip, replace the installed files and relaunch. User config files are
//! never overwritten: the release copy is dropped next to them as `<name>.new`
//! and any newly introduced keys are printed to the console.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use tracing::{info, warn};

const REPO: &str = "Sully-/ER_DynamicOverlay";
const USER_AGENT: &str = concat!("er_overlay_injector/", env!("CARGO_PKG_VERSION"));
const API_TIMEOUT: Duration = Duration::from_secs(10);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);
const JSON_LIMIT: u64 = 16 * 1024 * 1024;
/// How many recent releases to fetch when building the changelog window.
const RELEASES_PER_PAGE: u32 = 30;
/// Soft cap on changelog lines printed to the console before truncation.
const CHANGELOG_MAX_LINES: usize = 40;

/// Config files owned by the user: never overwritten on update.
const PRESERVED_CONFIGS: &[&str] = &["er_overlay.toml", "layouts/dashboard.toml"];

#[derive(Debug)]
pub enum UpdateOutcome {
    /// Already on the latest version (or newer).
    UpToDate,
    /// A newer version exists but the user declined.
    Declined,
    /// Files were replaced; the caller must relaunch the (new) executable.
    Updated,
}

#[derive(Clone, Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Clone, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

struct VersionedRelease {
    version: semver::Version,
    release: Release,
}

/// Query GitHub, and if a newer release exists, prompt then download/apply it.
pub fn check_and_maybe_update(current_version: &str) -> Result<UpdateOutcome> {
    let current = parse_version(current_version)?;

    println!("Checking for updates (current version: v{current}) ...");
    let releases = fetch_releases().context("querying GitHub releases")?;
    let newer = releases_newer_than(&releases, &current);

    let Some(latest) = newer.last() else {
        println!("ER Overlay is up to date (v{current}).");
        info!("Overlay is up to date (v{current})");
        return Ok(UpdateOutcome::UpToDate);
    };
    let latest_ver = latest.version.clone();

    let asset = latest
        .release
        .assets
        .iter()
        .find(|a| a.name.starts_with("er-overlay-") && a.name.ends_with(".zip"))
        .ok_or_else(|| {
            anyhow!(
                "release {} has no er-overlay-*.zip asset",
                latest.release.tag_name
            )
        })?;

    println!();
    println!("A new version of ER Overlay is available: v{latest_ver} (installed: v{current}).");
    print_changelog(&current, &latest_ver, &newer);
    println!();
    if !prompt_yes_no("Download and install it now? [Y/n] ")? {
        println!("Skipping update; keeping v{current} for this run.");
        info!("Update declined by user");
        return Ok(UpdateOutcome::Declined);
    }

    let install_dir = current_exe_dir()?;
    let tmp = fresh_temp_dir()?;

    println!("Downloading {} ...", asset.name);
    let zip_path = download(&asset.browser_download_url, &tmp).context("downloading release")?;

    println!("Extracting ...");
    let extracted = tmp.join("extracted");
    extract_zip(&zip_path, &extracted).context("extracting release")?;

    apply_update(&extracted, &install_dir).context("applying update")?;

    let _ = fs::remove_dir_all(&tmp);
    println!("Update to v{latest_ver} complete.");
    Ok(UpdateOutcome::Updated)
}

fn parse_version(s: &str) -> Result<semver::Version> {
    let trimmed = s.trim().trim_start_matches('v');
    semver::Version::parse(trimmed).with_context(|| format!("invalid version string: {s:?}"))
}

fn fetch_releases() -> Result<Vec<Release>> {
    let url = format!("https://api.github.com/repos/{REPO}/releases?per_page={RELEASES_PER_PAGE}");
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(API_TIMEOUT))
        .build()
        .into();

    let mut resp = agent
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/vnd.github+json")
        .call()?;

    let text = resp
        .body_mut()
        .with_config()
        .limit(JSON_LIMIT)
        .read_to_string()?;

    let releases: Vec<Release> = serde_json::from_str(&text).context("parsing release JSON")?;
    Ok(releases)
}

/// Stable releases strictly newer than `current`, oldest → newest.
fn releases_newer_than(releases: &[Release], current: &semver::Version) -> Vec<VersionedRelease> {
    let mut newer: Vec<VersionedRelease> = releases
        .iter()
        .filter(|r| !r.draft && !r.prerelease)
        .filter_map(|r| {
            let version = parse_version(&r.tag_name).ok()?;
            (version > *current).then(|| VersionedRelease {
                version,
                release: r.clone(),
            })
        })
        .collect();
    newer.sort_by(|a, b| a.version.cmp(&b.version));
    newer
}

fn print_changelog(
    current: &semver::Version,
    latest: &semver::Version,
    newer: &[VersionedRelease],
) {
    println!();
    if newer.len() == 1 {
        println!("What's new in v{latest}:");
    } else {
        println!("What's new since v{current}:");
    }

    let mut lines_left = CHANGELOG_MAX_LINES;
    let mut truncated = false;

    for entry in newer {
        if lines_left == 0 {
            truncated = true;
            break;
        }

        if newer.len() > 1 {
            println!();
            println!("--- v{} ---", entry.version);
            lines_left = lines_left.saturating_sub(2);
        }

        let notes = changelog_lines(&entry.release.body);
        if notes.is_empty() {
            println!("  (no release notes)");
            lines_left = lines_left.saturating_sub(1);
            continue;
        }

        for line in notes {
            if lines_left == 0 {
                truncated = true;
                break;
            }
            println!("  {line}");
            lines_left -= 1;
        }
    }

    if truncated {
        println!("  …");
    }
    println!("  Full changelog: https://github.com/{REPO}/compare/v{current}...v{latest}");
}

fn changelog_lines(body: &str) -> Vec<String> {
    body.lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        // Drop noisy auto-generated footers / compare links GitHub often appends.
        .filter(|line| !line.contains("Full Changelog"))
        .map(|line| {
            let trimmed = line.trim_start();
            // Light markdown cleanup for console readability.
            let cleaned = trimmed
                .trim_start_matches('#')
                .trim_start()
                .trim_start_matches(['*', '-'])
                .trim_start();
            if cleaned.is_empty() {
                trimmed.to_string()
            } else if trimmed.starts_with('#') {
                cleaned.to_string()
            } else if trimmed.starts_with(['*', '-']) {
                format!("- {cleaned}")
            } else {
                trimmed.to_string()
            }
        })
        .collect()
}

fn download(url: &str, dir: &Path) -> Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(DOWNLOAD_TIMEOUT))
        .build()
        .into();

    let mut resp = agent.get(url).header("User-Agent", USER_AGENT).call()?;

    let path = dir.join("update.zip");
    let mut file = fs::File::create(&path)?;
    let mut reader = resp.body_mut().as_reader();
    std::io::copy(&mut reader, &mut file)?;
    file.flush()?;
    Ok(path)
}

fn extract_zip(zip_path: &Path, dest: &Path) -> Result<()> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    fs::create_dir_all(dest)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let Some(name) = entry.enclosed_name() else {
            continue; // skip entries with unsafe paths (zip-slip guard)
        };
        let out_path = dest.join(name);
        if entry.is_dir() {
            fs::create_dir_all(&out_path)?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = fs::File::create(&out_path)?;
        std::io::copy(&mut entry, &mut out)?;
    }
    Ok(())
}

fn apply_update(src: &Path, install_dir: &Path) -> Result<()> {
    let exe_name = current_exe_file_name()?;

    // Print new config keys before touching anything (best-effort).
    announce_new_config_keys(src, install_dir);

    // Copy every file except the running exe (handled below) and preserved configs.
    copy_dir_filtered(src, install_dir, Path::new(""), &exe_name)?;

    // Swap the running executable last, using an OS-safe self-replace.
    let new_exe = src.join(&exe_name);
    if new_exe.is_file() {
        self_replace::self_replace(&new_exe).context("replacing injector executable")?;
        info!("Replaced injector executable");
    } else {
        warn!("New executable {exe_name} not found in release; keeping current one");
    }
    Ok(())
}

fn copy_dir_filtered(src: &Path, dest: &Path, rel: &Path, exe_name: &str) -> Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let name = entry.file_name();
        let src_path = entry.path();
        let child_rel = rel.join(&name);

        if file_type.is_dir() {
            copy_dir_filtered(&src_path, &dest.join(&name), &child_rel, exe_name)?;
            continue;
        }

        let rel_str = child_rel.to_string_lossy().replace('\\', "/");

        // The running exe cannot be overwritten while it runs; self_replace handles it.
        if rel_str.eq_ignore_ascii_case(exe_name) {
            continue;
        }

        let dest_path = dest.join(&name);

        // Preserve user configs: keep theirs, drop the new default alongside as `.new`.
        let is_preserved = PRESERVED_CONFIGS
            .iter()
            .any(|c| c.eq_ignore_ascii_case(&rel_str));
        if is_preserved && dest_path.exists() {
            let new_path = append_new_suffix(&dest_path);
            if let Some(parent) = new_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&src_path, &new_path)
                .with_context(|| format!("writing {}", new_path.display()))?;
            info!(
                "Kept existing {rel_str}; wrote release default to {}",
                new_path.display()
            );
            continue;
        }

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&src_path, &dest_path)
            .with_context(|| format!("copying to {}", dest_path.display()))?;
    }
    Ok(())
}

fn announce_new_config_keys(src: &Path, install_dir: &Path) {
    let name = "er_overlay.toml";
    let old_path = install_dir.join(name);
    let new_path = src.join(name);
    if !old_path.exists() || !new_path.exists() {
        return;
    }
    let (old, new) = match (read_toml(&old_path), read_toml(&new_path)) {
        (Ok(o), Ok(n)) => (o, n),
        _ => return,
    };

    let mut missing = Vec::new();
    collect_missing_keys("", &old, &new, &mut missing);
    if missing.is_empty() {
        return;
    }

    println!();
    println!("New options in {name} (defaults are used; full reference in {name}.new):");
    for (key, value) in missing {
        println!("  + {key} = {value}");
    }
}

fn collect_missing_keys(
    prefix: &str,
    old: &toml::Value,
    new: &toml::Value,
    out: &mut Vec<(String, String)>,
) {
    let (Some(old_t), Some(new_t)) = (old.as_table(), new.as_table()) else {
        return;
    };
    for (key, new_val) in new_t {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        match old_t.get(key) {
            None => out.push((path, fmt_value(new_val))),
            Some(old_val) => {
                if new_val.is_table() && old_val.is_table() {
                    collect_missing_keys(&path, old_val, new_val, out);
                }
            }
        }
    }
}

fn fmt_value(v: &toml::Value) -> String {
    match v {
        toml::Value::String(s) => format!("{s:?}"),
        toml::Value::Integer(i) => i.to_string(),
        toml::Value::Float(f) => f.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Datetime(d) => d.to_string(),
        toml::Value::Array(_) => "[ ... ]".to_string(),
        toml::Value::Table(_) => "{ ... }".to_string(),
    }
}

fn read_toml(path: &Path) -> Result<toml::Value> {
    let raw = fs::read_to_string(path)?;
    Ok(toml::from_str::<toml::Value>(&raw)?)
}

fn append_new_suffix(path: &Path) -> PathBuf {
    let mut os = path.as_os_str().to_os_string();
    os.push(".new");
    PathBuf::from(os)
}

fn current_exe_dir() -> Result<PathBuf> {
    Ok(std::env::current_exe()?
        .parent()
        .context("executable has no parent directory")?
        .to_path_buf())
}

fn current_exe_file_name() -> Result<String> {
    Ok(std::env::current_exe()?
        .file_name()
        .context("executable has no file name")?
        .to_string_lossy()
        .into_owned())
}

fn prompt_yes_no(msg: &str) -> Result<bool> {
    print!("{msg}");
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    let ans = line.trim().to_ascii_lowercase();
    Ok(matches!(ans.as_str(), "" | "y" | "yes" | "o" | "oui"))
}

fn fresh_temp_dir() -> Result<PathBuf> {
    let dir = std::env::temp_dir().join(format!("er_overlay_update_{}", std::process::id()));
    if dir.exists() {
        let _ = fs::remove_dir_all(&dir);
    }
    fs::create_dir_all(&dir)?;
    Ok(dir)
}
