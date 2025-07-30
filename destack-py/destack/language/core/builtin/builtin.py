from typing import TYPE_CHECKING

from .enum import Enum, builtin_enum

if TYPE_CHECKING:
    pass


#
# Enums
#


class EnumType(Enum):
    # core [1-100_000]
    ENUM_TYPE = 1
    NODE_TYPE = 2
    STRUCT_TYPE = 3
    TRAIT_TYPE = 4
    OBJECT_KIND = 7
    OBJECT_STABILITY = 8
    UNIVERSE_CATEGORY = 9
    NODE_DEFINITION_TYPE = 10
    OBJECT_DEFINITION_TYPE = 11
    STRUCT_DEFINITION_TYPE = 12
    PROPERTY_REFERENCE_TYPE = 13
    MATERIALIZATION = 14
    GRAPH_KEY = 20
    PLATFORM_TYPE = 30
    RUNTIME_LANGUAGE = 31
    OPERATING_SYSTEM = 40
    EVENT_STATUS = 50

    # type/value
    PRIMITIVE_TYPE = 100
    TYPE_CARDINALITY = 101
    SCALAR_TYPE = 102
    VALUE_FACTORY = 103
    PROPERTY_ZONE = 106
    EDGE_TYPE = 107
    EDGE_DIRECTION = 108
    CASCADE_ACTION = 109
    ENCODING = 110

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

    # time
    BRANCH_TYPE = 2_000
    SNAPSHOT_TYPE = 2_100
    SNAPSHOT_STATUS = 2_101

    # space
    # ...

    # base
    # ...

    # custom
    # ...

    # integrity
    INDEX_TYPE = 30_100
    CONSTRAINT_TYPE = 30_200
    MIGRATION_TYPE = 31_000

    # logic
    FUNCTION_OPERATOR = 40_002
    METHOD_TYPE = 40_100
    ACTION_TYPE = 40_200

    # universe [100_000-200_000]
    CLIENT_TYPE = 121_300

    # space [200_000-300_000]
    FOLDER_TYPE = 200_000

    # access [300_000-400_000]
    ROLE_TYPE = 300_200
    SANCTION_TYPE = 300_400
    ENTITLEMENT_TYPE = 300_500

    # data [400_000-500_000]
    FILE_RETENTION_MODE = 400_000
    FILE_TYPE = 400_002
    FILE_FORMAT = 400_003
    TEXT_SPAN_TYPE = 400_004
    ICON_TYPE = 400_005

    # media [500_000-600_000]
    # ...

    # localization [600_000-700_000]
    # ...

    # logic [700_000-800_000]
    TRIGGER_TYPE = 705_000
    TIMER_TYPE = 705_100
    DAY_OF_WEEK = 705_101
    MONTH = 705_102
    SCHEDULE_FREQUENCY = 705_103

    # quality [800_000-900_000]
    # ...

    # intelligence [900_000-1_000_000]
    MODEL_DEVELOPER = 920_000
    MODEL_PROVIDER = 920_001

    # infrastructure [1_000_000-1_100_000]
    CLOUD = 1_000_000
    REGION = 1_000_001
    REGION_AREA = 1_000_002
    REGION_CONTINENT = 1_000_003
    TENANCY = 1_000_004
    MACHINE_TYPE = 1_001_000

    # deployment [1_100_000-1_200_000]
    ENVIRONMENT_TYPE = 1_100_000
    # runtime
    RUN_STATUS = 1_110_000
    LOG_LEVEL = 1_110_300

    # observability [1_200_000-1_300_000]
    # ...

    # experience [1_300_000-1_400_000]
    # ...

    # social [1_400_000-1_500_000]
    NOTIFICATION_STATUS = 1_400_500

    # finance [1_500_000-1_600_000]
    # ...

    # scene [1_700_000-1_800_000]
    LAYER_TYPE = 1_700_300

    # view [1_800_000-1_900_000]
    # ...

    # paint [1_900_000-2_000_000]
    # ...

    # interaction [2_000_000-2_100_000]
    MOUSE_BUTTON = 2_000_010

    # style [2_100_000-2_200_000]
    COLOR_TYPE = 2_100_000
    COLOR_SHADE = 2_100_001
    COLOR_HUE = 2_100_002
    COLOR_INTENT = 2_100_003
    FILL_TYPE = 2_100_100
    FILL_POSITION = 2_100_101
    FILL_SIZE = 2_100_102
    FONT_TYPE = 2_100_200
    FONT_WEIGHT = 2_100_201
    FONT_SIZE = 2_100_202
    TEXT_ALIGN = 2_100_203
    TEXT_DECORATION = 2_100_204
    TEXT_TRANSFORM = 2_100_205
    BORDER_TYPE = 2_100_206
    SHADOW_TYPE = 2_100_207
    SHADOW_POSITION = 2_100_208
    GRADIENT_TYPE = 2_100_209
    STROKE_TYPE = 2_100_213
    TEXT_SPLIT_TYPE = 2_100_223
    OFFSCREEN_BEHAVIOR = 2_100_224

    # animation [2_200_000-2_300_000]
    TRANSITION_TYPE = 2_200_000
    SPRING_TYPE = 2_200_001
    EFFECT_TYPE = 2_200_002
    REPEAT_TYPE = 2_200_003
    EASING = 2_100_004

    # audio [2_300_000-2_400_000]
    # ...

    # geometry [2_400_000-2_500_000]
    ANCHOR = 2_400_000
    LENGTH_TYPE = 2_400_001
    LAYOUT = 2_400_002
    DISTRIBUTE = 2_400_003
    ALIGN = 2_400_004
    DIRECTION = 2_400_005
    OVERFLOW = 2_400_006
    ARROW_HEAD_TYPE = 2_401_200
    # physics [2_500_000-2_600_000]
    # ...

    # lighting [2_600_000-2_700_000]
    # ...


builtin_enum(EnumType.ENUM_TYPE)(EnumType)


@builtin_enum(EnumType.OBJECT_KIND)
class ObjectKind(Enum):
    NODE = 1
    STRUCT = 2


@builtin_enum(EnumType.OBJECT_STABILITY)
class ObjectStability(Enum):
    DYNAMIC = 1, "Definition may change in every compatible way"
    # GROWABLE = 2, "Definition may change with new properties at the end (only)"
    STATIC = 7, "Definition may never change"


@builtin_enum(EnumType.STRUCT_TYPE)
class StructType(Enum):
    # core [1-100_000]
    # root
    STRUCT = 1, "Struct", "Root of all Structs", "fas fa-shapes"
    MESSAGE = 2, "Message", "Message", "fas fa-envelope"

    OBJECT_DEFINITION = 10
    OBJECT_DEFINITION_REFERENCE = 11
    NODE_DEFINITION = 12
    NODE_DEFINITION_REFERENCE = 13
    STRUCT_DEFINITION = 15
    STRUCT_DEFINITION_REFERENCE = 16
    ENUM_DEFINITION = 17
    PROPERTY_DEFINITION = 18
    CONSTANT_DEFINITION = 19
    OPTION_DEFINITION = 20
    TAG_DEFINITION = 21
    # ALIAS_DEFINITION, UNION_DEFINITION, ...

    # type/value
    VALUE = 100
    TYPE = 101
    CHECKED_TYPE = 102
    NUMBER_CONSTRAINT = 110
    STRING_CONSTRAINT = 111
    COLLECTION_CONSTRAINT = 112

    # expressions
    EXPRESSION = 200
    JOIN = 202
    AGGREGATION = 203
    CONDITION = 204
    SORT = 205
    SELECT = 206

    # query
    QUERY = 300

    # references
    NODE_REFERENCE = 400
    PROPERTY_REFERENCE = 41

    # universe
    UNIVERSE_SIGNUP_REQUEST = 1_000
    UNIVERSE_SIGNUP_RESPONSE = 1_001
    UNIVERSE_SPAWN_REQUEST = 1_002
    UNIVERSE_SPAWN_RESPONSE = 1_003

    # custom
    CUSTOM_STRUCT = 20_200, "Custom Struct", "Custom Struct Instance", "fas fa-shapes"
    CUSTOM_MESSAGE = 20_300, "Custom Message", "Custom Message Instance", "fas fa-envelope"

    # integrity
    INDEX_DEFINITION = 30_100
    CONSTRAINT_DEFINITION = 30_200
    # EXPECTATION_DEFINITION = 30_300
    MIGRATION_DEFINITION = 31_000
    MIGRATION_OPERATION_DEFINITION = 31_100

    # logic
    FUNCTION_DEFINITION = 40_000
    METHOD_DEFINITION = 40_100
    ACTION_DEFINITION = 40_200

    # access
    PERMISSION_DEFINITION = 50_000

    # universe [100_000-200_000]
    # ...

    # space [200_000-300_000]
    # ...

    # access [300_000-400_000]
    # ...

    # data [400_000-500_000]
    TEXT = 400_020, None, None, "fas fa-text"
    TEXT_SPAN = 400_021, None, None, "fas fa-text"
    ICON = 400_031
    # ...

    # media [500_000-600_000]
    # ...

    # localization [600_000-700_000]
    # ...

    # logic [700_000-800_000]
    SCHEDULE = 700_001
    # ...

    # quality [800_000-900_000]
    # ...

    # intelligence [900_000-1_000_000]
    # ...

    # infrastructure [1_000_000-1_100_000]
    # ...

    # deployment [1_100_000-1_200_000]
    # ...

    # observability [1_200_000-1_300_000]
    # ...

    # experience [1_300_000-1_400_000]
    # ...

    # social [1_400_000-1_500_000]
    # ...

    # finance [1_500_000-1_600_000]
    # ...

    # scene [1_700_000-1_800_000]
    # ...

    # view [1_800_000-1_900_000]
    # ...

    # paint [1_900_000-2_000_000]
    # ...

    # interaction [2_000_000-2_100_000]
    # ...

    # style [2_100_000-2_200_000]
    COLOR = 2_100_300, "Color", None, "fas fa-palette"
    FILL = 2_100_400, "Fill", None, "fas fa-fill"
    FONT = 2_100_500, "Font", None, "fas fa-text"
    BORDER = 2_100_600, "Border", None, "fas fa-border-outer"
    SHADOW = 2_100_700, "Shadow", None, "fas fa-eclipse"
    GRADIENT = 2_100_800, "Gradient", None, "fas fa-gradient"
    GRADIENT_STOP = 2_100_801, "Gradient Stop", None, "fas fa-gradient"
    STROKE = 2_101_100, "Stroke", None, "fas fa-stroke"
    STROKE_CAP = 2_101_101, "Stroke Cap", None, "fas fa-stroke"
    STROKE_PATH = 2_101_102, "Stroke Path", None, "fas fa-stroke"
    STROKE_POINT = 2_101_103, "Stroke Point", None, "fas fa-stroke"

    # animation [2_200_000-2_300_000]
    TRANSITION = 2_200_000, "Transition", None, "fas fa-bezier-curve"
    EFFECT = 2_200_100, "Effect", None, "fas fa-sparkle"

    # audio [2_300_000-2_400_000]
    # ...

    # geometry [2_400_000-2_500_000]
    VECTOR2 = 2_400_000, None, None, "fas fa-vector-square"
    VECTOR2I = 2_400_001, None, None, "fas fa-vector-square"
    VECTOR3 = 2_400_002, None, None, "fas fa-vector-square"
    VECTOR3I = 2_400_003, None, None, "fas fa-vector-square"
    VECTOR4 = 2_400_004, None, None, "fas fa-vector-square"
    VECTOR4I = 2_400_005, None, None, "fas fa-vector-square"
    QUATERNION = 2_400_010, None, None, "fas fa-vector-square"
    LENGTH = 2_400_020, "Length", None, "fas fa-ruler"
    OFFSET2 = 2_400_021, "Position", None, "fas fa-location-crosshair"
    GRID2 = 2_400_022, "Grid", None, "fas fa-grid-2"
    GRID_SPAN2 = 2_400_023, "Grid Span", None, "fas fa-grid-2"
    INSET2 = 2_400_024, "Insets", None, "fas fa-corner"
    CORNER2 = 2_400_025, "Corners", None, "fas fa-corner"
    AXIS2 = 2_400_026, "Axis2", None, "fas fa-vector-square"
    AXIS3 = 2_400_027, "Axis3", None, "fas fa-vector-square"
    LINE2D = 2_410_100, "Line", None, "fas fa-line"
    ARROW2D = 2_410_200, "Arrow", None, "fas fa-arrow-right"
    RECTANGLE2D = 2_410_300, "Rectangle", None, "fas fa-rectangle"
    ELLIPSE2D = 2_410_400, "Ellipse", None, "fas fa-ellipse"
    CAPSULE2D = 2_410_500, "Capsule", None, "fas fa-capsule"
    STAR2D = 2_410_600, "Star", None, "fas fa-star"
    POLYGON2D = 2_411_000, "Polygon", None, "fas fa-polygon"
    PATH2D = 2_411_100, "Path", None, "fas fa-path"

    # physics [2_500_000-2_600_000]
    # ...

    # lighting [2_600_000-2_700_000]
    # ...

    # editor [3_000_000-3_100_000]
    # ...


@builtin_enum(EnumType.TRAIT_TYPE)
class TraitType(Enum):
    # core [1-100_000]
    # LOCAL?
    # storage
    # RELATIONAL/OLTP, INDEXED; ANALYTIC, ...?

    # common
    ORDERED = 10_000, "Ordered", "Is ordered", "fas fa-sort"
    # PAUSABLE?

    # universe [100_000-200_000]
    # ...

    # space [200_000-300_000]
    # ...

    # access [300_000-400_000]
    OWNABLE = 300_000, "Ownable", "Is ownable", "fas fa-user"
    OWNED = 300_001, "Owned", "Is owned", "fas fa-user"
    JOINABLE = 300_003, "Joinable", "Is joinable", "fas fa-users"
    ACTOR = 300_004, "Actor", "Is an Actor", "fas fa-user"

    # data [400_000-500_000]
    # ...

    # media [500_000-600_000]
    # ...

    # localization [600_000-700_000]
    # ...

    # logic [700_000-800_000]
    RUNNABLE = 700_001, "Runnable", "Can be run", "fas fa-play"

    # quality [800_000-900_000]
    # ...

    # intelligence [900_000-1_000_000]
    # ...

    # infrastructure [1_000_000-1_100_000]
    # ...

    # deployment [1_100_000-1_200_000]
    # ...

    # observability [1_200_000-1_300_000]
    # ...

    # experience [1_300_000-1_400_000]
    # ...

    # social [1_400_000-1_500_000]
    STARABLE = 1_400_030, "Starable", "Can be starred", "fas fa-star"
    REACTABLE = 1_400_032, "Reactable", "Can be reacted to", "fas fa-heart"
    FOLLOWABLE = 1_400_034, "Followable", "Can be followed", "fas fa-plus"
    # RATEABLE, VOTABLE, ...
    # ASSIGNABLE, MESSAGEABLE, CLOSABLE, LOCKABLE, ...

    # finance [1_500_000-1_600_000]
    # ...

    # scene [1_700_000-1_800_000]
    # ...

    # view [1_800_000-1_900_000]
    # ANIMATABLE/TWEENABLE, ...

    # paint [1_900_000-2_000_000]

    # interaction [2_000_000-2_100_000]
    INTERACTIVE = 2_000_000, "Interactive", "Can be interacted with", "fas fa-mouse-pointer"
    DRAGGABLE = 2_000_001, "Draggable", "Can be dragged", "fas fa-mouse-pointer"
    SELECTABLE = 2_000_002, "Selectable", "Can be selected", "fas fa-mouse-pointer"
    # ...

    # style [2_100_000-2_200_000]
    # ...

    # animation [2_200_000-2_300_000]
    # ...

    # audio [2_300_000-2_400_000]
    # ...

    # geometry [2_400_000-2_500_000]
    # ...

    # physics [2_500_000-2_600_000]
    # ...

    # lighting [2_600_000-2_700_000]
    # ...

    # editor [3_000_000-3_100_000]
    # ...


@builtin_enum(EnumType.NODE_TYPE)
class NodeType(Enum):
    # core [1-100_000]

    # root
    NODE = 1, "Node", "Root of all Nodes", "fas fa-dot"
    ENTITY = 2, "Entity", "Versioned, stateful Node", "fas fa-dot"
    EVENT = 3, "Event", "Immutable datum of something happening", "fas fa-dot"

    # space
    UNIVERSE = 1_000, "Universe", "The Destack computational universe", "fas fa-dot"
    SPACE = 1_100, "Space", "Universal Space", "fas fa-galaxy"
    # nocheckin: CONTEXT (as local partial instances attached to some Nodes?)
    #  (stacked local Context with mode/time/logging/tracing/baggage/custom stuff, tree down?,
    #   merge Oracle/Session.actor_ptr/.../epoch into Context?)

    # time
    BRANCH = 2_000, "Branch", None, "fas fa-code-branch"
    SNAPSHOT = 2_100, "Snapshot", "Point in Space-time", "fas fa-save"

    # base
    RESOURCE = 10_100, "Resource", "External asset outside of Destack", "fas fa-dot"
    METRIC = 10_200, "Metric", None, "fas fa-gauge"
    SERVICE = 10_300, "Service", None, "fas fa-screwdriver-wrench"
    VARIANT = 10_400, "Variant", "Variant of a Scene", "fas fa-shapes"
    ENTITY2D = 11_000, "Entity2D", "2D Entity", "fas fa-shapes"
    ENTITY3D = 11_100, "Entity3D", "3D Entity", "fas fa-shapes"
    TAG = 12_000, "Tag", None, "fas fa-tag"
    TAGGING = 12_100, "Tagging", None, "fas fa-tag"
    # TRAIT?
    # SLOT, LINK, ...
    # TIMELINE, TRACK, (KEY)FRAME, ...

    # custom
    CUSTOM_EVENT_DEFINITION = 20_000, "Custom Event", "Custom Event Definition", "fas fa-signal"
    CUSTOM_STRUCT_DEFINITION = 20_100, "Custom Struct", "Custom Struct Definition", "fas fa-shapes"
    CUSTOM_MESSAGE_DEFINITION = (
        20_200,
        "Custom Message",
        "Custom Message Definition",
        "fas fa-envelope",
    )
    CUSTOM_PROPERTY_DEFINITION = (
        20_300,
        "Custom Property",
        "Custom Property Definition",
        "fas fa-triangle",
    )
    CUSTOM_ENUM_DEFINITION = 20_400, "Custom Enum", "Custom Enum Definition", "fas fa-shapes"
    CUSTOM_OPTION_DEFINITION = 20_500, "Custom Option", "Custom Option Definition", "fas fa-circle"
    # CUSTOM_ALIAS, CUSTOM_UNION, ...

    # integrity
    INDEX = 30_100, "Index", "Index of an Entity", "fas fa-database"
    CONSTRAINT = 30_200, "Constraint", "Constraint of an Entity", "fas fa-database"
    # EXPECTATION, ...
    MIGRATION = 31_000, "Migration", "Migration of an Entity", "fas fa-database"
    MIGRATION_OPERATION = (
        31_100,
        "Migration Operation",
        "Migration Operation of an Entity",
        "fas fa-database",
    )

    # nocheckin: *_definition/custom_*_definition inconsistency (Index, Event, Method/Action, Permission, ...)

    # logic
    FUNCTION = 40_000, "Function", None, "fas fa-code"
    METHOD = 40_100, "Method", None, "fas fa-code"
    ACTION = 40_200, "Action", None, "fas fa-code"

    # access
    PERMISSION = 50_000, "Permission", "Permission for something", "fas fa-user-shield"

    # event
    CUSTOM_EVENT = 90_000, "Signal", "Custom Event instance", "fas fa-signal"
    EDIT_EVENT = 90_100, "Edit Event", None, "fas fa-file-lines"
    # CHANGE_EVENT?
    MEASUREMENT_EVENT = 90_200, "Measurement of a Metric", None, "fas fa-gauge"

    # universe [100_000-200_000]
    # UNIVERSE, ...
    HANDLE = 100_200, "Handle", "Unique @handle", "fas fa-at"
    # user
    USER = 121_000, "User", None, "fas fa-user"
    # FRIENDSHIP, FRIENDSHIP_INVITE, ...
    CLIENT = 121_300, "Client", None, "fas fa-desktop"
    # CREDENTIAL, ACCOUNT, PROFILE, ...
    # organization
    ORGANIZATION = 122_000, "Organization", None, "fas fa-building"
    TEAM = 122_100, "Team", "Team in an Organization", "fas fa-users"

    # space [200_000-300_000]
    # folder
    FOLDER = 240_000, "Folder", "Sub-space of a Space", "fas fa-folder-open"
    # APPLICATION (extends Folder?), ...
    # DEPENDENCY, ...
    # GROUP, ...
    # spacetime
    # VERSION, ...
    # HISTORY, REPLAY, ...
    # FORK, ...
    # LINK, PORTAL, ...

    # access [300_000-400_000]
    MEMBERSHIP = 360_000, "Membership", "Membership to something", "fas fa-user-group"
    MEMBERSHIP_EVENT = 360_001, "Membership Event", None, "fas fa-user-group"
    MEMBERSHIP_JOINED_EVENT = 360_002, "Membership Join Event", None, "fas fa-user-group"
    MEMBERSHIP_LEFT_EVENT = 360_003, "Membership Leave Event", None, "fas fa-user-group"
    INVITE = 360_100, "Invite", "Invite to a Space/Folder", "fas fa-user-plus"
    INVITE_EVENT = 360_101, "Invite Event", None, "fas fa-user-plus"
    INVITE_SENT_EVENT = 360_102, "Invite Sent Event", None, "fas fa-user-plus"
    INVITE_RESCINDED_EVENT = 360_103, "Invite Rescinded Event", None, "fas fa-user-plus"
    INVITE_ACCEPTED_EVENT = 360_104, "Invite Accepted Event", None, "fas fa-user-plus"
    INVITE_REJECTED_EVENT = 360_105, "Invite Rejected Event", None, "fas fa-user-plus"
    ROLE = 360_200, "Role", "Role in something", "fas fa-user-tag"
    ROLE_EVENT = 360_201, "Role Event", None, "fas fa-user-tag"
    ROLE_ASSIGNED_EVENT = 360_202, "Role Assigned Event", None, "fas fa-user-tag"
    ROLE_UNASSIGNED_EVENT = 360_203, "Role Unassigned Event", None, "fas fa-user-tag"
    SANCTION = 360_400, "Sanction", "Temporary or permanent restriction", "fas fa-user-minus"
    SANCTION_EVENT = 360_401, "Sanction Event", None, "fas fa-user-minus"
    SANCTION_REQUESTED_EVENT = 360_402, "Sanction Requested Event", None, "fas fa-user-minus"
    SANCTION_GRANTED_EVENT = 360_403, "Sanction Granted Event", None, "fas fa-user-minus"
    SANCTION_REVOKED_EVENT = 360_404, "Sanction Revoked Event", None, "fas fa-user-minus"
    SANCTION_EXPIRED_EVENT = 360_405, "Sanction Expired Event", None, "fas fa-user-minus"
    ENTITLEMENT = 360_500, "Entitlement", "Temporary or permanent grant", "fas fa-user-check"
    ENTITLEMENT_EVENT = 360_501, "Entitlement Event", None, "fas fa-user-check"
    ENTITLEMENT_REQUESTED_EVENT = 360_502, "Entitlement Requested Event", None, "fas fa-user-check"
    ENTITLEMENT_GRANTED_EVENT = 360_503, "Entitlement Granted Event", None, "fas fa-user-check"
    ENTITLEMENT_REVOKED_EVENT = 360_504, "Entitlement Revoked Event", None, "fas fa-user-check"
    ENTITLEMENT_EXPIRED_EVENT = 360_505, "Entitlement Expired Event", None, "fas fa-user-check"
    # CHALLENGE, ...

    # data [400_000-500_000]
    FILE = 480_000, "File", None, "fas fa-file"
    # DIRECTORY, SYNC, ...
    # INDEX, CONSTRAINT, MIGRATION, ...
    # REMOTE, ...
    # SECRET, ...

    # media [500_000-600_000]
    # STREAM, ...
    # ENCODING, ...

    # localization [600_000-700_000]
    # LOCALIZATION, STRING, TRANSLATION, ...
    # LOCALIZATION_VARIANT, GEO_VARIANT, ...

    # logic [700_000-800_000]
    SCRIPT = 700_000, "Script", None, "fas fa-code"
    TRIGGER = 705_000, "Trigger", None, "fas fa-bolt"
    TRIGGER_EVENT = 705_001, "Trigger Event", None, "fas fa-bolt"
    TIMER = 705_100, "Timer", None, "fas fa-clock"
    TIMER_EVENT = 705_101, "Timer Event", None, "fas fa-clock"
    TIMER_STARTED_EVENT = 705_102, "Timer Started Event", None, "fas fa-clock"
    TIMER_PAUSED_EVENT = 705_103, "Timer Paused Event", None, "fas fa-clock"
    TIMER_RESUMED_EVENT = 705_104, "Timer Resumed Event", None, "fas fa-clock"
    TIMER_COMPLETED_EVENT = 705_105, "Timer Completed Event", None, "fas fa-clock"
    TIMER_CANCELLED_EVENT = 705_106, "Timer Cancelled Event", None, "fas fa-clock"
    ROUTE = 710_000, "Route", None, "fas fa-route"
    # BREAKPOINT, ...
    # ROOM, TOPIC, CHANNEL, ...
    # QUEUE, TASK, ...
    # SEMAPHORE, LOCK/LATCH, ...
    # RATE_LIMIT, ...
    # STATE_MACHINE, STATE, STATE_TRANSITION, ...
    # PLATFORM_VARIANT, STATE_VARIANT, ...

    # quality [800_000-900_000]
    # TEST, TEST_SUITE, TEST_CASE, TEST_RESULT, ...
    # FIXTURE, MOCK, ...
    # LINT, WARNING, ERROR, ...
    # DEPRECATION, ...

    # intelligence [900_000-1_000_000]
    # MODEL, FINETUNE, ...
    # PROMPT, INFERENCE/COMPLETION/..., ...
    # RECOMMENDATION, ...

    # infrastructure [1_000_000-1_100_000]
    # DATABASE, ...
    MACHINE = 1_001_000, "Machine", "Machine for ephemeral computing", "fas fa-machine-classic"
    # SEARCH, VAULT, CACHE, S3, ...
    # GALAXY, ...
    # HOST, ENDPOINT, DEPLOYMENT, NETWORK, AUTOSCALER, ...

    # deployment [1_100_000-1_200_000]
    ENVIRONMENT = 1_100_000, "Environment", None, "fas fa-environment"
    # RELEASE, DEPLOYMENT, ...
    # PREVIEW, DRAFT, ROLLOUT, ...
    # TASK, TASK_GROUP/TASK_QUEUE, ...
    # JOB, ...
    RUN = 1_110_000, "Run", None, "fas fa-play"
    RUN_EVENT = 1_110_001, "Run Event", None, "fas fa-play"
    RUN_STARTED_EVENT = 1_110_002, "Run Started Event", None, "fas fa-play"
    RUN_PAUSE_REQUESTED_EVENT = 1_110_003, "Run Pause Requested Event", None, "fas fa-play"
    RUN_PAUSED_EVENT = 1_110_004, "Run Paused Event", None, "fas fa-play"
    RUN_RESUME_REQUESTED_EVENT = 1_110_005, "Run Resume Requested Event", None, "fas fa-play"
    RUN_RESUMED_EVENT = 1_110_006, "Run Resumed Event", None, "fas fa-play"
    RUN_STOP_REQUESTED_EVENT = 1_110_007, "Run Stop Requested Event", None, "fas fa-play"
    RUN_FAILED_EVENT = 1_110_008, "Run Failed Event", None, "fas fa-play"
    RUN_COMPLETED_EVENT = 1_110_009, "Run Completed Event", None, "fas fa-play"
    SPAN_EVENT = 1_110_010, "Span", None, "fas fa-ruler-horizontal"
    LOG_EVENT = 1_110_011, "Log", None, "fas fa-file-lines"

    # observability [1_200_000-1_300_000]
    # metric
    GAUGE_METRIC = 1_200_000, "Gauge Metric", None, "fas fa-gauge"
    GAUGE_MEASUREMENT_EVENT = 1_200_001, "Gauge Measurement", None, "fas fa-gauge"
    COUNTER_METRIC = 1_200_100, "Counter Metric", None, "fas fa-gauge"
    COUNTER_MEASUREMENT_EVENT = 1_200_101, "Counter Measurement", None, "fas fa-gauge"
    HISTOGRAM_METRIC = 1_200_200, "Histogram Metric", None, "fas fa-gauge"
    HISTOGRAM_MEASUREMENT_EVENT = 1_200_201, "Histogram Measurement", None, "fas fa-gauge"
    # INCIDENT, ESCALATION, ...

    # experience [1_300_000-1_400_000]
    # VISIT/SESSION, RECORDING/REPLAY, ...
    # SURVEY, ...
    # ONBOARDING, TOUR, FUNNEL, COHORT, JOURNEY, ..
    # FEATURE, FEATURE_FLAG, FEATURE_GATE, ...
    # SEGMENT, EXPERIMENT, ...
    # SETTINGS, ...

    # social [1_400_000-1_500_000]
    REACTION = 1_400_000, "Reaction", None, "fas fa-heart"
    REACTION_EVENT = 1_400_001, "Reaction Event", None, "fas fa-heart"
    REACTION_ADDED_EVENT = 1_400_002, "Reaction Added Event", None, "fas fa-heart"
    REACTION_REMOVED_EVENT = 1_400_003, "Reaction Removed Event", None, "fas fa-heart"
    STAR = 1_400_100, "Star", None, "fas fa-star"
    STAR_EVENT = 1_400_101, "Star Event", None, "fas fa-star"
    STAR_ADDED_EVENT = 1_400_102, "Star Added Event", None, "fas fa-star"
    STAR_REMOVED_EVENT = 1_400_103, "Star Removed Event", None, "fas fa-star"
    FOLLOW = 1_400_200, "Follow", None, "fas fa-plus"
    FOLLOW_EVENT = 1_400_201, "Follow Event", None, "fas fa-plus"
    FOLLOW_ADDED_EVENT = 1_400_202, "Follow Added Event", None, "fas fa-plus"
    FOLLOW_REMOVED_EVENT = 1_400_203, "Follow Removed Event", None, "fas fa-plus"
    NOTIFICATION = 1_400_500, "Notification", None, "fas fa-bell"
    NOTIFICATION_EVENT = 1_400_501, "Notification Event", None, "fas fa-bell"
    NOTIFICATION_SENT_EVENT = 1_400_502, "Notification Sent Event", None, "fas fa-bell"
    NOTIFICATION_RESCINDED_EVENT = 1_400_503, "Notification Rescinded Event", None, "fas fa-bell"
    NOTIFICATION_READ_EVENT = 1_400_504, "Notification Read Event", None, "fas fa-bell"
    NOTIFICATION_DISMISSED_EVENT = 1_400_505, "Notification Dismissed Event", None, "fas fa-bell"
    NOTIFICATION_EXPIRED_EVENT = 1_400_506, "Notification Expired Event", None, "fas fa-bell"
    # FEED, FEED_ITEM, ...
    # THREAD, MESSAGE, ...
    # POLL, VOTE, REVIEW, RATING, RANK, ...
    # ACHIEVEMENT, BADGE, WISHLIST/WATCHLIST, ...

    # finance [1_500_000-1_600_000]
    # WALLET, BALANCE, BUDGET, TRANSFER, CREDIT, ...
    # TIER, SUBSCRIPTION, PRODUCT, PRICE, ...
    # ORDER, INVOICE, DISCOUNT, DISPUTE, REFUND, ...

    # scene [1_700_000-1_800_000]
    STAGE = 1_700_000, "Stage", None, "fas fa-masks-theater"
    SCENE = 1_700_200, "Scene", "Scene of an Application", "fas fa-masks-theater"
    SCENE_EVENT = 1_700_201, "Scene Event", None, "fas fa-masks-theater"
    LAYER = 1_700_300, "Layer", "Layer of a Scene", "fas fa-layer-group"
    # BREAKPOINT_VARIANT, ...
    # VIEWPORT, OVERLAY, WIDGET, HUD, ...
    # ROOM, ...
    # FORM, MENU, ...
    # CULLING, ...

    # view [1_800_000-1_900_000]
    # container views
    VIEW = 1_800_000, "View", "View in a Scene", "fas fa-eye"
    VIEW_EVENT = 1_800_001, "View Event", None, "fas fa-eye"
    LAYOUT_VIEW = 1_800_100, "Container View", None, "fas fa-table"
    FRAME_VIEW = 1_800_200, "Frame View", "Fixed Container", "fas fa-frame"
    LABEL_VIEW = 1_800_300, "Label View", "Label Container", "fas fa-font-case"
    SPLIT_VIEW = 1_800_400, "Split View", "Split Container", "fas fa-columns"
    # SLOT_DEFINITION_VIEW, SLOT_VIEW, ...
    # FORM_VIEW, MENU_VIEW, ...
    # TAB_VIEW, ...
    # DRAWER_VIEW, SPLIT_DRAWER_VIEW, GRID/GRID_ELEMENT_VIEW, ...
    # POPOVER, SHEET, ALERT, HUD, ...
    # content views
    CONTENT_VIEW = 1_805_000, "Content View", None, "fas fa-text"
    TEXT_VIEW = 1_805_100, "Text View", "Text", "fas fa-text"
    # CODE_VIEW, ICON_VIEW, IMAGE_VIEW, AUDIO_VIEW, VIDEO_VIEW, DOCUMENT_VIEW, ...
    # input views
    INPUT_VIEW = 1_810_000, "Input View", None, "fas fa-hashtag"
    NUMBER_INPUT_VIEW = 1_810_100, "Number Input View", "Number Input", "fas fa-hashtag"
    SLIDER_INPUT_VIEW = 1_810_200, "Slider Input View", "Slider Input", "fas fa-slider"
    # STRING_INPUT_VIEW, TOGGLE_INPUT_VIEW, PICKER_INPUT_VIEW, COLOR_INPUT_VIEW, ...
    # ICON_INPUT_VIEW, FILE_INPUT_VIEW, DATETIME_INPUT_VIEW, DURATION_INPUT_VIEW, ...

    # paint [1_900_000-2_000_000]
    # RASTER/BITMAP, ...
    # DAB, PAINT, BRUSH, ...
    # SPRITE, SPRITE_SHEET, NINESLICE_SPRITE, TILING_SPRITE, ...
    # TEXTURE, ...

    # interaction [2_000_000-2_100_000]
    INPUT_EVENT = 2_000_000, "Input Event", None, "fas fa-mouse-pointer"
    # pointer events
    POINTER_EVENT = 2_000_100, "Pointer Event", None, "fas fa-mouse-pointer"
    POINTER_DOWN_EVENT = 2_000_101, "Pointer Down Event", None, "fas fa-mouse-pointer"
    POINTER_UP_EVENT = 2_000_102, "Pointer Up Event", None, "fas fa-mouse-pointer"
    POINTER_MOVE_EVENT = 2_000_103, "Pointer Move Event", None, "fas fa-mouse-pointer"
    POINTER_ENTER_EVENT = 2_000_104, "Pointer Enter Event", None, "fas fa-mouse-pointer"
    POINTER_OVER_EVENT = 2_000_105, "Pointer Over Event", None, "fas fa-mouse-pointer"
    POINTER_LEAVE_EVENT = 2_000_106, "Pointer Leave Event", None, "fas fa-mouse-pointer"
    POINTER_LONG_PRESS_EVENT = 2_000_107, "Long Press Event", None, "fas fa-mouse-pointer"
    # mouse events
    MOUSE_EVENT = 2_000_200, "Mouse Event", None, "fas fa-mouse-pointer"
    CLICK_EVENT = 2_000_201, "Click Event", None, "fas fa-mouse-pointer"
    SINGLE_CLICK_EVENT = 2_000_202, "Single Click Event", None, "fas fa-mouse-pointer"
    DOUBLE_CLICK_EVENT = 2_000_203, "Double Click Event", None, "fas fa-mouse-pointer"
    TRIPLE_CLICK_EVENT = 2_000_204, "Triple Click Event", None, "fas fa-mouse-pointer"
    WHEEL_EVENT = 2_000_210, "Wheel Event", None, "fas fa-mouse-pointer"
    # key events
    KEY_EVENT = 2_000_300, "Key Event", None, "fas fa-keyboard"
    KEY_DOWN_EVENT = 2_000_301, "Key Down Event", None, "fas fa-keyboard"
    KEY_UP_EVENT = 2_000_302, "Key Up Event", None, "fas fa-keyboard"
    KEY_PRESS_EVENT = 2_000_303, "Key Press Event", None, "fas fa-keyboard"
    # drag events
    DRAG_EVENT = 2_000_400, "Drag Event", None, "fas fa-arrows-up-down-left-right"
    DRAG_START_EVENT = 2_000_401, "Drag Start Event", None, "fas fa-arrows-up-down-left-right"
    DRAG_END_EVENT = 2_000_402, "Drag End Event", None, "fas fa-arrows-up-down-left-right"
    DRAG_OVER_EVENT = 2_000_403, "Drag Over Event", None, "fas fa-arrows-up-down-left-right"
    DRAG_ENTER_EVENT = 2_000_404, "Drag Enter Event", None, "fas fa-arrows-up-down-left-right"
    DRAG_LEAVE_EVENT = 2_000_405, "Drag Leave Event", None, "fas fa-arrows-up-down-left-right"
    DROP_EVENT = 2_000_406, "Drop Event", None, "fas fa-arrows-up-down-left-right"
    # clipboard events
    CLIPBOARD_EVENT = 2_000_500, "Clipboard Event", None, "fas fa-clipboard"
    COPY_EVENT = 2_000_501, "Copy Event", None, "fas fa-clipboard"
    CUT_EVENT = 2_000_502, "Cut Event", None, "fas fa-clipboard"
    PASTE_EVENT = 2_000_503, "Paste Event", None, "fas fa-clipboard"
    # focus events
    FOCUS_EVENT = 2_000_600, "Focus Event", None, "fas fa-keyboard"
    FOCUS_IN_EVENT = 2_000_601, "Focus In Event", None, "fas fa-keyboard"
    FOCUS_OUT_EVENT = 2_000_602, "Focus Out Event", None, "fas fa-keyboard"
    # command
    # COMMAND,  MODE, TOOL, SHORTCUT/KEYBINDING, ...
    # GESTURE, ...
    # CLIPBOARD, ...
    # CAMERA, SPEAKER, MICROPHONE, ...
    # AUDIO, AUDIO_PLAYER, VIDEO, VIDEO_PLAYER, ...

    # style [2_100_000-2_200_000]
    THEME = 2_100_000, "Theme", None, "fas fa-palette"
    PALETTE = 2_100_100, "Palette", None, "fas fa-palette"
    STYLE = 2_100_200, "Style", None, "fas fa-palette"
    COLOR_STYLE = 2_100_300, "Color Style", None, "fas fa-palette"
    FILL_STYLE = 2_100_400, "Fill Style", None, "fas fa-fill"
    FONT_STYLE = 2_100_500, "Font Style", None, "fas fa-text"
    BORDER_STYLE = 2_100_600, "Border Style", None, "fas fa-border-outer"
    SHADOW_STYLE = 2_100_700, "Shadow Style", None, "fas fa-eclipse"
    GRADIENT_STYLE = 2_100_800, "Gradient Style", None, "fas fa-gradient"
    STROKE_STYLE = 2_101_100, "Stroke Style", None, "fas fa-stroke"
    # BRUSH_STYLE, ...
    # SHADER, MATERIAL, ...

    # animation [2_200_000-2_300_000]
    TRANSITION_STYLE = 2_200_000, "Transition Style", None, "fas fa-bezier-curve"
    EFFECT_STYLE = 2_200_100, "Effect Style", None, "fas fa-sparkle"
    # ANIMATION, ANIMATION_TRACK, ANIMATION_KEYFRAME, ...
    # KEYFRAME_VARIANT, ...
    # RIG, ...
    # PARTICLE, EMITTER, ...

    # audio [2_300_000-2_400_000]
    # SOUND_SOURCE, ...

    # geometry [2_400_000-2_500_000]
    # VECTOR_NETWORK, VECTOR_POINT, VECTOR_SEGMENT, VECTOR_REGION, ...
    SHAPE2D = 2_410_000, "Shape2D", None, "fas fa-shapes"
    LINE_SHAPE2D = 2_410_100, "Line Shape2D", None, "fas fa-line"
    ARROW_SHAPE2D = 2_410_200, "Arrow Shape2D", None, "fas fa-arrow-right"
    RECTANGLE_SHAPE2D = 2_410_300, "Rectangle Shape2D", None, "fas fa-rectangle"
    ELLIPSE_SHAPE2D = 2_410_400, "Ellipse Shape2D", None, "fas fa-ellipse"
    CAPSULE_SHAPE2D = 2_410_500, "Capsule Shape2D", None, "fas fa-capsule"
    STAR_SHAPE2D = 2_410_600, "Star Shape2D", None, "fas fa-star"
    POLYGON_SHAPE2D = 2_411_000, "Polygon Shape2D", None, "fas fa-polygon"
    PATH_SHAPE2D = 2_411_100, "Path Shape2D", None, "fas fa-path"
    SHAPE3D = 2_415_000, "Shape3D", None, "fas fa-shapes"

    # physics [2_500_000-2_600_000]
    # BODY, BODY2D, ...
    # BODY_EVENT, CONTACT_EVENT, COLLISION_EVENT, ...
    # RIGID_BODY, SOFT_BODY, ...
    # COLLIDER, ...
    # SKELETON, BONE, ...
    # JOINT, FIXED_JOINT, FREE_JOINT, SPHERICAL_JOINT, SPRING, MOTOR, ...

    # lighting [2_600_000-2_700_000]
    # LIGHT, LIGHT2D, ...
    # POINT_LIGHT, DIRECTIONAL_LIGHT, SPOT_LIGHT, AMBIENT_LIGHT, ...
    # OCCLUDER, ...
    # ...

    # editor [3_000_000-3_100_000]
    # INSPECTOR_VIEW, ...
    # ...


@builtin_enum(EnumType.UNIVERSE_CATEGORY)
class UniverseCategory(Enum):
    """How the system is organized."""

    CORE = 1, "Core", "Intrinsics"
    UNIVERSE = 100_000, "Universe", "Global computational universe"
    SPACE = 200_000, "Space", "Spacetime organization"
    ACCESS = 300_000, "Access", "Access control"
    DATA = 400_000, "Data", "Core data"
    MEDIA = 500_000, "Media", "Media and streaming"
    LOCALIZATION = 600_000, "Localization", "Localization and internationalization"
    LOGIC = 700_000, "Logic", "Core logic"
    QUALITY = 800_000, "Quality", "Quality management"
    INTELLIGENCE = 900_000, "Intelligence", "Artificial intelligence"
    INFRASTRUCTURE = 1_000_000, "Infrastructure", "Devices, hardware and plumbing"
    DEPLOYMENT = 1_100_000, "Deployment", "Deployment and runtime"
    OBSERVABILITY = 1_200_000, "Observability", "Analytics about everything"
    EXPERIENCE = 1_300_000, "Experience", "User experience"
    SOCIAL = 1_400_000, "Social", "Social interactions"
    FINANCE = 1_500_000, "Finance", "Financial operations"
    SCENE = 1_700_000, "Scene", "Stage building"
    VIEW = 1_800_000, "View", "View building"
    PAINT = 1_900_000, "Paint", "Drawing, rendering and painting"
    INTERACTION = 2_000_000, "Interaction", "Interaction design"
    STYLE = 2_100_000, "Style", "Appearance and materials"
    ANIMATION = 2_200_000, "Animation", "Motion design"
    AUDIO = 2_300_000, "Audio", "Audio management"
    GEOMETRY = 2_400_000, "Geometry", "Meshes, skeletons and maths"
    PHYSICS = 2_500_000, "Physics", "Physics simulation"
    LIGHTING = 2_600_000, "Lighting", "Lighting and shadows"
    EDITOR = 3_000_000, "Editor", "Editor and studio"


ENUM_TYPES: tuple[EnumType, ...] = tuple(EnumType)
NODE_TYPES: tuple[NodeType, ...] = tuple(NodeType)
STRUCT_TYPES: tuple[StructType, ...] = tuple(StructType)
TRAIT_TYPES: tuple[TraitType, ...] = tuple(TraitType)
