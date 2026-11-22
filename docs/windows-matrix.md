# Windows test pass

Run on the Windows 11 test box (build 26100, Ryzen 5 3600, RTX 2060 driver 591.86) on 19 Nov 2026, issue #871.

Launcher: bench-suite v0.9.1 installed from its `.msi`, started in the interactive session, window rendered. Tools were installed through the launcher's own install path (`install::place` plus `keep_source`, run by the ignored test `the_named_tool_installs_the_way_the_launcher_does` with `BENCH_TOOL=<name>`), because the install button cannot be clicked over ssh.

Columns: install is the launcher install; source is the tarball kept beside the install (`tools\<name>\source`); start is a launch in the interactive session with a screenshot and a responding process after 10 s; smoke is a CLI or server action.

| app | version | install | source | start | smoke | issues |
|---|---|---|---|---|---|---|
| bench-suite | 0.9.1 | msi | n/a | ok | tool list and install buttons render | none |
| gcad | 0.8.0 | msi | yes | ok | `--version`, `docs` (114 KB) | msi ships no `skills/`, so no agent skill is placed, gcad #877 |
| agentee | 0.5.0 | zip | yes | ok (`view`) | `new board`, `check`, `list` | none, skill placed in `~/.agents/skills/agentee` |
| calcbench | 0.1.0 | msi | yes | ok | `"60 mph to m/s"` gives 26.8224 m / s | none |
| cycler | 0.2.1 | msi | yes | ok | `--help` (no `--version` flag) | none |
| testbench | 0.1.4 | msi | yes | window opens blank white (45 s; `WGPU_BACKEND` dx12, vulkan, gl no change) | `--version`, `--help` | testbench-ui renders nothing, bench-suite #882 |
| ngrind | 0.2.0 | msi | yes | window opens blank white (rechecked after 30 s) | `devices` finds cuda, wgpu and cpu on the 2060; `pow --help` | ngrind-ui renders nothing, ngrind #876 |
| inference-bench | 0.1.1 | msi | yes | ok | `launch --plan`, model download, nvrtc DLLs fetched (#712 fixed), API key generated (#713 fixed) | model load fails on sm_75: `FlashMma: needs Mma16816`, inference-bench #878 |
| agentbench | 0.9.0 | msi | yes | ok, connects | the app starts its own server (127.0.0.1:7420, two established connections), shows the projects page and the first-run provider setup; a separate `serve` gives `/health` `{"ok":true}` and 401 on `/` | none. The launcher's latest is 0.9.0, not 0.9.1 |
| waveshark | 0.5.4 | msi | yes | ok | `--version` | none |
| radio_tool | 0.5.1 | msi | yes | window opens blank white | `--version` | radio_tool-ui renders nothing, bench-suite #882 |
| thermview | 0.1.3 | msi | yes | ok | start only | none |
| printfarm | 0.2.2 | msi | yes | ok | start only | none |
| antenna-toolbox | 0.2.2 | msi | yes | ok | start only | none |
| battery-control | 0.1.1 | msi | yes | window opens blank white | start only | battery-control renders nothing, bench-suite #882 |
| coldbench | 0.3.0 | zip | yes | ok | start only | none |
| kids-free-games | 0.1.5 | not in launcher; zip downloaded directly | no | ok, welcome screen | first screen | none |
| zap-stream-app | 1.3.1 | not in launcher; zip downloaded directly | no | ok, terms and stream list | first screen | none |
| cambench | none | no release yet | no | pending a release | pending | no release, launcher test `agent_tools_install_their_skill_from_their_latest_release` fails on it |
| wallet | none | no release, no repo called wallet | no | not tested | not tested | nothing to install |
| mailbench | none | no release (#847) | no | pending #847 | pending | waiting on #847 |

## Launcher findings

- Windows install, unpack, source tarball and skill placement work for every listed tool.
- The ignored test `every_tool_installs_from_its_latest_release` stalled on the first download (a 0 byte `gcad` zip, no progress for 10 minutes while a direct download of the same asset took 15 s). `egui_bench::update::download` sets a connect timeout only, so a stalled body never errors. Not fixed here, it lives in egui_bench.
- Four GUIs (ngrind-ui, testbench-ui, battery-control, radio_tool-ui) show a blank white window while the process stays responding. Cause: eframe's glow renderer. Fixed on branches `issue-882-wgpu` (testbench, battery-control, radio_tool) and `issue-876` (ngrind): native wgpu plus a startup render check that falls back across DX12, Vulkan and GL, and a `--render-check` flag. `--render-check` on the RTX 2060 draws 9 colours on DX12, Vulkan and GL for testbench-ui and battery-control. The windowed screenshot is not taken yet (the box was at the lock screen); the next release of each app needs a recheck in a signed-in session. Rendered and viewed: bench-suite, gcad, agentee, calcbench, cycler, antenna-toolbox, coldbench, inference-bench, waveshark, thermview, printfarm, agentbench, kids-free-games, zap-stream-app.
- Only agentee ships a skill that the launcher places; gcad's msi misses it.
- Smoke tests were start plus CLI only for the GUI-only tools. Nothing was clicked inside the windows.
