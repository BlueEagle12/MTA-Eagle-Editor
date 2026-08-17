# Eagle Editor

Eagle Editor is a native 3D world and asset editor for
**Multi Theft Auto: San Andreas**. It opens an Eagle/MTA resource as an
interactive project for map preview, RenderWare asset editing, LOD generation,
lighting, physics, validation, and repair.

> This is an early release. Back up production resources or keep them under
> version control before using bulk repair, optimization, or save operations.

## Getting started

### Windows

1. Extract the complete archive to a writable folder.
2. Keep `EagleEditor.exe` and the included DLL files together.
3. Run `EagleEditor.exe`.
4. Open an existing resource root in the Project Manager, or choose
   **New Project**.

Windows may show a SmartScreen warning because this release is not code-signed.
Verify the archive checksum from the release page before choosing **Run
anyway**.

### Linux

The Linux build requires an x86-64 X11 desktop and working OpenGL drivers.

```bash
chmod +x EagleEditor
./EagleEditor
```

On either platform, a resource can also be opened directly:

```text
EagleEditor "/path/to/resource"
```

## Project format

Open the root of an Eagle/MTA resource rather than an individual map or asset.
A typical project contains:

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

`EagleScene.eaglescne` contains versioned editor metadata such as lights,
material emitters, shadow overrides, and editable collision shapes. It does not
need to be listed in `meta.xml` and is not read by the runtime loader.

## Main features

- Interactive 3D preview of map placements, DFF geometry, textures,
  collisions, and IMG archives.
- DFF and COL editing, including materials, UVs, normals, 2DFX lights,
  collision capsules, and rotated cuboids.
- Automatic LOD geometry, texture atlas, bounds collision, placement
  assignment, validation, and undo/redo.
- Asset optimization, DFF repair, collision-bounds repair, and project
  validation on background workers.
- Day/night lights, material emitters, shadow controls, and vertex-light
  preview and evaluation.
- LOD coverage and draw-distance review tools.
- Breakable prop physics and custom GTA traffic-light authoring.
- Race radar generation for the full San Andreas map area.
- WIP snapshots that keep unfinished work separate from the live resource.

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

## Saving

**Save WIP** writes an isolated snapshot to `.light_mapper_wip/latest/` inside
the resource. A normal **Save** promotes the current editor state into the
resource and removes that snapshot.

Bulk generation, repair, optimization, and save jobs run in the background.
Wait for the active job to finish before closing the editor or modifying the
same resource with another tool.

## Native Blender map builder

Importing a `.blend` scene requires **Blender 4.2 or newer**. Eagle now includes
the MTA Tool Kit workflow and uses its own shared DFF/TXD code, so DragonFF and
the separate Tool Kit add-on are not required. RRW Material Tools remains
optional for advanced material authoring.

Eagle checks `BLENDER_PATH` first, then `blender` on `PATH`, followed by common
installation locations. If multiple Blender versions are installed, set
`BLENDER_PATH` to the exact executable.

Drag a saved `.blend` file onto an open Eagle map, or use **Import Blender**.
The setup prompt controls visual chunk size, mesh splitting, and origin
centering. Blender evaluates the scene once; Eagle natively slices geometry,
builds DFFs, follows each definition's TXD setting, builds TXDs/IMG archives,
and writes the map. The import log is retained at `logs/blender-import.log`.

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
  — destructible objects
- [Custom Traffic Lights](https://github.com/BlueEagle12/MTA-Eagle-Editor/wiki/Custom-Traffic-Lights)
  — native GTA traffic-light setup

## Troubleshooting

- **Linux window does not open:** update the graphics driver and ensure the
  system OpenGL and X11 libraries are installed.
- **Windows reports a missing DLL:** re-extract the complete archive and keep
  the executable beside the bundled DLL files.
- **Blender import fails:** check `BLENDER_PATH`, the Blender version, and
  `logs/blender-import.log` in the project.

Report reproducible problems on the
[GitHub issue tracker](https://github.com/BlueEagle12/MTA-Eagle-Editor/issues)
with the operating system, steps to reproduce, and relevant project or log
details.

## License

Eagle Editor is distributed under the GNU General Public License v3.0.
