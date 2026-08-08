# Eagle Editor

**A native 3D world and asset editor for Multi Theft Auto: San Andreas.**

[![Release](https://img.shields.io/badge/release-v0.1.3-2563eb)](https://github.com/BlueEagle12/MTA-Eagle-Editor/releases)
[![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux-4b5563)](#download)
[![License](https://img.shields.io/badge/license-GPL--3.0-green)](LICENSE)

Eagle Editor opens an Eagle/MTA resource as an interactive 3D project. Preview
the complete map, inspect and edit RenderWare assets, build LODs, repair common
asset problems, configure lighting and physics, and save the result back to the
resource without moving between several separate tools.

## What’s new in 0.1.3

- **Per-DFF optimization and repair:** inspect an individual model in the DFF
  editor, choose the cleanup passes to run, preview the result, and stage it
  directly into the resource archive.
- **RenderWare alpha-face ordering:** optimize transparent faces to render
  after opaque geometry and preserve that draw order in the exported BinMesh,
  fixing common in-game glass, fence, and foliage depth artifacts.
- **DFF editor polish:** improved optimization controls, texture-pairing flow,
  material editing, and related editor interaction fixes.

> [!IMPORTANT]
> Eagle Editor is an early public release. Keep a backup or use version control for
> production resources, especially before running bulk repair or optimization.

## Highlights

- Native 3D preview of map placements, DFF geometry, textures, collisions, and
  IMG archives.
- Project Manager for opening an existing resource or creating a new one.
- DFF and COL editing, including editable collision boxes, spheres, capsules,
  rotated cuboids, materials, normals, UVs, and 2DFX lights.
- Automatic LOD generation with simplified geometry, a compact texture atlas,
  bounds collision, placement assignment, validation, and undo/redo.
- Background asset optimization, DFF repair, collision-bounds repair, and
  definition validation.
- Day/night lights, material emitters, shadow-casting controls, and vertex-light
  preview and evaluation tools.
- LOD coverage review with draw-distance preview, association overlays, density
  warnings, and one-click generation for uncovered objects.
- Physics authoring for breakable props, per-object overrides, and custom
  traffic-light behavior.
- Race radar generation for the full San Andreas map area.
- Non-destructive WIP snapshots and versioned project metadata in
  `EagleScene.eaglescne`.

## Download

Prebuilt Windows and Linux packages are available on the
[GitHub Releases page](https://github.com/BlueEagle12/MTA-Eagle-Editor/releases).
Each release includes a `SHA256SUMS.txt` file for archive verification.
Maintainers should follow [RELEASING.md](RELEASING.md) when preparing a new
version.

## Getting started

### Windows

1. Download and extract the Windows archive.
2. Keep `EagleEditor.exe` and the included DLL files in the same folder.
3. Run `EagleEditor.exe`.
4. In the Project Manager, open the root folder of an existing Eagle resource,
   or choose **New Project**.

Windows may display a SmartScreen warning because the first release is not
code-signed. Verify the archive checksum below before choosing **Run anyway**.

### Linux

The Linux build requires an x86-64 desktop with X11 and OpenGL drivers.

```bash
tar -xzf EagleEditor-v0.1.3-linux-x86_64.tar.gz
cd EagleEditor-v0.1.3-linux-x86_64
chmod +x EagleEditor
./EagleEditor
```

Select the resource root in the Project Manager. You can also open a resource
directly from the command line on either platform:

```text
EagleEditor "/path/to/resource"
```

On first start, Eagle asks for the GTA: San Andreas installation folder and
verifies it by checking for `models/gta3.img`. The selected path is stored in
the user-local Eagle Editor preferences and can be changed later in the editor's
Preferences dialog.

## Build from source

Eagle Editor requires Rust 1.85 or newer. Clone the repository, then build and
test it from the repository root:

```bash
git clone https://github.com/BlueEagle12/MTA-Eagle-Editor.git
cd MTA-Eagle-Editor
cargo test --locked
cargo build --release --locked
```

The executable is written to `target/release/eagle-editor` on Linux or
`target/release/eagle-editor.exe` on Windows. To build and immediately open a
resource:

```bash
./scripts/run.sh "/path/to/resource"
```

On Linux, install a native compiler toolchain, OpenGL development files, and
X11 development files if they are not already present. See
[CONTRIBUTING.md](CONTRIBUTING.md) for the project conventions and local checks.

The LOD audit reads `eagleLoader/config.xml` relative to the working directory
when it is available. Set `EAGLE_LOADER_CONFIG` to an explicit `config.xml`
path when developing against a loader stored elsewhere.

## Project format

An existing project is an MTA resource containing Eagle's map and asset layout.
The editor recognizes, among other files:

```text
resource/
├── meta.xml
├── EagleScene.eaglescne
├── eagleZones.txt
├── imgs/*.img
└── zones/
    ├── **/*.map
    └── **/*.definition
```

`EagleScene.eaglescne` stores editor-only, versioned project metadata such as
lights, material emitters, shadow overrides, and editable collision shapes. It
does not need to be listed in `meta.xml` and is not read by the runtime loader.

**Save WIP** writes an isolated snapshot under
`.light_mapper_wip/latest/`. A normal **Save** promotes the current work into
the resource and removes that snapshot.

## Viewport controls

| Input | Action |
| --- | --- |
| `W` `A` `S` `D` | Move camera |
| `Q` / `E` | Move down / up |
| Right mouse drag | Look around |
| `Shift` | Move faster |
| `C` | Toggle Freeroam / Focus camera |
| `.` | Focus the selected item |
| Right-click an object row | Move the camera to that object |

## Optional Blender integration

Importing a `.blend` scene requires **Blender 4.2 or newer** and these add-ons
enabled in the same Blender installation/profile used by Eagle:

- [BlueEagle12/MTA-Tool-Kit](https://github.com/BlueEagle12/MTA-Tool-Kit)
  (`eagle_mta_toolkit`) — required for resource generation and Eagle scene
  metadata.
- [DragonFF](https://github.com/Parik27/DragonFF) — required for DFF and COL
  export.
- RRW Material Tools — optional advanced-material authoring tools.

Eagle checks the `BLENDER_PATH` environment variable first, then `blender` on
`PATH`, followed by common installation locations. If more than one Blender
version is installed, set `BLENDER_PATH` to the exact executable whose profile
has the required add-ons enabled.

## Documentation

The complete documentation is maintained in the
[Eagle Editor Wiki](https://github.com/BlueEagle12/MTA-Eagle-Editor/wiki):

- [EagleScene](https://github.com/BlueEagle12/MTA-Eagle-Editor/wiki/EagleScene)
  — project metadata and saving
- [Lights](https://github.com/BlueEagle12/MTA-Eagle-Editor/wiki/Lights) —
  importing and exporting lights
- [Material Emitters](https://github.com/BlueEagle12/MTA-Eagle-Editor/wiki/Material-Emitters)
  and [Shadow Casters](https://github.com/BlueEagle12/MTA-Eagle-Editor/wiki/Shadow-Casters)
  — lighting controls
- [Collision Capsules](https://github.com/BlueEagle12/MTA-Eagle-Editor/wiki/Collision-Capsules)
  and [Collision Cuboids](https://github.com/BlueEagle12/MTA-Eagle-Editor/wiki/Collision-Cuboids)
  — collision authoring
- [Breakable Props](https://github.com/BlueEagle12/MTA-Eagle-Editor/wiki/Breakable-Props)
  — destructible object workflow
- [Custom Traffic Lights](https://github.com/BlueEagle12/MTA-Eagle-Editor/wiki/Custom-Traffic-Lights)
  — native GTA traffic-light setup

## Troubleshooting and feedback

- On Linux, make sure current OpenGL drivers and the system X11 libraries are
  installed if the editor does not open.
- On Windows, do not move the executable away from its bundled DLL files.
- For Blender import failures, verify `BLENDER_PATH`, Blender 4.2+, and the
  add-ons enabled in that exact Blender profile.
- Include the operating system, steps to reproduce, and relevant project/log
  details when filing an
  [issue](https://github.com/BlueEagle12/MTA-Eagle-Editor/issues).

## License

Eagle Editor is distributed under the [GNU General Public License v3.0](LICENSE).
Bundled third-party components and trademarks are described in
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
