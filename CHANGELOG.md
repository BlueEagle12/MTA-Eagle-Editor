# Changelog

## v0.1.8 — 2026-10-07

- Added standard MTA:SA `.map` support, project-relative map registration, placement reassignment, and preservation of map metadata, comments, unsupported elements, and nested data when saving.
- Added Eagle zone creation and reassignment with undo/redo, plus automatic definition ownership based on each model's live instances.
- Added the default GTA:SA exterior world preview with streamed objects, LODs, and water. Standalone default-map changes are temporary and saving is disabled; resource projects support world removals and conversion to editable placements.
- Expanded DFF vehicle authoring with hierarchy validation, material and dummy presets, component parenting, body colors, embedded VLO generation, and vehicle collision generation.
- Added same-DFF internal element separation and pivot tools, with improved 2DFX placement from the active selection.
- Improved world edit and undo/redo performance by rebuilding affected render cells instead of the entire scene.
- Added resource-owned UV1 lightmap previews and the shader-only material plugin host with discovery, validation, and enable/disable controls in Preferences. Material plugins are separate installs.
- Added background scenery and camera-follow flag previews; improved alpha texture mipmaps, native SA flags, material cleanup, and LOD handling.
- Added Blender installation selection and search in Preferences, improved short-window preference scrolling, compact toolbar actions, project navigation, and placement on visible surfaces.
- Fixed Windows drag-and-drop handling for long paths and save/recovery checks for pending binary edits.

[Full changelog](https://github.com/BlueEagle12/MTA-Eagle-Editor/compare/v0.1.7...v0.1.8)

## v0.1.7 — 2026-09-10

- Added resizable side panels across all tabs, with widths remembered per tab during the session.
