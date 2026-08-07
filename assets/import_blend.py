"""Headless .blend -> Eagle resource bridge.

This script runs inside Blender. Eagle launches it with:
  blender --background scene.blend --python import_blend.py -- ROOT RESULT_JSON
"""

import importlib
import json
import os
import re
import sys
import traceback

import bpy


class _MissingDragonFF(RuntimeError):
    pass


def _args():
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    if len(args) != 2:
        raise RuntimeError("Expected resource root and result JSON path")
    return os.path.abspath(args[0]), os.path.abspath(args[1])


def _load_toolkit():
    errors = []
    for package in (
        "bl_ext.user_default.eagle_mta_toolkit",
        "eagle_mta_toolkit",
    ):
        try:
            toolkit = importlib.import_module(package)
            if not hasattr(bpy.types.Object, "definition_props"):
                try:
                    toolkit.register()
                except Exception:
                    pass
            pipeline = importlib.import_module(package + ".pipeline.generate_resource")
            bridge = importlib.import_module(package + ".pipeline.dragonff_bridge")
            ensure_dragonff = getattr(bridge, "_ensure_dragonff_loaded", None)
            if ensure_dragonff is not None:
                try:
                    ensure_dragonff()
                except Exception as exc:
                    raise _MissingDragonFF(
                        "DragonFF is not installed/enabled in this Blender profile: "
                        + str(exc)
                    ) from exc
            return toolkit, pipeline, bridge
        except _MissingDragonFF:
            raise
        except Exception as exc:
            errors.append(f"{package}: {exc}")
    raise RuntimeError(
        "RRW:MTA Toolkit is not installed/enabled in this Blender profile ("
        + "; ".join(errors)
        + ")"
    )


def _safe_zone_name(name):
    clean = re.sub(r"[<>:\"/\\|?*]", "_", (name or "").strip())
    clean = re.sub(r"\s+", "_", clean).strip("._")
    return clean or "zone"


def _collection_zones(source_path):
    zones = []
    used = set()
    for collection in bpy.context.scene.collection.children:
        visible_meshes = [
            obj
            for obj in collection.all_objects
            if obj.type == "MESH" and not obj.name.lower().endswith(":col")
        ]
        if not visible_meshes:
            continue
        base = _safe_zone_name(collection.name)
        zone = base
        suffix = 2
        while zone.lower() in used:
            zone = f"{base}_{suffix}"
            suffix += 1
        used.add(zone.lower())
        zones.append((collection.name, zone))

    direct_meshes = [
        obj
        for obj in bpy.context.scene.collection.objects
        if obj.type == "MESH" and not obj.name.lower().endswith(":col")
    ]
    if direct_meshes:
        base = _safe_zone_name(os.path.splitext(os.path.basename(source_path))[0])
        zone = base
        suffix = 2
        while zone.lower() in used:
            zone = f"{base}_{suffix}"
            suffix += 1
        zones.append((None, zone))

    if not zones:
        raise RuntimeError("The Blender scene has no mesh objects in any collection")
    return zones


def _zone_objects(collection_name):
    if collection_name is None:
        return set(bpy.context.scene.collection.objects)
    collection = bpy.data.collections.get(collection_name)
    if collection is None:
        raise RuntimeError(f"Collection disappeared while importing: {collection_name}")
    return set(collection.all_objects)


def _prepare_zone(collection_name, zone, pipeline):
    keep = _zone_objects(collection_name)
    helpers = [
        obj for obj in keep if obj.type == "MESH" and obj.name.lower().endswith(":col")
    ]
    visuals = [obj for obj in keep if obj.type == "MESH" and obj not in helpers]
    visual_by_name = {obj.name.lower(): obj for obj in visuals}

    custom_cols = []
    warnings = []
    for helper in helpers:
        target_name = helper.name[:-4].strip()
        target = visual_by_name.get(target_name.lower())
        if target is None:
            warnings.append(
                f"{zone}: collision helper {helper.name!r} has no matching {target_name!r} DFF object"
            )
            continue
        custom_cols.append(
            {
                "target_original": target,
                "target_name": target_name,
                "mesh": helper.data.copy(),
                "matrix_world": helper.matrix_world.copy(),
            }
        )

    for obj in list(bpy.data.objects):
        if obj.type == "MESH" and (obj not in keep or obj in helpers):
            bpy.data.objects.remove(obj, do_unlink=True)

    for obj in visuals:
        if obj.name not in bpy.data.objects:
            continue
        dff = getattr(obj, "dff", None)
        if dff is not None:
            # DragonFF defaults split-normal export to false. That reduces the
            # mesh to one normal per source vertex and destroys hard edges,
            # flat shading, weighted normals, and other authored loop normals.
            # DFF can represent those discontinuities by duplicating vertices,
            # so the editor import must always request the complete loop stream.
            dff.export_normals = True
            dff.export_split_normals = True
        props = getattr(obj, "definition_props", None)
        if props is None:
            continue
        txd = str(getattr(props, "txd", "") or "").strip()
        if txd.lower() in ("", "texture", "textures"):
            props.txd = zone
        else:
            txd = re.sub(r"\.txd$", "", txd, flags=re.IGNORECASE)
            txd = re.sub(r"[<>:\"/\\|?*]", "_", txd).strip()
            props.txd = txd or zone

    return custom_cols, warnings


def _clean_model_name(name, pipeline):
    clean = pipeline.strip_default_object_prefix(name)
    clean, _ = os.path.splitext(re.sub(r"\s+", "", clean))
    clean = re.sub(r"(\.\d+|_root)$", "", clean) or "object"
    encoded = clean.encode("utf-8", errors="replace")
    if len(encoded) <= 15:
        return clean
    result = ""
    for char in clean:
        if len((result + char).encode("utf-8", errors="replace")) > 15:
            break
        result += char
    return result or "object"


def _install_unique_chunk_names(pipeline):
    """Give every grid-split object its own model/definition identifier.

    The toolkit's stock pipeline creates separate Blender objects but assigns
    every chunk the same export name, so DragonFF de-duplicates them.  Keeping
    the Blender numeric suffix as a compact `_cNNN` suffix makes the chunks
    independently placeable while respecting IMG's short-name convention.
    """
    original = getattr(pipeline, "_eagle_original_get_clean_definition_name", None)
    if original is None:
        original = pipeline.get_clean_definition_name
        pipeline._eagle_original_get_clean_definition_name = original

    def unique_name(definition_props, fallback_name):
        owner = getattr(definition_props, "id_data", None)
        owner_name = getattr(owner, "name", "") or ""
        match = re.search(r"\.(\d+)$", owner_name)
        if match:
            suffix = f"_c{int(match.group(1)):03d}"
            base = _clean_model_name(fallback_name, pipeline)
            while len((base + suffix).encode("utf-8", errors="replace")) > 15 and base:
                base = base[:-1]
            return (base or "obj") + suffix
        return original(definition_props, fallback_name)

    pipeline.get_clean_definition_name = unique_name


def _write_custom_collisions(records, root, zone, pipeline, bridge):
    count = 0
    warnings = []
    col_dir = os.path.join(root, "zones", zone, "col")
    os.makedirs(col_dir, exist_ok=True)
    for record in records:
        target_name = _clean_model_name(record["target_name"], pipeline)
        targets = []
        original_target = record.get("target_original")
        if original_target is not None and getattr(original_target, "name", "") in bpy.data.objects:
            targets.append(original_target)
        exact_target = bpy.data.objects.get(target_name)
        if not targets and exact_target is not None:
            targets.append(exact_target)
        if not targets:
            chunk_base = target_name
            while len((chunk_base + "_c000").encode("utf-8", errors="replace")) > 15 and chunk_base:
                chunk_base = chunk_base[:-1]
            chunk_pattern = re.compile(r"^" + re.escape(chunk_base) + r"_c\d{3}$", re.IGNORECASE)
            targets = [
                obj
                for obj in bpy.data.objects
                if obj.type == "MESH" and chunk_pattern.match(obj.name)
            ]
        if not targets:
            warnings.append(
                f"{zone}: {record['target_name']}:COL could not find its exported DFF"
            )
            continue
        for target in targets:
            mesh = record["mesh"].copy()
            temp = bpy.data.objects.new("__EAGLE_CUSTOM_COL", mesh)
            bpy.context.scene.collection.objects.link(temp)
            try:
                relative = target.matrix_world.inverted() @ record["matrix_world"]
                mesh.transform(relative)
                temp.matrix_world = target.matrix_world.copy()
                with pipeline.temporarily_center_objects(temp):
                    data = bridge.export_col_object_to_memory(
                        temp, target.name, version=3, apply_transformations=True
                    )
                if not data:
                    raise RuntimeError("DragonFF returned an empty COL")
                with open(os.path.join(col_dir, target.name + ".col"), "wb") as handle:
                    handle.write(data)
                count += 1
            except Exception as exc:
                warnings.append(f"{zone}: custom collision {record['target_name']}: {exc}")
            finally:
                bpy.data.objects.remove(temp, do_unlink=True)
                if mesh.users == 0:
                    bpy.data.meshes.remove(mesh)
        source_mesh = record["mesh"]
        if source_mesh.users == 0:
            bpy.data.meshes.remove(source_mesh)
    return count, warnings


def main():
    root, result_path = _args()
    source = os.path.abspath(bpy.data.filepath)
    if not source.lower().endswith(".blend"):
        raise RuntimeError("Import Blender requires a saved .blend file")
    os.makedirs(root, exist_ok=True)

    toolkit, pipeline, bridge = _load_toolkit()
    zones = _collection_zones(source)
    summary = {"zones": [], "custom_collisions": 0, "warnings": []}

    for collection_name, zone in zones:
        bpy.ops.wm.open_mainfile(filepath=source)
        toolkit, pipeline, bridge = _load_toolkit()
        custom_cols, warnings = _prepare_zone(collection_name, zone, pipeline)
        summary["warnings"].extend(warnings)
        _install_unique_chunk_names(pipeline)

        map_settings = getattr(bpy.context.scene, "eaglemta_map_settings", None)
        advanced = bool(getattr(map_settings, "use_rrw_materials", False))
        toolkit.run_headless(
            root,
            zone,
            BUILDIMG=False,
            EXPORT=True,
            RENAMEOBJS=True,
            CENTERORIGINS=True,
            SCENERYCHUNKSIZE=512.0,
            OUTPUTCOLLISIONS=True,
            OPTIMIZECOLLISIONS=False,
            SEARCHCOLLISIONS=False,
            BAKELIGHTING=False,
            BAKELIGHTMAPS=False,
            ADVANCEDMATERIALS=advanced,
            num_splits=1,
        )
        custom_count, custom_warnings = _write_custom_collisions(
            custom_cols, root, zone, pipeline, bridge
        )
        summary["custom_collisions"] += custom_count
        summary["warnings"].extend(custom_warnings)
        summary["zones"].append(zone)

        for generated in (
            os.path.join(root, zone + ".blend"),
            os.path.join(root, "output.blend"),
        ):
            try:
                os.remove(generated)
            except FileNotFoundError:
                pass

    with open(result_path, "w", encoding="utf-8") as handle:
        json.dump(summary, handle, indent=2)
    print("EAGLE_IMPORT_RESULT=" + json.dumps(summary, separators=(",", ":")))


if __name__ == "__main__":
    result_path = None
    try:
        _, result_path = _args()
        main()
    except Exception as exc:
        traceback.print_exc()
        if result_path:
            try:
                with open(result_path, "w", encoding="utf-8") as handle:
                    json.dump({"error": str(exc)}, handle)
            except Exception:
                pass
        raise
