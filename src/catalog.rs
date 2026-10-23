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
    Develop,
    Keys,
}

impl Group {
    pub const ALL: [Group; 5] =
        [Group::Design, Group::Radio, Group::Measure, Group::Develop, Group::Keys];

    pub fn label(self) -> &'static str {
        match self {
            Group::Design => "design",
            Group::Radio => "radio",
            Group::Measure => "measurement",
            Group::Develop => "development",
            Group::Keys => "keys",
        }
    }
}

pub struct Tool {
    pub name: &'static str,
    pub title: &'static str,
    pub tagline: &'static str,
    pub blurb: &'static str,
    pub guide: &'static str,
    pub features: &'static [&'static str],
    pub cli: &'static [&'static str],
    pub cli_bin: &'static str,
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

pub const TOOLS: [Tool; 12] = [
    Tool {
        name: "gcad",
        title: "gcad",
        tagline: "Parametric CAD, one operation per line",
        blurb: "Parts and assemblies written like G-code. Mates, joints, torque sims, and \
                STEP, STL, 3MF or drawings out.",
        guide: crate::guides::GCAD,
        features: &[
            "Parametric parts written one operation per line, like G-code",
            "Assemblies with mates, turning and sliding joints and an interference check",
            "Sheet metal with DXF flat patterns, weldments with a cut list, and a library of screws, nuts and bearings",
            "Torque simulation that sizes joint motors",
            "Export to STEP, STL, OBJ, 3MF, SVG and PDF drawings, with a bill of materials",
            "A viewer that rebuilds the part whenever the file is saved",
        ],
        cli: &["check", "measure", "query", "export", "sim", "materials", "bom", "render", "docs"],
        cli_bin: "",
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
        guide: crate::guides::AGENTEE,
        features: &[
            "Boards, symbols, footprints, schematics and layouts as plain TOML",
            "Imports parts from the installed KiCad libraries",
            "Routing, zone fill and design rule checks on a layout",
            "FDTD, DC drop, thermal, channel and power delivery simulations",
            "Gerbers, drill files, BOM, placement and a STEP export for the case",
            "Prices a BOM at Mouser, Farnell and DigiKey",
        ],
        cli: &[
            "check",
            "drc",
            "list",
            "show",
            "render",
            "new",
            "import",
            "search",
            "sim",
            "sparam",
            "models",
            "fab",
            "export",
            "assemble",
            "parts",
            "bom",
            "fill",
            "route",
            "tie",
            "tune",
            "neck",
            "place",
            "silk",
            "testpoints",
            "calc",
            "layout",
            "fit",
            "pinswap",
            "stackups",
            "edit",
            "datasheet",
            "library",
            "docs",
        ],
        cli_bin: "",
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
        guide: crate::guides::PRINTFARM,
        features: &[
            "Every 3D printer in one window, with state, temperatures and a camera",
            "A job queue with the part drawn from the G-code itself",
            "Prints run from the printer's SD card, so closing the window does not stop one",
            "Bed mesh, SD card files and a console for each printer",
            "Marlin printers over USB today, with Moonraker, OctoPrint, PrusaLink and Bambu LAN planned",
        ],
        cli: &[],
        cli_bin: "",
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
        guide: crate::guides::ANTENNA_TOOLBOX,
        features: &[
            "37 antenna templates solved by method of moments, FDTD or physical optics",
            "A cut sheet, drawing and feed detail for each design",
            "Drives a NanoVNA over USB, with SOL calibration and a Smith chart",
            "Terrain path profiles with Longley-Rice and ITU-R models",
            "Coverage maps and an HF sky map with reliability by hour and band",
            "RF exposure checks against ICNIRP and FCC limits",
        ],
        cli: &[],
        cli_bin: "",
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
        guide: crate::guides::WAVESHARK,
        features: &[
            "Leave an SDR on a band and see what is transmitting, decoded or not",
            "Decodes pagers, DMR, TETRA, M17, LoRa mesh, utility meters and weather sensors",
            "Aircraft and vessel tracking, with SSTV and DVB-T pictures",
            "Voice in WFM, NFM, AM, SSB and CW, with a local speech transcript",
            "Works with RTL-SDR, HackRF and LimeSDR, or a tuner on another machine",
            "Has an agent of its own that can drive the receiver",
        ],
        cli: &[],
        cli_bin: "",
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
        name: "radio_tool",
        title: "radio_tool",
        tagline: "Firmware and codeplugs for handheld radios",
        blurb: "Flash firmware and read or write codeplugs on TYT, Baofeng, Radioddity, \
                Yaesu and Ailunce radios, over USB, a cable or Bluetooth.",
        guide: crate::guides::RADIO_TOOL,
        features: &[
            "Flashes firmware on TYT, Baofeng, Radioddity, Yaesu and Ailunce handhelds",
            "Reads and writes codeplugs over USB, a serial cable or Bluetooth",
            "A window for picking the radio and flashing or editing it",
            "Says what a firmware file is and lists the channels in a codeplug, with no radio attached",
            "Builds a firmware file from raw segments, and takes one apart",
            "A command line for scripts and for files",
        ],
        cli: &["fw-info", "codeplug-info", "wrap", "unwrap", "make-xor", "models"],
        cli_bin: "radio_tool",
        group: Group::Radio,
        repo: "v0l/radio_tool",
        bin: "radio_tool-ui",
        verb: &[],
        opens: Opens::Files(&[".rdt"]),
        version_flag: true,
        skill: None,
        asset: unversioned,
    },
    Tool {
        name: "thermview",
        title: "thermview",
        tagline: "Radiometric thermal image viewer",
        blurb: "Temperatures from InfiRay, Hikmicro, DJI and FLIR files, with spot, line and \
                area readings.",
        guide: crate::guides::THERMVIEW,
        features: &[
            "Reads InfiRay, Hikmicro, FLIR and DJI radiometric images",
            "Nine palettes, a colour bar with a draggable window, and linear, log or equalised mapping",
            "Spot readings, hottest and coldest markers and a temperature under the pointer",
            "Recalibrates FLIR images from emissivity, distance and air conditions",
            "Exports the image with its markers as a PNG",
        ],
        cli: &["info", "render"],
        cli_bin: "",
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
        name: "battery-control",
        title: "Battery Control",
        tagline: "Monitor and control any battery",
        blurb: "Cells, charge, temperatures, outputs and settings for BMSes and power stations \
                over Bluetooth or a serial cable.",
        guide: crate::guides::BATTERY_CONTROL,
        features: &[
            "Live charge, voltage, current, every cell and temperature, with a history plot",
            "Reads Anker SOLIX, JK, JBD, SOK, Renogy, Pace, Seplos, Daly, Bluetti, Jackery, EcoFlow, Victron and Pylontech",
            "Scans Bluetooth and probes USB serial ports for devices",
            "Switches charge and discharge, the balancer and output ports where the device allows it",
            "A demo pack with no hardware, started with --demo",
        ],
        cli: &[],
        cli_bin: "",
        group: Group::Measure,
        repo: "v0l/battery-control",
        bin: "battery-control",
        verb: &[],
        opens: Opens::Nothing,
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
        guide: crate::guides::CYCLER,
        features: &[
            "Charges, rests and discharges a pack and counts amp-hours",
            "Decides on the highest or lowest cell, not the pack voltage",
            "Charge in standard, top-balance and bulk-only modes, discharge in CC, CV, CR or CP",
            "Stops at a state of charge, and runs cycle plans that measure real capacity",
            "Logs every sample to CSV and shows the BMS alarms",
            "Works with a pack that has no BMS, on voltage limits",
        ],
        cli: &["devices", "profile"],
        cli_bin: "cycler",
        group: Group::Measure,
        repo: "v0l/cycler",
        bin: "cycler-ui",
        verb: &[],
        opens: Opens::Nothing,
        version_flag: false,
        skill: None,
        asset: tagged,
    },
    Tool {
        name: "agentbench",
        title: "agentbench",
        tagline: "Coding agents in your git projects",
        blurb: "Run coding agents in git worktrees and keep an issue tracker beside them, \
                from one window on desktop, web or Android.",
        guide: crate::guides::AGENTBENCH,
        features: &[
            "Chats with coding agents, each pinned to its own git worktree",
            "Agents made on an issue start in a fresh worktree named after it",
            "An issue tracker per project, kept in one SQLite file",
            "Attach pictures, sound and video to a message",
            "Works with a server on another machine, and with any installed font",
        ],
        cli: &[],
        cli_bin: "",
        group: Group::Develop,
        repo: "v0l/agentbench",
        bin: "agentbench",
        verb: &[],
        opens: Opens::Nothing,
        version_flag: true,
        skill: None,
        asset: tagged,
    },
    Tool {
        name: "coldbench",
        title: "coldbench",
        tagline: "Bitcoin wallet for hardware signers",
        blurb: "A watch-only wallet for Jade, Coldcard and Trezor with Sparrow's pages, \
                where every payment is checked and signed on the device.",
        guide: crate::guides::COLDBENCH,
        features: &[
            "Transactions, send, receive, addresses, UTXOs and settings, laid out like Sparrow",
            "Talks to Jade, Coldcard and Trezor directly, and says when the device wants its PIN",
            "Coin control, freezing, labels, send all, fee picks and fee bumps",
            "Stonewall privacy spends, and every transaction drawn as a flow of coins",
            "Signs on the device, or saves and loads PSBTs for an SD card or another signer",
            "Shows an address on the device to check it, and signs or verifies messages",
            "Splits its addresses over many public servers so none sees the whole wallet, or uses yours",
        ],
        cli: &[],
        cli_bin: "",
        group: Group::Keys,
        repo: "v0l/coldbench",
        bin: "coldbench",
        verb: &[],
        opens: Opens::Files(&[".psbt"]),
        version_flag: true,
        skill: None,
        asset: unversioned,
    },
    Tool {
        name: "ngrind",
        title: "ngrind",
        tagline: "Vanity npubs and proof of work, on the GPU",
        blurb: "Mine vanity npubs and NIP-13 proof of work events on an NVIDIA or any Vulkan, \
                Metal or DirectX 12 GPU, or on the CPU.",
        guide: crate::guides::NGRIND,
        features: &[
            "Mines vanity npubs by prefix or suffix, or hex keys",
            "Mines NIP-13 proof of work for a Nostr event, to a difficulty you choose",
            "CUDA on NVIDIA, wgpu on any GPU with Vulkan, Metal or DirectX 12, and the CPU",
            "Picks the fastest backend itself, and switches backend mid-run in the window",
            "Shows the speed and the chance of a hit so far",
            "A command line for scripts, with resumable searches on CUDA",
        ],
        cli: &["devices"],
        cli_bin: "ngrind",
        group: Group::Keys,
        repo: "v0l/ngrind",
        bin: "ngrind-ui",
        verb: &[],
        opens: Opens::Nothing,
        version_flag: false,
        skill: None,
        asset: unversioned,
    },
];

pub const SUITE: Tool = Tool {
    name: "bench-suite",
    title: "bench suite",
    tagline: "",
    blurb: "",
    guide: "",
    features: &[],
    cli: &[],
    cli_bin: "",
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
            name("battery-control", Target::LinuxX64, "v0.2.0"),
            "battery-control-x86_64-unknown-linux-gnu.tar.gz"
        );
        assert_eq!(
            name("agentbench", Target::WindowsX64, "v0.1.0"),
            "agentbench-v0.1.0-x86_64-pc-windows-msvc.zip"
        );
        assert_eq!(
            name("coldbench", Target::LinuxX64, "v0.1.0"),
            "coldbench-x86_64-unknown-linux-gnu.tar.gz"
        );
        assert_eq!(
            name("printfarm", Target::MacArm64, "v0.1.0"),
            "printfarm-aarch64-apple-darwin.tar.gz"
        );
        assert_eq!(name("waveshark", Target::LinuxX64, "v0.4.0"), "waveshark-0.4.0-linux-x86_64");
        assert_eq!(
            name("radio_tool", Target::WindowsX64, "v0.4.0"),
            "radio_tool-x86_64-pc-windows-msvc.zip"
        );
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
        assert_eq!(routed("payment.psbt"), Some("coldbench"));
        assert_eq!(routed("case-left.gcode"), Some("printfarm"));
        assert_eq!(routed("dm1701.rdt"), Some("radio_tool"));
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
    fn every_tool_says_what_it_can_do_in_a_few_plain_lines() {
        for tool in TOOLS.iter() {
            let n = tool.features.len();
            assert!((4..=7).contains(&n), "{} has {n} feature lines", tool.name);
            for line in tool.features {
                assert!(line.len() > 20 && line.len() < 140, "{}: {line:?}", tool.name);
                assert!(!line.ends_with('.'), "{}: {line:?}", tool.name);
                assert!(!line.contains('`'), "{}: {line:?}", tool.name);
                assert!(!line.contains('\u{2014}') && !line.contains('\u{2013}'), "{line:?}");
            }
        }
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
