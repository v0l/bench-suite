//! Makes an installed tool show up like any other application: a `.desktop` entry and icon on
//! Linux, a Start Menu shortcut on Windows. macOS needs nothing here, the `.app` bundle in
//! `~/Applications` is enough for Spotlight and Launchpad.

use crate::catalog::{Group, Opens, Tool};
use std::fs;
use std::path::{Path, PathBuf};

pub fn register(tool: &Tool, exe: &Path) -> Result<(), String> {
    if cfg!(target_os = "linux") {
        write_entry(tool, exe, &dirs::data_dir().ok_or("no data folder")?)
    } else if cfg!(windows) {
        let link = shortcut_path(tool).ok_or("no Start Menu folder")?;
        make_shortcut(&link, exe, tool)
    } else {
        Ok(())
    }
}

/// Registers the tool unless it already is, for tools installed before this existed or whose
/// entry was removed.
pub fn ensure(tool: &Tool, exe: &Path) -> Result<(), String> {
    let current = if cfg!(target_os = "linux") {
        dirs::data_dir().is_some_and(|data| {
            fs::read_to_string(entry_path(tool, &data)).is_ok_and(|t| t == desktop_entry(tool, exe))
                && icon_path(tool, &data).is_file()
        })
    } else if cfg!(windows) {
        shortcut_path(tool).is_some_and(|l| l.is_file())
    } else {
        true
    };
    if current { Ok(()) } else { register(tool, exe) }
}

pub fn unregister(tool: &Tool) -> Result<(), String> {
    if cfg!(target_os = "linux") {
        remove_entry(tool, &dirs::data_dir().ok_or("no data folder")?)
    } else if cfg!(windows) {
        match shortcut_path(tool) {
            Some(link) => remove_file(&link),
            None => Ok(()),
        }
    } else {
        Ok(())
    }
}

fn remove_file(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            Err(format!("{}: {e}", path.display()))
        }
        _ => Ok(()),
    }
}

fn write_file(path: &Path, data: &[u8]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    fs::write(path, data).map_err(|e| format!("{}: {e}", path.display()))
}

fn entry_path(tool: &Tool, data: &Path) -> PathBuf {
    data.join("applications").join(format!("{}.desktop", tool.name))
}

fn icon_path(tool: &Tool, data: &Path) -> PathBuf {
    data.join("icons/hicolor/scalable/apps").join(format!("{}.svg", tool.name))
}

fn write_entry(tool: &Tool, exe: &Path, data: &Path) -> Result<(), String> {
    write_file(&icon_path(tool, data), icon(tool.name))?;
    write_file(&entry_path(tool, data), desktop_entry(tool, exe).as_bytes())
}

fn remove_entry(tool: &Tool, data: &Path) -> Result<(), String> {
    remove_file(&entry_path(tool, data))?;
    remove_file(&icon_path(tool, data))
}

/// Quotes one argument of a desktop entry `Exec` line.
fn quote(arg: &str) -> String {
    let mut out = String::from("\"");
    for c in arg.chars() {
        match c {
            '"' | '`' | '$' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            '%' => out.push_str("%%"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn categories(group: Group) -> &'static str {
    match group {
        Group::Design => "Graphics;Engineering;",
        Group::Radio => "Network;HamRadio;",
        Group::Measure => "Utility;Engineering;Electronics;",
        Group::Develop => "Development;",
        Group::Keys => "Utility;",
    }
}

fn desktop_entry(tool: &Tool, exe: &Path) -> String {
    let mut exec = vec![quote(&exe.to_string_lossy())];
    exec.extend(tool.verb.iter().map(|v| quote(v)));
    if tool.opens != Opens::Nothing {
        exec.push("%F".to_string());
    }
    format!(
        "[Desktop Entry]\nType=Application\nName={}\nComment={}\nExec={}\nIcon={}\nTerminal=false\nCategories={}\n",
        tool.title,
        tool.tagline,
        exec.join(" "),
        tool.name,
        categories(tool.group),
    )
}

fn shortcut_path(tool: &Tool) -> Option<PathBuf> {
    Some(
        dirs::data_dir()?
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join(format!("{}.lnk", tool.title)),
    )
}

#[cfg(windows)]
fn make_shortcut(link: &Path, exe: &Path, tool: &Tool) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    if let Some(dir) = link.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    // The paths travel in the environment so no quoting of them is needed.
    let script = "$s = (New-Object -ComObject WScript.Shell).CreateShortcut($env:BENCH_LINK); \
                  $s.TargetPath = $env:BENCH_TARGET; \
                  $s.WorkingDirectory = $env:BENCH_DIR; \
                  $s.Description = $env:BENCH_DESC; \
                  $s.IconLocation = $env:BENCH_TARGET; \
                  $s.Save()";
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script])
        .env("BENCH_LINK", link)
        .env("BENCH_TARGET", exe)
        .env("BENCH_DIR", exe.parent().unwrap_or(Path::new(".")))
        .env("BENCH_DESC", tool.tagline)
        .creation_flags(0x0800_0000)
        .output()
        .map_err(|e| format!("powershell: {e}"))?;
    if out.status.success() && link.is_file() {
        Ok(())
    } else {
        Err(format!(
            "could not add {} to the Start Menu: {}",
            tool.title,
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

#[cfg(not(windows))]
fn make_shortcut(_link: &Path, _exe: &Path, _tool: &Tool) -> Result<(), String> {
    Ok(())
}

fn icon(name: &str) -> &'static [u8] {
    match name {
        "gcad" => include_bytes!("../assets/icons/gcad.svg"),
        "agentee" => include_bytes!("../assets/icons/agentee.svg"),
        "antenna-toolbox" => include_bytes!("../assets/icons/antenna-toolbox.svg"),
        "printfarm" => include_bytes!("../assets/icons/printfarm.svg"),
        "thermview" => include_bytes!("../assets/icons/thermview.svg"),
        "battery-control" => include_bytes!("../assets/icons/battery-control.svg"),
        "cycler" => include_bytes!("../assets/icons/cycler.svg"),
        "waveshark" => include_bytes!("../assets/icons/waveshark.svg"),
        "radio_tool" => include_bytes!("../assets/icons/radio_tool.svg"),
        "agentbench" => include_bytes!("../assets/icons/agentbench.svg"),
        "ngrind" => include_bytes!("../assets/icons/ngrind.svg"),
        "coldbench" => include_bytes!("../assets/icons/coldbench.svg"),
        _ => include_bytes!("../assets/icons/bench-suite.svg"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::TOOLS;

    #[test]
    fn quoting_follows_the_desktop_entry_rules() {
        assert_eq!(quote("/home/a b/$x/100%"), r#""/home/a b/\$x/100%%""#);
    }

    #[test]
    fn entries_launch_the_installed_binary_with_the_tools_verb() {
        let agentee = TOOLS.iter().find(|t| t.name == "agentee").unwrap();
        let text = desktop_entry(agentee, Path::new("/opt/a b/agentee"));
        assert!(text.contains("Name=agentee\n"));
        assert!(text.contains("Exec=\"/opt/a b/agentee\" \"view\" %F\n"));
        assert!(text.contains("Icon=agentee\n"));
        let printfarm = TOOLS.iter().find(|t| t.name == "printfarm").unwrap();
        assert!(
            desktop_entry(printfarm, Path::new("/x/printfarm"))
                .contains("Exec=\"/x/printfarm\" %F\n")
        );
        let agentbench = TOOLS.iter().find(|t| t.name == "agentbench").unwrap();
        assert!(
            desktop_entry(agentbench, Path::new("/x/agentbench"))
                .contains("Exec=\"/x/agentbench\"\n")
        );
    }

    #[test]
    fn every_tool_has_its_own_icon() {
        for tool in &TOOLS {
            assert!(icon(tool.name).starts_with(b"<"), "{}", tool.name);
            assert_ne!(icon(tool.name), icon("bench-suite"), "{}", tool.name);
        }
    }

    #[test]
    fn registering_writes_and_removes_the_entry_and_icon() {
        let data = std::env::temp_dir().join(format!("bench_launcher_{}", std::process::id()));
        let _ = fs::remove_dir_all(&data);
        let tool = &TOOLS[0];
        write_entry(tool, Path::new("/x/gcad"), &data).unwrap();
        assert!(entry_path(tool, &data).is_file());
        assert!(icon_path(tool, &data).is_file());
        remove_entry(tool, &data).unwrap();
        remove_entry(tool, &data).unwrap();
        assert!(!entry_path(tool, &data).exists());
        assert!(!icon_path(tool, &data).exists());
        let _ = fs::remove_dir_all(&data);
    }
}
