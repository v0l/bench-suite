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
| [radio_tool](https://github.com/v0l/radio_tool) | Firmware and codeplugs for TYT, Baofeng, Radioddity, Yaesu and Ailunce radios |
| [thermview](https://github.com/v0l/thermview) | Radiometric thermal images from InfiRay, Hikmicro, DJI and FLIR |
| [Cycler](https://github.com/v0l/cycler) | Cell-level battery cycle testing |
| [Battery Control](https://github.com/v0l/battery-control) | Monitor and control BMSes and power stations over Bluetooth or serial |

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

## Agent chat

The Agent key in the header turns the window into a coding agent. Under Model it takes the
address of any server speaking the OpenAI chat completions API with tool calls (OpenAI,
OpenRouter, llama.cpp, vLLM, Ollama), the model to ask for, a key where the server wants one,
and a workspace folder (`~/Documents/bench-suite` by default).

The agent loads a tool's skill, writes the tool's files in the workspace, and runs the tool's
headless command line to check them, then opens the result in the tool's own window. It reads
the skills in `~/.agents/skills` and in the workspace's `.agents/skills`. File tools stay inside
the workspace. `run_tool` only runs the subcommands listed for each tool in `src/catalog.rs`,
with paths inside the workspace. `bash` runs shell commands in an OS sandbox (Landlock on Linux,
Seatbelt on macOS): the system and the tools are readable, only the workspace is writable, the
rest of your files are hidden from it and the network is open. Windows has no sandbox yet, so
`bash` is not offered there. When the work needs a folder outside the workspace the agent calls `request_access` and you
choose read only, read and write, or no; the sandbox and the file tools then reach exactly that
folder and nothing around it. `run_tool` takes a shared folder's path only if it was shared for
writing. It can also install, update, start and (after you confirm)
uninstall tools.

Settings are kept in `bench-suite/agent` in your config folder, readable by you only.

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
