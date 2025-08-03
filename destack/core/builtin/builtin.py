from typing import TYPE_CHECKING

from .enum import EnumDeclaration, declare_enum

if TYPE_CHECKING:
    pass


#
# Enums
#


class EnumType(EnumDeclaration):
    #
    # CORE
    #

    # builtin [1]
    OBJECT_KIND = 1
    ENUM_TYPE = 2
    NODE_TYPE = 3
    STRUCT_TYPE = 4
    TRAIT_TYPE = 5
    HANDLE_TYPE = 6
    OBJECT_STABILITY = 8
    UNIVERSE_DOMAIN = 9
    UNIVERSE_CATEGORY = 10
    PROPERTY_REFERENCE_TYPE = 13
    MATERIALIZATION = 14
    RUNTIME_PLATFORM = 30
    RUNTIME_LANGUAGE = 31
    RUNTIME_TYPE = 32
    EVENT_STATUS = 50

    # common [100_000]
    # type/value
    PRIMITIVE_TYPE = 100_000
    TYPE_CARDINALITY = 100_001
    SCALAR_TYPE = 100_002
    VALUE_FACTORY = 100_003
    PROPERTY_ZONE = 100_006
    EDGE_TYPE = 100_007
    EDGE_DIRECTION = 100_008
    CASCADE_ACTION = 100_009
    ENCODING = 100_010

    # edit
    EDIT_TYPE = 200
    EDIT_OPERATION = 201

    # query
    CONDITIONAL_TYPE = 300
    AGGREGATION_TYPE = 301
    SORT_MODE = 302
    SORT_TYPE = 303
    JOIN_TYPE = 304
    EXPRESSION_TYPE = 306
    QUERY_TYPE = 320

    # universe [200_000]
    CLIENT_TYPE = 200_200

    # space [300_000]
    FOLDER_TYPE = 300_500
    BRANCH_TYPE = 300_300
    SNAPSHOT_TYPE = 300_400
    SNAPSHOT_STATUS = 300_401

    # runtime [400_000]
    # ...

    # generate [500_000]
    # ...

    #
    # BASICS
    #

    # entity [10_000_000]
    FILE_RETENTION_MODE = 10_000_000
    FILE_TYPE = 10_000_001
    TEXT_SPAN_TYPE = 10_000_002
    ICON_TYPE = 10_000_003
    INDEX_TYPE = 10_000_100
    CONSTRAINT_TYPE = 10_000_200
    MIGRATION_TYPE = 10_000_300

    # logic [10_100_000]
    FUNCTION_OPERATOR = 10_100_400
    METHOD_TYPE = 10_100_500
    ACTION_TYPE = 10_100_600
    TRIGGER_TYPE = 10_100_700
    TIMER_TYPE = 10_100_800
    DAY_OF_WEEK = 10_100_801
    MONTH = 10_100_802
    SCHEDULE_FREQUENCY = 10_100_803
    ENVIRONMENT_TYPE = 10_100_000
    RUN_STATUS = 10_101_000
    LOG_LEVEL = 10_101_100

    # intelligence [10_200_000]

    # access [10_300_000]
    ROLE_TYPE = 10_300_300
    SANCTION_TYPE = 10_300_400
    ENTITLEMENT_TYPE = 10_300_500

    # quality [10_400_000]
    # ...

    #
    # SIMULATION
    #

    # geometry [20_000_000]
    ANCHOR = 20_000_000
    LENGTH_TYPE = 20_000_001
    LAYOUT = 20_000_002
    DISTRIBUTE = 20_000_003
    ALIGN = 20_000_004
    DIRECTION = 20_000_005
    OVERFLOW = 20_000_006
    ARROW_HEAD_TYPE = 20_000_200

    # physics [20_100_000]
    # ...

    # perception [20_200_000]
    MOUSE_BUTTON = 20_200_000

    # animation [20_300_000]
    TRANSITION_TYPE = 20_300_000
    SPRING_TYPE = 20_300_001
    EFFECT_TYPE = 20_300_100
    REPEAT_TYPE = 20_300_002
    EASING = 20_300_003

    #
    # CANVAS
    #

    # audio [30_000_000]
    # ...

    # image [30_100_000]
    # ...

    # video [30_200_000]
    # ...

    # model [30_300_000]
    # ...

    # paint [30_400_000]
    # ...

    # style [30_500_000]
    # ...

    #
    # STAGE
    #

    # scene [40_000_000]
    LAYER_TYPE = 40_000_200

    # view [40_100_000]
    # ...

    # style [40_200_000]
    COLOR_TYPE = 40_200_000
    COLOR_SHADE = 40_200_001
    COLOR_HUE = 40_200_002
    COLOR_INTENT = 40_200_003
    FILL_TYPE = 40_200_400
    FILL_POSITION = 40_200_401
    FILL_SIZE = 40_200_402
    FONT_TYPE = 40_200_500
    FONT_WEIGHT = 40_200_501
    FONT_SIZE = 40_200_502
    TEXT_ALIGN = 40_200_503
    TEXT_DECORATION = 40_200_504
    TEXT_TRANSFORM = 40_200_505
    BORDER_TYPE = 40_200_600
    SHADOW_TYPE = 40_200_700
    SHADOW_POSITION = 40_200_701
    GRADIENT_TYPE = 40_200_800
    STROKE_TYPE = 40_200_900
    TEXT_SPLIT_TYPE = 40_200_901
    OFFSCREEN_BEHAVIOR = 40_200_902

    # rendering [40_200_000]
    # ...

    # shaders [40_300_000]
    # ...

    # lighting [40_400_000]
    # ...

    #
    # DEPLOYMENT
    #

    # cloud [50_000_000]
    REGION = 50_000_001
    REGION_AREA = 50_000_002
    REGION_CONTINENT = 50_000_003
    TENANCY = 50_000_004
    MACHINE_TYPE = 50_000_000

    # observability [50_100_000]
    # ...

    # experience [50_200_000]
    # ...

    #
    # DISTRIBUTION
    #

    # localization [60_000_000]
    # ...

    # legal [60_100_000]
    # ...

    # social [60_200_000]
    NOTIFICATION_STATUS = 60_200_300

    # finance [60_300_000]
    # ...

    # commerce [60_400_000]
    # ...

    #
    # STUDIO
    #

    # editor [100_000_000]
    # ...


declare_enum(EnumType.ENUM_TYPE)(EnumType)


@declare_enum(EnumType.OBJECT_KIND)
class ObjectKind(EnumDeclaration):
    NODE = 1, "Node", "Object with data, logic and universally addressable identity"
    STRUCT = 2, "Struct", "Object with data and logic (embedded elsewhere)"
    HANDLE = 3, "Handle", "Object with special data and logic (runtime only)"


@declare_enum(EnumType.OBJECT_STABILITY)
class ObjectStability(EnumDeclaration):
    DYNAMIC = 1, "Definition may change in every compatible way"
    # GROWABLE = 2, "Definition may change with new properties at the end (only)"
    # NOTE :Performance: ObjectStability.GROWABLE is annoying to implement but probably worth it
    STATIC = 7, "Definition may never change"


@declare_enum(EnumType.TRAIT_TYPE)
class TraitType(EnumDeclaration):
    #
    # CORE
    #

    # builtin [1]
    # LOCAL?
    # storage
    # RELATIONAL/OLTP, INDEXED; ANALYTIC, ...?
    ORDERED = 1, "Ordered", "Is ordered"
    # PAUSABLE?
    RESOURCE = 2, "Resource", "Is a Resource"
    VARIANT = 3, "Variant", "Is a Variant"

    # common [100_000]
    # ...

    # universe [200_000]
    # ...

    # space [300_000]
    # ...

    # runtime [400_000]
    # ...

    # generate [500_000]
    # ...

    #
    # BASICS
    #

    # entity [10_000_000]
    # ...

    # logic [10_100_000]
    RUNNABLE = 10_100_000, "Runnable", "Can be run"

    # intelligence [10_200_000]
    # ...

    # access [10_300_000]
    OWNABLE = 10_300_000, "Ownable", "Is ownable"
    OWNED = 10_300_001, "Owned", "Is owned"
    JOINABLE = 10_300_002, "Joinable", "Is joinable"
    ACTOR = 10_300_003, "Actor", "Is an Actor"

    # quality [10_400_000]
    # ...

    #
    # SIMULATION
    #

    # geometry [20_000_000]
    # ...

    # physics [20_100_000]
    # ...

    # perception [20_200_000]
    INTERACTIVE = 20_200_000, "Interactive", "Can be interacted with"
    DRAGGABLE = 20_200_001, "Draggable", "Can be dragged"
    SELECTABLE = 20_200_002, "Selectable", "Can be selected"
    # ...

    # animation [20_300_000]
    # ...

    #
    # CANVAS
    #

    # audio [30_000_000]
    # ...

    # image [30_100_000]
    # ...

    # video [30_200_000]
    # ...

    # model [30_300_000]
    # ...

    # paint [30_400_000]
    # ...

    # style [30_500_000]
    # ...

    #
    # STAGE
    #

    # scene [40_000_000]
    # ...

    # view [40_100_000]
    # ANIMATABLE/TWEENABLE, ...

    # rendering [40_200_000]
    # ...

    # shaders [40_300_000]
    # ...

    # lighting [40_400_000]
    # ...

    #
    # DEPLOYMENT
    #

    # cloud [50_000_000]
    # ...

    # observability [50_100_000]
    # ...

    # experience [50_200_000]
    # ...

    #
    # DISTRIBUTION
    #

    # localization [60_000_000]
    # ...

    # legal [60_100_000]
    # ...

    # social [60_200_000]
    STARABLE = 60_200_100, "Starable", "Can be starred"
    REACTABLE = 60_200_000, "Reactable", "Can be reacted to"
    FOLLOWABLE = 60_200_200, "Followable", "Can be followed"
    # RATEABLE, VOTABLE, ...
    # ASSIGNABLE, MESSAGEABLE, CLOSABLE, LOCKABLE, ...

    # finance [60_300_000]
    # ...

    # commerce [60_400_000]
    # ...


@declare_enum(EnumType.NODE_TYPE)
class NodeType(EnumDeclaration):
    #
    # CORE
    #

    # builtin [1]
    # root
    NODE = 1, "Node", "Root of all Nodes"
    ENTITY = 2, "Entity", "Versioned, stateful Node"
    EVENT = 3, "Event", "Immutable datum of something happening"
    EDIT_EVENT = 10, "Edit Event", None
    # CHANGE_EVENT?

    # common [100_000]
    # ...

    # universe [200_000]
    UNIVERSE = 200_000, "Universe", "The Destack computational universe"
    # GALAXY, ...
    # HANDLE?, ...
    # user
    USER = 200_100, "User", None
    # FRIENDSHIP, FRIENDSHIP_INVITE, ...
    CLIENT = 200_200, "Client", None
    # CREDENTIAL, ACCOUNT, PROFILE, ...
    # organization
    ORGANIZATION = 200_300, "Organization", None
    TEAM = 200_400, "Team", "Team in an Organization"

    # space [300_000]
    SPACE = 300_000, "Space", "Universal Space"
    TAG = 300_100, "Tag", None
    TAGGING = 300_200, "Tagging", None
    # TRAIT?
    # FRAGMENT (multiple disjoint trees)
    # SLOT (inside tree)
    # LINK/PORTAL (to another subtree)
    # TIMELINE, TRACK, (KEY)FRAME, ...
    BRANCH = 300_300, "Branch", None
    SNAPSHOT = 300_400, "Snapshot", "Point in Space-time"
    FOLDER = 300_500, "Folder", "Sub-space of a Space"
    # APPLICATION (extends Folder?), ...
    # DEPENDENCY, ...
    # VERSION, ...
    # HISTORY, REPLAY, ...
    # FORK, ...
    # LINK, PORTAL, ...

    # runtime [400_000]
    # ...

    # generate [500_000]
    # ...

    #
    # BASICS
    #

    # entity [10_000_000]
    FILE = 10_000_000, "File", None
    # DIRECTORY, SYNC, ...
    # INDEX, CONSTRAINT, MIGRATION, ...
    # REMOTE, ...
    # SECRET, ...
    INDEX = 10_000_100, "Index", "Index of an Entity"
    CONSTRAINT = 10_000_200, "Constraint", "Constraint of an Entity"
    # EXPECTATION, ...
    MIGRATION = 10_000_300, "Migration", "Migration of an Entity"
    MIGRATION_OPERATION = (10_000_301, "Migration Operation", "Migration Operation of an Entity")
    # custom
    CUSTOM_EVENT_DEFINITION = 10_000_400, "Custom Event", "Custom Event Definition"
    CUSTOM_STRUCT_DEFINITION = (
        10_000_500,
        "Custom Struct",
        "Custom Struct Definition",
    )
    CUSTOM_MESSAGE_DEFINITION = (
        10_000_600,
        "Custom Message",
        "Custom Message Definition",
    )
    CUSTOM_PROPERTY_DEFINITION = (
        10_000_700,
        "Custom Property",
        "Custom Property Definition",
    )
    CUSTOM_ENUM_DEFINITION = 10_000_800, "Custom Enum", "Custom Enum Definition"
    CUSTOM_OPTION_DEFINITION = (
        10_000_900,
        "Custom Option",
        "Custom Option Definition",
    )
    # CUSTOM_ALIAS_DEFINITION, CUSTOM_UNION_DEFINITION, ...

    # logic [10_100_000]
    ENVIRONMENT = 10_100_000, "Environment", None
    MODE = 10_100_100, "Mode", None
    SCRIPT = 10_100_200, "Script", None
    CUSTOM_EVENT = 10_100_300, "Signal", "Custom Event instance"
    MEASUREMENT_EVENT = 10_100_301, "Measurement of a Metric", None
    FUNCTION = 10_100_400, "Function", None
    METHOD = 10_100_500, "Method", None
    ACTION = 10_100_600, "Action", None
    TRIGGER = 10_100_700, "Trigger", None
    TRIGGER_EVENT = 10_100_701, "Trigger Event", None
    TIMER = 10_100_800, "Timer", None
    TIMER_EVENT = 10_100_801, "Timer Event", None
    TIMER_STARTED_EVENT = 10_100_802, "Timer Started Event", None
    TIMER_PAUSED_EVENT = 10_100_803, "Timer Paused Event", None
    TIMER_RESUMED_EVENT = 10_100_804, "Timer Resumed Event", None
    TIMER_COMPLETED_EVENT = 10_100_805, "Timer Completed Event", None
    TIMER_CANCELLED_EVENT = 10_100_806, "Timer Cancelled Event", None
    ROUTE = 10_100_900, "Route", None
    # EFFECT, ...
    # BREAKPOINT, ...
    # ROOM, TOPIC, CHANNEL, ...
    # QUEUE, TASK, ...
    # SEMAPHORE, LOCK/LATCH, ...
    # RATE_LIMIT, ...
    # STATE_MACHINE, STATE, STATE_TRANSITION, ...
    # PLATFORM_VARIANT, STATE_VARIANT, ...
    # RELEASE, DEPLOYMENT, ...
    # PREVIEW, DRAFT, ROLLOUT, ...
    # TASK, TASK_GROUP/TASK_QUEUE, ...
    # JOB, ...
    RUN = 10_101_000, "Run", None
    RUN_EVENT = 10_101_001, "Run Event", None
    RUN_STARTED_EVENT = 10_101_002, "Run Started Event", None
    RUN_PAUSE_REQUESTED_EVENT = 10_101_003, "Run Pause Requested Event", None
    RUN_PAUSED_EVENT = 10_101_004, "Run Paused Event", None
    RUN_RESUME_REQUESTED_EVENT = 10_101_005, "Run Resume Requested Event", None
    RUN_RESUMED_EVENT = 10_101_006, "Run Resumed Event", None
    RUN_STOP_REQUESTED_EVENT = 10_101_007, "Run Stop Requested Event", None
    RUN_FAILED_EVENT = 10_101_008, "Run Failed Event", None
    RUN_COMPLETED_EVENT = 10_101_009, "Run Completed Event", None
    SPAN_EVENT = 10_101_010, "Span", None
    LOG_EVENT = 10_101_011, "Log", None

    # intelligence [10_200_000]
    # MODEL, FINETUNE, ...
    # PROMPT, INFERENCE/COMPLETION/..., ...
    # RECOMMENDATION, ...

    # access [10_300_000]
    PERMISSION = 10_300_000, "Permission", "Permission for something"
    MEMBERSHIP = 10_300_100, "Membership", "Membership to something"
    MEMBERSHIP_EVENT = 10_300_101, "Membership Event", None
    MEMBERSHIP_JOINED_EVENT = 10_300_102, "Membership Join Event", None
    MEMBERSHIP_LEFT_EVENT = 10_300_103, "Membership Leave Event", None
    INVITE = 10_300_200, "Invite", "Invite to a Space/Folder"
    INVITE_EVENT = 10_300_201, "Invite Event", None
    INVITE_SENT_EVENT = 10_300_202, "Invite Sent Event", None
    INVITE_RESCINDED_EVENT = 10_300_203, "Invite Rescinded Event", None
    INVITE_ACCEPTED_EVENT = 10_300_204, "Invite Accepted Event", None
    INVITE_REJECTED_EVENT = 10_300_205, "Invite Rejected Event", None
    ROLE = 10_300_300, "Role", "Role in something"
    ROLE_EVENT = 10_300_301, "Role Event", None
    ROLE_ASSIGNED_EVENT = 10_300_302, "Role Assigned Event", None
    ROLE_UNASSIGNED_EVENT = 10_300_303, "Role Unassigned Event", None
    SANCTION = 10_300_400, "Sanction", "Temporary or permanent restriction"
    SANCTION_EVENT = 10_300_401, "Sanction Event", None
    SANCTION_REQUESTED_EVENT = 10_300_402, "Sanction Requested Event", None
    SANCTION_GRANTED_EVENT = 10_300_403, "Sanction Granted Event", None
    SANCTION_REVOKED_EVENT = 10_300_404, "Sanction Revoked Event", None
    SANCTION_EXPIRED_EVENT = 10_300_405, "Sanction Expired Event", None
    ENTITLEMENT = 10_300_500, "Entitlement", "Temporary or permanent grant"
    ENTITLEMENT_EVENT = 10_300_501, "Entitlement Event", None
    ENTITLEMENT_REQUESTED_EVENT = (
        10_300_502,
        "Entitlement Requested Event",
        None,
    )
    ENTITLEMENT_GRANTED_EVENT = 10_300_503, "Entitlement Granted Event", None
    ENTITLEMENT_REVOKED_EVENT = 10_300_504, "Entitlement Revoked Event", None
    ENTITLEMENT_EXPIRED_EVENT = 10_300_505, "Entitlement Expired Event", None
    # CHALLENGE, ...

    # quality [10_400_000]
    # TEST, TEST_SUITE, TEST_CASE, TEST_RESULT, ...
    # FIXTURE, MOCK, ...
    # LINT, WARNING, ERROR, ...
    # DEPRECATION, ...

    #
    # SIMULATION
    #

    # geometry [20_000_000]
    # VECTOR_NETWORK, VECTOR_POINT, VECTOR_SEGMENT, VECTOR_REGION, ...
    ENTITY2D = 20_000_000, "Entity2D", "2D Entity"
    ENTITY3D = 20_000_100, "Entity3D", "3D Entity"
    SHAPE2D = 20_000_200, "Shape2D", None
    LINE_SHAPE2D = 20_000_201, "Line Shape2D", None
    ARROW_SHAPE2D = 20_000_202, "Arrow Shape2D", None
    RECTANGLE_SHAPE2D = 20_000_203, "Rectangle Shape2D", None
    ELLIPSE_SHAPE2D = 20_000_204, "Ellipse Shape2D", None
    CAPSULE_SHAPE2D = 20_000_205, "Capsule Shape2D", None
    STAR_SHAPE2D = 20_000_206, "Star Shape2D", None
    POLYGON_SHAPE2D = 20_000_207, "Polygon Shape2D", None
    PATH_SHAPE2D = 20_000_208, "Path Shape2D", None
    SHAPE3D = 20_000_300, "Shape3D", None

    # physics [20_100_000]
    # BODY, BODY2D, ...
    # BODY_EVENT, CONTACT_EVENT, COLLISION_EVENT, ...
    # RIGID_BODY, SOFT_BODY, ...
    # CLOTH, ...
    # LIQUID, ...
    # COLLIDER, ...
    # SKELETON, BONE, ...
    # JOINT, FIXED_JOINT, FREE_JOINT, SPHERICAL_JOINT, SPRING, MOTOR, ...
    # NAVIGATION, ...

    # perception [20_200_000]
    INPUT_EVENT = 20_200_000, "Input Event", None
    # pointer events
    POINTER_EVENT = 20_200_100, "Pointer Event", None
    POINTER_DOWN_EVENT = 20_200_101, "Pointer Down Event", None
    POINTER_UP_EVENT = 20_200_102, "Pointer Up Event", None
    POINTER_MOVE_EVENT = 20_200_103, "Pointer Move Event", None
    POINTER_ENTER_EVENT = 20_200_104, "Pointer Enter Event", None
    POINTER_OVER_EVENT = 20_200_105, "Pointer Over Event", None
    POINTER_LEAVE_EVENT = 20_200_106, "Pointer Leave Event", None
    POINTER_LONG_PRESS_EVENT = 20_200_107, "Long Press Event", None
    # mouse events
    MOUSE_EVENT = 20_200_200, "Mouse Event", None
    CLICK_EVENT = 20_200_201, "Click Event", None
    SINGLE_CLICK_EVENT = 20_200_202, "Single Click Event", None
    DOUBLE_CLICK_EVENT = 20_200_203, "Double Click Event", None
    TRIPLE_CLICK_EVENT = 20_200_204, "Triple Click Event", None
    WHEEL_EVENT = 20_200_210, "Wheel Event", None
    # key events
    KEY_EVENT = 20_200_300, "Key Event", None
    KEY_DOWN_EVENT = 20_200_301, "Key Down Event", None
    KEY_UP_EVENT = 20_200_302, "Key Up Event", None
    KEY_PRESS_EVENT = 20_200_303, "Key Press Event", None
    # drag events
    DRAG_EVENT = 20_200_400, "Drag Event", None
    DRAG_START_EVENT = 20_200_401, "Drag Start Event", None
    DRAG_END_EVENT = 20_200_402, "Drag End Event", None
    DRAG_OVER_EVENT = 20_200_403, "Drag Over Event", None
    DRAG_ENTER_EVENT = 20_200_404, "Drag Enter Event", None
    DRAG_LEAVE_EVENT = 20_200_405, "Drag Leave Event", None
    DROP_EVENT = 20_200_406, "Drop Event", None
    # clipboard events
    CLIPBOARD_EVENT = 20_200_500, "Clipboard Event", None
    COPY_EVENT = 20_200_501, "Copy Event", None
    CUT_EVENT = 20_200_502, "Cut Event", None
    PASTE_EVENT = 20_200_503, "Paste Event", None
    # focus events
    FOCUS_EVENT = 20_200_600, "Focus Event", None
    FOCUS_IN_EVENT = 20_200_601, "Focus In Event", None
    FOCUS_OUT_EVENT = 20_200_602, "Focus Out Event", None
    # command
    # COMMAND,  MODE, TOOL, SHORTCUT/KEYBINDING, ...
    # GESTURE, ...
    # CLIPBOARD, ...
    # CAMERA, SPEAKER, MICROPHONE, ...
    # AUDIO, AUDIO_PLAYER, VIDEO, VIDEO_PLAYER, ...

    # animation [20_300_000]
    TRANSITION_STYLE = 20_300_000, "Transition Style", None
    EFFECT_STYLE = 20_300_100, "Effect Style", None
    # ANIMATION, ANIMATION_TRACK, ANIMATION_KEYFRAME, ...
    # KEYFRAME_VARIANT, ...
    # RIG, ...

    #
    # CANVAS
    #

    # audio [30_000_000]
    # SOUND_SOURCE, ...

    # image [30_100_000]
    # ...

    # video [30_200_000]
    # STREAM, ...
    # ENCODING, ...

    # model [30_300_000]
    # ...

    # paint [30_400_000]
    # RASTER/BITMAP, ...
    # DAB, PAINT, BRUSH, ...
    # SPRITE, SPRITE_SHEET, NINESLICE_SPRITE, TILING_SPRITE, ...
    # TEXTURE, ...

    # style [30_500_000]
    THEME = 30_500_000, "Theme", None
    PALETTE = 30_500_100, "Palette", None
    STYLE = 30_500_200, "Style", None
    COLOR_STYLE = 30_500_300, "Color Style", None
    FILL_STYLE = 30_500_400, "Fill Style", None
    FONT_STYLE = 30_500_500, "Font Style", None
    BORDER_STYLE = 30_500_600, "Border Style", None
    SHADOW_STYLE = 30_500_700, "Shadow Style", None
    GRADIENT_STYLE = 30_500_800, "Gradient Style", None
    STROKE_STYLE = 30_500_900, "Stroke Style", None
    # BRUSH_STYLE, ...

    #
    # STAGE
    #

    # scene [40_000_000]
    STAGE = 40_000_000, "Stage", None
    SCENE = 40_000_100, "Scene", "Scene of an Application"
    SCENE_EVENT = 40_000_101, "Scene Event", None
    LAYER = 40_000_200, "Layer", "Layer of a Scene"
    # BREAKPOINT_VARIANT, ...
    # VIEWPORT, OVERLAY, WIDGET, HUD, ...
    # ROOM, ...
    # FORM, MENU, INVENTORY, ...

    # view [40_100_000]
    # container views
    VIEW = 40_100_000, "View", "View in a Scene"
    VIEW_EVENT = 40_100_001, "View Event", None
    LAYOUT_VIEW = 40_100_100, "Container View", None
    FRAME_VIEW = 40_100_200, "Frame View", "Fixed Container"
    LABEL_VIEW = 40_100_300, "Label View", "Label Container"
    SPLIT_VIEW = 40_100_400, "Split View", "Split Container"
    # SLOT_DEFINITION_VIEW, SLOT_VIEW, ...
    # FORM_VIEW, MENU_VIEW, ...
    # TAB_VIEW, ...
    # DRAWER_VIEW, SPLIT_DRAWER_VIEW, GRID/GRID_ELEMENT_VIEW, ...
    # POPOVER, SHEET, ALERT, HUD, ...
    # content views
    CONTENT_VIEW = 40_100_500, "Content View", None
    TEXT_VIEW = 40_100_501, "Text View", "Text"
    # CODE_VIEW, ICON_VIEW, IMAGE_VIEW, AUDIO_VIEW, VIDEO_VIEW, DOCUMENT_VIEW, ...
    # input views
    INPUT_VIEW = 40_100_600, "Input View", None
    NUMBER_INPUT_VIEW = 40_100_601, "Number Input View", "Number Input"
    SLIDER_INPUT_VIEW = 40_100_602, "Slider Input View", "Slider Input"
    # STRING_INPUT_VIEW, TOGGLE_INPUT_VIEW, PICKER_INPUT_VIEW, COLOR_INPUT_VIEW, ...
    # ICON_INPUT_VIEW, FILE_INPUT_VIEW, DATETIME_INPUT_VIEW, DURATION_INPUT_VIEW, ...

    # rendering [40_200_000]
    # ...

    # shaders [40_300_000]
    # ...

    # lighting [40_400_000]
    # LIGHT, LIGHT2D, ...
    # POINT_LIGHT, DIRECTIONAL_LIGHT, SPOT_LIGHT, AMBIENT_LIGHT, ...
    # OCCLUDER, ...
    # ...

    #
    # DEPLOYMENT
    #

    # cloud [50_000_000]
    MACHINE = 50_000_000, "Machine", "Machine for ephemeral computing"
    # DATABASE, SEARCH, VAULT, CACHE, S3, ...
    # HOST, ENDPOINT, NETWORK, AUTOSCALER, ...

    # observability [50_100_000]
    # metric
    METRIC = 50_100_000, "Metric", None
    GAUGE_METRIC = 50_100_100, "Gauge Metric", None
    GAUGE_MEASUREMENT_EVENT = 50_100_101, "Gauge Measurement", None
    COUNTER_METRIC = 50_100_200, "Counter Metric", None
    COUNTER_MEASUREMENT_EVENT = 50_100_201, "Counter Measurement", None
    HISTOGRAM_METRIC = 50_100_300, "Histogram Metric", None
    HISTOGRAM_MEASUREMENT_EVENT = 50_100_301, "Histogram Measurement", None
    # INCIDENT, ESCALATION, ...

    # experience [50_200_000]
    # VISIT/SESSION, RECORDING/REPLAY, ...
    # SURVEY, ...
    # ONBOARDING, TOUR, FUNNEL, COHORT, JOURNEY, ..
    # FEATURE, FEATURE_FLAG, FEATURE_GATE, ...
    # SEGMENT, EXPERIMENT, ...
    # SETTINGS, ...

    #
    # DISTRIBUTION
    #

    # localization [60_000_000]
    # LOCALIZATION, STRING, TRANSLATION, ...
    # LOCALIZATION_VARIANT, GEO_VARIANT, ...

    # legal [60_100_000]
    # ...

    # social [60_200_000]
    REACTION = 60_200_000, "Reaction", None
    REACTION_EVENT = 60_200_001, "Reaction Event", None
    REACTION_ADDED_EVENT = 60_200_002, "Reaction Added Event", None
    REACTION_REMOVED_EVENT = 60_200_003, "Reaction Removed Event", None
    STAR = 60_200_100, "Star", None
    STAR_EVENT = 60_200_101, "Star Event", None
    STAR_ADDED_EVENT = 60_200_102, "Star Added Event", None
    STAR_REMOVED_EVENT = 60_200_103, "Star Removed Event", None
    FOLLOW = 60_200_200, "Follow", None
    FOLLOW_EVENT = 60_200_201, "Follow Event", None
    FOLLOW_ADDED_EVENT = 60_200_202, "Follow Added Event", None
    FOLLOW_REMOVED_EVENT = 60_200_203, "Follow Removed Event", None
    NOTIFICATION = 60_200_300, "Notification", None
    NOTIFICATION_EVENT = 60_200_301, "Notification Event", None
    NOTIFICATION_SENT_EVENT = 60_200_302, "Notification Sent Event", None
    NOTIFICATION_RESCINDED_EVENT = 60_200_303, "Notification Rescinded Event", None
    NOTIFICATION_READ_EVENT = 60_200_304, "Notification Read Event", None
    NOTIFICATION_DISMISSED_EVENT = 60_200_305, "Notification Dismissed Event", None
    NOTIFICATION_EXPIRED_EVENT = 60_200_306, "Notification Expired Event", None
    # FEED, FEED_ITEM, ...
    # THREAD, MESSAGE, ...
    # POLL, VOTE, REVIEW, RATING, RANK, ...
    # ACHIEVEMENT, BADGE, WISHLIST/WATCHLIST, ...

    # finance [60_300_000]
    # WALLET, BALANCE, BUDGET, TRANSFER, CREDIT, ...
    # ...

    # commerce [60_400_000]
    # TIER, SUBSCRIPTION, PRODUCT, PRICE, ...
    # ORDER, INVOICE, DISCOUNT, DISPUTE, REFUND, ...


@declare_enum(EnumType.STRUCT_TYPE)
class StructType(EnumDeclaration):
    #
    # CORE
    #

    # builtin [1]
    # root
    STRUCT = 1, "Struct", "Root of all Structs"
    MESSAGE = 2, "Message", "Message"

    OBJECT_DEFINITION = 10
    OBJECT_DEFINITION_REFERENCE = 11
    NODE_DEFINITION = 12
    STRUCT_DEFINITION = 15
    HANDLE_DEFINITION = 17
    ENUM_DEFINITION = 19
    PROPERTY_DEFINITION = 20
    CONSTANT_DEFINITION = 21
    OPTION_DEFINITION = 22
    TAG_DEFINITION = 23
    # ALIAS_DEFINITION, UNION_DEFINITION, ...

    # common [100_000]
    # ...

    # type/value
    VALUE = 100_000
    TYPE = 100_001
    NUMBER_CONSTRAINT = 100_010
    STRING_CONSTRAINT = 100_011
    COLLECTION_CONSTRAINT = 100_012

    # expressions
    EXPRESSION = 100_020
    JOIN = 100_022
    AGGREGATION = 100_023
    CONDITION = 100_024
    SORT = 100_025
    SELECT = 100_026

    # query
    QUERY = 100_030

    # references
    NODE_REFERENCE = 100_040
    PROPERTY_REFERENCE = 100_041

    # universe [200_000]
    UNIVERSE_SIGNUP_REQUEST = 200_000
    UNIVERSE_SIGNUP_RESPONSE = 200_001
    UNIVERSE_SPAWN_REQUEST = 200_002
    UNIVERSE_SPAWN_RESPONSE = 200_003
    # ...

    # space [300_000]
    # ...

    # runtime [400_000]
    # ...

    # generate [500_000]
    # ...

    #
    # BASICS
    #

    # entity [10_000_000]
    INDEX_DEFINITION = 10_000_100
    CONSTRAINT_DEFINITION = 10_000_200
    # EXPECTATION_DEFINITION = 10_000_300
    MIGRATION_DEFINITION = 10_000_300
    MIGRATION_OPERATION_DEFINITION = 10_000_301
    TEXT = 10_000_010, None, None
    TEXT_SPAN = 10_000_011, None, None
    ICON = 10_000_012
    # custom
    CUSTOM_STRUCT = 10_000_500, "Custom Struct", "Custom Struct Instance"
    CUSTOM_MESSAGE = 10_000_600, "Custom Message", "Custom Message Instance"
    # ...

    # logic [10_100_000]
    FUNCTION_DEFINITION = 10_100_400
    METHOD_DEFINITION = 10_100_500
    ACTION_DEFINITION = 10_100_600
    SCHEDULE = 10_100_810
    # ...

    # intelligence [10_200_000]
    # ...

    # access [10_300_000]
    PERMISSION_DEFINITION = 10_300_000

    # quality [10_400_000]
    # ...

    #
    # SIMULATION
    #

    # geometry [20_000_000]
    VECTOR2 = 20_000_010, None, None
    VECTOR2I = 20_000_011, None, None
    VECTOR3 = 20_000_012, None, None
    VECTOR3I = 20_000_013, None, None
    VECTOR4 = 20_000_014, None, None
    VECTOR4I = 20_000_015, None, None
    QUATERNION = 20_000_020, None, None
    LENGTH = 20_000_030, "Length", None
    OFFSET2 = 20_000_031, "Position", None
    GRID2 = 20_000_032, "Grid", None
    GRID_SPAN2 = 20_000_033, "Grid Span", None
    INSET2 = 20_000_034, "Insets", None
    CORNER2 = 20_000_035, "Corners", None
    AXIS2 = 20_000_036, "Axis2", None
    AXIS3 = 20_000_037, "Axis3", None
    FORM2D = 20_000_200, "Form2D", None
    LINE2D = 20_000_201, "Line", None
    ARROW2D = 20_000_202, "Arrow", None
    RECTANGLE2D = 20_000_203, "Rectangle", None
    ELLIPSE2D = 20_000_204, "Ellipse", None
    CAPSULE2D = 20_000_205, "Capsule", None
    STAR2D = 20_000_206, "Star", None
    POLYGON2D = 20_000_207, "Polygon", None
    PATH2D = 20_000_208, "Path", None
    FORM3D = 20_000_300, "Form3D", None

    # physics [20_100_000]
    # ...

    # perception [20_200_000]
    # ...

    # animation [20_300_000]
    TRANSITION = 20_300_000, "Transition", None
    EFFECT = 20_300_100, "Effect", None

    #
    # CANVAS
    #

    # audio [30_000_000]
    # ...

    # image [30_100_000]
    # ...

    # video [30_200_000]
    # ...

    # model [30_300_000]
    # ...

    # paint [30_400_000]
    # ...

    # style [30_500_000]
    COLOR = 30_500_000, "Color", None
    FILL = 30_500_100, "Fill", None
    FONT = 30_500_200, "Font", None
    BORDER = 30_500_300, "Border", None
    SHADOW = 30_500_400, "Shadow", None
    GRADIENT = 30_500_500, "Gradient", None
    GRADIENT_STOP = 30_500_501, "Gradient Stop", None
    STROKE = 30_500_600, "Stroke", None
    STROKE_CAP = 30_500_601, "Stroke Cap", None
    STROKE_PATH = 30_500_602, "Stroke Path", None
    STROKE_POINT = 30_500_603, "Stroke Point", None

    #
    # STAGE
    #

    # scene [40_000_000]
    # ...

    # view [40_100_000]
    # ...

    # rendering [40_200_000]
    # ...

    # shaders [40_300_000]
    # ...

    # lighting [40_400_000]
    # ...

    #
    # DEPLOYMENT
    #

    # cloud [50_000_000]
    # ...

    # observability [50_100_000]
    # ...

    # experience [50_200_000]
    # ...

    #
    # DISTRIBUTION
    #

    # localization [60_000_000]
    # ...

    # legal [60_100_000]
    # ...

    # social [60_200_000]
    # ...

    # finance [60_300_000]
    # ...

    # commerce [60_400_000]
    # ...


@declare_enum(EnumType.HANDLE_TYPE)
class HandleType(EnumDeclaration):
    HANDLE = 1

    SESSION = 10
    GRAPH = 11
    CONNECTION = 12
    STREAM = 13

    CONTEXT = 20
    LOGGER = 21
    TRACER = 22

    HASHER = 30
    ENCODER = 31
    BINARY_WRITER = 32
    BINARY_READER = 33


@declare_enum(EnumType.UNIVERSE_DOMAIN)
class UniverseDomain(EnumDeclaration):
    """The Destack Universe is organized into domains."""

    CORE = 1, "Core", "Universe intrinsics"
    BASICS = 10_000_000, "Basics", "Scaffolding the Universe"
    SIMULATION = 20_000_000, "Simulation", "Modeling the Universe"
    CANVAS = 30_000_000, "Crafting", "Imagining the Universe"
    STAGE = 40_000_000, "Stage", "Presenting the Universe"
    DEPLOYMENT = 50_000_000, "Deployment", "Operating the Universe"
    DISTRIBUTION = 60_000_000, "Distribution", "Distributing the Universe"


@declare_enum(EnumType.UNIVERSE_CATEGORY)
class UniverseCategory(EnumDeclaration):
    """How the Destack Universe is organized (domains > categories)."""

    # core
    BUILTIN = 1, "Core", "Primitives and intrinsics"
    COMMON = 100_000, "Common", "Common and shared"
    UNIVERSE = 200_000, "Universe", "Global computational universe"
    SPACE = 300_000, "Space", "Spacetime organization"
    RUNTIME = 400_000, "Runtime", "Runtime and execution"
    GENERATE = 500_000, "Generate", "SDK generation"
    # CLI, ENCODER, GRAPH, ...

    # basics
    ENTITY = 10_000_000, "Entity", "Entity management"
    LOGIC = 10_100_000, "Logic", "Scripting and behavior"
    INTELLIGENCE = 10_200_000, "Intelligence", "Artificial intelligence"  # AI
    ACCESS = 10_300_000, "Access", "Access and identity"
    QUALITY = 10_400_000, "Quality", "Quality assurance"
    STUDIO = 10_500_000, "Studio", "Editing the Universe"
    # STREAMING, INTERNET, ...

    # simulation
    GEOMETRY = 20_000_000, "Geometry", "Geometric representations"
    PHYSICS = 20_100_000, "Physics", "Physics simulation"
    PERCEPTION = 20_200_000, "Perception", "Sensing and interaction"
    ANIMATION = 20_300_000, "Animation", "Motion and time choreography"
    # CHARACTER/HUMAN?, ...
    # GEOGRAPHY/GEOLOCATION/MAPPING?, ...
    # MATERIAL?, MECHANICAL, ELECTRICAL, THERMODYNAMICS, INTERSTELLAR, ...
    # GEOLOGY, BIOLOGY, CHEMISTRY, ECOLOGICAL, ...

    # canvas
    AUDIO = 30_000_000, "Audio", "Audio and sound production"
    IMAGE = 30_100_000, "Image", "Image and photo production"
    VIDEO = 30_200_000, "Video", "Video production"
    MODEL = 30_300_000, "Model", "Modeling and sculpting"
    PAINT = 30_400_000, "Paint", "Drawing and painting"
    STYLE = 30_500_000, "Style", "Appearance and theming"
    # MATERIAL?, NARRATIVE, ...

    # stage
    SCENE = 40_000_000, "Scene", "Stage building"
    VIEW = 40_100_000, "View", "View building"
    RENDERING = 40_200_000, "Rendering", "Rendering and shading"
    SHADERS = 40_300_000, "Shaders", "Shader programming"
    LIGHTING = 40_400_000, "Lighting", "Lighting and shadows"
    # CAMERA/VIEWPORT, XR, PARTICLE, ...

    # deployment
    CLOUD = 50_000_000, "Cloud", "Cloud computing infrastructure"
    OBSERVABILITY = 50_100_000, "Observability", "Telemetry on everything"
    EXPERIENCE = 50_200_000, "Experience", "User experience"
    # PRINTING, ACTUATION, ROBOTICS, ...
    # TRANSPORTATION, ENERGY, DEFENSE, ...
    # CONSUMER/HOME, PHARMACEUTICAL, ...

    # distribution
    LOCALIZATION = 60_000_000, "Localization", "Localization and internationalization"
    LEGAL = 60_100_000, "Legal", "Legal, compliance and policy"
    SOCIAL = 60_200_000, "Social", "Interactions, reputation and trust"
    FINANCE = 60_300_000, "Finance", "Accounting and finance"
    COMMERCE = 60_400_000, "Commerce", "Billing and monetization"
    # ACCESSIBILITY, GOVERNANCE, ...
    # CONTENT, COST, CRYPTO, ...
