from typing import TYPE_CHECKING

from .enum import OptionEnum, declare_enum, declare_option

if TYPE_CHECKING:
    pass


#
# Enums
#


class EnumType(OptionEnum):
    #
    # CORE
    #

    # builtin [1]
    OBJECT_KIND = declare_option(1)
    ENUM_TYPE = declare_option(2)
    NODE_TYPE = declare_option(3)
    STRUCT_TYPE = declare_option(4)
    TRAIT_TYPE = declare_option(5)
    HANDLE_TYPE = declare_option(6)
    OBJECT_STABILITY = declare_option(8)
    UNIVERSE_DOMAIN = declare_option(9)
    UNIVERSE_CATEGORY = declare_option(10)
    PROPERTY_REFERENCE_TYPE = declare_option(13)
    MATERIALIZATION = declare_option(14)
    RUNTIME_PLATFORM = declare_option(30)
    RUNTIME_LANGUAGE = declare_option(31)
    RUNTIME_TYPE = declare_option(32)
    EVENT_STATUS = declare_option(50)

    # common [100_000]
    # type/value
    PRIMITIVE_TYPE = declare_option(100_000)
    TYPE_CARDINALITY = declare_option(100_001)
    SCALAR_TYPE = declare_option(100_002)
    VALUE_FACTORY = declare_option(100_003)
    PROPERTY_ZONE = declare_option(100_006)
    EDGE_TYPE = declare_option(100_007)
    EDGE_DIRECTION = declare_option(100_008)
    CASCADE_ACTION = declare_option(100_009)
    ENCODING = declare_option(100_010)

    # edit
    EDIT_TYPE = declare_option(200)
    EDIT_OPERATION = declare_option(201)

    # query
    CONDITIONAL_TYPE = declare_option(300)
    AGGREGATION_TYPE = declare_option(301)
    SORT_MODE = declare_option(302)
    SORT_TYPE = declare_option(303)
    JOIN_TYPE = declare_option(304)
    EXPRESSION_TYPE = declare_option(306)
    QUERY_TYPE = declare_option(320)

    # universe [200_000]
    CLIENT_TYPE = declare_option(200_200)

    # space [300_000]
    FOLDER_TYPE = declare_option(300_500)
    BRANCH_TYPE = declare_option(300_300)
    SNAPSHOT_TYPE = declare_option(300_400)
    SNAPSHOT_STATUS = declare_option(300_401)

    # runtime [400_000]
    # ...

    # generate [500_000]
    # ...

    #
    # BASICS
    #

    # entity [10_000_000]
    FILE_TYPE = declare_option(10_000_001)
    TEXT_SPAN_TYPE = declare_option(10_000_002)
    ICON_TYPE = declare_option(10_000_003)
    INDEX_TYPE = declare_option(10_000_100)
    CONSTRAINT_TYPE = declare_option(10_000_200)
    MIGRATION_TYPE = declare_option(10_000_300)

    # logic [10_100_000]
    FUNCTION_OPERATOR = declare_option(10_100_400)
    METHOD_TYPE = declare_option(10_100_500)
    ACTION_TYPE = declare_option(10_100_600)
    TRIGGER_TYPE = declare_option(10_100_700)
    TIMER_TYPE = declare_option(10_100_800)
    DAY_OF_WEEK = declare_option(10_100_801)
    MONTH = declare_option(10_100_802)
    SCHEDULE_FREQUENCY = declare_option(10_100_803)
    ENVIRONMENT_TYPE = declare_option(10_100_000)
    RUN_STATUS = declare_option(10_101_000)
    LOG_LEVEL = declare_option(10_101_100)

    # intelligence [10_200_000]

    # access [10_300_000]
    ROLE_TYPE = declare_option(10_300_300)
    SANCTION_TYPE = declare_option(10_300_400)
    ENTITLEMENT_TYPE = declare_option(10_300_500)

    # quality [10_400_000]
    # ...

    #
    # SIMULATION
    #

    # geometry [20_000_000]
    ANCHOR = declare_option(20_000_000)
    LENGTH_TYPE = declare_option(20_000_001)
    LAYOUT = declare_option(20_000_002)
    DISTRIBUTE = declare_option(20_000_003)
    ALIGN = declare_option(20_000_004)
    DIRECTION = declare_option(20_000_005)
    OVERFLOW = declare_option(20_000_006)
    ARROW_HEAD_TYPE = declare_option(20_000_200)

    # physics [20_100_000]
    # ...

    # perception [20_200_000]
    MOUSE_BUTTON = declare_option(20_200_000)

    # animation [20_300_000]
    TRANSITION_TYPE = declare_option(20_300_000)
    SPRING_TYPE = declare_option(20_300_001)
    EFFECT_TYPE = declare_option(20_300_100)
    REPEAT_TYPE = declare_option(20_300_002)
    EASING = declare_option(20_300_003)

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
    # ...

    # style [40_200_000]
    COLOR_TYPE = declare_option(40_200_000)
    COLOR_SHADE = declare_option(40_200_001)
    COLOR_HUE = declare_option(40_200_002)
    COLOR_INTENT = declare_option(40_200_003)
    FILL_TYPE = declare_option(40_200_400)
    FILL_POSITION = declare_option(40_200_401)
    FILL_SIZE = declare_option(40_200_402)
    FONT_TYPE = declare_option(40_200_500)
    FONT_WEIGHT = declare_option(40_200_501)
    FONT_SIZE = declare_option(40_200_502)
    TEXT_ALIGN = declare_option(40_200_503)
    TEXT_DECORATION = declare_option(40_200_504)
    TEXT_TRANSFORM = declare_option(40_200_505)
    BORDER_TYPE = declare_option(40_200_600)
    SHADOW_TYPE = declare_option(40_200_700)
    SHADOW_POSITION = declare_option(40_200_701)
    GRADIENT_TYPE = declare_option(40_200_800)
    STROKE_TYPE = declare_option(40_200_900)
    TEXT_SPLIT_TYPE = declare_option(40_200_901)
    OFFSCREEN_BEHAVIOR = declare_option(40_200_902)

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
    REGION = declare_option(50_000_001)
    REGION_AREA = declare_option(50_000_002)
    REGION_CONTINENT = declare_option(50_000_003)
    TENANCY = declare_option(50_000_004)
    MACHINE_TYPE = declare_option(50_000_000)

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
    NOTIFICATION_STATUS = declare_option(60_200_300)

    # finance [60_300_000]
    # ...

    # commerce [60_400_000]
    # ...

    #
    # STUDIO
    #

    # editor [100_000_000]
    # ...


EnumType = declare_enum(EnumType.ENUM_TYPE)(EnumType)


@declare_enum(EnumType.OBJECT_KIND)
class ObjectKind(OptionEnum):
    NODE = declare_option(
        1,
        "Node",
        description="Object with data, logic and universally addressable identity",
    )
    STRUCT = declare_option(
        2,
        "Struct",
        description="Object with data and logic (embedded elsewhere)",
    )
    HANDLE = declare_option(
        3,
        "Handle",
        description="Object with special data and logic (runtime only)",
    )


@declare_enum(EnumType.OBJECT_STABILITY)
class ObjectStability(OptionEnum):
    DYNAMIC = declare_option(
        1,
        "Dynamic",
        description="Definition may change in every compatible way",
    )
    # GROWABLE = 2, "Definition may change with new properties at the end (only)"
    # NOTE :Performance: ObjectStability.GROWABLE is annoying to implement but probably worth it
    STATIC = declare_option(
        7,
        "Static",
        description="Definition may never change",
    )


@declare_enum(EnumType.TRAIT_TYPE)
class TraitType(OptionEnum):
    #
    # CORE
    #

    # builtin [1]
    # LOCAL?
    # storage
    # RELATIONAL/OLTP, INDEXED; ANALYTIC, ...?
    ORDERED = declare_option(1, "Ordered", description="Is ordered")
    # PAUSABLE?
    RESOURCE = declare_option(2, "Resource", description="Is a Resource")
    VARIANT = declare_option(3, "Variant", description="Is a Variant")

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
    RUNNABLE = declare_option(10_100_000, "Runnable", description="Can be run")

    # intelligence [10_200_000]
    # ...

    # access [10_300_000]
    OWNABLE = declare_option(10_300_000, "Ownable", description="Is ownable")
    OWNED = declare_option(10_300_001, "Owned", description="Is owned")
    JOINABLE = declare_option(10_300_002, "Joinable", description="Is joinable")
    ACTOR = declare_option(10_300_003, "Actor", description="Is an Actor")

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
    INTERACTIVE = declare_option(20_200_000, "Interactive", description="Can be interacted with")
    DRAGGABLE = declare_option(20_200_001, "Draggable", description="Can be dragged")
    SELECTABLE = declare_option(20_200_002, "Selectable", description="Can be selected")
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
    STARABLE = declare_option(60_200_100, "Starable", description="Can be starred")
    REACTABLE = declare_option(60_200_000, "Reactable", description="Can be reacted to")
    FOLLOWABLE = declare_option(60_200_200, "Followable", description="Can be followed")
    # RATEABLE, VOTABLE, ...
    # ASSIGNABLE, MESSAGEABLE, CLOSABLE, LOCKABLE, ...

    # finance [60_300_000]
    # ...

    # commerce [60_400_000]
    # ...


@declare_enum(EnumType.NODE_TYPE)
class NodeType(OptionEnum):
    #
    # CORE
    #

    # builtin [1]
    # root
    NODE = declare_option(1, "Node", description="Root of all Nodes")
    ENTITY = declare_option(2, "Entity", description="Versioned, stateful Node")
    EVENT = declare_option(3, "Event", description="Immutable datum of something happening")
    EDIT_EVENT = declare_option(10, "Edit Event")
    # CHANGE_EVENT?

    # common [100_000]
    # ...

    # universe [200_000]
    UNIVERSE = declare_option(200_000, "Universe", description="The Destack computational universe")
    # GALAXY, ...
    # HANDLE?, ...
    # user
    USER = declare_option(200_100, "User")
    # FRIENDSHIP, FRIENDSHIP_INVITE, ...
    CLIENT = declare_option(200_200, "Client")
    # CREDENTIAL, ACCOUNT, PROFILE, ...
    # organization
    ORGANIZATION = declare_option(200_300, "Organization")
    TEAM = declare_option(200_400, "Team", description="Team in an Organization")

    # space [300_000]
    SPACE = declare_option(300_000, "Space", description="Universal Space")
    TAG = declare_option(300_100, "Tag")
    TAGGING = declare_option(300_200, "Tagging")
    # TRAIT?
    # FRAGMENT (multiple disjoint trees)
    # SLOT (inside tree)
    # LINK/PORTAL (to another subtree)
    # TIMELINE, TRACK, (KEY)FRAME, ...
    BRANCH = declare_option(300_300, "Branch")
    SNAPSHOT = declare_option(300_400, "Snapshot", description="Point in Space-time")
    FOLDER = declare_option(300_500, "Folder", description="Sub-space of a Space")
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
    FILE = declare_option(10_000_000, "File")
    # DIRECTORY, SYNC, ...
    # INDEX, CONSTRAINT, MIGRATION, ...
    # REMOTE, FOREIGN_DATA/ENTITY_WRAPPER, ...
    # SECRET, ...
    INDEX = declare_option(10_000_100, "Index", description="Index of an Entity")
    CONSTRAINT = declare_option(10_000_200, "Constraint", description="Constraint of an Entity")
    # EXPECTATION, ...
    MIGRATION = declare_option(10_000_300, "Migration", description="Migration of an Entity")
    MIGRATION_OPERATION = declare_option(
        10_000_301, "Migration Operation", description="Migration Operation of an Entity"
    )
    # custom
    CUSTOM_EVENT_DEFINITION = declare_option(
        10_000_400, "Custom Event", description="Custom Event Definition"
    )
    CUSTOM_STRUCT_DEFINITION = declare_option(
        10_000_500,
        "Custom Struct",
        description="Custom Struct Definition",
    )
    CUSTOM_MESSAGE_DEFINITION = declare_option(
        10_000_600,
        "Custom Message",
        description="Custom Message Definition",
    )
    CUSTOM_PROPERTY_DEFINITION = declare_option(
        10_000_700,
        "Custom Property",
        description="Custom Property Definition",
    )
    CUSTOM_ENUM_DEFINITION = declare_option(
        10_000_800, "Custom Enum", description="Custom Enum Definition"
    )
    CUSTOM_OPTION_DEFINITION = declare_option(
        10_000_900,
        "Custom Option",
        description="Custom Option Definition",
    )
    # CUSTOM_ALIAS_DEFINITION, CUSTOM_UNION_DEFINITION, ...

    # logic [10_100_000]
    ENVIRONMENT = declare_option(10_100_000, "Environment")
    MODE = declare_option(10_100_100, "Mode")
    SCRIPT = declare_option(10_100_200, "Script")
    CUSTOM_EVENT = declare_option(10_100_300, "Signal", description="Custom Event instance")
    MEASUREMENT_EVENT = declare_option(10_100_301, "Measurement of a Metric")
    FUNCTION = declare_option(10_100_400, "Function")
    METHOD = declare_option(10_100_500, "Method")
    ACTION = declare_option(10_100_600, "Action")
    TRIGGER = declare_option(10_100_700, "Trigger")
    TRIGGER_EVENT = declare_option(10_100_701, "Trigger Event")
    TIMER = declare_option(10_100_800, "Timer")
    TIMER_EVENT = declare_option(10_100_801, "Timer Event")
    TIMER_STARTED_EVENT = declare_option(10_100_802, "Timer Started Event")
    TIMER_PAUSED_EVENT = declare_option(10_100_803, "Timer Paused Event")
    TIMER_RESUMED_EVENT = declare_option(10_100_804, "Timer Resumed Event")
    TIMER_COMPLETED_EVENT = declare_option(10_100_805, "Timer Completed Event")
    TIMER_CANCELLED_EVENT = declare_option(10_100_806, "Timer Cancelled Event")
    ROUTE = declare_option(10_100_900, "Route")
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
    RUN = declare_option(10_101_000, "Run")
    RUN_EVENT = declare_option(10_101_001, "Run Event")
    RUN_STARTED_EVENT = declare_option(10_101_002, "Run Started Event")
    RUN_PAUSE_REQUESTED_EVENT = declare_option(10_101_003, "Run Pause Requested Event")
    RUN_PAUSED_EVENT = declare_option(10_101_004, "Run Paused Event")
    RUN_RESUME_REQUESTED_EVENT = declare_option(10_101_005, "Run Resume Requested Event")
    RUN_RESUMED_EVENT = declare_option(10_101_006, "Run Resumed Event")
    RUN_STOP_REQUESTED_EVENT = declare_option(10_101_007, "Run Stop Requested Event")
    RUN_FAILED_EVENT = declare_option(10_101_008, "Run Failed Event")
    RUN_COMPLETED_EVENT = declare_option(10_101_009, "Run Completed Event")
    SPAN_EVENT = declare_option(10_101_010, "Span")
    LOG_EVENT = declare_option(10_101_011, "Log")

    # intelligence [10_200_000]
    # MODEL, FINETUNE, ...
    # PROMPT, INFERENCE/COMPLETION/..., ...
    # RECOMMENDATION, ...

    # access [10_300_000]
    PERMISSION = declare_option(10_300_000, "Permission", description="Permission for something")
    MEMBERSHIP = declare_option(10_300_100, "Membership", description="Membership to something")
    MEMBERSHIP_EVENT = declare_option(10_300_101, "Membership Event")
    MEMBERSHIP_JOINED_EVENT = declare_option(10_300_102, "Membership Join Event")
    MEMBERSHIP_LEFT_EVENT = declare_option(10_300_103, "Membership Leave Event")
    INVITE = declare_option(10_300_200, "Invite", description="Invite to a Space/Folder")
    INVITE_EVENT = declare_option(10_300_201, "Invite Event")
    INVITE_SENT_EVENT = declare_option(10_300_202, "Invite Sent Event")
    INVITE_RESCINDED_EVENT = declare_option(10_300_203, "Invite Rescinded Event")
    INVITE_ACCEPTED_EVENT = declare_option(10_300_204, "Invite Accepted Event")
    INVITE_REJECTED_EVENT = declare_option(10_300_205, "Invite Rejected Event")
    ROLE = declare_option(10_300_300, "Role", description="Role in something")
    ROLE_EVENT = declare_option(10_300_301, "Role Event")
    ROLE_ASSIGNED_EVENT = declare_option(10_300_302, "Role Assigned Event")
    ROLE_UNASSIGNED_EVENT = declare_option(10_300_303, "Role Unassigned Event")
    SANCTION = declare_option(
        10_300_400, "Sanction", description="Temporary or permanent restriction"
    )
    SANCTION_EVENT = declare_option(10_300_401, "Sanction Event")
    SANCTION_REQUESTED_EVENT = declare_option(10_300_402, "Sanction Requested Event")
    SANCTION_GRANTED_EVENT = declare_option(10_300_403, "Sanction Granted Event")
    SANCTION_REVOKED_EVENT = declare_option(10_300_404, "Sanction Revoked Event")
    SANCTION_EXPIRED_EVENT = declare_option(10_300_405, "Sanction Expired Event")
    ENTITLEMENT = declare_option(
        10_300_500, "Entitlement", description="Temporary or permanent grant"
    )
    ENTITLEMENT_EVENT = declare_option(10_300_501, "Entitlement Event")
    ENTITLEMENT_REQUESTED_EVENT = declare_option(
        10_300_502,
        "Entitlement Requested Event",
    )
    ENTITLEMENT_GRANTED_EVENT = declare_option(10_300_503, "Entitlement Granted Event")
    ENTITLEMENT_REVOKED_EVENT = declare_option(10_300_504, "Entitlement Revoked Event")
    ENTITLEMENT_EXPIRED_EVENT = declare_option(10_300_505, "Entitlement Expired Event")
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
    ENTITY2D = declare_option(20_000_000, "Entity2D", description="2D Entity")
    ENTITY3D = declare_option(20_000_100, "Entity3D", description="3D Entity")
    SHAPE2D = declare_option(20_000_200, "Shape2D")
    LINE_SHAPE2D = declare_option(20_000_201, "Line Shape2D")
    ARROW_SHAPE2D = declare_option(20_000_202, "Arrow Shape2D")
    RECTANGLE_SHAPE2D = declare_option(20_000_203, "Rectangle Shape2D")
    ELLIPSE_SHAPE2D = declare_option(20_000_204, "Ellipse Shape2D")
    CAPSULE_SHAPE2D = declare_option(20_000_205, "Capsule Shape2D")
    STAR_SHAPE2D = declare_option(20_000_206, "Star Shape2D")
    POLYGON_SHAPE2D = declare_option(20_000_207, "Polygon Shape2D")
    PATH_SHAPE2D = declare_option(20_000_208, "Path Shape2D")
    SHAPE3D = declare_option(20_000_300, "Shape3D")

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
    INPUT_EVENT = declare_option(20_200_000, "Input Event")
    # pointer events
    POINTER_EVENT = declare_option(20_200_100, "Pointer Event")
    POINTER_DOWN_EVENT = declare_option(20_200_101, "Pointer Down Event")
    POINTER_UP_EVENT = declare_option(20_200_102, "Pointer Up Event")
    POINTER_MOVE_EVENT = declare_option(20_200_103, "Pointer Move Event")
    POINTER_ENTER_EVENT = declare_option(20_200_104, "Pointer Enter Event")
    POINTER_OVER_EVENT = declare_option(20_200_105, "Pointer Over Event")
    POINTER_LEAVE_EVENT = declare_option(20_200_106, "Pointer Leave Event")
    POINTER_LONG_PRESS_EVENT = declare_option(20_200_107, "Long Press Event")
    # mouse events
    MOUSE_EVENT = declare_option(20_200_200, "Mouse Event")
    CLICK_EVENT = declare_option(20_200_201, "Click Event")
    SINGLE_CLICK_EVENT = declare_option(20_200_202, "Single Click Event")
    DOUBLE_CLICK_EVENT = declare_option(20_200_203, "Double Click Event")
    TRIPLE_CLICK_EVENT = declare_option(20_200_204, "Triple Click Event")
    WHEEL_EVENT = declare_option(20_200_210, "Wheel Event")
    # key events
    KEY_EVENT = declare_option(20_200_300, "Key Event")
    KEY_DOWN_EVENT = declare_option(20_200_301, "Key Down Event")
    KEY_UP_EVENT = declare_option(20_200_302, "Key Up Event")
    KEY_PRESS_EVENT = declare_option(20_200_303, "Key Press Event")
    # drag events
    DRAG_EVENT = declare_option(20_200_400, "Drag Event")
    DRAG_START_EVENT = declare_option(20_200_401, "Drag Start Event")
    DRAG_END_EVENT = declare_option(20_200_402, "Drag End Event")
    DRAG_OVER_EVENT = declare_option(20_200_403, "Drag Over Event")
    DRAG_ENTER_EVENT = declare_option(20_200_404, "Drag Enter Event")
    DRAG_LEAVE_EVENT = declare_option(20_200_405, "Drag Leave Event")
    DROP_EVENT = declare_option(20_200_406, "Drop Event")
    # clipboard events
    CLIPBOARD_EVENT = declare_option(20_200_500, "Clipboard Event")
    COPY_EVENT = declare_option(20_200_501, "Copy Event")
    CUT_EVENT = declare_option(20_200_502, "Cut Event")
    PASTE_EVENT = declare_option(20_200_503, "Paste Event")
    # focus events
    FOCUS_EVENT = declare_option(20_200_600, "Focus Event")
    FOCUS_IN_EVENT = declare_option(20_200_601, "Focus In Event")
    FOCUS_OUT_EVENT = declare_option(20_200_602, "Focus Out Event")
    # command
    # COMMAND,  MODE, TOOL, SHORTCUT/KEYBINDING, ...
    # GESTURE, ...
    # CLIPBOARD, ...
    # CAMERA, SPEAKER, MICROPHONE, ...
    # AUDIO, AUDIO_PLAYER, VIDEO, VIDEO_PLAYER, ...

    # animation [20_300_000]
    TRANSITION_STYLE = declare_option(20_300_000, "Transition Style")
    EFFECT_STYLE = declare_option(20_300_100, "Effect Style")
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
    THEME = declare_option(30_500_000, "Theme")
    PALETTE = declare_option(30_500_100, "Palette")
    STYLE = declare_option(30_500_200, "Style")
    COLOR_STYLE = declare_option(30_500_300, "Color Style")
    FILL_STYLE = declare_option(30_500_400, "Fill Style")
    FONT_STYLE = declare_option(30_500_500, "Font Style")
    BORDER_STYLE = declare_option(30_500_600, "Border Style")
    SHADOW_STYLE = declare_option(30_500_700, "Shadow Style")
    GRADIENT_STYLE = declare_option(30_500_800, "Gradient Style")
    STROKE_STYLE = declare_option(30_500_900, "Stroke Style")
    # BRUSH_STYLE, ...

    #
    # STAGE
    #

    # scene [40_000_000]
    STAGE = declare_option(40_000_000, "Stage")
    SCENE = declare_option(40_000_100, "Scene", description="Scene of an Application")
    SCENE_EVENT = declare_option(40_000_101, "Scene Event")
    LAYER = declare_option(40_000_200, "Layer", description="Layer of a Scene")
    # BREAKPOINT_VARIANT, ...
    # VIEWPORT, OVERLAY, WIDGET, HUD, ...
    # ROOM, ...
    # FORM, MENU, INVENTORY, ...

    # view [40_100_000]
    # container views
    VIEW = declare_option(40_100_000, "View", description="View in a Scene")
    VIEW_EVENT = declare_option(40_100_001, "View Event")
    LAYOUT_VIEW = declare_option(40_100_100, "Container View")
    FRAME_VIEW = declare_option(40_100_200, "Frame View", description="Fixed Container")
    LABEL_VIEW = declare_option(40_100_300, "Label View", description="Label Container")
    SPLIT_VIEW = declare_option(40_100_400, "Split View", description="Split Container")
    # SLOT_DEFINITION_VIEW, SLOT_VIEW, ...
    # FORM_VIEW, MENU_VIEW, ...
    # TAB_VIEW, ...
    # DRAWER_VIEW, SPLIT_DRAWER_VIEW, GRID/GRID_ELEMENT_VIEW, ...
    # POPOVER, SHEET, ALERT, HUD, ...
    # content views
    CONTENT_VIEW = declare_option(40_100_500, "Content View")
    TEXT_VIEW = declare_option(40_100_501, "Text View", description="Text")
    # CODE_VIEW, ICON_VIEW, IMAGE_VIEW, AUDIO_VIEW, VIDEO_VIEW, DOCUMENT_VIEW, ...
    # input views
    INPUT_VIEW = declare_option(40_100_600, "Input View")
    NUMBER_INPUT_VIEW = declare_option(40_100_601, "Number Input View", description="Number Input")
    SLIDER_INPUT_VIEW = declare_option(40_100_602, "Slider Input View", description="Slider Input")
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
    MACHINE = declare_option(50_000_000, "Machine", description="Machine for ephemeral computing")
    # DATABASE, SEARCH, VAULT, CACHE, S3, ...
    # HOST, ENDPOINT, NETWORK, AUTOSCALER, ...

    # observability [50_100_000]
    # metric
    METRIC = declare_option(50_100_000, "Metric")
    GAUGE_METRIC = declare_option(50_100_100, "Gauge Metric")
    GAUGE_MEASUREMENT_EVENT = declare_option(50_100_101, "Gauge Measurement")
    COUNTER_METRIC = declare_option(50_100_200, "Counter Metric")
    COUNTER_MEASUREMENT_EVENT = declare_option(50_100_201, "Counter Measurement")
    HISTOGRAM_METRIC = declare_option(50_100_300, "Histogram Metric")
    HISTOGRAM_MEASUREMENT_EVENT = declare_option(50_100_301, "Histogram Measurement")
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
    REACTION = declare_option(60_200_000, "Reaction")
    REACTION_EVENT = declare_option(60_200_001, "Reaction Event")
    REACTION_ADDED_EVENT = declare_option(60_200_002, "Reaction Added Event")
    REACTION_REMOVED_EVENT = declare_option(60_200_003, "Reaction Removed Event")
    STAR = declare_option(60_200_100, "Star")
    STAR_EVENT = declare_option(60_200_101, "Star Event")
    STAR_ADDED_EVENT = declare_option(60_200_102, "Star Added Event")
    STAR_REMOVED_EVENT = declare_option(60_200_103, "Star Removed Event")
    FOLLOW = declare_option(60_200_200, "Follow")
    FOLLOW_EVENT = declare_option(60_200_201, "Follow Event")
    FOLLOW_ADDED_EVENT = declare_option(60_200_202, "Follow Added Event")
    FOLLOW_REMOVED_EVENT = declare_option(60_200_203, "Follow Removed Event")
    NOTIFICATION = declare_option(60_200_300, "Notification")
    NOTIFICATION_EVENT = declare_option(60_200_301, "Notification Event")
    NOTIFICATION_SENT_EVENT = declare_option(60_200_302, "Notification Sent Event")
    NOTIFICATION_RESCINDED_EVENT = declare_option(60_200_303, "Notification Rescinded Event")
    NOTIFICATION_READ_EVENT = declare_option(60_200_304, "Notification Read Event")
    NOTIFICATION_DISMISSED_EVENT = declare_option(60_200_305, "Notification Dismissed Event")
    NOTIFICATION_EXPIRED_EVENT = declare_option(60_200_306, "Notification Expired Event")
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
class StructType(OptionEnum):
    #
    # CORE
    #

    # builtin [1]
    # root
    STRUCT = declare_option(1, "Struct", description="Root of all Structs")
    MESSAGE = declare_option(2, "Message", description="Message")

    OBJECT_DEFINITION = declare_option(10)
    OBJECT_DEFINITION_REFERENCE = declare_option(11)
    NODE_DEFINITION = declare_option(12)
    STRUCT_DEFINITION = declare_option(15)
    HANDLE_DEFINITION = declare_option(17)
    ENUM_DEFINITION = declare_option(19)
    PROPERTY_DEFINITION = declare_option(20)
    CONSTANT_DEFINITION = declare_option(21)
    OPTION_DEFINITION = declare_option(22)
    TAG_DEFINITION = declare_option(23)
    # ALIAS_DEFINITION, UNION_DEFINITION, ...

    # common [100_000]
    # ...

    # type/value
    VALUE = declare_option(100_000)
    TYPE = declare_option(100_001)
    NUMBER_CONSTRAINT = declare_option(100_010)
    STRING_CONSTRAINT = declare_option(100_011)
    COLLECTION_CONSTRAINT = declare_option(100_012)

    # expressions
    EXPRESSION = declare_option(100_020)
    JOIN = declare_option(100_022)
    AGGREGATION = declare_option(100_023)
    CONDITION = declare_option(100_024)
    SORT = declare_option(100_025)
    SELECT = declare_option(100_026)

    # query
    QUERY = declare_option(100_030)

    # references
    NODE_REFERENCE = declare_option(100_040)
    PROPERTY_REFERENCE = declare_option(100_041)

    # universe [200_000]
    UNIVERSE_SIGNUP_REQUEST = declare_option(200_000)
    UNIVERSE_SIGNUP_RESPONSE = declare_option(200_001)
    UNIVERSE_SPAWN_REQUEST = declare_option(200_002)
    UNIVERSE_SPAWN_RESPONSE = declare_option(200_003)
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
    INDEX_DEFINITION = declare_option(10_000_100)
    CONSTRAINT_DEFINITION = declare_option(10_000_200)
    # EXPECTATION_DEFINITION = 10_000_300
    MIGRATION_DEFINITION = declare_option(10_000_300)
    MIGRATION_OPERATION_DEFINITION = declare_option(10_000_301)
    TEXT = declare_option(10_000_010)
    TEXT_SPAN = declare_option(10_000_011)
    ICON = declare_option(10_000_012)
    # custom
    CUSTOM_STRUCT = declare_option(
        10_000_500, "Custom Struct", description="Custom Struct Instance"
    )
    CUSTOM_MESSAGE = declare_option(
        10_000_600, "Custom Message", description="Custom Message Instance"
    )
    # ...

    # logic [10_100_000]
    FUNCTION_DEFINITION = declare_option(10_100_400)
    METHOD_DEFINITION = declare_option(10_100_500)
    ACTION_DEFINITION = declare_option(10_100_600)
    SCHEDULE = declare_option(10_100_810)
    # ...

    # intelligence [10_200_000]
    # ...

    # access [10_300_000]
    PERMISSION_DEFINITION = declare_option(10_300_000)

    # quality [10_400_000]
    # ...

    #
    # SIMULATION
    #

    # geometry [20_000_000]
    VECTOR2 = declare_option(20_000_010)
    VECTOR2I = declare_option(20_000_011)
    VECTOR3 = declare_option(20_000_012)
    VECTOR3I = declare_option(20_000_013)
    VECTOR4 = declare_option(20_000_014)
    VECTOR4I = declare_option(20_000_015)
    QUATERNION = declare_option(20_000_020)
    LENGTH = declare_option(20_000_030, "Length")
    OFFSET2 = declare_option(20_000_031, "Position")
    GRID2 = declare_option(20_000_032, "Grid")
    GRID_SPAN2 = declare_option(20_000_033, "Grid Span")
    INSET2 = declare_option(20_000_034, "Insets")
    CORNER2 = declare_option(20_000_035, "Corners")
    AXIS2 = declare_option(20_000_036, "Axis2")
    AXIS3 = declare_option(20_000_037, "Axis3")
    FORM2D = declare_option(20_000_200, "Form2D")
    LINE2D = declare_option(20_000_201, "Line")
    ARROW2D = declare_option(20_000_202, "Arrow")
    RECTANGLE2D = declare_option(20_000_203, "Rectangle")
    ELLIPSE2D = declare_option(20_000_204, "Ellipse")
    CAPSULE2D = declare_option(20_000_205, "Capsule")
    STAR2D = declare_option(20_000_206, "Star")
    POLYGON2D = declare_option(20_000_207, "Polygon")
    PATH2D = declare_option(20_000_208, "Path")
    FORM3D = declare_option(20_000_300, "Form3D")

    # physics [20_100_000]
    # ...

    # perception [20_200_000]
    # ...

    # animation [20_300_000]
    TRANSITION = declare_option(20_300_000, "Transition")
    EFFECT = declare_option(20_300_100, "Effect")

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
    COLOR = declare_option(30_500_000, "Color")
    FILL = declare_option(30_500_100, "Fill")
    FONT = declare_option(30_500_200, "Font")
    BORDER = declare_option(30_500_300, "Border")
    SHADOW = declare_option(30_500_400, "Shadow")
    GRADIENT = declare_option(30_500_500, "Gradient")
    GRADIENT_STOP = declare_option(30_500_501, "Gradient Stop")
    STROKE = declare_option(30_500_600, "Stroke")
    STROKE_CAP = declare_option(30_500_601, "Stroke Cap")
    STROKE_PATH = declare_option(30_500_602, "Stroke Path")
    STROKE_POINT = declare_option(30_500_603, "Stroke Point")

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
class HandleType(OptionEnum):
    HANDLE = declare_option(1)

    SESSION = declare_option(10)
    GRAPH = declare_option(11)
    CONNECTION = declare_option(12)
    STREAM = declare_option(13)

    CONTEXT = declare_option(20)
    LOGGER = declare_option(21)
    TRACER = declare_option(22)

    HASHER = declare_option(30)
    ENCODER = declare_option(31)
    BINARY_WRITER = declare_option(32)
    BINARY_READER = declare_option(33)


@declare_enum(EnumType.UNIVERSE_DOMAIN)
class UniverseDomain(OptionEnum):
    """The Destack Universe is organized into domains."""

    CORE = declare_option(1, "Core", description="Universe intrinsics")
    BASICS = declare_option(10_000_000, "Basics", description="Scaffolding the Universe")
    SIMULATION = declare_option(20_000_000, "Simulation", description="Modeling the Universe")
    CANVAS = declare_option(30_000_000, "Crafting", description="Imagining the Universe")
    STAGE = declare_option(40_000_000, "Stage", description="Presenting the Universe")
    DEPLOYMENT = declare_option(50_000_000, "Deployment", description="Operating the Universe")
    DISTRIBUTION = declare_option(
        60_000_000, "Distribution", description="Distributing the Universe"
    )


@declare_enum(EnumType.UNIVERSE_CATEGORY)
class UniverseCategory(OptionEnum):
    """How the Destack Universe is organized (domains > categories)."""

    # core
    BUILTIN = declare_option(1, "Core", description="Primitives and intrinsics")
    COMMON = declare_option(100_000, "Common", description="Common and shared")
    UNIVERSE = declare_option(200_000, "Universe", description="Global computational universe")
    SPACE = declare_option(300_000, "Space", description="Spacetime organization")
    RUNTIME = declare_option(400_000, "Runtime", description="Runtime and execution")
    GENERATE = declare_option(500_000, "Generate", description="SDK generation")
    # CLI, ENCODER, GRAPH, ...

    # basics
    ENTITY = declare_option(10_000_000, "Entity", description="Entity management")
    LOGIC = declare_option(10_100_000, "Logic", description="Scripting and behavior")
    INTELLIGENCE = declare_option(
        10_200_000, "Intelligence", description="Artificial intelligence"
    )  # AI
    ACCESS = declare_option(10_300_000, "Access", description="Access and identity")
    QUALITY = declare_option(10_400_000, "Quality", description="Quality assurance")
    STUDIO = declare_option(10_500_000, "Studio", description="Editing the Universe")
    # STREAMING, INTERNET, ...

    # simulation
    GEOMETRY = declare_option(20_000_000, "Geometry", description="Geometric representations")
    PHYSICS = declare_option(20_100_000, "Physics", description="Physics simulation")
    PERCEPTION = declare_option(20_200_000, "Perception", description="Sensing and interaction")
    ANIMATION = declare_option(20_300_000, "Animation", description="Motion and time choreography")
    # CHARACTER/HUMAN?, ...
    # GEOGRAPHY/GEOLOCATION/MAPPING?, ...
    # MATERIAL?, MECHANICAL, ELECTRICAL, THERMODYNAMICS, INTERSTELLAR, ...
    # GEOLOGY, BIOLOGY, CHEMISTRY, ECOLOGICAL, ...

    # canvas
    AUDIO = declare_option(30_000_000, "Audio", description="Audio and sound production")
    IMAGE = declare_option(30_100_000, "Image", description="Image and photo production")
    VIDEO = declare_option(30_200_000, "Video", description="Video production")
    MODEL = declare_option(30_300_000, "Model", description="Modeling and sculpting")
    PAINT = declare_option(30_400_000, "Paint", description="Drawing and painting")
    STYLE = declare_option(30_500_000, "Style", description="Appearance and theming")
    # MATERIAL?, NARRATIVE, ...

    # stage
    SCENE = declare_option(40_000_000, "Scene", description="Stage building")
    VIEW = declare_option(40_100_000, "View", description="View building")
    RENDERING = declare_option(40_200_000, "Rendering", description="Rendering and shading")
    SHADERS = declare_option(40_300_000, "Shaders", description="Shader programming")
    LIGHTING = declare_option(40_400_000, "Lighting", description="Lighting and shadows")
    # CAMERA/VIEWPORT, XR, PARTICLE, ...

    # deployment
    CLOUD = declare_option(50_000_000, "Cloud", description="Cloud computing infrastructure")
    OBSERVABILITY = declare_option(
        50_100_000, "Observability", description="Telemetry on everything"
    )
    EXPERIENCE = declare_option(50_200_000, "Experience", description="User experience")
    # PRINTING, ACTUATION, ROBOTICS, ...
    # TRANSPORTATION, ENERGY, DEFENSE, ...
    # CONSUMER/HOME, PHARMACEUTICAL, ...

    # distribution
    LOCALIZATION = declare_option(
        60_000_000, "Localization", description="Localization and internationalization"
    )
    LEGAL = declare_option(60_100_000, "Legal", description="Legal, compliance and policy")
    SOCIAL = declare_option(60_200_000, "Social", description="Interactions, reputation and trust")
    FINANCE = declare_option(60_300_000, "Finance", description="Accounting and finance")
    COMMERCE = declare_option(60_400_000, "Commerce", description="Billing and monetization")
    # ACCESSIBILITY, GOVERNANCE, ...
    # CONTENT, COST, CRYPTO, ...
