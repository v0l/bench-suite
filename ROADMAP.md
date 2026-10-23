# Roadmap

Tools we plan to add to the suite. When one ships, move it to the table in
`README.md` and add it to `TOOLS` in `src/catalog.rs`.

| tool | what it does | status |
|---|---|---|
| printfarm network printers | Backends for Moonraker, OctoPrint, PrusaLink and Bambu LAN behind printfarm's `Printer` trait, and gcad parts sliced and sent in one step | idea |

## printfarm

printfarm shipped with one backend, Marlin over USB, which streams the print
from the desktop. The rest of the plan:

- Network printers keep their own host (Klipper with Moonraker, OctoPrint,
  PrusaLink, Bambu LAN), so a print does not depend on the desktop staying up.
  printfarm uploads the file and reads state back; it is not a print host.
- Slicing goes through the PrusaSlicer CLI with a saved profile.
