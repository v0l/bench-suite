# Changelog

All notable changes to bench suite are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed
- radio_tool opens its window, and a `.rdt` codeplug dropped on the suite opens in it.

## [0.4.2] - 2026-10-19

### Added
- A page for every tool, opened by clicking its card: what it can do, a screenshot from its README, what it opens, what the coding agent can run on it, and its README and changelog.
- radio_tool, firmware and codeplugs for TYT, Baofeng, Radioddity, Yaesu and Ailunce radios. The agent can run its file commands and open its window.

## [0.4.1] - 2026-10-19

### Added
- A README and changelog reader for every tool and the suite, in the card menu and from "what is new" on the update banner, with a key to open the page in a browser.

## [0.4.0] - 2026-10-19

### Added
- Battery Control, a live monitor and control window for BMSes and power stations.
- A coding agent that fills the window, opened with the Agent key in the header. It talks to any
  OpenAI-compatible server (set under Model) and renders its replies as markdown, with code
  blocks you can copy, tables, lists and links.
- It loads the agent skills of the installed tools (`load_skill`), reads and writes files in a
  workspace folder, and runs each tool's headless command line (`gcad check`, `agentee edit`,
  `thermview render` and the like) to check its own work. It cannot leave the workspace, run
  anything but the listed subcommands, or start a tool's window except through `launch`.
- A `bash` tool runs shell commands in an OS sandbox (Landlock on Linux, Seatbelt on macOS, through
  the `nono` crate): it reads the system and the installed tools, writes only inside the workspace,
  and cannot read the rest of your files. The network is open. Not available on Windows.
- `request_access` asks you to share a folder outside the workspace, with a reason, and you choose
  read only, read and write, or no. The file tools and `bash` can then use it until you click it
  away or clear the chat. Credential folders and the whole home folder are refused without asking,
  and the tab reads "agent needs you" while a request waits.
- It can also list the tools, check for updates, install or update one, start one on a file, and
  ask you to confirm an uninstall. It is told what each tool does and which files it opens.

### Fixed
- Restarting after an update opened nothing and said file not found, because the running program
  was looked up by the name the update had just moved it to.

## [0.3.0] - 2026-10-18

### Added
- printfarm, every 3D printer in one window. A `.gcode` dropped on the launcher is queued in it.

## [0.2.0] - 2026-10-14

### Added
- Installing or updating gcad or agentee installs the agent skill its release ships, in
  `~/.agents/skills` where Codex, Cursor, OpenCode, Gemini CLI, GitHub Copilot and pi read it,
  linked for Claude Code, Windsurf, Goose, Roo Code, Kiro and Qwen Code where they are set up.
  A skill you put there yourself is left alone, and uninstalling removes only what the suite
  made. The card says where the skill went.

## [0.1.1] - 2026-10-13

### Added
- Windows `.msi` and macOS `.dmg` installers, with the suite's icon.
- agentee opens on a project folder you pick.
- A tool that exits with an error shows the last line it wrote.

### Fixed
- The layout in a narrow window.

## [0.1.0] - 2026-10-13

First release.

### Added
- One window for gcad, agentee, Antenna Toolbox, WaveShark, thermview and Cycler: install,
  update, launch and uninstall each from its own GitHub releases, checked against the sha256
  published with them.
- Updates itself when a new launcher is out.
- Finds a tool installed some other way, beside the launcher, in `BENCH_SUITE_TOOLS` or on
  `PATH`, and reads its version.
- Drop a file on the window to open it in the tool that reads it, with recent files per tool.
- Release checks cached with ETags so they do not run into GitHub's limit, and `GITHUB_TOKEN`
  or `GH_TOKEN` used when set.

[Unreleased]: https://github.com/v0l/bench-suite/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/v0l/bench-suite/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/v0l/bench-suite/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/v0l/bench-suite/releases/tag/v0.1.0
