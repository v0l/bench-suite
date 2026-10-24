//! Runs the platform's own installers, as if the user had opened the `.msi`, `.dmg`, `.deb` or
//! `.rpm` themselves, and finds and removes what they installed.
//!
//! - Windows: `msiexec`, which asks for permission itself.
//! - Linux: `apt-get`/`dpkg` or `dnf`/`zypper`/`rpm`, run through `pkexec` so polkit asks.
//! - macOS: the `.app` goes into `/Applications`, or `~/Applications` if that is not writable.

use crate::catalog::{Archive, Tool};
use egui_bench::update::replace;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn on_path(name: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else { return false };
    std::env::split_paths(&path).any(|d| d.join(name).is_file())
}

/// The package format this Linux machine installs, if it can install one at all.
pub fn linux_format() -> Option<Archive> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    format_for(on_path)
}

fn format_for(has: impl Fn(&str) -> bool) -> Option<Archive> {
    if !has("pkexec") {
        return None;
    }
    if has("dnf") || has("zypper") {
        Some(Archive::Rpm)
    } else if has("dpkg") {
        Some(Archive::Deb)
    } else if has("rpm") {
        Some(Archive::Rpm)
    } else {
        None
    }
}

fn args(parts: &[&str], file: &Path) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).chain([file.to_string_lossy().into_owned()]).collect()
}

fn install_line(archive: Archive, file: &Path, has: &dyn Fn(&str) -> bool) -> Vec<String> {
    match archive {
        Archive::Deb if has("apt-get") => args(&["pkexec", "apt-get", "install", "-y"], file),
        Archive::Deb => args(&["pkexec", "dpkg", "-i"], file),
        Archive::Rpm if has("dnf") => args(&["pkexec", "dnf", "install", "-y"], file),
        Archive::Rpm if has("zypper") => args(
            &["pkexec", "zypper", "--non-interactive", "install", "--allow-unsigned-rpm"],
            file,
        ),
        _ => args(&["pkexec", "rpm", "-U"], file),
    }
}

fn remove_line(archive: Archive, package: &str, has: &dyn Fn(&str) -> bool) -> Vec<String> {
    let line: &[&str] = match archive {
        Archive::Deb if has("apt-get") => &["pkexec", "apt-get", "remove", "-y"],
        Archive::Deb => &["pkexec", "dpkg", "-r"],
        Archive::Rpm if has("dnf") => &["pkexec", "dnf", "remove", "-y"],
        Archive::Rpm if has("zypper") => &["pkexec", "zypper", "--non-interactive", "remove"],
        _ => &["pkexec", "rpm", "-e"],
    };
    args(line, Path::new(package))
}

fn run(line: &[String], ok: &[i32]) -> Result<(), String> {
    let (program, rest) = line.split_first().ok_or("nothing to run")?;
    let status = Command::new(program)
        .args(rest)
        .status()
        .map_err(|e| format!("could not run {program}: {e}"))?;
    let code = status.code().unwrap_or(-1);
    if status.success() || ok.contains(&code) {
        return Ok(());
    }
    Err(match (program.as_str(), code) {
        ("pkexec", 126 | 127) => "permission was not given".to_string(),
        ("msiexec", 1602 | 1223) => "the install was cancelled".to_string(),
        _ => format!("{program} failed: {status}"),
    })
}

pub fn msi_install(file: &Path) -> Result<(), String> {
    let line = [
        "msiexec".to_string(),
        "/i".into(),
        file.to_string_lossy().into_owned(),
        "/passive".into(),
        "/norestart".into(),
    ];
    // 3010 and 1641 mean it worked and a restart would finish the job
    run(&line, &[3010, 1641])
}

pub fn msi_remove(file: &Path) -> Result<(), String> {
    if !file.is_file() {
        return Err("it was not installed from here: remove it in Settings > Apps".into());
    }
    let line = [
        "msiexec".to_string(),
        "/x".into(),
        file.to_string_lossy().into_owned(),
        "/passive".into(),
        "/norestart".into(),
    ];
    // 1605: it is not installed any more
    run(&line, &[3010, 1641, 1605])
}

pub fn package_install(archive: Archive, file: &Path) -> Result<(), String> {
    run(&install_line(archive, file, &on_path), &[])
}

/// Removes the system package that owns `exe`.
pub fn package_remove(exe: &Path) -> Result<(), String> {
    let (archive, name) = owner(exe)?;
    run(&remove_line(archive, &name, &on_path), &[])
}

fn owner(exe: &Path) -> Result<(Archive, String), String> {
    let ask = |program: &str, args: &[&str]| {
        Command::new(program)
            .args(args)
            .arg(exe)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
    };
    if let Some(name) = ask("dpkg-query", &["-S"]).and_then(|out| dpkg_owner(&out)) {
        return Ok((Archive::Deb, name));
    }
    if let Some(out) = ask("rpm", &["-qf", "--qf", "%{NAME}"]) {
        let name = out.trim();
        if !name.is_empty() {
            return Ok((Archive::Rpm, name.to_string()));
        }
    }
    Err(format!("{} does not belong to a package", exe.display()))
}

/// `dpkg-query -S` prints `name: /path` or `name:arch: /path`.
fn dpkg_owner(out: &str) -> Option<String> {
    let line = out.lines().next()?;
    let (names, _path) = line.rsplit_once(": ")?;
    let name = names.split(':').next()?.trim();
    (!name.is_empty()).then(|| name.to_string())
}

fn apps_folders() -> Vec<PathBuf> {
    let mut out = vec![PathBuf::from("/Applications")];
    if let Some(home) = dirs::home_dir() {
        out.push(home.join("Applications"));
    }
    out
}

/// Drops each `.app` into the first of `folders` that takes it.
pub fn app_install(apps: &[PathBuf], folders: &[PathBuf]) -> Result<(), String> {
    for app in apps {
        let name = app.file_name().ok_or("unnamed .app")?;
        let mut last = String::from("no Applications folder");
        let mut done = false;
        for folder in folders {
            if fs::create_dir_all(folder).is_err() {
                continue;
            }
            match replace(app, &folder.join(name)) {
                Ok(()) => {
                    done = true;
                    break;
                }
                Err(e) => last = e,
            }
        }
        if !done {
            return Err(last);
        }
    }
    Ok(())
}

pub fn mac_folders() -> Vec<PathBuf> {
    apps_folders()
}

/// Deletes the `.app` bundle that holds `exe`.
pub fn app_remove(exe: &Path) -> Result<(), String> {
    let bundle = exe
        .ancestors()
        .find(|p| p.extension().is_some_and(|e| e == "app"))
        .ok_or_else(|| format!("{} is not inside an .app", exe.display()))?;
    fs::remove_dir_all(bundle).map_err(|e| format!("{}: {e}", bundle.display()))
}

/// Folders an installer may have put `tool` in, to look through.
pub fn places(tool: &Tool) -> Vec<PathBuf> {
    let _ = tool;
    let mut out = Vec::new();
    if cfg!(windows) {
        for var in ["ProgramFiles", "LocalAppData"] {
            let Some(base) = std::env::var_os(var).map(PathBuf::from) else { continue };
            let base = if var == "LocalAppData" { base.join("Programs") } else { base };
            out.extend(children(&base));
        }
    } else if cfg!(target_os = "macos") {
        out.extend(apps_folders());
    } else {
        out.extend(["/usr/bin", "/usr/local/bin"].map(PathBuf::from));
        out.extend(children(Path::new("/opt")));
    }
    out
}

/// Whether `exe` lives where an installer puts things, not in some other folder on `PATH`.
pub fn is_installed_app(exe: &Path) -> bool {
    let roots: Vec<PathBuf> = if cfg!(windows) {
        let programs = std::env::var_os("LocalAppData").map(|p| PathBuf::from(p).join("Programs"));
        let files = std::env::var_os("ProgramFiles").map(PathBuf::from);
        files.into_iter().chain(programs).collect()
    } else if cfg!(target_os = "macos") {
        apps_folders()
    } else {
        ["/usr", "/opt"].map(PathBuf::from).to_vec()
    };
    roots.iter().any(|r| exe.starts_with(r))
}

fn children(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<PathBuf> =
        entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has(names: &'static [&'static str]) -> impl Fn(&str) -> bool {
        move |n| names.contains(&n)
    }

    #[test]
    fn the_package_format_follows_the_package_manager() {
        assert_eq!(format_for(has(&["pkexec", "dpkg", "apt-get"])), Some(Archive::Deb));
        assert_eq!(format_for(has(&["pkexec", "rpm", "dnf"])), Some(Archive::Rpm));
        assert_eq!(format_for(has(&["pkexec", "rpm", "zypper"])), Some(Archive::Rpm));
        assert_eq!(format_for(has(&["pkexec", "dpkg", "rpm"])), Some(Archive::Deb));
        assert_eq!(format_for(has(&["pkexec", "rpm"])), Some(Archive::Rpm));
        assert_eq!(format_for(has(&["dpkg", "apt-get"])), None, "nothing to ask permission with");
        assert_eq!(format_for(has(&["pkexec"])), None);
    }

    #[test]
    fn packages_install_and_remove_through_pkexec() {
        let f = Path::new("/tmp/a.deb");
        let deb = install_line(Archive::Deb, f, &has(&["apt-get"]));
        assert_eq!(deb, ["pkexec", "apt-get", "install", "-y", "/tmp/a.deb"]);
        assert_eq!(
            install_line(Archive::Deb, f, &has(&[])),
            ["pkexec", "dpkg", "-i", "/tmp/a.deb"]
        );
        let rpm = Path::new("/tmp/a.rpm");
        assert_eq!(
            install_line(Archive::Rpm, rpm, &has(&["dnf"])),
            ["pkexec", "dnf", "install", "-y", "/tmp/a.rpm"]
        );
        assert_eq!(
            install_line(Archive::Rpm, rpm, &has(&[])),
            ["pkexec", "rpm", "-U", "/tmp/a.rpm"]
        );
        assert_eq!(
            remove_line(Archive::Deb, "cycler", &has(&["apt-get"])),
            ["pkexec", "apt-get", "remove", "-y", "cycler"]
        );
        assert_eq!(
            remove_line(Archive::Rpm, "cycler", &has(&["zypper"])),
            ["pkexec", "zypper", "--non-interactive", "remove", "cycler"]
        );
    }

    #[test]
    fn dpkg_names_the_owner_with_or_without_an_architecture() {
        assert_eq!(dpkg_owner("cycler: /usr/bin/cycler-ui\n").as_deref(), Some("cycler"));
        assert_eq!(dpkg_owner("cycler:amd64: /usr/bin/cycler-ui\n").as_deref(), Some("cycler"));
        assert_eq!(dpkg_owner(""), None);
    }

    #[test]
    fn apps_go_to_the_first_folder_that_takes_them() {
        let base = std::env::temp_dir().join(format!("bench_apps_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let app = base.join("stage/Tool.app");
        let stage = |text: &str| {
            fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
            fs::write(app.join("Contents/MacOS/tool"), text).unwrap();
        };
        stage("v1");
        // a plain file where the first folder should be, so it cannot be used
        fs::write(base.join("blocked"), "").unwrap();
        let folders = [base.join("blocked"), base.join("Applications")];
        app_install(std::slice::from_ref(&app), &folders).unwrap();
        let exe = base.join("Applications/Tool.app/Contents/MacOS/tool");
        assert_eq!(fs::read_to_string(&exe).unwrap(), "v1");
        // installing again replaces it
        stage("v2");
        app_install(&[app], &folders).unwrap();
        assert_eq!(fs::read_to_string(&exe).unwrap(), "v2");
        app_remove(&exe).unwrap();
        assert!(!base.join("Applications/Tool.app").exists());
        fs::remove_dir_all(&base).unwrap();
    }
}
