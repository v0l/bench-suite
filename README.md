# bench suite

A launcher for the bench tools. It installs each one from that tool's own GitHub
releases, keeps it up to date, and starts it.

| tool | what it does |
|---|---|
| [gcad](https://github.com/v0l/gcad) | Parametric CAD written like G-code, one operation per line |
| [agentee](https://github.com/v0l/agentee) | Boards, schematics, layouts and simulations as plain TOML |
| [printfarm](https://github.com/v0l/printfarm) | Every 3D printer in one window: state, temperatures, camera, job queue and history |
| [Antenna Toolbox](https://github.com/v0l/antenna-toolbox) | Antenna modelling, NanoVNA control and radio path planning |
| [WaveShark](https://github.com/v0l/waveshark) | Wireshark for the radio spectrum |
| [thermview](https://github.com/v0l/thermview) | Radiometric thermal images from InfiRay, Hikmicro, DJI and FLIR |
| [Cycler](https://github.com/v0l/cycler) | Cell-level battery cycle testing |

Every tool keeps its own repo, version and release. The suite only reads them.

## Install

Download `bench-suite` for your machine from
[releases](https://github.com/v0l/bench-suite/releases), or on Linux and macOS:

```sh
curl -fsSL https://benchsuite.net/install.sh | sh
```

## What it does

On start it asks GitHub for the latest release of every tool and of itself.
A tool that is missing gets an install button, one that is behind gets an
update button, and the launcher offers to replace itself when a new one is out.
Downloads are checked against the sha256 GitHub publishes for them.

Tools the suite installs live in its data folder
(`~/.local/share/bench-suite/tools` on Linux,
`~/Library/Application Support/bench-suite/tools` on macOS,
`%LOCALAPPDATA%\bench-suite\tools` on Windows). A tool you installed some other
way, beside the launcher, in a folder listed in `BENCH_SUITE_TOOLS` or on
`PATH`, is found and launched too, and its version is read with `--version`
where the tool has one.

## Agent skills

gcad and agentee are made to be driven by a coding agent, and each release ships
the skill that teaches one how. Installing or updating either puts its skill in
`~/.agents/skills/<tool>`, which Codex, Cursor, OpenCode, Gemini CLI, GitHub
Copilot, pi and most other agents read, the same layout `npx skills add -g`
uses. Claude Code, Windsurf, Goose, Roo Code, Kiro and Qwen Code read their own
folders, so where one of those is set up the suite links its skills folder to
that copy. A skill you put there yourself is left alone, and uninstalling a
tool removes only the copy and links the suite made.

Drop a file on the window and it opens in the tool that reads it: a `.gcad` in
gcad, a `.pcb.toml` or a project folder in agentee, a radiometric `.jpg` in
thermview, a `.gcode` in printfarm.

GitHub answers 60 unauthenticated checks an hour. The last answer for each tool
is kept and asked about again with its ETag, which does not count against that,
and `GITHUB_TOKEN` or `GH_TOKEN` is used when set.

## Adding a tool

Add an entry to `TOOLS` in `src/catalog.rs`: its repo, binary, the arguments
that open a file, and the name of its release asset for each platform. Its
icon goes in `assets/icons/<name>.svg`, drawn on the same 64 px tile as the
others.

## Release

Move the `Unreleased` entries in `CHANGELOG.md` under the new version, then
tag `v*`. CI builds the launcher for each platform and publishes
`bench-suite-<target>.tar.gz` (`.zip` on Windows), the asset names the
launcher looks for when it updates itself, with that version's changelog
section as the release notes. A tag with no section fails the release.

Every tool in the suite keeps a `CHANGELOG.md` the same way, and the
launcher's "Release notes" link opens the GitHub release built from it.
