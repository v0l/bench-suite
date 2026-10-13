# bench suite

A launcher for the bench tools. It installs each one from that tool's own GitHub
releases, keeps it up to date, and starts it.

| tool | what it does |
|---|---|
| [gcad](https://github.com/v0l/gcad) | Parametric CAD written like G-code, one operation per line |
| [agentee](https://github.com/v0l/agentee) | Boards, schematics, layouts and simulations as plain TOML |
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

Drop a file on the window and it opens in the tool that reads it: a `.gcad` in
gcad, a `.pcb.toml` or a project folder in agentee, a radiometric `.jpg` in
thermview.

GitHub answers 60 unauthenticated checks an hour. The last answer for each tool
is kept and asked about again with its ETag, which does not count against that,
and `GITHUB_TOKEN` or `GH_TOKEN` is used when set.

## Adding a tool

Add an entry to `TOOLS` in `src/catalog.rs`: its repo, binary, the arguments
that open a file, and the name of its release asset for each platform. Its
icon goes in `assets/icons/<name>.svg`, drawn on the same 64 px tile as the
others.

## Release

Tag `v*`. CI builds the launcher for each platform and publishes
`bench-suite-<target>.tar.gz` (`.zip` on Windows), the asset names the
launcher looks for when it updates itself.
