pub const GCAD: &str = "\
Parametric CAD written like G-code: a text file where every line is one operation (sketch, \
extrude, hole, fillet, shell, gear, text), run top to bottom. Pure Rust on the monstertruck \
B-rep kernel.\n\
Files: .gcad is one part. .gasm is an assembly, and anything with more than one part is one: it \
brings parts in, places them with mates, joins them with turning and sliding joints and checks \
they do not overlap. .gsim is a torque simulation that sizes joint motors. .gdwg is a workshop \
drawing sheet that sits beside an assembly.\n\
Covers sheet metal with exportable DXF flat patterns, weldments with a cut list, enclosures, and \
a built-in library of screws, nuts and bearings. Exports STEP, STL, OBJ, 3MF, SVG and PDF \
drawings and a bill of materials.\n\
Launching it on a file opens a viewer that rebuilds whenever the file is saved. It has a command \
line for check, render, export, bom, measure, query, sim and docs, which is how a coding agent \
checks its work.\n\
Reach for it when the user wants a mechanical part, a bracket, a case, a gear train, a robot \
arm or an assembly of them.";

pub const AGENTEE: &str = "\
Electronics design as plain TOML. An agent writes and edits the files, agentee checks them, \
computes what the stackup gives and draws them. The window is where the human watches, and on a \
layout they can move parts, route tracks and place vias by hand, saved back into the same TOML.\n\
Files: *.board.toml (fab preset and rules, stackup, outline, vias, net classes with impedance \
and current targets), *.sym.toml (schematic symbol), *.fp.toml (footprint), *.sch.toml \
(schematic), *.pcb.toml (layout, checked for connectivity and clearance), *.sim.toml (FDTD, DC \
drop, thermal, cascade, channel and PDN simulations).\n\
Imports parts from the installed KiCad libraries, routes, fills zones, writes the fab package \
(Gerbers, drill, BOM, placement), prices a BOM at Mouser, Farnell and DigiKey, and exports the \
board as STEP for enclosure CAD. Its command line has check, edit, render, calc, sim, fab and \
more, and `agentee mcp` serves a project to an agent.\n\
The suite starts it with the `view` verb, so launching it opens a live viewer. Give it a project \
folder or any of the files above and it reloads as they change.\n\
Reach for it when the user wants a schematic, a PCB, a board stackup, a trace sized for \
impedance or current, or an RF or power simulation of a layout.";

pub const PRINTFARM: &str = "\
One window for every 3D printer: state, temperatures, progress and a camera per printer, a job \
queue with pictures, a 3D preview drawn from the G-code itself for any slicer, the bed mesh, SD \
card files and a console, and the history of every print.\n\
A job is copied to the printer's SD card and printed from there, so closing printfarm or pulling \
the cable does not stop it, and reopening picks the print back up.\n\
Marlin printers over USB work today. Moonraker, OctoPrint, PrusaLink and Bambu LAN are planned, \
so say so rather than promising them.\n\
Launching it on a .gcode or .gco file queues that job. printfarm does not slice: the file must \
already be G-code, so a part from gcad needs slicing first.\n\
Reach for it when the user wants to see, queue or start prints.";

pub const ANTENNA_TOOLBOX: &str = "\
A desktop app for designing antennas, measuring them and planning radio paths. It opens no \
files, so launch it bare.\n\
Design: 37 templates (Yagi, Moxon, log-periodic, quad, J-pole, collinear, QFH, discone, helix, \
dish, Vivaldi, microstrip patch, printed inverted-F, random wire on a 9:1 unun and more) solved \
by method of moments, surface MoM, physical optics or FDTD, with a cut sheet, an optimiser, \
match synthesis, RF exposure checks against ICNIRP and FCC limits, and NEC-2 deck import and \
export.\n\
VNA: drives a NanoVNA-H or H4 over USB with SOL calibration and a Smith chart.\n\
Path: point to point terrain profiles with Longley-Rice, ITU-R P.528, P.2108 and P.452, \
broadcast coverage maps, and an HF sky map with ITU-R P.533 reliability by hour and band.\n\
Reach for it when the user wants an antenna designed or tuned, a NanoVNA sweep, or to know \
whether a radio link will work.";

pub const WAVESHARK: &str = "\
Wireshark for the radio spectrum, an OSINT tool for RF. A cheap SDR left on a band says what is \
transmitting around you, decoded or not, instead of only what is on one frequency you were told \
about. It opens no files, so launch it bare.\n\
Decodes weather stations and sensors, remotes and door contacts, pagers (POCSAG), DMR, TETRA, \
M17, LoRaWAN and Meshtastic, wireless M-Bus utility meters, ADS-B aircraft, WFM with RDS, NFM, \
AM, SSB and CW voice with a local speech transcript, and SSTV and DVB-T pictures. It transmits \
on a radio that can.\n\
Needs hardware: an RTL2832U dongle, a HackRF One or a LimeSDR, or a tuner on another machine \
over rtl_tcp. Without one it has nothing to receive.\n\
It has an agent of its own in its Agent view and serves MCP on 127.0.0.1:8931 while it runs. \
This chat does not drive it: launch it and the user works there.\n\
Reach for it when the user wants to see or decode what is on the air.";

pub const RADIO_TOOL: &str = "\
Firmware and codeplug tool for handheld radios, with a window and a command line. launch opens \
the window, where the user picks the radio and flashes or edits it. A release from before the \
window only prints its help when launched, so for file work use it through run_tool.\n\
Without a radio it reads files: `fw-info FILE` says what a firmware file is, `codeplug-info FILE` \
lists a codeplug's channels (TYT/Baofeng DM-1701 RDT, Baofeng UV-5R and UV-17Pro images, CHIRP \
images of those), `wrap -r MODEL -s ADDR:FILE -o OUT` builds a firmware file from raw segments \
and `unwrap FILE -o PREFIX` takes one apart, `make-xor FILE` guesses an unknown firmware's key, \
and `models` lists the radios it knows.\n\
With a radio attached it flashes firmware and reads and writes codeplugs over USB, a serial \
cable or Bluetooth. Those commands change the radio and are not open to this chat: launch the \
window for the user, or tell them the command to run themselves, for example `radio_tool flash \
-i fw.bin` to check a firmware against the radio and the same with `--write` to flash it.\n\
Reach for it when the user has a handheld radio to flash or program, or a firmware or codeplug \
file to inspect or build.";

pub const THERMVIEW: &str = "\
A thermal imaging analysis tool for thermographers. It reads, renders and measures radiometric \
images: InfiRay .irg, Hikmicro JPEG, FLIR R-JPEG and AFF .img, and DJI R-JPEG.\n\
Nine palettes, a colour bar with a draggable window, linear, log and equalised mappings, \
hottest and coldest markers, spot measurements, a pointer readout, and PNG export. For FLIR \
R-JPEG it recalibrates from the raw counts when emissivity, distance, reflected temperature, \
air temperature or humidity change.\n\
It opens .irg, .jpg, .jpeg, .img and .seq. An ordinary JPEG opens too but carries no \
temperatures, so only radiometric files give readings. The command line has `info` and \
`render` for use without a window.\n\
Reach for it when the user has a thermal camera file and wants temperatures from it.";

pub const CYCLER: &str = "\
Battery cycle testing driven by the cells rather than the pack voltage. It charges, rests and \
discharges and counts amp-hours, and decides on the highest or lowest cell. The suite starts \
its window, the `cycler-ui` binary. It opens no files.\n\
Charge in standard, top-balance and bulk-only modes, discharge in CC, CV, CR or CP, stop at a \
state of charge, and run cycle plans that measure capacity against the nameplate. It logs every \
sample to CSV and shows the BMS's own alarms. A battery with no BMS works on voltage limits \
alone.\n\
Built for a Pylontech US2000C on the bench with an OWON supply and a programmable load, and \
speaks to other packs through the battery-control backends (Seplos, Pace, Daly, JK, JBD and \
more, some over Bluetooth).\n\
It drives real chargers and loads. Launch it and let the user choose the settings; never \
suggest voltage or current limits for a pack you have not been told about.\n\
Reach for it when the user wants to charge, discharge or measure the capacity of cells.";

pub const BATTERY_CONTROL: &str = "\
Live monitor and control for batteries, BMSes and power stations through one window, whatever \
the make. It scans Bluetooth and probes USB serial ports, then shows charge, voltage, current, \
every cell, temperatures, ports and alarms, with a history plot. It opens no files.\n\
Where the device allows it, it switches the charge and discharge MOSFETs, the balancer and \
output ports, writes settings, and runs the pairing flow some power stations need.\n\
Reads Anker SOLIX, JK, JBD, SOK, Renogy, Pace, Seplos, Daly, Bluetti, Jackery, EcoFlow, Victron \
and Pylontech. `--demo` shows a made-up pack with no hardware. The suite starts its window.\n\
It drives real hardware. Launch it and let the user choose what to switch or write.\n\
Reach for it when the user wants to see what a battery is doing or change one of its settings.";

pub const FOREMAN: &str = "\
Runs coding agents in a git project's worktrees and keeps an issue tracker beside them. A \
project is a repository's main checkout, a worktree is a branch an agent works in, and a chat is \
one agent pinned to one worktree. Issues belong to a project, and starting an agent on one gives \
it a fresh worktree. It opens no files, and the suite starts its window.\n\
The window starts a server in the background and connects to it, and the server keeps running \
after the window closes so agents carry on. It can also talk to a server on another machine. \
Everything is kept in one SQLite file.\n\
It runs agents that change files and run commands in the user's repositories, so launch it and \
let the user choose the project and agent; do not start chats for them.\n\
Reach for it when the user wants to run a coding agent on a repository or track issues for one.";

pub const COLDBENCH: &str = "\
A watch-only Bitcoin wallet for hardware signers: Blockstream Jade, Coldcard and Trezor. It keeps \
only public keys; every payment is built here and checked and signed on the device. Pages follow \
Sparrow's: transactions, send, receive, addresses, UTXOs and settings.\n\
It talks to the devices itself rather than through Python HWI, and says when a device is waiting \
for its PIN. It syncs by spreading the wallet's addresses over many public Esplora and Electrum \
servers so no one server sees them all, or from the user's own servers. It can build Stonewall \
spends for privacy and draws each transaction as a flow of coins. It opens .psbt files to review \
them or add the signatures in them.\n\
It moves real money. Launch it and let the user build, check, sign and broadcast; never fill in an \
address or amount for them, and never broadcast. `--devices` lists plugged in signers.\n\
Reach for it when the user wants to see a hardware wallet's balance or history, receive to it, or \
pay from it.";

pub const NGRIND: &str = "\
Mines vanity npubs and NIP-13 proof of work events for Nostr, on the GPU or the CPU. Backends are \
cuda (NVIDIA, by a wide margin the fastest), wgpu (any GPU with Vulkan, Metal or DirectX 12) and \
cpu, and by default it picks the best one present.\n\
A vanity search walks the curve from a random point looking for an npub with a given prefix or \
suffix (or a hex key), and prints each key it finds. A proof of work run grinds the nonce of a \
Nostr event until its id has the difficulty asked for.\n\
Launching it opens a window with the backend toggle, the speed and the chance of a hit so far. The \
agent can run only `ngrind devices`, which lists the backends and GPUs. `vanity` prints secret \
keys and `pow` takes an nsec, so those are not open to this chat: launch the window for the \
user, or tell them the command to run themselves, for example `ngrind vanity --prefix alice \
--count 3` or `ngrind pow --file note.json --difficulty 28 --nsec nsec1...`.\n\
Reach for it when the user wants a vanity npub or a Nostr note with proof of work.";
