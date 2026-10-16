use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub const VERSIONS_FILE: &str = "VERSIONS";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    LinuxX64,
    LinuxArm64,
    MacArm64,
    MacX64,
    WindowsX64,
}

impl Target {
    pub fn label(self) -> &'static str {
        match self {
            Target::LinuxX64 => "Linux x86-64",
            Target::LinuxArm64 => "Linux ARM64",
            Target::MacArm64 => "macOS Apple silicon",
            Target::MacX64 => "macOS Intel",
            Target::WindowsX64 => "Windows x86-64",
        }
    }

    pub fn triple(self) -> &'static str {
        match self {
            Target::LinuxX64 => "x86_64-unknown-linux-gnu",
            Target::LinuxArm64 => "aarch64-unknown-linux-gnu",
            Target::MacArm64 => "aarch64-apple-darwin",
            Target::MacX64 => "x86_64-apple-darwin",
            Target::WindowsX64 => "x86_64-pc-windows-msvc",
        }
    }

    pub fn host() -> Option<Target> {
        match (std::env::consts::OS, std::env::consts::ARCH) {
            ("linux", "x86_64") => Some(Target::LinuxX64),
            ("linux", "aarch64") => Some(Target::LinuxArm64),
            ("macos", "aarch64") => Some(Target::MacArm64),
            ("macos", "x86_64") => Some(Target::MacX64),
            ("windows", "x86_64") => Some(Target::WindowsX64),
            _ => None,
        }
    }

    pub fn is_windows(self) -> bool {
        self == Target::WindowsX64
    }

    pub fn exe(self, bin: &str) -> String {
        if self.is_windows() { format!("{bin}.exe") } else { bin.to_string() }
    }

    fn archive(self) -> (&'static str, Archive) {
        if self.is_windows() { ("zip", Archive::Zip) } else { ("tar.gz", Archive::TarGz) }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Archive {
    TarGz,
    Zip,
    Bare,
    Dmg,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Package {
    pub name: String,
    pub archive: Archive,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Opens {
    Nothing,
    Files(&'static [&'static str]),
    FilesOrFolder(&'static [&'static str]),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Group {
    Design,
    Radio,
    Measure,
}

impl Group {
    pub const ALL: [Group; 3] = [Group::Design, Group::Radio, Group::Measure];

    pub fn label(self) -> &'static str {
        match self {
            Group::Design => "design",
            Group::Radio => "radio",
            Group::Measure => "measurement",
        }
    }
}

pub struct Tool {
    pub name: &'static str,
    pub title: &'static str,
    pub tagline: &'static str,
    pub blurb: &'static str,
    pub group: Group,
    pub repo: &'static str,
    pub bin: &'static str,
    pub verb: &'static [&'static str],
    pub opens: Opens,
    pub version_flag: bool,
    pub skill: Option<&'static str>,
    asset: fn(&Tool, Target, &str) -> Option<Package>,
}

impl Tool {
    pub fn package(&self, target: Target, tag: &str) -> Option<Package> {
        (self.asset)(self, target, tag)
    }

    pub fn suffixes(&self) -> &'static [&'static str] {
        match self.opens {
            Opens::Nothing => &[],
            Opens::Files(s) | Opens::FilesOrFolder(s) => s,
        }
    }

    pub fn takes_folders(&self) -> bool {
        matches!(self.opens, Opens::FilesOrFolder(_))
    }

    pub fn opens(&self, path: &Path) -> bool {
        if path.is_dir() {
            return self.takes_folders();
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            return false;
        };
        let name = name.to_ascii_lowercase();
        self.suffixes().iter().any(|s| name.ends_with(&s.to_ascii_lowercase()))
    }

    pub fn args(&self, path: Option<&Path>) -> Vec<OsString> {
        let mut args: Vec<OsString> = self.verb.iter().map(OsString::from).collect();
        if let Some(p) = path {
            args.push(p.as_os_str().to_owned());
        }
        args
    }
}

fn unversioned(tool: &Tool, target: Target, _tag: &str) -> Option<Package> {
    let (ext, archive) = target.archive();
    Some(Package { name: format!("{}-{}.{ext}", tool.name, target.triple()), archive })
}

fn tagged(tool: &Tool, target: Target, tag: &str) -> Option<Package> {
    let (ext, archive) = target.archive();
    Some(Package { name: format!("{}-{tag}-{}.{ext}", tool.name, target.triple()), archive })
}

fn waveshark(tool: &Tool, target: Target, tag: &str) -> Option<Package> {
    let v = tag.trim_start_matches('v');
    let n = tool.name;
    let (name, archive) = match target {
        Target::LinuxX64 => (format!("{n}-{v}-linux-x86_64"), Archive::Bare),
        Target::LinuxArm64 => (format!("{n}-{v}-linux-arm64"), Archive::Bare),
        Target::MacArm64 => (format!("{n}-{v}-macos-arm64.dmg"), Archive::Dmg),
        Target::WindowsX64 => (format!("{n}-{v}-windows-x86_64.zip"), Archive::Zip),
        Target::MacX64 => return None,
    };
    Some(Package { name, archive })
}

pub const TOOLS: [Tool; 7] = [
    Tool {
        name: "gcad",
        title: "gcad",
        tagline: "Parametric CAD, one operation per line",
        blurb: "Parts and assemblies written like G-code. Mates, joints, torque sims, and \
                STEP, STL, 3MF or drawings out.",
        group: Group::Design,
        repo: "v0l/gcad",
        bin: "gcad",
        verb: &[],
        opens: Opens::Files(&[".gcad", ".gasm", ".gsim"]),
        version_flag: true,
        skill: Some("gcad"),
        asset: unversioned,
    },
    Tool {
        name: "agentee",
        title: "agentee",
        tagline: "Electronics design in plain TOML",
        blurb: "Boards, schematics, layouts and simulations as text files. Opens a project \
                folder and reloads as the files change.",
        group: Group::Design,
        repo: "v0l/agentee",
        bin: "agentee",
        verb: &["view"],
        opens: Opens::FilesOrFolder(&[
            ".board.toml",
            ".sym.toml",
            ".fp.toml",
            ".sch.toml",
            ".pcb.toml",
            ".sim.toml",
        ]),
        version_flag: true,
        skill: Some("agentee"),
        asset: unversioned,
    },
    Tool {
        name: "printfarm",
        title: "printfarm",
        tagline: "Every 3D printer in one window",
        blurb: "State, temperatures, progress and camera for each printer, with a job queue \
                and the history of every print.",
        group: Group::Design,
        repo: "v0l/printfarm",
        bin: "printfarm",
        verb: &[],
        opens: Opens::Files(&[".gcode", ".gco"]),
        version_flag: true,
        skill: None,
        asset: unversioned,
    },
    Tool {
        name: "antenna-toolbox",
        title: "Antenna Toolbox",
        tagline: "Antenna models, NanoVNA and path planning",
        blurb: "Model an antenna, sweep it with a NanoVNA, and plan a radio path over terrain.",
        group: Group::Radio,
        repo: "v0l/antenna-toolbox",
        bin: "antenna-toolbox",
        verb: &[],
        opens: Opens::Nothing,
        version_flag: false,
        skill: None,
        asset: tagged,
    },
    Tool {
        name: "waveshark",
        title: "WaveShark",
        tagline: "Wireshark for the radio spectrum",
        blurb: "Leave an SDR on a band and it tells you what is transmitting around you, \
                decoded or not.",
        group: Group::Radio,
        repo: "v0l/waveshark",
        bin: "waveshark",
        verb: &[],
        opens: Opens::Nothing,
        version_flag: true,
        skill: None,
        asset: waveshark,
    },
    Tool {
        name: "thermview",
        title: "thermview",
        tagline: "Radiometric thermal image viewer",
        blurb: "Temperatures from InfiRay, Hikmicro, DJI and FLIR files, with spot, line and \
                area readings.",
        group: Group::Measure,
        repo: "v0l/thermview",
        bin: "thermview",
        verb: &[],
        opens: Opens::Files(&[".irg", ".jpg", ".jpeg", ".img", ".seq"]),
        version_flag: true,
        skill: None,
        asset: unversioned,
    },
    Tool {
        name: "cycler",
        title: "Cycler",
        tagline: "Battery cell cycle testing",
        blurb: "Charge, discharge and capacity runs on a cell, through the BMS or charger \
                it is wired to.",
        group: Group::Measure,
        repo: "v0l/cycler",
        bin: "cycler-ui",
        verb: &[],
        opens: Opens::Nothing,
        version_flag: false,
        skill: None,
        asset: tagged,
    },
];

pub const SUITE: Tool = Tool {
    name: "bench-suite",
    title: "bench suite",
    tagline: "",
    blurb: "",
    group: Group::Design,
    repo: "v0l/bench-suite",
    bin: "bench-suite",
    verb: &[],
    opens: Opens::Nothing,
    version_flag: false,
    skill: None,
    asset: unversioned,
};

pub fn for_path(path: &Path) -> Option<&'static Tool> {
    TOOLS.iter().find(|t| t.opens(path))
}

pub fn locate(dirs: &[PathBuf], bin: &str) -> Option<PathBuf> {
    let exe = format!("{bin}{}", std::env::consts::EXE_SUFFIX);
    for dir in dirs {
        let beside = dir.join(&exe);
        if beside.is_file() {
            return Some(beside);
        }
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        for entry in entries.flatten() {
            let app = entry.path();
            if app.extension().is_some_and(|e| e == "app") {
                let inner = app.join("Contents").join("MacOS").join(bin);
                if inner.is_file() {
                    return Some(inner);
                }
            }
        }
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|d| d.join(&exe)).find(|p| p.is_file())
}

pub fn read_versions(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|l| {
            let (name, tag) = l.trim().split_once(' ')?;
            Some((name.to_string(), tag.trim().to_string()))
        })
        .collect()
}

pub fn write_versions(versions: &[(String, String)]) -> String {
    versions.iter().map(|(n, t)| format!("{n} {t}\n")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_names_match_the_published_releases() {
        let name = |tool: &str, target, tag| tool_named(tool).package(target, tag).unwrap().name;
        assert_eq!(
            name("gcad", Target::LinuxX64, "v0.3.1"),
            "gcad-x86_64-unknown-linux-gnu.tar.gz"
        );
        assert_eq!(
            name("thermview", Target::WindowsX64, "v0.1.0"),
            "thermview-x86_64-pc-windows-msvc.zip"
        );
        assert_eq!(
            name("cycler", Target::MacArm64, "v0.1.1"),
            "cycler-v0.1.1-aarch64-apple-darwin.tar.gz"
        );
        assert_eq!(
            name("antenna-toolbox", Target::LinuxArm64, "v0.1.0"),
            "antenna-toolbox-v0.1.0-aarch64-unknown-linux-gnu.tar.gz"
        );
        assert_eq!(
            name("printfarm", Target::MacArm64, "v0.1.0"),
            "printfarm-aarch64-apple-darwin.tar.gz"
        );
        assert_eq!(name("waveshark", Target::LinuxX64, "v0.4.0"), "waveshark-0.4.0-linux-x86_64");
        assert_eq!(
            SUITE.package(Target::WindowsX64, "v0.1.0").unwrap().name,
            "bench-suite-x86_64-pc-windows-msvc.zip"
        );
        assert_eq!(
            name("waveshark", Target::MacArm64, "v0.4.0"),
            "waveshark-0.4.0-macos-arm64.dmg"
        );
        assert_eq!(
            name("waveshark", Target::WindowsX64, "v0.4.0"),
            "waveshark-0.4.0-windows-x86_64.zip"
        );
    }

    fn tool_named(name: &str) -> &'static Tool {
        TOOLS.iter().find(|t| t.name == name).unwrap()
    }

    #[test]
    fn files_route_to_the_tool_that_opens_them() {
        let routed = |p: &str| for_path(Path::new(p)).map(|t| t.name);
        assert_eq!(routed("bracket.gasm"), Some("gcad"));
        assert_eq!(routed("main.PCB.toml"), Some("agentee"));
        assert_eq!(routed("IR_0001.JPG"), Some("thermview"));
        assert_eq!(routed("case-left.gcode"), Some("printfarm"));
        assert_eq!(routed("notes.txt"), None);
        assert_eq!(routed("Cargo.toml"), None);
    }

    #[test]
    fn agentee_is_started_with_its_view_verb() {
        let args = tool_named("agentee").args(Some(Path::new("/tmp/board")));
        assert_eq!(args, ["view", "/tmp/board"]);
        assert!(tool_named("gcad").args(None).is_empty());
    }

    #[test]
    fn the_agent_tools_name_the_skill_their_release_ships() {
        let skills: Vec<_> = TOOLS.iter().filter_map(|t| t.skill.map(|s| (t.name, s))).collect();
        assert_eq!(skills, [("gcad", "gcad"), ("agentee", "agentee")]);
    }

    #[test]
    fn versions_round_trip() {
        let v = vec![("gcad".to_string(), "v0.3.1".to_string())];
        assert_eq!(read_versions(&write_versions(&v)), v);
    }

    #[test]
    fn locate_finds_a_binary_inside_an_app_bundle() {
        let dir = std::env::temp_dir().join(format!("bench_locate_{}", std::process::id()));
        let inner = dir.join("WaveShark.app/Contents/MacOS");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(inner.join("bench-test-tool"), "").unwrap();
        let found = locate(std::slice::from_ref(&dir), "bench-test-tool").unwrap();
        assert!(found.ends_with("WaveShark.app/Contents/MacOS/bench-test-tool"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
