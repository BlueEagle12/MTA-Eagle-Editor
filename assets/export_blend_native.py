"""Fast evaluated .blend -> Eagle native mesh bridge.

Blender evaluates the dependency graph and writes geometry/material streams only.
Eagle Editor owns spatial slicing, DFF/TXD/IMG generation, and map definitions.
"""

import json
import math
import os
import re
import struct
import sys
import traceback

import bpy
from bpy.props import BoolProperty, EnumProperty, IntProperty, PointerProperty, StringProperty
from bpy.types import PropertyGroup
from bpy_extras.node_shader_utils import PrincipledBSDFWrapper
from mathutils import Matrix


MAGIC = b"EAGMSH02"
PROGRESS = "EAGLE_IMPORT_PROGRESS|"


class _EagleDefinitionProperties(PropertyGroup):
    definition_id: StringProperty(default="")
    txd: StringProperty(default="texture")
    type: EnumProperty(items=(("object", "Object", ""), ("building", "Building", ""), ("scenery", "Scenery", "")), default="object")
    override_lod_distance: BoolProperty(default=False)
    lodDistance: IntProperty(default=300)
    lodParent: StringProperty(default="")
    dimension: IntProperty(default=0)
    interior: IntProperty(default=0)
    timed: BoolProperty(default=False)
    timeIn: IntProperty(default=0)
    timeOut: IntProperty(default=24)
    disable_collisions: BoolProperty(default=False)
    disable_backface_culling: BoolProperty(default=False)


def ensure_metadata_schema():
    if hasattr(bpy.types.Object, "definition_props"):
        return
    try:
        bpy.utils.register_class(_EagleDefinitionProperties)
    except RuntimeError:
        pass
    bpy.types.Object.definition_props = PointerProperty(type=_EagleDefinitionProperties)


def progress(fraction, message):
    clean = str(message).replace("\r", " ").replace("\n", " ").replace("|", "/")
    print(f"{PROGRESS}{max(0.0, min(1.0, float(fraction))):.4f}|{clean}", flush=True)


def args():
    values = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    if len(values) != 3:
        raise RuntimeError("Expected resource root, result JSON path, and import options")
    return os.path.abspath(values[0]), os.path.abspath(values[1]), json.loads(values[2])


def safe_name(value, fallback="object"):
    value = re.sub(r"[^A-Za-z0-9_-]+", "_", str(value or "").strip()).strip("_")
    return value or fallback


def instance_name(value):
    # Blender duplicates datablocks as Name.001, Name.002, etc. The suffix is
    # an instance label, not part of the RenderWare model identity.
    return safe_name(re.sub(r"\.\d{3}$", "", str(value or "").strip()))


def texture_name(value):
    value = re.sub(r"\.(bmp|jpe?g|png|tga|dds|tiff?)$", "", str(value or ""), flags=re.I)
    value = re.sub(r"\.\d{3}$", "", value)
    output = "".join(
        char.lower() if char.isascii() and (char.isalnum() or char in "_-")
        else "_" if char.isspace() else ""
        for char in value.strip()
    )[:31]
    return output or "texture"


def txd_name(value):
    value = re.sub(r"\.txd$", "", str(value or "").strip(), flags=re.I)
    value = re.sub(r"[<>:\\|?*/\s]+", "_", value).strip("._").lower()
    return value or "texture"


def prop(owner, name, default=None):
    if owner is None:
        return default
    try:
        value = getattr(owner, name)
        return default if value is None else value
    except Exception:
        try:
            return owner.get(name, default)
        except Exception:
            return default


def definition_props(obj):
    return getattr(obj, "definition_props", None)


def top_level_zones(source):
    zones = []
    used = set()
    assigned = set()
    for collection in bpy.context.scene.collection.children:
        objects = [obj for obj in collection.all_objects if obj.type == "MESH" and not obj.name.lower().endswith(":col")]
        if not objects:
            continue
        base = safe_name(collection.name, "zone")
        zone = base
        suffix = 2
        while zone.lower() in used:
            zone = f"{base}_{suffix}"
            suffix += 1
        used.add(zone.lower())
        zones.append((zone, objects))
        assigned.update(obj.as_pointer() for obj in objects)
    direct = [
        obj for obj in bpy.context.scene.collection.objects
        if obj.type == "MESH" and not obj.name.lower().endswith(":col") and obj.as_pointer() not in assigned
    ]
    if direct:
        zones.append((safe_name(os.path.splitext(os.path.basename(source))[0], "zone"), direct))
    if not zones:
        raise RuntimeError("The Blender scene has no visible mesh objects")
    return zones


def write_string(handle, value):
    encoded = str(value or "").encode("utf-8", errors="replace")
    if len(encoded) > 65535:
        encoded = encoded[:65535]
    handle.write(struct.pack("<H", len(encoded)))
    handle.write(encoded)


def material_image(material):
    if material is None or not material.use_nodes:
        return None
    try:
        return PrincipledBSDFWrapper(material, is_readonly=True).base_color_texture.image
    except Exception:
        return None


def material_alpha_image(material, base_image):
    if material is None or not material.use_nodes or material.node_tree is None:
        return None, None
    for node in material.node_tree.nodes:
        if node.type != "BSDF_PRINCIPLED":
            continue
        alpha = node.inputs.get("Alpha")
        if alpha is None or not alpha.is_linked:
            continue
        link = alpha.links[0]
        source = link.from_node
        if source.type != "TEX_IMAGE" or source.image is None:
            continue
        channel = "alpha" if link.from_socket.name.lower() == "alpha" else "color"
        if source.image == base_image and channel == "alpha":
            return None, None
        return source.image, channel
    return None, None


def save_image_png(image, output):
    old_format = image.file_format
    old_path = image.filepath_raw
    try:
        image.file_format = "PNG"
        try:
            image.save_render(output)
        except Exception:
            image.filepath_raw = output
            image.save()
    finally:
        image.file_format = old_format
        image.filepath_raw = old_path


def save_masked_texture_png(image, mask, mask_channel, output):
    if mask is None:
        save_image_png(image, output)
        return
    width, height = (int(image.size[0]), int(image.size[1]))
    mask_width, mask_height = (int(mask.size[0]), int(mask.size[1]))
    if width <= 0 or height <= 0 or mask_width <= 0 or mask_height <= 0:
        raise RuntimeError(f"Cannot combine empty base/mask image for {image.name}")
    base_pixels = list(image.pixels[:])
    mask_pixels = list(mask.pixels[:])
    if len(base_pixels) < width * height * 4 or len(mask_pixels) < mask_width * mask_height * 4:
        raise RuntimeError(f"Cannot read base/mask pixels for {image.name}")
    combined = bpy.data.images.new(
        f"Eagle Combined {image.name}",
        width=width,
        height=height,
        alpha=True,
    )
    try:
        try:
            combined.colorspace_settings.name = image.colorspace_settings.name
        except Exception:
            pass
        combined.alpha_mode = "STRAIGHT"
        for y in range(height):
            mask_y = min(mask_height - 1, int((y + 0.5) * mask_height / height))
            for x in range(width):
                mask_x = min(mask_width - 1, int((x + 0.5) * mask_width / width))
                base_at = (y * width + x) * 4
                mask_at = (mask_y * mask_width + mask_x) * 4
                if mask_channel == "alpha":
                    value = mask_pixels[mask_at + 3]
                else:
                    value = (
                        mask_pixels[mask_at]
                        + mask_pixels[mask_at + 1]
                        + mask_pixels[mask_at + 2]
                    ) / 3.0
                base_pixels[base_at + 3] *= max(0.0, min(1.0, value))
        combined.pixels = base_pixels
        save_image_png(combined, output)
    finally:
        bpy.data.images.remove(combined)


def files_identical(first, second):
    if os.path.getsize(first) != os.path.getsize(second):
        return False
    with open(first, "rb") as left, open(second, "rb") as right:
        while True:
            left_chunk = left.read(1024 * 1024)
            right_chunk = right.read(1024 * 1024)
            if left_chunk != right_chunk:
                return False
            if not left_chunk:
                return True


def stage_texture(image, mask, mask_channel, root, txd, name, staged):
    if image is None:
        return ""
    image_pointer = (
        image.as_pointer(),
        mask.as_pointer() if mask is not None else 0,
        mask_channel or "",
    )
    folder = os.path.join(root, "txd_build", txd)
    os.makedirs(folder, exist_ok=True)
    comparison = None
    resolved = name
    suffix = 2
    try:
        while True:
            key = (txd.lower(), resolved.lower())
            previous = staged.get(key)
            output = os.path.join(folder, resolved + ".png")
            if previous is None:
                if comparison is None:
                    save_masked_texture_png(image, mask, mask_channel, output)
                else:
                    os.replace(comparison, output)
                    comparison = None
                staged[key] = {image_pointer}
                if resolved != name:
                    print(
                        f"EAGLE_IMPORT_WARNING|Renamed conflicting texture "
                        f"'{name}' to '{resolved}' in {txd}.txd",
                        flush=True,
                    )
                return resolved
            if image_pointer in previous:
                return resolved
            if comparison is None:
                comparison = os.path.join(
                    folder,
                    f".eagle_compare_{os.getpid()}_{abs(hash(image_pointer))}.png",
                )
                save_masked_texture_png(image, mask, mask_channel, comparison)
            if files_identical(output, comparison):
                previous.add(image_pointer)
                print(
                    f"EAGLE_IMPORT_WARNING|Reused identical texture "
                    f"'{resolved}' in {txd}.txd",
                    flush=True,
                )
                return resolved
            ending = f"_{suffix}"
            resolved = f"{name[:31 - len(ending)]}{ending}"
            suffix += 1
    finally:
        if comparison is not None and os.path.exists(comparison):
            os.remove(comparison)


def color_layer(mesh, preferred, fallback_active=False):
    attributes = getattr(mesh, "color_attributes", None)
    if not attributes:
        return None
    for name in preferred:
        layer = attributes.get(name)
        if layer is not None and layer.domain == "CORNER":
            return layer
    active = getattr(attributes, "active_color", None)
    return active if fallback_active and active is not None and active.domain == "CORNER" else None


def rgba(layer, loop_index):
    if layer is None or loop_index >= len(layer.data):
        return (255, 255, 255, 255)
    value = layer.data[loop_index].color
    return tuple(max(0, min(255, round(float(channel) * 255.0))) for channel in value[:4])


def export_object(handle, obj, zone, root, depsgraph, staged):
    evaluated = obj.evaluated_get(depsgraph)
    mesh = evaluated.to_mesh(preserve_all_data_layers=True, depsgraph=depsgraph)
    if mesh is None:
        return False
    try:
        mesh.calc_loop_triangles()
        if not mesh.loop_triangles:
            return False
        props = definition_props(obj)
        name = instance_name(prop(props, "definition_id", "") or obj.name)
        txd = txd_name(prop(props, "txd", "texture"))
        tag = str(prop(props, "type", "object") or "object").lower()
        if tag not in {"object", "building", "scenery"}:
            tag = "object"
        lod = float(prop(props, "lodDistance", 700 if tag == "building" else 300) or 300)
        dimension = int(prop(props, "dimension", 0) or 0)
        interior = int(prop(props, "interior", 0) or 0)
        materials = list(mesh.materials)
        if not materials:
            materials = [None]
        texture_names = []
        for index, material in enumerate(materials):
            material_name = material.name if material is not None else f"material_{index}"
            image = material_image(material)
            mask, mask_channel = material_alpha_image(material, image)
            tex = texture_name(material_name) if image is not None else ""
            tex = stage_texture(image, mask, mask_channel, root, txd, tex, staged)
            texture_names.append(tex)

        uv_layers = [layer for layer in mesh.uv_layers if layer is not None][:8]
        day = color_layer(mesh, ("Day", "day", "Col", "Color"), fallback_active=True)
        night = color_layer(mesh, ("Night", "night", "NightColor"))
        world = evaluated.matrix_world
        origin, rotation, scale = world.decompose()
        model_matrix = Matrix.Diagonal((scale.x, scale.y, scale.z, 1.0))
        normal_matrix = model_matrix.to_3x3().inverted_safe().transposed()
        euler = rotation.to_euler("XYZ")
        vertex_count = len(mesh.loop_triangles) * 3

        write_string(handle, zone)
        write_string(handle, name)
        write_string(handle, txd)
        write_string(handle, tag)
        write_string(handle, str(prop(props, "lodParent", "") or ""))
        color_flags = (1 if day is not None else 0) | (2 if night is not None else 0)
        time_in = int(prop(props, "timeIn", 0) or 0)
        time_out = int(prop(props, "timeOut", 24) or 24)
        definition_flags = 0
        if bool(prop(props, "disable_backface_culling", False)):
            definition_flags |= 1 << 21
        if bool(prop(props, "disable_collisions", False)):
            definition_flags |= 1 << 50
        handle.write(struct.pack("<fii6fiiQBBHI", lod, dimension, interior, origin.x, origin.y, origin.z, math.degrees(euler.x), math.degrees(euler.y), math.degrees(euler.z), time_in, time_out, definition_flags, len(uv_layers), color_flags, len(materials), vertex_count))
        handle.write(struct.pack("<I", len(mesh.loop_triangles)))
        for index, material in enumerate(materials):
            write_string(handle, texture_names[index])
            diffuse = material.diffuse_color if material is not None else (1.0, 1.0, 1.0, 1.0)
            handle.write(struct.pack("<4f", *[float(value) for value in diffuse[:4]]))

        vertex_index = 0
        triangles = []
        for triangle in mesh.loop_triangles:
            indices = []
            for loop_index in triangle.loops:
                loop = mesh.loops[loop_index]
                position = model_matrix @ mesh.vertices[loop.vertex_index].co
                normal = (normal_matrix @ loop.normal).normalized()
                handle.write(struct.pack("<3f3f", position.x, position.y, position.z, normal.x, normal.y, normal.z))
                for layer in uv_layers:
                    uv = layer.data[loop_index].uv
                    handle.write(struct.pack("<2f", float(uv.x), float(uv.y)))
                handle.write(bytes(rgba(day, loop_index)))
                handle.write(bytes(rgba(night, loop_index)))
                indices.append(vertex_index)
                vertex_index += 1
            triangles.append((indices[0], indices[1], indices[2], max(0, min(65535, int(triangle.material_index)))))
        for triangle in triangles:
            handle.write(struct.pack("<IIIH", *triangle))
        return True
    finally:
        evaluated.to_mesh_clear()


def main():
    root, result_path, options = args()
    source = os.path.abspath(bpy.data.filepath)
    ensure_metadata_schema()
    os.makedirs(root, exist_ok=True)
    geometry_path = result_path + ".eagmesh"
    zones = top_level_zones(source)
    objects = [(zone, obj) for zone, members in zones for obj in members]
    progress(0.04, f"Evaluating {len(objects)} Blender mesh objects")
    depsgraph = bpy.context.evaluated_depsgraph_get()
    staged = {}
    count = 0
    with open(geometry_path, "wb") as handle:
        handle.write(MAGIC)
        handle.write(struct.pack("<I", 0))
        for index, (zone, obj) in enumerate(objects, 1):
            if export_object(handle, obj, zone, root, depsgraph, staged):
                count += 1
            if index == 1 or index % 100 == 0 or index == len(objects):
                progress(0.04 + 0.76 * index / max(1, len(objects)), f"Evaluated {index}/{len(objects)} mesh objects")
        handle.seek(len(MAGIC))
        handle.write(struct.pack("<I", count))
    result = {
        "version": 1,
        "native_geometry": geometry_path,
        "zones": [zone for zone, _ in zones],
        "objects": count,
        "warnings": [],
        "options": options,
    }
    with open(result_path, "w", encoding="utf-8") as handle:
        json.dump(result, handle)
    progress(0.82, "Blender evaluation complete; Eagle Editor owns the remaining build")


if __name__ == "__main__":
    result_path = None
    try:
        _, result_path, _ = args()
        main()
    except Exception as error:
        traceback.print_exc()
        if result_path:
            with open(result_path, "w", encoding="utf-8") as handle:
                json.dump({"error": str(error)}, handle)
        raise
