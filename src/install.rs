use crate::catalog::{
    Archive, Package, Target, Tool, VERSIONS_FILE, read_versions, write_versions,
};
use egui_bench::update::{find_file, replace, unpack};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static VERSIONS_LOCK: Mutex<()> = Mutex::new(());

pub fn root() -> PathBuf {
    dirs::data_local_dir().unwrap_or_else(std::env::temp_dir).join("bench-suite").join("tools")
}

pub fn cache() -> PathBuf {
    dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("bench-suite").join("releases")
}

pub fn log(tool: &Tool) -> PathBuf {
    cache().parent().unwrap_or(&cache()).join("logs").join(format!("{}.log", tool.name))
}

pub fn home(tool: &Tool) -> PathBuf {
    root().join(tool.name)
}

pub fn versions() -> Vec<(String, String)> {
    fs::read_to_string(root().join(VERSIONS_FILE)).map(|t| read_versions(&t)).unwrap_or_default()
}

fn record(tool: &Tool, version: &str) -> Result<(), String> {
    let _held = VERSIONS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut all = versions();
    all.retain(|(n, _)| n != tool.name);
    all.push((tool.name.to_string(), version.to_string()));
    fs::create_dir_all(root()).map_err(|e| e.to_string())?;
    fs::write(root().join(VERSIONS_FILE), write_versions(&all)).map_err(|e| e.to_string())
}

pub fn place(
    tool: &Tool,
    target: Target,
    package: &Package,
    version: &str,
    file: &Path,
) -> Result<(), String> {
    let into = home(tool);
    let staging = root().join(format!(".{}.unpack", tool.name));
    let _ = fs::remove_dir_all(&staging);
    let placed = unpack(file, &staging).and_then(|()| match package.archive {
        Archive::Bare => replace(&staging.join(&package.name), &into.join(target.exe(tool.bin))),
        Archive::TarGz | Archive::Zip => {
            let exe = target.exe(tool.bin);
            let main = find_file(&staging, &exe)
                .ok_or_else(|| format!("{exe} is not in {}", package.name))?;
            let folder = main.parent().ok_or("the archive has no folder")?;
            for entry in fs::read_dir(folder).map_err(|e| e.to_string())?.flatten() {
                replace(&entry.path(), &into.join(entry.file_name()))?;
            }
            Ok(())
        }
        Archive::Dmg => {
            let apps: Vec<PathBuf> = fs::read_dir(&staging)
                .map_err(|e| e.to_string())?
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "app"))
                .collect();
            if apps.is_empty() {
                return Err(format!("no .app in {}", package.name));
            }
            for app in apps {
                replace(&app, &into.join(app.file_name().ok_or("unnamed .app")?))?;
            }
            Ok(())
        }
    });
    let _ = fs::remove_dir_all(&staging);
    placed?;
    record(tool, version)?;
    if let Some(exe) = crate::catalog::locate(&[into], tool.bin) {
        crate::skills::install(tool, &exe, version)?;
    }
    Ok(())
}

pub fn remove(tool: &Tool) -> Result<(), String> {
    crate::skills::remove(tool)?;
    let _held = VERSIONS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = home(tool);
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let mut all = versions();
    all.retain(|(n, _)| n != tool.name);
    fs::write(root().join(VERSIONS_FILE), write_versions(&all)).map_err(|e| e.to_string())
}

pub fn reveal(path: &Path) -> Result<(), String> {
    let dir = if path.is_dir() { path } else { path.parent().ok_or("no folder")? };
    let opener = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener).arg(dir).spawn().map(|_| ()).map_err(|e| e.to_string())
}

pub fn place_self(target: Target, file: &Path) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    place_suite(target, file, &fs::canonicalize(&exe).unwrap_or(exe))
}

fn place_suite(target: Target, file: &Path, dest: &Path) -> Result<(), String> {
    let staging = std::env::temp_dir().join(format!("bench-suite-self-{}", std::process::id()));
    let _ = fs::remove_dir_all(&staging);
    let placed = unpack(file, &staging).and_then(|()| {
        let exe = target.exe(crate::catalog::SUITE.bin);
        let new =
            find_file(&staging, &exe).ok_or_else(|| format!("{exe} is not in the release"))?;
        replace(&new, dest)
    });
    let _ = fs::remove_dir_all(&staging);
    placed
}

pub fn sweep_self() {
    let Ok(exe) = std::env::current_exe() else { return };
    let exe = fs::canonicalize(&exe).unwrap_or(exe);
    let (Some(dir), Some(name)) = (exe.parent(), exe.file_name()) else { return };
    let name = name.to_string_lossy();
    let _ = fs::remove_file(dir.join(format!("{name}.old")));
    let _ = fs::remove_file(dir.join(format!(".{name}.new")));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::TOOLS;
    use egui_bench::update::{download, latest, probe_version};

    #[test]
    #[ignore = "downloads the suite's latest release"]
    fn the_suite_replaces_itself_from_its_latest_release() {
        let target = Target::host().unwrap();
        let release = latest(crate::catalog::SUITE.repo, None).unwrap();
        let package = crate::catalog::SUITE.package(target, &release.tag).unwrap();
        let asset = release.asset(&package.name).unwrap();
        let scratch = std::env::temp_dir().join("bench-suite-test-self");
        let file = download(asset, &scratch, |_, _| {}).unwrap();
        let dest = scratch.join("installed").join(target.exe("bench-suite"));
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::write(&dest, "old launcher").unwrap();
        place_suite(target, &file, &dest).unwrap();
        assert!(fs::metadata(&dest).unwrap().len() > 1_000_000);
        let left: Vec<_> = fs::read_dir(dest.parent().unwrap())
            .unwrap()
            .flatten()
            .map(|e| e.file_name())
            .collect();
        assert_eq!(left.len(), 1);
        fs::remove_dir_all(&scratch).unwrap();
    }

    #[test]
    #[ignore = "downloads gcad and agentee and writes their skills under HOME"]
    fn agent_tools_install_their_skill_from_their_latest_release() {
        let target = Target::host().unwrap();
        for tool in TOOLS.iter().filter(|t| t.skill.is_some()) {
            let release = latest(tool.repo, Some(&cache())).unwrap();
            let package = tool.package(target, &release.tag).unwrap();
            let asset = release.asset(&package.name).unwrap();
            let scratch = std::env::temp_dir().join(format!("bench-suite-skill-{}", tool.name));
            let file = download(asset, &scratch, |_, _| {}).unwrap();
            place(tool, target, &package, &release.version, &file).unwrap();
            let _ = fs::remove_dir_all(&scratch);
            let placed = crate::skills::placed(tool);
            assert!(!placed.is_empty(), "{}", tool.name);
            for p in &placed {
                assert!(p.path.join("SKILL.md").is_file(), "{}", p.path.display());
            }
            assert!(crate::skills::current(tool, &release.version), "{}", tool.name);
            remove(tool).unwrap();
            assert!(
                crate::skills::placed(tool).iter().all(|p| p.how == crate::skills::How::Theirs)
            );
        }
    }

    #[test]
    #[ignore = "downloads every tool's latest release"]
    fn every_tool_installs_from_its_latest_release() {
        let target = Target::host().unwrap();
        for tool in &TOOLS {
            let release = latest(tool.repo, Some(&cache())).unwrap();
            let package = tool.package(target, &release.tag).unwrap();
            let asset = release.asset(&package.name).unwrap();
            let scratch = std::env::temp_dir().join(format!("bench-suite-test-{}", tool.name));
            let file = download(asset, &scratch, |_, _| {}).unwrap();
            place(tool, target, &package, &release.version, &file).unwrap();
            let _ = fs::remove_dir_all(&scratch);
            let exe = crate::catalog::locate(&[home(tool)], tool.bin).unwrap();
            if tool.version_flag {
                assert_eq!(probe_version(&exe), Some(release.version.clone()), "{}", tool.name);
            }
            assert!(versions().contains(&(tool.name.to_string(), release.version)));
        }
    }
}
