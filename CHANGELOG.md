# Changelog

All notable changes to bench suite are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

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

[Unreleased]: https://github.com/v0l/bench-suite/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/v0l/bench-suite/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/v0l/bench-suite/releases/tag/v0.1.0
