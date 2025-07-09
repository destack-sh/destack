from datetime import date, datetime, time, timedelta
from decimal import Decimal
from typing import TYPE_CHECKING

from destack.utils.uuid import UUID

from .enum import Enum, builtin_enum

if TYPE_CHECKING:
    pass


#
# Enums
#


class EnumType(Enum):
    # meta [1-20_000]
    ENUM_TYPE = 1
    NODE_TYPE = 2
    STRUCT_TYPE = 3
    TRAIT_TYPE = 4
    EVENT_STATUS = 8
    UNIVERSE_CATEGORY = 9
    NODE_DEFINITION_TYPE = 10
    OBJECT_DEFINITION_TYPE = 11
    STRUCT_DEFINITION_TYPE = 12
    PROPERTY_REFERENCE_TYPE = 13
    MATERIALIZATION = 14
    STORE_KEY = 20
    STORE_SCOPE = 21
    STORE_DOMAIN = 22
    STORE_TIER = 23
    STORE_IMPLEMENTATION = 25
    PLATFORM_TYPE = 30
    RUNTIME_LANGUAGE = 31
    OPERATING_SYSTEM = 40
    EDIT_TYPE = 50
    EDIT_OPERATION = 51
    PRIMITIVE_TYPE = 60
    TYPE_CARDINALITY = 61
    SCALAR_TYPE = 62
    VALUE_FACTORY = 63
    STRING_FORMAT = 64
    NUMBER_FORMAT = 65
    PROPERTY_TYPE = 66
    EDGE_TYPE = 67
    EDGE_DIRECTION = 68
    CASCADE_ACTION = 69
    RESOURCE_STATUS = 1100
    SNAPSHOT_TYPE = 1300
    SNAPSHOT_STATUS = 1301

    # query
    CONDITIONAL_TYPE = 10_103
    AGGREGATION_TYPE = 10_104
    SORT_MODE = 10_105
    SORT_TYPE = 10_106
    JOIN_TYPE = 10_107
    FUNCTION_TYPE = 10_108
    EXPRESSION_TYPE = 10_109
    QUERY_TYPE = 10_120
    QUERY_UPDATE_TYPE = 10_121

    # universe [20_000-40_000]
    SPACE_STATUS = 20_001
    USER_STATUS = 21_200
    ORGANIZATION_STATUS = 22_500
    CLIENT_TYPE = 21_700

    # space [40_000-60_000]
    FOLDER_TYPE = 40_000

    # access [60_000-80_000]
    ROLE_TYPE = 60_200
    PERMISSION_TYPE = 60_300
    SANCTION_TYPE = 60_400
    ENTITLEMENT_TYPE = 60_500

    # data [80_000-100_000]
    FILE_RETENTION_MODE = 80_000
    FILE_SOURCE = 80_001
    FILE_TYPE = 80_002
    FILE_FORMAT = 80_003
    TEXT_SPAN_TYPE = 80_004
    ICON_TYPE = 80_005

    # logic [100_000-120_000]
    METHOD_CARDINALITY = 102_001
    TRIGGER_TYPE = 105_000
    TIMER_TYPE = 105_100
    DAY_OF_WEEK = 105_101
    MONTH = 105_102
    SCHEDULE_FREQUENCY = 105_103
    CURSOR_STATUS = 105_501

    # intelligence [120_000-140_000]
    MODEL_DEVELOPER = 120_000
    MODEL_PROVIDER = 120_001

    # infrastructure [140_000-160_000]
    CLOUD = 140_000
    REGION = 140_001
    REGION_AREA = 140_002
    REGION_CONTINENT = 140_003
    TENANCY = 140_004
    DATABASE_TYPE = 140_005
    MACHINE_TYPE = 140_100

    # deployment [160_000-180_000]
    ENVIRONMENT_TYPE = 160_000

    # runtime [170_000-180_000]
    RUN_STATUS = 170_000
    LOG_LEVEL = 170_300

    # observability [180_000-200_000]

    # optimization [200_000-220_000]

    # social [220_000-240_000]
    THREAD_STATUS = 220_000
    NOTIFICATION_STATUS = 220_500

    # finance [240_000-260_000]

    # scene [500_000-520_000]
    WINDOW_TYPE = 500_000
    LAYER_TYPE = 500_200
    VARIANT_TYPE = 500_300
    VARIANT_STATE_TYPE = 500_301

    # view [520_000-540_000]

    # canvas [540_000-560_000]
    CANVAS_TYPE = 540_000
    ARROW_HEAD_TYPE = 540_300

    # interaction [560_000-580_000]
    MODE_TYPE = 560_000
    TOOL_TYPE = 560_001
    MOUSE_BUTTON = 560_010

    # animation [580_000-600_000]

    # style [600_000-620_000]
    COLOR_TYPE = 600_000
    COLOR_SHADE = 600_001
    COLOR_HUE = 600_002
    COLOR_INTENT = 600_003
    FILL_TYPE = 600_100
    FILL_POSITION = 600_101
    FILL_SIZE = 600_102
    FONT_TYPE = 600_200
    FONT_WEIGHT = 600_201
    FONT_SIZE = 600_202
    TEXT_ALIGN = 600_203
    TEXT_DECORATION = 600_204
    TEXT_TRANSFORM = 600_205
    BORDER_TYPE = 600_206
    SHADOW_TYPE = 600_207
    SHADOW_POSITION = 600_208
    GRADIENT_TYPE = 600_209
    TRANSITION_TYPE = 600_210
    SPRING_TYPE = 600_211
    EFFECT_TYPE = 600_212
    STROKE_TYPE = 600_213
    POSITION_TYPE = 600_214
    LENGTH_UNIT = 600_215
    LAYOUT = 600_216
    DISTRIBUTE = 600_217
    ALIGN = 600_218
    DIRECTION = 600_219
    OVERFLOW = 600_220
    DIMENSION_TYPE = 600_221
    REPEAT_TYPE = 600_222
    TEXT_SPLIT_TYPE = 600_223
    OFFSCREEN_BEHAVIOR = 600_224
    EASING = 600_225


builtin_enum(EnumType.ENUM_TYPE)(EnumType)


@builtin_enum(EnumType.STRUCT_TYPE)
class StructType(Enum):
    # meta [1-20_000]
    STRUCT = 1, "Struct", "Root of all Structs", "fas fa-shapes"
    CUSTOM_STRUCT = 2
    # definitions
    BUILTIN_DEFINITION = 100
    NODE_DEFINITION = 101
    TRAIT_DEFINITION = 102
    STRUCT_DEFINITION = 103
    ENUM_DEFINITION = 104
    PROPERTY_DEFINITION = 110
    PROPERTY_GROUP_DEFINITION = 111
    OPTION_DEFINITION = 112
    OPTION_GROUP_DEFINITION = 113
    CONSTANT_DEFINITION = 120
    METHOD_DEFINITION = 130
    ACTION_DEFINITION = 131
    PERMISSION_DEFINITION = 140
    # references
    NODE_DEFINITION_REFERENCE = 200
    OBJECT_DEFINITION_REFERENCE = 201
    STRUCT_DEFINITION_REFERENCE = 202
    # METHOD_REFERENCE, ACTION_REFERENCE, ...
    NODE_REFERENCE = 250
    PROPERTY_REFERENCE = 251
    # expressions
    EXPRESSION = 500
    FUNCTION = 501
    JOIN = 502
    AGGREGATION = 503
    CONDITION = 504
    SORT = 505
    SELECT = 506
    QUERY = 550
    QUERY_RESULT = 551
    QUERY_RESULT_GROUP = 552
    QUERY_UPDATE = 553
    HISTOGRAM = 554
    # values
    VALUE = 600
    TYPE = 601
    # constraints
    NUMBER_CONSTRAINT = 650
    STRING_CONSTRAINT = 651
    COLLECTION_CONSTRAINT = 652
    NODE_CONSTRAINT = 653
    # geometry
    VECTOR = 700, None, None, "fas fa-vector-square"
    VECTORF = 710
    VECTOR2F = 711, None, None, "fas fa-vector-square"
    VECTOR3F = 712, None, None, "fas fa-vector-square"
    VECTOR4F = 713, None, None, "fas fa-vector-square"
    VECTORI = 720
    VECTOR2I = 721, None, None, "fas fa-vector-square"
    VECTOR3I = 722, None, None, "fas fa-vector-square"
    VECTOR4I = 723, None, None, "fas fa-vector-square"

    # universe [20_000-40_000]
    # ...

    # space [40_000-60_000]
    # ...

    # access [60_000-80_000]
    # ...

    # data [80_000-100_000]
    TEXT = 80_020, None, None, "fas fa-text"
    TEXT_SPAN = 80_021, None, None, "fas fa-text"
    ICON = 80_031
    # ...

    # logic [100_000-120_000]
    SCHEDULE = 100_001
    # ...

    # intelligence [120_000-140_000]
    # ...

    # infrastructure [140_000-160_000]
    DATABASE_INFO = 140_001
    GALAXY_INFO = 140_101
    # ...

    # deployment [160_000-180_000]
    # ...

    # observability [180_000-200_000]
    # ...

    # optimization [200_000-220_000]
    # ...

    # social [220_000-240_000]
    # ...

    # finance [240_000-260_000]
    # ...

    # scene [500_000-520_000]
    # ...

    # view [520_000-540_000]
    # ...

    # canvas [540_000-560_000]
    LINE = 540_200, "Line", None, "fas fa-line"
    ARROW = 540_300, "Arrow", None, "fas fa-arrow-right"

    # interaction [560_000-580_000]
    # ...

    # animation [580_000-600_000]
    # ...

    # style [600_000-620_000]
    LENGTH = 600_018, "Length", None, "fas fa-ruler"
    POSITION = 600_020, "Position", None, "fas fa-location-crosshair"
    DIMENSION = 600_022, "Dimension", None, "fas fa-ruler"
    GRID = 600_026, "Grid", None, "fas fa-grid-2"
    GRID_SPAN = 600_028, "Grid Span", None, "fas fa-grid-2"
    INSETS = 600_030, "Insets", None, "fas fa-corner"
    CORNERS = 600_032, "Corners", None, "fas fa-corner"
    AXIS2 = 600_034, "Axis2", None, "fas fa-vector-square"
    AXIS3 = 600_036, "Axis3", None, "fas fa-vector-square"
    COLOR = 600_300, "Color", None, "fas fa-palette"
    FILL = 600_400, "Fill", None, "fas fa-fill"
    FONT = 600_500, "Font", None, "fas fa-text"
    BORDER = 600_600, "Border", None, "fas fa-border-outer"
    SHADOW = 600_700, "Shadow", None, "fas fa-eclipse"
    GRADIENT = 600_800, "Gradient", None, "fas fa-gradient"
    GRADIENT_STOP = 600_801, "Gradient Stop", None, "fas fa-gradient"
    TRANSITION = 600_900, "Transition", None, "fas fa-bezier-curve"
    EFFECT = 600_1000, "Effect", None, "fas fa-sparkle"
    STROKE = 600_1100, "Stroke", None, "fas fa-stroke"
    STROKE_CAP = 600_1101, "Stroke Cap", None, "fas fa-stroke"
    STROKE_PATH = 600_1102, "Stroke Path", None, "fas fa-stroke"
    STROKE_POINT = 600_1103, "Stroke Point", None, "fas fa-stroke"


@builtin_enum(EnumType.TRAIT_TYPE)
class TraitType(Enum):
    # meta [1-20_000]
    # where
    GLOBAL = 1, "Global", "Is global", "fas fa-globe"
    SPATIAL = 2, "Spatial", "Is in a Space", "fas fa-solar-system"
    # LOCAL?
    # storage
    # RELATIONAL/OLTP, INDEXED; ANALYTIC, ...?

    # common
    ORDERED = 100, "Ordered", "Is ordered", "fas fa-sort"
    ARCHIVABLE = 101, "Archivable", "Can be archived", "fas fa-box-archive"
    DELETABLE = 102, "Deletable", "Can be deleted", "fas fa-trash"
    # PAUSABLE?
    CUSTOMIZABLE = (
        110,
        "Customizable",
        "Can be customized with custom Properties",
        "fas fa-paint-roller",
    )
    EXTENSIBLE = 111, "Extensible", "Can be extended by custom Nodes", "fas fa-expand"
    IRREVERSIBLE = 120, "Irreversible", "Cannot be rewound", "fas fa-clock-rotate-left"

    # universe [20_000-40_000]
    # ...

    # space [40_000-60_000]
    TAGGABLE = 40_000, "Taggable", "Can be tagged", "fas fa-tag"

    # access [60_000-80_000]
    OWNABLE = 60_000, "Ownable", "Is ownable", "fas fa-user"
    OWNED = 60_001, "Owned", "Is owned", "fas fa-user"
    OWNER = 60_002, "Owner", "Is an Owner", "fas fa-user"
    JOINABLE = 60_003, "Joinable", "Is joinable", "fas fa-users"
    SUBJECT = 60_004, "Subject", "Is a Subject", "fas fa-user"

    # data [80_000-100_000]
    # ...

    # logic [100_000-120_000]
    RUNNABLE = 100_001, "Runnable", "Can be run", "fas fa-play"
    SCRIPTABLE = 100_002, "Scriptable", "Can be scripted", "fas fa-code"
    SOURCEABLE = 100_003, "Sourcable", "Can be defined in a Script", "fas fa-code"

    # intelligence [120_000-140_000]
    # ...

    # infrastructure [140_000-160_000]
    # ...

    # deployment [160_000-180_000]
    # ...

    # observability [180_000-200_000]
    # ...

    # optimization [200_000-220_000]
    # ...

    # social [220_000-240_000]
    STARABLE = 220_030, "Starable", "Can be starred", "fas fa-star"
    REACTABLE = 220_032, "Reactable", "Can be reacted to", "fas fa-heart"
    FOLLOWABLE = 220_034, "Followable", "Can be followed", "fas fa-plus"
    # RATEABLE, VOTABLE, ...
    # ASSIGNABLE, MESSAGEABLE, CLOSABLE, LOCKABLE, ...

    # finance [240_000-260_000]
    # ...

    # scene [500_000-520_000]
    # ...

    # view [520_000-540_000]
    # ...

    # canvas [540_000-560_000]
    # ...

    # interaction [560_000-580_000]
    # ...

    # animation [580_000-600_000]
    # ...

    # style [600_000-620_000]
    # ...


@builtin_enum(EnumType.NODE_TYPE)
class NodeType(Enum):
    # meta [1-10_000]
    # root
    NODE = 1, "Node", "Root of all Nodes", "fas fa-dot"
    ENTITY = 2, "Entity", "Versioned, stateful Node", "fas fa-dot"
    EVENT = 3, "Event", "Immutable datum of something happening", "fas fa-dot"
    # custom
    CUSTOM_ENTITY_DEFINITION = 100, "Custom Entity Definition", None, "fas fa-table"
    CUSTOM_TRAIT_DEFINITION = 101, "Custom Trait Definition", None, "fas fa-table"
    CUSTOM_EVENT_DEFINITION = 102, "Custom Event Definition", None, "fas fa-signal"
    CUSTOM_STRUCT_DEFINITION = 103, "Custom Struct Definition", None, "fas fa-shapes"
    CUSTOM_ENUM_DEFINITION = 104, "Custom Enum Definition", None, "fas fa-shapes"
    CUSTOM_PROPERTY = 110, "Custom Property", None, "fas fa-triangle"
    CUSTOM_PROPERTY_GROUP = 111, "Custom Property Group", None, "fas fa-table"
    CUSTOM_OPTION = 120, "Custom Option", None, "fas fa-circle"
    CUSTOM_OPTION_GROUP = 121, "Custom Option Group", None, "fas fa-table"
    # entity
    RECORD = 1000, "Record", "Custom Entity", "fas fa-database"
    RESOURCE = 1100, "Resource", "External asset outside of Destack", "fas fa-dot"
    METRIC = 1200, "Metric", None, "fas fa-gauge"
    SNAPSHOT = 1300, "Snapshot", "Point in Space-time", "fas fa-save"
    # event
    SIGNAL = 2000, "Signal", "Custom Event", "fas fa-signal"
    EDIT_EVENT = 2001, "Edit Event", None, "fas fa-file-lines"
    # CHANGE_EVENT?
    MEASUREMENT_EVENT = 2010, "Measurement", None, "fas fa-gauge"

    # universe [20_000-40_000]
    UNIVERSE = 20_000, "Universe", "Universal Space", None
    SPACE = 20_100, "Space", "Universal Space", "https://heydestack.com/favicon.ico"
    HANDLE = 20_200, "Handle", "Unique @handle", "fas fa-at"
    # user
    USER = 21_000, "User", None, "fas fa-user"
    FRIENDSHIP = 21_100, "Friendship", "Friendship between two Users", "fas fa-user-friends"
    FRIENDSHIP_INVITE = (
        21_200,
        "Friendship Invite",
        "Invite to be friends with another User",
        "fas fa-user-plus",
    )
    FRIENDSHIP_INVITE_EVENT = 21_201, "Friendship Invite Event", None, "fas fa-user-plus"
    FRIENDSHIP_INVITE_SENT_EVENT = 21_202, "Friendship Invite Sent Event", None, "fas fa-user-plus"
    FRIENDSHIP_INVITE_RESCINDED_EVENT = (
        21_203,
        "Friendship Invite Rescinded Event",
        None,
        "fas fa-user-plus",
    )
    FRIENDSHIP_INVITE_ACCEPTED_EVENT = (
        21_204,
        "Friendship Invite Accepted Event",
        None,
        "fas fa-user-plus",
    )
    FRIENDSHIP_INVITE_REJECTED_EVENT = (
        21_205,
        "Friendship Invite Rejected Event",
        None,
        "fas fa-user-plus",
    )
    CLIENT = 21_300, "Client", None, "fas fa-desktop"
    # CREDENTIAL, ACCOUNT, PROFILE, ...
    # organization
    ORGANIZATION = 22_000, "Organization", None, "fas fa-building"
    TEAM = 22_100, "Team", "Team in an Organization", "fas fa-users"

    # space [40_000-60_000]
    # folder
    FOLDER = 40_000, "Folder", "Sub-space of a Space", "fas fa-folder-open"
    # DEPENDENCY, ...
    TAG = 41_000, "Tag", None, "fas fa-tag"
    TAGGING = 41_100, "Tagging", None, "fas fa-tag"
    # spacetime
    BRANCH = 45_000, "Branch", None, "fas fa-code-branch"
    # HISTORY, REPLAY, ...
    # FORK, ...

    # access [60_000-80_000]
    MEMBERSHIP = 60_000, "Membership", "Membership in a Space/Folder", "fas fa-user-group"
    MEMBERSHIP_EVENT = 60_001, "Membership Event", None, "fas fa-user-group"
    MEMBERSHIP_JOINED_EVENT = 60_002, "Membership Join Event", None, "fas fa-user-group"
    MEMBERSHIP_LEFT_EVENT = 60_003, "Membership Leave Event", None, "fas fa-user-group"
    INVITE = 60_100, "Invite", "Invite to a Space/Folder", "fas fa-user-plus"
    INVITE_EVENT = 60_101, "Invite Event", None, "fas fa-user-plus"
    INVITE_SENT_EVENT = 60_102, "Invite Sent Event", None, "fas fa-user-plus"
    INVITE_RESCINDED_EVENT = 60_103, "Invite Rescinded Event", None, "fas fa-user-plus"
    INVITE_ACCEPTED_EVENT = 60_104, "Invite Accepted Event", None, "fas fa-user-plus"
    INVITE_REJECTED_EVENT = 60_105, "Invite Rejected Event", None, "fas fa-user-plus"
    ROLE = 60_200, "Role", "Role in something", "fas fa-user-tag"
    ROLE_EVENT = 60_201, "Role Event", None, "fas fa-user-tag"
    ROLE_ASSIGNED_EVENT = 60_202, "Role Assigned Event", None, "fas fa-user-tag"
    ROLE_UNASSIGNED_EVENT = 60_203, "Role Unassigned Event", None, "fas fa-user-tag"
    PERMISSION = 60_300, "Permission", "Permission for something", "fas fa-user-shield"
    SANCTION = 60_400, "Sanction", "Temporary or permanent restriction", "fas fa-user-minus"
    SANCTION_EVENT = 60_401, "Sanction Event", None, "fas fa-user-minus"
    SANCTION_REQUESTED_EVENT = 60_402, "Sanction Requested Event", None, "fas fa-user-minus"
    SANCTION_GRANTED_EVENT = 60_403, "Sanction Granted Event", None, "fas fa-user-minus"
    SANCTION_REVOKED_EVENT = 60_404, "Sanction Revoked Event", None, "fas fa-user-minus"
    SANCTION_EXPIRED_EVENT = 60_405, "Sanction Expired Event", None, "fas fa-user-minus"
    ENTITLEMENT = 60_500, "Entitlement", "Temporary or permanent grant", "fas fa-user-check"
    ENTITLEMENT_EVENT = 60_501, "Entitlement Event", None, "fas fa-user-check"
    ENTITLEMENT_REQUESTED_EVENT = 60_502, "Entitlement Requested Event", None, "fas fa-user-check"
    ENTITLEMENT_GRANTED_EVENT = 60_503, "Entitlement Granted Event", None, "fas fa-user-check"
    ENTITLEMENT_REVOKED_EVENT = 60_504, "Entitlement Revoked Event", None, "fas fa-user-check"
    ENTITLEMENT_EXPIRED_EVENT = 60_505, "Entitlement Expired Event", None, "fas fa-user-check"
    AGENT = 60_600, "Agent", None, "fas fa-robot"
    # CHALLENGE, ...

    # data [80_000-100_000]
    FILE = 80_000, "File", None, "fas fa-file"
    # INDEX, CONSTRAINT, MIGRATION, ...
    # MIRROR/SYNC, ...
    # STREAM, SECRET, ...
    # LOCALE, STRING, TRANSLATION, ...
    # SETTINGS, ...

    # logic [100_000-120_000]
    SERVICE = 100_000, "Service", None, "fas fa-screwdriver-wrench"
    SCRIPT = 105_000, "Script", None, "fas fa-code"
    METHOD = 106_000, "Method", None, "fas fa-code"
    ACTION = 106_100, "Action", None, "fas fa-code"
    TRIGGER = 107_000, "Trigger", None, "fas fa-bolt"
    TRIGGER_EVENT = 107_001, "Trigger Event", None, "fas fa-bolt"
    TIMER = 107_100, "Timer", None, "fas fa-clock"
    TIMER_EVENT = 107_101, "Timer Event", None, "fas fa-clock"
    TIMER_STARTED_EVENT = 107_102, "Timer Started Event", None, "fas fa-clock"
    TIMER_COMPLETED_EVENT = 107_103, "Timer Completed Event", None, "fas fa-clock"
    TIMER_CANCELLED_EVENT = 107_104, "Timer Cancelled Event", None, "fas fa-clock"
    CURSOR = 108_000, "Cursor", None, "fas fa-mouse-pointer"
    EVENT_CURSOR = 108_100, "Event Cursor", None, "fas fa-signal"
    SCREEN_CURSOR = 108_200, "Screen Cursor", None, "fas fa-mouse"
    THREAD_CURSOR = 108_300, "Thread Cursor", None, "fas fa-magnifying-glass"
    # QUERY_CURSOR, WEB_CURSOR, ...
    ROUTE = 110_000, "Route", None, "fas fa-route"
    # BREAKPOINT, ...
    # ROOM, CHANNEL, ...
    # SEMAPHORE, LOCK/LATCH, ...
    # RATE_LIMIT, ...
    # test
    # TEST, TEST_SUITE, TEST_CASE, TEST_RESULT, ...
    # FIXTURE, MOCK, ...
    # LINT, WARNING, ERROR, ...

    # intelligence [120_000-140_000]
    # MODEL, FINETUNE, ...
    # PROMPT, INFERENCE/COMPLETION/..., ...
    # RECOMMENDATION, ...

    # infrastructure [140_000-160_000]
    DATABASE = 140_000, "Database", "Database for Postgres data", "fas fa-database"
    MACHINE = 140_100, "Machine", "Machine for ephemeral computing", "fas fa-machine-classic"
    # SEARCH/INDEX, VAULT, CACHE, S3, ...
    # GALAXY, ...
    # HOST, ENDPOINT, DEPLOYMENT, NETWORK, AUTOSCALER, ...

    # deployment [160_000-180_000]
    ENVIRONMENT = 160_000, "Environment", None, "fas fa-environment"
    # VERSION, DEPLOYMENT, ...
    # PREVIEW, DRAFT, RELEASE, ROLLOUT, ...
    # TASK, TASK_GROUP/TASK_QUEUE, ...
    # QUEUE, RUN_QUEUE, ...
    # JOB, ...
    RUN = 170_000, "Run", None, "fas fa-play"
    RUN_EVENT = 170_001, "Run Event", None, "fas fa-play"
    RUN_STARTED_EVENT = 170_002, "Run Started Event", None, "fas fa-play"
    RUN_PAUSE_REQUESTED_EVENT = 170_003, "Run Pause Requested Event", None, "fas fa-play"
    RUN_PAUSED_EVENT = 170_004, "Run Paused Event", None, "fas fa-play"
    RUN_RESUME_REQUESTED_EVENT = 170_005, "Run Resume Requested Event", None, "fas fa-play"
    RUN_RESUMED_EVENT = 170_006, "Run Resumed Event", None, "fas fa-play"
    RUN_STOP_REQUESTED_EVENT = 170_007, "Run Stop Requested Event", None, "fas fa-play"
    RUN_FAILED_EVENT = 170_008, "Run Failed Event", None, "fas fa-play"
    RUN_COMPLETED_EVENT = 170_009, "Run Completed Event", None, "fas fa-play"
    SPAN_EVENT = 170_101, "Span", None, "fas fa-ruler-horizontal"
    LOG_EVENT = 170_301, "Log", None, "fas fa-file-lines"

    # observability [180_000-200_000]
    # metric
    GAUGE_METRIC = 180_000, "Gauge Metric", None, "fas fa-gauge"
    GAUGE_MEASUREMENT_EVENT = 180_001, "Gauge Measurement", None, "fas fa-gauge"
    COUNTER_METRIC = 180_100, "Counter Metric", None, "fas fa-gauge"
    COUNTER_MEASUREMENT_EVENT = 180_101, "Counter Measurement", None, "fas fa-gauge"
    HISTOGRAM_METRIC = 180_200, "Histogram Metric", None, "fas fa-gauge"
    HISTOGRAM_MEASUREMENT_EVENT = 180_201, "Histogram Measurement", None, "fas fa-gauge"
    # INCIDENT, ESCALATION, ...
    # VISIT, RECORDING/REPLAY,

    # optimization [200_000-220_000]
    # SURVEY, ...
    # ONBOARDING, TOUR, FUNNEL, COHORT, JOURNEY, ..
    # FEATURE, FEATURE_FLAG, FEATURE_GATE, ...
    # SEGMENT, EXPERIMENT, ...

    # social [220_000-240_000]
    THREAD = 220_000, "Thread", None, "fas fa-reel"
    MESSAGE = 220_100, "Message", None, "fas fa-message"
    REACTION = 220_200, "Reaction", None, "fas fa-heart"
    REACTION_EVENT = 220_201, "Reaction Event", None, "fas fa-heart"
    REACTION_ADDED_EVENT = 220_202, "Reaction Added Event", None, "fas fa-heart"
    REACTION_REMOVED_EVENT = 220_203, "Reaction Removed Event", None, "fas fa-heart"
    STAR = 220_300, "Star", None, "fas fa-star"
    STAR_EVENT = 220_301, "Star Event", None, "fas fa-star"
    STAR_ADDED_EVENT = 220_302, "Star Added Event", None, "fas fa-star"
    STAR_REMOVED_EVENT = 220_303, "Star Removed Event", None, "fas fa-star"
    FOLLOW = 220_400, "Follow", None, "fas fa-plus"
    FOLLOW_EVENT = 220_401, "Follow Event", None, "fas fa-plus"
    FOLLOW_ADDED_EVENT = 220_402, "Follow Added Event", None, "fas fa-plus"
    FOLLOW_REMOVED_EVENT = 220_403, "Follow Removed Event", None, "fas fa-plus"
    NOTIFICATION = 220_500, "Notification", None, "fas fa-bell"
    NOTIFICATION_EVENT = 220_501, "Notification Event", None, "fas fa-bell"
    NOTIFICATION_SENT_EVENT = 220_502, "Notification Sent Event", None, "fas fa-bell"
    NOTIFICATION_RESCINDED_EVENT = 220_503, "Notification Rescinded Event", None, "fas fa-bell"
    NOTIFICATION_READ_EVENT = 220_504, "Notification Read Event", None, "fas fa-bell"
    NOTIFICATION_DISMISSED_EVENT = 220_505, "Notification Dismissed Event", None, "fas fa-bell"
    NOTIFICATION_EXPIRED_EVENT = 220_506, "Notification Expired Event", None, "fas fa-bell"
    # FEED, FEED_ITEM, ...
    # POLL, VOTE, REVIEW, RATING, RANK, ...
    # ACHIEVEMENT, BADGE, WISHLIST/WATCHLIST, ...

    # finance [240_000-260_000]
    # WALLET, BALANCE, BUDGET, TRANSFER, CREDIT, ...
    # TIER, SUBSCRIPTION, PRODUCT, PRICE, ...
    # ORDER, INVOICE, DISCOUNT, DISPUTE, REFUND, ...

    # scene [500_000-520_000]
    WINDOW = 500_000, "Window", None, "fas fa-galaxy"
    SCENE = 500_100, "Scene", "Scene of an Application", "fas fa-masks-theater"
    SCENE_EVENT = 500_101, "Scene Event", None, "fas fa-masks-theater"
    LAYER = 500_200, "Layer", "Layer of a Scene", "fas fa-layer-group"
    VARIANT = 500_300, "Variant", "Variant of a Scene", "fas fa-shapes"
    # VIEWPORT, OVERLAY, WIDGET,
    # FORM, MENU, ...

    # view [520_000-540_000]
    # container views
    VIEW = 520_000, "View", "View in a Scene", "fas fa-eye"
    VIEW_EVENT = 520_001, "View Event", None, "fas fa-eye"
    VIEW_ENTERED_EVENT = 520_002, "View Entered Event", None, "fas fa-eye"
    VIEW_EXITED_EVENT = 520_003, "View Exited Event", None, "fas fa-eye"
    CONTAINER_VIEW = 520_100, "Container View", None, "fas fa-table"
    FRAME_VIEW = 520_200, "Frame View", "Fixed Container", "fas fa-frame"
    LABEL_VIEW = 520_300, "Label View", "Label Container", "fas fa-font-case"
    SPLIT_VIEW = 520_400, "Split View", "Split Container", "fas fa-columns"
    # SLOT_DEFINITION_VIEW, SLOT_VIEW, ...
    # FORM_VIEW, MENU_VIEW, ...
    # TAB_VIEW, ...
    # DRAWER_VIEW, SPLIT_DRAWER_VIEW, GRID/GRID_ELEMENT_VIEW, ...
    # content views
    CONTENT_VIEW = 525_000, "Content View", None, "fas fa-text"
    TEXT_VIEW = 525_100, "Text View", "Text", "fas fa-text"
    # CODE_VIEW, ICON_VIEW, IMAGE_VIEW, AUDIO_VIEW, VIDEO_VIEW, DOCUMENT_VIEW, ...
    # input views
    INPUT_VIEW = 530_000, "Input View", None, "fas fa-hashtag"
    NUMBER_INPUT_VIEW = 530_100, "Number Input View", "Number Input", "fas fa-hashtag"
    SLIDER_INPUT_VIEW = 530_200, "Slider Input View", "Slider Input", "fas fa-slider"
    # STRING_INPUT_VIEW, TOGGLE_INPUT_VIEW, PICKER_INPUT_VIEW, COLOR_INPUT_VIEW, ...
    # ICON_INPUT_VIEW, FILE_INPUT_VIEW, DATETIME_INPUT_VIEW, DURATION_INPUT_VIEW, ...
    # NOTE :Architecture: node and internal views should probably be defined in user space?
    # node/internal views
    INTERNAL_VIEW = 535_000, "Internal View", None, "fas fa-eye"
    # WIZARD_VIEW, ...

    # canvas [540_000-560_000]
    CANVAS = 540_000, "Canvas", None, "fas fa-canvas"
    SHAPE = 540_100, "Shape", None, "fas fa-shapes"
    LINE_SHAPE = 540_200, "Line Shape", None, "fas fa-line"
    ARROW_SHAPE = 540_300, "Arrow Shape", None, "fas fa-arrow-right"
    ANNOTATION_SHAPE = 540_400, "Annotation Shape", None, "fas fa-comment"
    # VECTOR/POINT, VECTOR_NETWORK, ...
    # BITMAP, ...

    # interaction [560_000-580_000]
    INPUT_EVENT = 560_000, "Input Event", None, "fas fa-mouse-pointer"
    # pointer events
    POINTER_EVENT = 560_100, "Pointer Event", None, "fas fa-mouse-pointer"
    POINTER_DOWN_EVENT = 560_101, "Pointer Down Event", None, "fas fa-mouse-pointer"
    POINTER_UP_EVENT = 560_102, "Pointer Up Event", None, "fas fa-mouse-pointer"
    POINTER_MOVE_EVENT = 560_103, "Pointer Move Event", None, "fas fa-mouse-pointer"
    POINTER_ENTER_EVENT = 560_104, "Pointer Enter Event", None, "fas fa-mouse-pointer"
    POINTER_OVER_EVENT = 560_105, "Pointer Over Event", None, "fas fa-mouse-pointer"
    POINTER_LEAVE_EVENT = 560_106, "Pointer Leave Event", None, "fas fa-mouse-pointer"
    POINTER_LONG_PRESS_EVENT = 560_107, "Long Press Event", None, "fas fa-mouse-pointer"
    # mouse events
    MOUSE_EVENT = 560_200, "Mouse Event", None, "fas fa-mouse-pointer"
    CLICK_EVENT = 560_201, "Click Event", None, "fas fa-mouse-pointer"
    SINGLE_CLICK_EVENT = 560_202, "Single Click Event", None, "fas fa-mouse-pointer"
    DOUBLE_CLICK_EVENT = 560_203, "Double Click Event", None, "fas fa-mouse-pointer"
    TRIPLE_CLICK_EVENT = 560_204, "Triple Click Event", None, "fas fa-mouse-pointer"
    WHEEL_EVENT = 560_205, "Wheel Event", None, "fas fa-mouse-pointer"
    # keyboard events
    KEYBOARD_EVENT = 560_300, "Key Event", None, "fas fa-keyboard"
    KEY_DOWN_EVENT = 560_301, "Key Down Event", None, "fas fa-keyboard"
    KEY_UP_EVENT = 560_302, "Key Up Event", None, "fas fa-keyboard"
    KEY_PRESS_EVENT = 560_303, "Key Press Event", None, "fas fa-keyboard"
    # drag events
    DRAG_EVENT = 560_400, "Drag Event", None, "fas fa-arrows-up-down-left-right"
    DRAG_START_EVENT = 560_401, "Drag Start Event", None, "fas fa-arrows-up-down-left-right"
    DRAG_END_EVENT = 560_402, "Drag End Event", None, "fas fa-arrows-up-down-left-right"
    DRAG_OVER_EVENT = 560_403, "Drag Over Event", None, "fas fa-arrows-up-down-left-right"
    DRAG_ENTER_EVENT = 560_404, "Drag Enter Event", None, "fas fa-arrows-up-down-left-right"
    DRAG_LEAVE_EVENT = 560_405, "Drag Leave Event", None, "fas fa-arrows-up-down-left-right"
    DROP_EVENT = 560_406, "Drop Event", None, "fas fa-arrows-up-down-left-right"
    # clipboard events
    CLIPBOARD_EVENT = 560_500, "Clipboard Event", None, "fas fa-clipboard"
    COPY_EVENT = 560_501, "Copy Event", None, "fas fa-clipboard"
    CUT_EVENT = 560_502, "Cut Event", None, "fas fa-clipboard"
    PASTE_EVENT = 560_503, "Paste Event", None, "fas fa-clipboard"
    # focus events
    FOCUS_EVENT = 560_600, "Focus Event", None, "fas fa-keyboard"
    FOCUS_IN_EVENT = 560_601, "Focus In Event", None, "fas fa-keyboard"
    FOCUS_OUT_EVENT = 560_602, "Focus Out Event", None, "fas fa-keyboard"
    # command
    # COMMAND,  MODE, TOOL, SHORTCUT/KEYBINDING, ...
    # GESTURE, ...
    # CLIPBOARD, ...
    # CAMERA, SPEAKER, MICROPHONE, ...
    # AUDIO, AUDIO_PLAYER, VIDEO, VIDEO_PLAYER, ...

    # animation [580_000-600_000]
    # ANIMATION, TRACK, KEYFRAME, ...

    # style [600_000-620_000]
    THEME = 600_000, "Theme", None, "fas fa-palette"
    PALETTE = 600_100, "Palette", None, "fas fa-palette"
    STYLE = 600_200, "Style", None, "fas fa-palette"
    COLOR_STYLE = 600_300, "Color Style", None, "fas fa-palette"
    FILL_STYLE = 600_400, "Fill Style", None, "fas fa-fill"
    FONT_STYLE = 600_500, "Font Style", None, "fas fa-text"
    BORDER_STYLE = 600_600, "Border Style", None, "fas fa-border-outer"
    SHADOW_STYLE = 600_700, "Shadow Style", None, "fas fa-eclipse"
    GRADIENT_STYLE = 600_800, "Gradient Style", None, "fas fa-gradient"
    TRANSITION_STYLE = 600_900, "Transition Style", None, "fas fa-bezier-curve"
    EFFECT_STYLE = 600_1000, "Effect Style", None, "fas fa-sparkle"
    STROKE_STYLE = 600_1100, "Stroke Style", None, "fas fa-stroke"
    # BRUSH_STYLE, ...
    # SHADER, MATERIAL, ...


@builtin_enum(EnumType.UNIVERSE_CATEGORY)
class UniverseCategory(Enum):
    """How the system is organized."""

    META = 1, "Meta", "Information about the system"
    UNIVERSE = 20_000, "Universe", "Global computational universe"
    SPACE = 40_000, "Space", "Spacetime organization"
    ACCESS = 60_000, "Access", "Access control"
    DATA = 80_000, "Data", "Schemas, files and streams"
    LOGIC = 100_000, "Logic", "Logic, workflows and operations"
    INTELLIGENCE = 120_000, "Intelligence", "Artificial intelligence"
    INFRASTRUCTURE = 140_000, "Infrastructure", "Infrastructure management"
    DEPLOYMENT = 160_000, "Deployment", "Deployment of the system"
    OBSERVABILITY = 180_000, "Observability", "Analytics of the system"
    OPTIMIZATION = 200_000, "Optimization", "Improve the system"
    SOCIAL = 220_000, "Social", "Social interactions"
    FINANCE = 240_000, "Finance", "Financial operations"
    SCENE = 500_000, "Scene", "Scene construction"
    VIEW = 520_000, "View", "View building"
    CANVAS = 540_000, "Canvas", "Drawing and painting"
    INTERACTION = 560_000, "Interaction", "Interaction design"
    ANIMATION = 580_000, "Animation", "Animate views"
    STYLE = 600_000, "Style", "Style user interfaces"


ENUM_TYPES: tuple[EnumType, ...] = tuple(EnumType)
NODE_TYPES: tuple[NodeType, ...] = tuple(NodeType)
STRUCT_TYPES: tuple[StructType, ...] = tuple(StructType)
TRAIT_TYPES: tuple[TraitType, ...] = tuple(TraitType)


@builtin_enum(EnumType.PROPERTY_TYPE)
class PropertyType(Enum):
    MEMBER = 1, "Member", None, None
    CONSTANT = 2, "Constant", None, None
    # COMPUTED?
    INPUT = 10, "Input", None, None
    OUTPUT = 11, "Output", None, None


@builtin_enum(EnumType.STORE_KEY)
class StoreKey(Enum):
    """The role of a Store (scope + domain + tier)."""

    GLOBAL_ENTITY_PRIMARY = 1110
    SPATIAL_ENTITY_PRIMARY = 1120
    # SPATIAL_ENTITY_SEARCH, SPATIAL_ENTITY_BACKUP, ...
    SPATIAL_EVENT_PRIMARY = 2110
    # SPATIAL_EVENT_SEARCH, SPATIAL_EVENT_AGGREGATE, ...


@builtin_enum(EnumType.STORE_SCOPE)
class StoreScope(Enum):
    """The scope of a Store."""

    GLOBAL = 1000
    SPATIAL = 2000


@builtin_enum(EnumType.STORE_DOMAIN)
class StoreDomain(Enum):
    """The domain of a Store."""

    ENTITY = 100
    EVENT = 500


@builtin_enum(EnumType.STORE_TIER)
class StoreTier(Enum):
    """The tier of a Store."""

    PRIMARY = 10
    # PRIMARY_FAST, PRIMARY_RELATIONAL, ...
    # SEARCH = 20
    # AGGREGATE = 30


@builtin_enum(EnumType.STORE_IMPLEMENTATION)
class StoreImplementation(Enum):
    MEMORY = 1
    POSTGRES = 10
    # CASSANDRA, ELASTICSEARCH, REDIS, ...


@builtin_enum(EnumType.RUNTIME_LANGUAGE)
class RuntimeLanguage(Enum):
    PYTHON = 1
    JAVASCRIPT = 2
    # RUST, JAVA, SWIFT, ...


@builtin_enum(EnumType.PLATFORM_TYPE)
class PlatformType(Enum):
    SYSTEM = 1
    RUNTIME = 2
    WEB = 10
    # MOBILE, DESKTOP, ...
    # EMAIL?


@builtin_enum(EnumType.OPERATING_SYSTEM)
class OperatingSystem(Enum):
    # desktop
    LINUX = 1, "Linux", "Linux operating system", "fab fa-linux"
    WINDOWS = 2, "Windows", "Microsoft Windows", "fab fa-windows"
    MACOS = 3, "macOS", "Apple macOS", "fab fa-apple"
    # mobile
    ANDROID = 50, "Android", "Google Android", "fab fa-android"
    IOS = 51, "iOS", "Apple iOS", "fab fa-apple"
    # WATCHOS, TVOS, IPADOS, ...


@builtin_enum(EnumType.ENVIRONMENT_TYPE)
class EnvironmentType(Enum):
    SYSTEM = 1, "System", "Managed by the system", "fas fa-cog"
    DEVELOPMENT = 3, "Development", "Active in development", "fas fa-flask"
    TEST = 5, "Test", "Active in test", "fas fa-flask"
    STAGING = 7, "Staging", "Active in staging", "fas fa-globe"
    PRODUCTION = 10, "Production", "Active in production", "fas fa-globe"


@builtin_enum(EnumType.MODE_TYPE)
class ModeType(Enum):
    EDIT = 1
    DEBUG = 2
    INSPECT = 3
    PREVIEW = 4
    USE = 5


@builtin_enum(EnumType.TOOL_TYPE)
class ToolType(Enum):
    SELECT = 1
    DRAG = 2
    INSPECT = 10
    ANNOTATE = 11
    # ...


@builtin_enum(EnumType.CLOUD)
class Cloud(Enum):
    """The cloud provider."""

    # own
    ...
    PRIVATE = 1
    # big general
    AWS = 10
    AZURE = 11
    GCP = 12
    HETZNER = 20

    @property
    def slug(self) -> str:
        return self.name.lower().replace("_", "-")


@builtin_enum(EnumType.REGION_CONTINENT)
class RegionContinent(Enum):
    """
    'Continents' of Regions.
    """

    EUROPE = 1_000, "Europe", None, "🇪🇺"
    NORTH_AMERICA = 2_000, "North America", None, "🇺🇸"
    SOUTH_AMERICA = 3_000, "South America", None, "🇧🇷"
    MIDDLE_EAST = 4_000, "Middle East", None, "🇸🇦"
    AFRICA = 5_000, "Africa", None, "🇿🇦"
    ASIA = 6_000, "Asia", None, "🇮🇳"
    AUSTRALIA = 7_000, "Australia", None, "🇦🇺"
    PRIVATE = 9_000

    @property
    def slug(self) -> str:
        return REGION_CONTINENT_SLUGS[self]

    @staticmethod
    def get_by_slug(slug: str) -> "RegionContinent":
        return REGION_CONTINENT_BY_SLUG[slug]


REGION_CONTINENT_SLUGS: dict[RegionContinent, str] = {
    RegionContinent.EUROPE: "eu",
    RegionContinent.NORTH_AMERICA: "na",
    RegionContinent.SOUTH_AMERICA: "sa",
    RegionContinent.MIDDLE_EAST: "me",
    RegionContinent.AFRICA: "af",
    RegionContinent.ASIA: "as",
    RegionContinent.AUSTRALIA: "au",
}
REGION_CONTINENT_BY_SLUG = {v: k for k, v in REGION_CONTINENT_SLUGS.items()}


@builtin_enum(EnumType.REGION_AREA)
class RegionArea(Enum):
    """
    A larger Area of Regions within a Continent.
    """

    EUROPE_CENTRAL = 1_000, None, None, "🇪🇺"
    NORTH_AMERICA_EAST = 2_000, None, None, "🇺🇸"
    NORTH_AMERICA_WEST = 2_200, None, None, "🇺🇸"
    SOUTH_AMERICA_EAST = 3_000, None, None, "🇧🇷"
    MIDDLE_EAST_CENTRAL = 4_000, None, None, "🇸🇦"
    MIDDLE_EAST_WEST = 4_200, None, None, "🇸🇦"
    AFRICA_SOUTH = 5_000, None, None, "🇿🇦"
    ASIA_WEST = 6_000
    ASIA_SOUTH = 6_200
    ASIA_EAST = 6_400
    AUSTRALIA_SOUTH = 7_000, None, None, "🇦🇺"

    @property
    def continent(self) -> RegionContinent:
        return RegionContinent((self.id // 1_000) * 1_000)

    @property
    def slug(self) -> str:
        return REGION_AREA_SLUGS[self]

    @staticmethod
    def get_by_slug(slug: str) -> "RegionArea":
        return REGION_AREA_BY_SLUG[slug]


REGION_AREA_SLUGS: dict[RegionArea, str] = {
    RegionArea.EUROPE_CENTRAL: "eu-central",
    RegionArea.NORTH_AMERICA_EAST: "na-east",
    RegionArea.NORTH_AMERICA_WEST: "na-west",
    RegionArea.SOUTH_AMERICA_EAST: "sa-east",
    RegionArea.MIDDLE_EAST_CENTRAL: "me-central",
    RegionArea.MIDDLE_EAST_WEST: "me-west",
    RegionArea.AFRICA_SOUTH: "af-south",
    RegionArea.ASIA_WEST: "as-west",
    RegionArea.ASIA_SOUTH: "as-south",
    RegionArea.ASIA_EAST: "as-east",
    RegionArea.AUSTRALIA_SOUTH: "au-south",
}
REGION_AREA_BY_SLUG = {v: k for k, v in REGION_AREA_SLUGS.items()}
# register_constant("REGION_AREA_BY_SLUG", REGION_AREA_BY_SLUG)


@builtin_enum(EnumType.REGION)
class Region(Enum):
    """Regions in an Area on a Continent."""

    # eu-central
    ZURICH = 1_000, None, None, "🇨🇭"
    FRANKFURT = 1_010, None, None, "🇩🇪"

    # na-east
    VIRGINIA = 2_000, None, None, "🇺🇸"
    OHIO = 2_010, None, None, "🇺🇸"

    # na-west
    OREGON = 2_200, None, None, "🇺🇸"

    # sa-east
    SAO_PAULO = 3_000, None, None, "🇧🇷"

    ...

    # af-south
    CAPE_TOWN = 5_000, None, None, "🇿🇦"

    # as-east
    MUMBAI = 6_000, None, None, "🇮🇳"

    # as-south
    SINGAPORE = 6_200, None, None, "🇸🇬"

    # as-east
    TOKYO = 6_400, None, None, "🇯🇵"

    # au-south
    SYDNEY = 7_000, None, None, "🇦🇺"

    @property
    def continent(self) -> RegionContinent:
        return RegionContinent((self.id // 1_000) * 1_000)

    @property
    def area(self) -> RegionArea:
        return RegionArea((self.id // 200) * 200)

    @property
    def slug(self) -> str:
        continent = self.continent
        return continent.slug + "-" + self.name.replace("_", "-").lower()

    @staticmethod
    def get_by_slug(slug: str) -> "Region":
        return REGION_BY_SLUG[slug]


REGION_BY_SLUG = {r.slug: r for r in Region}


@builtin_enum(EnumType.EDGE_TYPE)
class EdgeType(Enum):
    PARENT = 1
    REGULAR = 5

    @property
    def is_node_tree(self):
        return self.id <= 4

    @property
    def is_node(self):
        return self.id < 10


@builtin_enum(EnumType.CASCADE_ACTION)
class CascadeAction(Enum):
    RESTRICT = 1
    CASCADE = 2
    SET_NULL = 3
    # SET_DEFAULT, NONE, ...


@builtin_enum(EnumType.EDGE_DIRECTION)
class EdgeDirection(Enum):
    PARENT = 1
    CHILD = 2
    SIDE = 3


@builtin_enum(EnumType.PRIMITIVE_TYPE)
class PrimitiveType(Enum):
    """
    A fundamental scalar data type.
    """

    BOOLEAN = 1, "Boolean", "Yes or no", "fas fa-toggle-large-on"
    # INT8? UINTs?
    # range: -32_768 to 32_767
    INT16 = 4, "Integer", "Very small integer", "fas fa-tally"
    # range: -2_147_483_648 to 2_147_483_647
    INT32 = 5, "Integer", "Small integer", "fas fa-tally"
    # range: -9_223_372_036_854_775_808 to 9_223_372_036_854_775_807
    INT64 = 6, "Integer", "Integer number", "fas fa-tally"
    # numeric(precision, scale)
    DECIMAL = 10, "Decimal", "Decimal number", "fas fa-tally"
    # FLOAT16?
    # range: 1.175494351e-38 to 3.402823466e+38
    FLOAT32 = 16, "Float", "Small float", "fas fa-hashtag"
    # range: 2.2250738585072014e-308 to 1.7976931348623157e+308
    FLOAT64 = 17, "Float", "Floating point number", "fas fa-hashtag"
    STRING = 20, "String", "Plain text", "fas fa-font-case"
    UUID = 21, "UUID", "UUID", "fas fa-fingerprint"
    JSON = 22, "JSON", "JSON", "fas fa-brackets-curly"
    BYTES = 25, "Bytes", "Binary data", "fas fa-file-lines"
    # VECTOR?
    # time
    DATETIME = 30, "Date & Time", "Date & time", "fas fa-calendar-days"
    DATE = 31, "Date", "Date", "fas fa-calendar-days"
    TIME = 32, "Time", "Time", "fas fa-clock"
    DURATION = 33, "Duration", "Duration", "fas fa-stopwatch"

    @property
    def is_numeric(self) -> bool:
        return self.id >= 2 and self.id < 20

    @property
    def is_int(self) -> bool:
        return self.id >= 2 and self.id <= 10

    @property
    def is_float(self) -> bool:
        return self.id >= 15 and self.id < 20


PY_TYPE_BY_PRIMITIVE_TYPE: dict[PrimitiveType, type] = {
    PrimitiveType.BOOLEAN: bool,
    PrimitiveType.INT16: int,
    PrimitiveType.INT32: int,
    PrimitiveType.INT64: int,
    PrimitiveType.DECIMAL: Decimal,
    PrimitiveType.FLOAT32: float,
    PrimitiveType.FLOAT64: float,
    PrimitiveType.STRING: str,
    PrimitiveType.BYTES: bytes,
    PrimitiveType.UUID: UUID,
    PrimitiveType.DATETIME: datetime,
    PrimitiveType.DATE: date,
    PrimitiveType.TIME: time,
    PrimitiveType.DURATION: timedelta,
}
PRIMITIVE_TYPE_BY_PY_TYPE: dict[type, PrimitiveType] = {
    bool: PrimitiveType.BOOLEAN,
    int: PrimitiveType.INT64,
    Decimal: PrimitiveType.DECIMAL,
    float: PrimitiveType.FLOAT64,
    str: PrimitiveType.STRING,
    UUID: PrimitiveType.UUID,
    bytes: PrimitiveType.BYTES,
    datetime: PrimitiveType.DATETIME,
    date: PrimitiveType.DATE,
    time: PrimitiveType.TIME,
    timedelta: PrimitiveType.DURATION,
}
PRIMITIVE_PY_TYPES = tuple(PRIMITIVE_TYPE_BY_PY_TYPE.keys())


@builtin_enum(EnumType.TYPE_CARDINALITY)
class TypeCardinality(Enum):
    """The 'kind' of a Type."""

    SCALAR = 1
    LIST = 2
    # SET?
    MAP = 4
    # OPTION = 5
    # LITERAL = 6
    # UNION = 7


@builtin_enum(EnumType.SCALAR_TYPE)
class ScalarType(Enum):
    """The type of a scalar."""

    PRIMITIVE = 1
    ENUM = 2
    NODE_REFERENCE = 3
    NODE_VALUE = 4
    STRUCT = 5
    # CUSTOM_ENUM, CUSTOM_STRUCT, ...? (or are they just NodeReferences/Structs?)


@builtin_enum(EnumType.VALUE_FACTORY)
class ValueFactory(Enum):
    """The factory to use for generating values."""

    UUID = 1
    NOW = 2
    REGION = 3
    SELF = 4


@builtin_enum(EnumType.ROLE_TYPE)
class RoleType(Enum):
    SYSTEM = 1
    OWNER = 2
    ADMIN = 3
    DEVELOPER = 5
    USER = 7
    SPECTATOR = 10


@builtin_enum(EnumType.RESOURCE_STATUS)
class ResourceStatus(Enum):
    """Generalized status of a Resource in its lifecycle."""

    # pre
    PENDING = (1, "Pending", "Waiting for provisioning", "fas fa-hourglass-start")
    CREATING = (2, "Creating", "Actively provisioning", "fas fa-hourglass-start")
    RETRYING = (3, "Retrying", "Retrying provisioning", "fas fa-exclamation-triangle")

    # active states
    AVAILABLE = (10, "Available", "Operational and available", "fas fa-check-circle")
    SLEEPING = (11, "Sleeping", "Available but not running", "fas fa-moon")
    UNAVAILABLE = (15, "Unavailable", "Unavailable or not responding", "fas fa-plug-circle-xmark")
    IMPAIRED = (
        16,
        "Impaired",
        "Operational but experiencing issues",
        "fas fa-exclamation-triangle",
    )
    # terminal
    OFFLINE = (30, "Offline", "Decommissioned and unavailable", "fas fa-power-off")
    FAILED = (31, "Failed", "Failed to provision", "fas fa-exclamation-triangle")

    @property
    def is_pre(self) -> bool:
        """Whether this Resource is in the pre-provisioning state."""
        return 1 <= self.value < 10

    @property
    def is_extant(self) -> bool:
        """Whether this Resource does/should exist."""
        return 10 <= self.value <= 20

    @property
    def is_terminal(self) -> bool:
        """Whether this Resource is terminal."""
        return 30 <= self.value <= 40


@builtin_enum(EnumType.CLIENT_TYPE)
class ClientType(Enum):
    # user
    WEB = 1
    BROWSER_PLUGIN = 2
    DESKTOP = 3
    MOBILE = 4
    # system
    MACHINE = 10


@builtin_enum(EnumType.TENANCY)
class Tenancy(Enum):
    DEDICATED = 1
    SHARED = 2
