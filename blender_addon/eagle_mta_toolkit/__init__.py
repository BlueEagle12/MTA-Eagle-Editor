"""Eagle Editor's Blender authoring metadata add-on.

Asset generation lives in Eagle Editor. This add-on only stores scene/object
metadata so interactive Blender authoring and headless Eagle import agree.
"""

import bpy
from bpy.props import BoolProperty, EnumProperty, FloatProperty, IntProperty, PointerProperty, StringProperty
from bpy.types import Panel, PropertyGroup


bl_info = {
    "name": "Eagle MTA Map Authoring",
    "author": "Blue Eagle Team",
    "version": (3, 0, 0),
    "blender": (4, 2, 0),
    "description": "Author MTA map definitions for Eagle Editor's native map builder",
    "category": "Object",
}


class EagleDefinitionProperties(PropertyGroup):
    definition_id: StringProperty(
        name="Definition ID",
        description="Stable model identifier. Empty uses the Blender object name",
        default="",
    )
    txd: StringProperty(
        name="Assigned TXD",
        description="Texture dictionary built and assigned to this model by Eagle Editor",
        default="texture",
    )
    type: EnumProperty(
        name="Map Element Type",
        description="How Eagle Loader creates placements of this model",
        items=(
            ("object", "Object", "Dynamic or normally streamed map object"),
            ("building", "Building", "Static building with a longer default draw distance"),
            ("scenery", "Scenery", "Large visual scenery; collisions are generated separately"),
        ),
        default="object",
    )
    override_lod_distance: BoolProperty(
        name="Override Draw Distance",
        description="Use the distance below instead of Eagle's default for this element type",
        default=False,
    )
    lodDistance: IntProperty(
        name="Draw Distance",
        description="Maximum distance in game units before this model stops drawing or changes LOD",
        default=300,
        min=1,
    )
    lodParent: StringProperty(
        name="LOD Parent",
        description="Definition ID of the lower-detail model; Self marks the highest LOD",
        default="",
    )
    dimension: IntProperty(
        name="Dimension",
        description="MTA dimension for placements of this model; 0 is the main world",
        default=0,
    )
    interior: IntProperty(
        name="Interior",
        description="MTA interior for placements of this model; 0 is the outside world",
        default=0,
    )
    timed: BoolProperty(
        name="Timed Visibility",
        description="Only draw this model during the configured in-game hours",
        default=False,
    )
    timeIn: IntProperty(name="Visible From", description="First visible in-game hour", default=0, min=0, max=23)
    timeOut: IntProperty(name="Visible Until", description="Hour when the model stops being visible", default=24, min=0, max=24)
    disable_collisions: BoolProperty(
        name="Disable Collisions",
        description="Do not assign collision geometry to this model definition",
        default=False,
    )
    disable_backface_culling: BoolProperty(
        name="Double-Sided",
        description="Render both sides of faces; useful for thin signs, leaves, and fences",
        default=False,
    )


class EagleMapSettings(PropertyGroup):
    map_name: StringProperty(name="Map Name", description="Human-readable map name written to zone metadata", default="Eagle Blender Import")
    zone_name: StringProperty(name="Default Zone", description="Zone name used for objects directly under the Scene Collection", default="FREERIDE")
    map_author: StringProperty(name="Author", description="Author written to generated map metadata", default="")
    map_version: StringProperty(name="Version", description="Version written to generated map metadata", default="1.0")
    offset_x: FloatProperty(name="Offset X", description="World offset applied during native map generation", default=0.0)
    offset_y: FloatProperty(name="Offset Y", description="World offset applied during native map generation", default=0.0)
    offset_z: FloatProperty(name="Offset Z", description="World offset applied during native map generation", default=0.0)


class EAGLE_PT_definition(Panel):
    bl_label = "Eagle MTA Definition"
    bl_idname = "EAGLE_PT_definition"
    bl_space_type = "PROPERTIES"
    bl_region_type = "WINDOW"
    bl_context = "object"

    @classmethod
    def poll(cls, context):
        return context.object is not None and context.object.type == "MESH"

    def draw(self, context):
        layout = self.layout
        props = context.object.definition_props
        layout.prop(props, "definition_id")
        layout.prop(props, "txd")
        layout.prop(props, "type")
        layout.prop(props, "override_lod_distance")
        row = layout.row()
        row.enabled = props.override_lod_distance
        row.prop(props, "lodDistance")
        layout.prop(props, "lodParent")
        row = layout.row(align=True)
        row.prop(props, "dimension")
        row.prop(props, "interior")
        layout.prop(props, "timed")
        if props.timed:
            row = layout.row(align=True)
            row.prop(props, "timeIn")
            row.prop(props, "timeOut")
        layout.separator()
        layout.prop(props, "disable_collisions")
        layout.prop(props, "disable_backface_culling")


class EAGLE_PT_map(Panel):
    bl_label = "Eagle Map Settings"
    bl_idname = "EAGLE_PT_map"
    bl_space_type = "PROPERTIES"
    bl_region_type = "WINDOW"
    bl_context = "scene"

    def draw(self, context):
        layout = self.layout
        props = context.scene.eaglemta_map_settings
        layout.prop(props, "map_name")
        layout.prop(props, "zone_name")
        layout.prop(props, "map_author")
        layout.prop(props, "map_version")
        layout.label(text="World Offset")
        row = layout.row(align=True)
        row.prop(props, "offset_x")
        row.prop(props, "offset_y")
        row.prop(props, "offset_z")
        layout.separator()
        layout.label(text="Build by dragging this .blend into Eagle Editor.", icon="INFO")


CLASSES = (EagleDefinitionProperties, EagleMapSettings, EAGLE_PT_definition, EAGLE_PT_map)


def register():
    for cls in CLASSES:
        bpy.utils.register_class(cls)
    bpy.types.Object.definition_props = PointerProperty(type=EagleDefinitionProperties)
    bpy.types.Scene.eaglemta_map_settings = PointerProperty(type=EagleMapSettings)


def unregister():
    if hasattr(bpy.types.Scene, "eaglemta_map_settings"):
        del bpy.types.Scene.eaglemta_map_settings
    if hasattr(bpy.types.Object, "definition_props"):
        del bpy.types.Object.definition_props
    for cls in reversed(CLASSES):
        bpy.utils.unregister_class(cls)
