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
    # meta [1-10_000]
    ENUM_TYPE = 1
    NODE_TYPE = 2
    STRUCT_TYPE = 3
    TRAIT_TYPE = 4
    NODE_DEFINITION_TYPE = 10
    OBJECT_DEFINITION_TYPE = 11
    STRUCT_DEFINITION_TYPE = 12
    PROPERTY_REFERENCE_TYPE = 13
    MATERIALIZATION = 14
    STORE_TYPE = 21
    STORE_IMPLEMENTATION = 22
    PLATFORM_TYPE = 30
    RUNTIME_LANGUAGE = 31
    OPERATING_SYSTEM = 40
    EDIT_TYPE = 50
    EDIT_OPERATION = 51
    CHANGE_STATUS = 52
    CHANGE_DEBOUNCE = 53
    PRIMITIVE_TYPE = 60
    TYPE_CARDINALITY = 61
    SCALAR_TYPE = 62
    VALUE_FACTORY = 63
    STRING_FORMAT = 64
    NUMBER_FORMAT = 65
    CUSTOM_PROPERTY_TYPE = 66
    EDGE_TYPE = 67
    EDGE_DIRECTION = 68
    CASCADE_ACTION = 69
    RESOURCE_STATUS = 1100
    SNAPSHOT_TYPE = 1300

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

    # space [10_000-20_000]
    SPACE_STATUS = 10_001
    USER_STATUS = 10_200
    ORGANIZATION_STATUS = 10_500
    CLIENT_TYPE = 10_700

    # access [20_000-30_000]
    JOINABLE_PERMISSION = 20_000
    MEMBERSHIP_PERMISSION = 20_001
    ROLE_TYPE = 20_200
    PERMISSION_TYPE = 20_300
    SANCTION_TYPE = 20_400
    ENTITLEMENT_TYPE = 20_500
    # ...

    # folder [30_000-40_000]
    FOLDER_TYPE = 30_000

    # history [40_000-50_000]
    # ...

    # entity [50_000-60_000]
    # ...

    # data [60_000-70_000]
    FILE_RETENTION_MODE = 60_000
    FILE_SOURCE = 60_001
    FILE_TYPE = 60_002
    FILE_FORMAT = 60_003
    TEXT_SPAN_TYPE = 60_004
    ICON_TYPE = 60_005
    # ...

    # logic [70_000-80_000]
    TRIGGER_TYPE = 70_400
    SCHEDULE_FREQUENCY = 70_500
    DAY_OF_WEEK = 70_501
    MONTH = 70_502
    TIMER_TYPE = 70_503
    ACTION_CARDINALITY = 70_200
    CURSOR_STATUS = 70_600
    # ...

    # test [80_000-90_000]
    # ...

    # runtime [90_000-100_000]
    RUN_STATUS = 90_000
    INTERRUPTION_TYPE = 90_200
    INTERRUPTION_STATUS = 90_201
    INTERRUPTION_RESPONSE = 90_202
    LOG_LEVEL = 90_300
    # ...

    # deployment [100_000-110_000]
    ENVIRONMENT_TYPE = 100_000

    # product [110_000-120_000]
    # ...

    # social [120_000-130_000]
    THREAD_STATUS = 120_000
    NOTIFICATION_STATUS = 120_500

    # finance [130_000-140_000]
    # ...

    # locale [140_000-150_000]
    # ...

    # internet [150_000-160_000]
    # ...

    # infra [160_000-170_000]
    CLOUD = 160_000
    REGION = 160_001
    REGION_AREA = 160_002
    REGION_CONTINENT = 160_003
    TENANCY = 160_004
    DATABASE_TYPE = 160_005
    MACHINE_TYPE = 160_100
    # SEARCH_TYPE, WAREHOUSE_TYPE, ...
    # ...

    # intelligence [170_000-180_000]
    MODEL_DEVELOPER = 170_000
    MODEL_PROVIDER = 170_001
    # ...

    # world [180_000-190_000]
    # ...

    # scene [190_000-200_000]
    WINDOW_TYPE = 190_000
    LAYER_TYPE = 190_200
    VARIANT_TYPE = 190_300
    VARIANT_STATE_TYPE = 190_301

    # interaction [200_000-210_000]
    MODE_TYPE = 200_000
    TOOL_TYPE = 200_001
    MOUSE_BUTTON = 200_010

    # container views [210_000-220_000]
    # ...

    # content views [220_000-230_000]
    # ...

    # input views [230_000-240_000]
    # ...

    # node/internal views [240_000-250_000]
    # ...

    # canvas [250_000-260_000]
    CANVAS_TYPE = 250_000
    ARROW_HEAD_TYPE = 250_300

    # animation [260_000-270_000]
    # ...

    # style [270_000-280_000]
    COLOR_TYPE = 270_000
    COLOR_SHADE = 270_001
    COLOR_HUE = 270_002
    COLOR_INTENT = 270_003
    FILL_TYPE = 270_100
    FILL_POSITION = 270_101
    FILL_SIZE = 270_102
    FONT_TYPE = 270_200
    FONT_WEIGHT = 270_201
    FONT_SIZE = 270_202
    TEXT_ALIGN = 270_203
    TEXT_DECORATION = 270_204
    TEXT_TRANSFORM = 270_205
    BORDER_TYPE = 270_206
    SHADOW_TYPE = 270_207
    SHADOW_POSITION = 270_208
    GRADIENT_TYPE = 270_209
    TRANSITION_TYPE = 270_210
    SPRING_TYPE = 270_211
    EFFECT_TYPE = 270_212
    STROKE_TYPE = 270_213
    POSITION_TYPE = 270_214
    LENGTH_UNIT = 270_215
    LAYOUT = 270_216
    DISTRIBUTE = 270_217
    ALIGN = 270_218
    DIRECTION = 270_219
    OVERFLOW = 270_220
    DIMENSION_TYPE = 270_221
    REPEAT_TYPE = 270_222
    TEXT_SPLIT_TYPE = 270_223
    OFFSCREEN_BEHAVIOR = 270_224
    EASING = 270_225


builtin_enum(EnumType.ENUM_TYPE)(EnumType)


@builtin_enum(EnumType.STRUCT_TYPE)
class StructType(Enum):
    # meta [1-10_000]
    NODE_REFERENCE = 100
    PROPERTY_REFERENCE = 101
    PROPERTY_DEFINITION = 102
    TRAIT_DEFINITION = 103
    NODE_DEFINITION = 104
    STRUCT_DEFINITION = 105
    ENUM_DEFINITION = 106
    OPTION_DEFINITION = 107
    PERMISSION_DEFINITION = 108
    CONSTANT_DEFINITION = 109
    # FUNCTION_DEFINITION, ACTION_DEFINITION, ...?
    NODE_DEFINITION_REFERENCE = 150
    OBJECT_DEFINITION_REFERENCE = 151
    STRUCT_DEFINITION_REFERENCE = 152
    CUSTOM_STRUCT = 153
    EDIT = 200
    CHANGE = 201
    CHANGE_RESULT = 202
    ORIGIN = 203
    EXPRESSION = 300
    FUNCTION = 301
    JOIN = 302
    AGGREGATION = 303
    CONDITION = 304
    SORT = 305
    SELECT = 306
    QUERY = 307
    QUERY_RESULT = 308
    QUERY_RESULT_GROUP = 309
    QUERY_UPDATE = 310
    HISTOGRAM = 311
    SELECTION = 312
    VALUE = 400
    TYPE = 401
    NUMBER_CONSTRAINT = 410
    STRING_CONSTRAINT = 411
    COLLECTION_CONSTRAINT = 412
    NODE_CONSTRAINT = 413
    # geometry
    VECTOR2 = 500, None, None, "fas fa-vector-square"
    VECTOR3 = 501, None, None, "fas fa-vector-square"
    VECTOR4 = 502, None, None, "fas fa-vector-square"
    VECTOR2I = 503, None, None, "fas fa-vector-square"
    VECTOR3I = 504, None, None, "fas fa-vector-square"
    VECTOR4I = 505, None, None, "fas fa-vector-square"

    # space [10_000-20_000]
    # ...

    # access [20_000-30_000]
    # ...

    # folder [30_000-40_000]
    # ...

    # spacetime [40_000-50_000]
    # ...

    # entity [50_000-60_000]
    # ...

    # data [60_000-70_000]
    TEXT = 60_020, None, None, "fas fa-text"
    TEXT_SPAN = 60_021, None, None, "fas fa-text"
    ICON = 60_031
    # ...

    # logic [70_000-80_000]
    SCHEDULE = 70_001
    # ...

    # test [80_000-90_000]
    # ...

    # runtime [90_000-100_000]
    # ...

    # deployment [100_000-110_000]
    # ...

    # product [110_000-120_000]
    # ...

    # social [120_000-130_000]
    # ...

    # finance [130_000-140_000]
    # ...

    # locale [140_000-150_000]
    # ...

    # internet [150_000-160_000]
    # ...

    # infra [160_000-170_000]
    DATABASE_INFO = 160_001
    GALAXY_INFO = 160_101

    # intelligence [170_000-180_000]
    # ...

    # world [180_000-190_000]
    # ...

    # scene [190_000-200_000]
    # ...

    # interaction [200_000-210_000]
    # ...

    # container views [210_000-220_000]
    # ...

    # content views [220_000-230_000]
    # ...

    # input views [230_000-240_000]
    # ...

    # node/internal views [240_000-250_000]
    # ...

    # canvas [250_000-260_000]
    LINE = 250_200, "Line", None, "fas fa-line"
    ARROW = 250_300, "Arrow", None, "fas fa-arrow-right"

    # animation [260_000-270_000]
    # ...

    # style [270_000-280_000]
    LENGTH = 270_018, "Length", None, "fas fa-ruler"
    POSITION = 270_020, "Position", None, "fas fa-location-crosshair"
    DIMENSION = 270_022, "Dimension", None, "fas fa-ruler"
    GRID = 270_026, "Grid", None, "fas fa-grid-2"
    GRID_SPAN = 270_028, "Grid Span", None, "fas fa-grid-2"
    INSETS = 270_030, "Insets", None, "fas fa-corner"
    CORNERS = 270_032, "Corners", None, "fas fa-corner"
    AXIS2 = 270_034, "Axis2", None, "fas fa-vector-square"
    AXIS3 = 270_036, "Axis3", None, "fas fa-vector-square"
    COLOR = 270_300, "Color", None, "fas fa-palette"
    FILL = 270_400, "Fill", None, "fas fa-fill"
    FONT = 270_500, "Font", None, "fas fa-text"
    BORDER = 270_600, "Border", None, "fas fa-border-outer"
    SHADOW = 270_700, "Shadow", None, "fas fa-eclipse"
    GRADIENT = 270_800, "Gradient", None, "fas fa-gradient"
    GRADIENT_STOP = 270_801, "Gradient Stop", None, "fas fa-gradient"
    TRANSITION = 270_900, "Transition", None, "fas fa-bezier-curve"
    EFFECT = 270_1000, "Effect", None, "fas fa-sparkle"
    STROKE = 270_1100, "Stroke", None, "fas fa-stroke"
    STROKE_CAP = 270_1101, "Stroke Cap", None, "fas fa-stroke"
    STROKE_PATH = 270_1102, "Stroke Path", None, "fas fa-stroke"
    STROKE_POINT = 270_1103, "Stroke Point", None, "fas fa-stroke"


@builtin_enum(EnumType.TRAIT_TYPE)
class TraitType(Enum):
    # meta [1-10_000]
    # where
    GLOBAL = 1, "Global", "Is global", "fas fa-globe"
    SPATIAL = 2, "Spatial", "Is in a Space", "fas fa-solar-system"
    # LOCAL?
    # storage
    # RELATIONAL/OLTP, INDEXED; ANALYTIC, ...?

    # behavior
    ORDERED = 100, "Ordered", "Is ordered", "fas fa-sort"
    ARCHIVABLE = 101, "Archivable", "Can be archived", "fas fa-box-archive"
    DELETABLE = 102, "Deletable", "Can be deleted", "fas fa-trash"
    CUSTOMIZABLE = (
        110,
        "Customizable",
        "Can be customized with custom Properties",
        "fas fa-paint-roller",
    )
    EXTENSIBLE = 111, "Extensible", "Can be extended by custom Nodes", "fas fa-expand"

    # space [10_000-20_000]
    # ...

    # access [20_000-30_000]
    OWNABLE = 20_000, "Ownable", "Is ownable", "fas fa-user"
    OWNER = 20_001, "Owner", "Is an Owner", "fas fa-user"
    JOINABLE = 20_002, "Joinable", "Is joinable", "fas fa-users"
    SUBJECT = 20_003, "Subject", "Is a Subject", "fas fa-user"

    # folder [30_000-40_000]
    TAGGABLE = 30_000, "Taggable", "Can be tagged", "fas fa-tag"

    # spacetime [40_000-50_000]
    # ...

    # entity [50_000-60_000]
    # ...

    # data [60_000-70_000]
    # ...

    # logic [70_000-80_000]
    RUNNABLE = 70_001, "Runnable", "Can be run", "fas fa-play"
    SCRIPTABLE = 70_002, "Scriptable", "Can be scripted", "fas fa-code"
    SOURCEABLE = 70_003, "Sourcable", "Can be defined in a Script", "fas fa-code"

    # test [80_000-90_000]
    # ...

    # runtime [90_000-100_000]
    # ...

    # deployment [100_000-110_000]
    # ...

    # product [110_000-120_000]
    # ...

    # social [120_000-130_000]
    STARABLE = 120_030, "Starable", "Can be starred", "fas fa-star"
    REACTABLE = 120_032, "Reactable", "Can be reacted to", "fas fa-heart"
    FOLLOWABLE = 120_034, "Followable", "Can be followed", "fas fa-plus"
    # RATEABLE, VOTABLE, ...
    # ASSIGNABLE, MESSAGEABLE, CLOSABLE, LOCKABLE, ...

    # finance [130_000-140_000]
    # ...

    # locale [140_000-150_000]
    # ...

    # internet [150_000-160_000]
    # ...

    # infra [160_000-170_000]
    # ...

    # intelligence [170_000-180_000]
    # ...

    # world [180_000-190_000]
    # ...

    # scene [190_000-200_000]
    # ...

    # interaction [200_000-210_000]
    # ...

    # container views [210_000-220_000]
    # ...

    # content views [220_000-230_000]
    # ...

    # input views [230_000-240_000]
    # ...

    # node/internal views [240_000-250_000]
    # ...

    # canvas [250_000-260_000]
    # ...

    # animation [260_000-270_000]
    # ...

    # style [270_000-280_000]
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
    CUSTOM_PROPERTY = 105, "Custom Property", None, "fas fa-triangle"
    CUSTOM_OPTION = 106, "Custom Option", None, "fas fa-circle"
    # entity
    RECORD = 1000, "Record", "Custom Entity", "fas fa-database"
    RESOURCE = 1100, "Resource", "External asset outside of Destack", "fas fa-dot"
    METRIC = 1200, "Metric", None, "fas fa-gauge"
    SNAPSHOT = 1300, "Snapshot", "Point in Space-time", "fas fa-save"
    # event
    SIGNAL = 2000, "Signal", "Custom Event", "fas fa-signal"
    EDIT_EVENT = 2001, "Edit Event", None, "fas fa-file-lines"
    CHANGE_EVENT = 2002, "Change Event", None, "fas fa-file-lines"
    QUERY_EVENT = 2003, "Query Event", None, "fas fa-file-lines"
    MEASUREMENT_EVENT = 2004, "Measurement", None, "fas fa-gauge"

    # space [10_000-20_000]
    SPACE = 10_000, "Space", "Universal Space", "https://heydestack.com/favicon.ico"
    HANDLE = 10_100, "Handle", "Unique @handle", "fas fa-at"
    USER = 10_200, "User", None, "fas fa-user"
    FRIENDSHIP = 10_300, "Friendship", "Friendship between two Users", "fas fa-user-friends"
    FRIENDSHIP_INVITE = (
        10_400,
        "Friendship Invite",
        "Invite to be friends with another User",
        "fas fa-user-plus",
    )
    FRIENDSHIP_INVITE_EVENT = 10_401, "Friendship Invite Event", None, "fas fa-user-plus"
    FRIENDSHIP_INVITE_SENT_EVENT = 10_402, "Friendship Invite Sent Event", None, "fas fa-user-plus"
    FRIENDSHIP_INVITE_RESCINDED_EVENT = (
        10_403,
        "Friendship Invite Rescinded Event",
        None,
        "fas fa-user-plus",
    )
    FRIENDSHIP_INVITE_ACCEPTED_EVENT = (
        10_404,
        "Friendship Invite Accepted Event",
        None,
        "fas fa-user-plus",
    )
    FRIENDSHIP_INVITE_REJECTED_EVENT = (
        10_405,
        "Friendship Invite Rejected Event",
        None,
        "fas fa-user-plus",
    )
    ORGANIZATION = 10_500, "Organization", None, "fas fa-building"
    TEAM = 10_600, "Team", "Team in an Organization", "fas fa-users"
    # CREDENTIAL, ACCOUNT, PROFILE, ...
    CLIENT = 10_700, "Client", None, "fas fa-desktop"

    # access [20_000-30_000]
    MEMBERSHIP = 20_000, "Membership", "Membership in a Space/Folder", "fas fa-user-group"
    MEMBERSHIP_EVENT = 20_001, "Membership Event", None, "fas fa-user-group"
    MEMBERSHIP_JOINED_EVENT = 20_002, "Membership Join Event", None, "fas fa-user-group"
    MEMBERSHIP_LEFT_EVENT = 20_003, "Membership Leave Event", None, "fas fa-user-group"
    INVITE = 20_100, "Invite", "Invite to a Space/Folder", "fas fa-user-plus"
    INVITE_EVENT = 20_101, "Invite Event", None, "fas fa-user-plus"
    INVITE_SENT_EVENT = 20_102, "Invite Sent Event", None, "fas fa-user-plus"
    INVITE_RESCINDED_EVENT = 20_103, "Invite Rescinded Event", None, "fas fa-user-plus"
    INVITE_ACCEPTED_EVENT = 20_104, "Invite Accepted Event", None, "fas fa-user-plus"
    INVITE_REJECTED_EVENT = 20_105, "Invite Rejected Event", None, "fas fa-user-plus"
    ROLE = 20_200, "Role", "Role in something", "fas fa-user-tag"
    ROLE_EVENT = 20_201, "Role Event", None, "fas fa-user-tag"
    ROLE_ASSIGNED_EVENT = 20_202, "Role Assigned Event", None, "fas fa-user-tag"
    ROLE_UNASSIGNED_EVENT = 20_203, "Role Unassigned Event", None, "fas fa-user-tag"
    PERMISSION = 20_300, "Permission", "Permission for something", "fas fa-user-shield"
    SANCTION = 20_400, "Sanction", "Temporary or permanent restriction", "fas fa-user-minus"
    SANCTION_EVENT = 20_401, "Sanction Event", None, "fas fa-user-minus"
    SANCTION_REQUESTED_EVENT = 20_402, "Sanction Requested Event", None, "fas fa-user-minus"
    SANCTION_GRANTED_EVENT = 20_403, "Sanction Granted Event", None, "fas fa-user-minus"
    SANCTION_REVOKED_EVENT = 20_404, "Sanction Revoked Event", None, "fas fa-user-minus"
    SANCTION_EXPIRED_EVENT = 20_405, "Sanction Expired Event", None, "fas fa-user-minus"
    ENTITLEMENT = 20_500, "Entitlement", "Temporary or permanent grant", "fas fa-user-check"
    ENTITLEMENT_EVENT = 20_501, "Entitlement Event", None, "fas fa-user-check"
    ENTITLEMENT_REQUESTED_EVENT = 20_502, "Entitlement Requested Event", None, "fas fa-user-check"
    ENTITLEMENT_GRANTED_EVENT = 20_503, "Entitlement Granted Event", None, "fas fa-user-check"
    ENTITLEMENT_REVOKED_EVENT = 20_504, "Entitlement Revoked Event", None, "fas fa-user-check"
    ENTITLEMENT_EXPIRED_EVENT = 20_505, "Entitlement Expired Event", None, "fas fa-user-check"
    AGENT = 20_600, "Agent", None, "fas fa-robot"
    # CHALLENGE, ...

    # folder [30_000-40_000]
    FOLDER = 30_000, "Folder", "Sub-space of a Space", "fas fa-folder-open"
    # DEPENDENCY, ...
    TAG = 30_100, "Tag", None, "fas fa-tag"
    TAGGING = 30_101, "Tagging", None, "fas fa-tag"

    # spacetime [40_000-50_000]
    BRANCH = 40_000, "Branch", None, "fas fa-code-branch"
    # HISTORY, REPLAY, ...
    # FORK, ...

    # entity [50_000-60_000]
    # INDEX, CONSTRAINT, MIGRATION, ...
    # MIRROR/SYNC, ...
    # TRAIT_DEFINITION/TRAIT_IMPLEMENTATION, INTERFACE, ...

    # data [60_000-70_000]
    FILE = 60_000, "File", None, "fas fa-file"
    # STREAM, SECRET, ...

    # logic [70_000-80_000]
    SCRIPT = 70_000, "Script", None, "fas fa-code"
    SERVICE = 70_100, "Service", None, "fas fa-screwdriver-wrench"
    # FUNCTION = 70_200, "Function", None, "fas fa-code"
    ACTION = 70_300, "Action", None, "fas fa-code"
    ROUTE = 71_000, "Route", None, "fas fa-route"
    TRIGGER = 72_000, "Trigger", None, "fas fa-bolt"
    TRIGGER_EVENT = 72_001, "Trigger Event", None, "fas fa-bolt"
    TIMER = 72_100, "Timer", None, "fas fa-clock"
    TIMER_EVENT = 72_101, "Timer Event", None, "fas fa-clock"
    TIMER_STARTED_EVENT = 72_102, "Timer Started Event", None, "fas fa-clock"
    TIMER_COMPLETED_EVENT = 72_103, "Timer Completed Event", None, "fas fa-clock"
    TIMER_CANCELLED_EVENT = 72_104, "Timer Cancelled Event", None, "fas fa-clock"
    CURSOR = 72_500, "Cursor", None, "fas fa-mouse-pointer"
    EVENT_CURSOR = 72_600, "Event Cursor", None, "fas fa-signal"
    SCREEN_CURSOR = 72_700, "Screen Cursor", None, "fas fa-mouse"
    THREAD_CURSOR = 72_800, "Thread Cursor", None, "fas fa-magnifying-glass"
    # QUERY_CURSOR, WEB_CURSOR, ...
    # BREAKPOINT, ...
    # ROOM, CHANNEL, ...
    # SEMAPHORE, LOCK/LATCH, ...
    # TASK, TASK_GROUP/TASK_QUEUE, ...
    # RATE_LIMIT, ...

    # test [80_000-90_000]
    # TEST, TEST_SUITE, TEST_CASE, TEST_RESULT, ...
    # FIXTURE, MOCK, ...
    # LINT, WARNING, ERROR, ...

    # runtime [90_000-100_000]
    RUN = 90_000, "Run", None, "fas fa-play"
    RUN_EVENT = 90_001, "Run Event", None, "fas fa-play"
    RUN_STARTED_EVENT = 90_002, "Run Started Event", None, "fas fa-play"
    RUN_PAUSE_REQUESTED_EVENT = 90_003, "Run Pause Requested Event", None, "fas fa-play"
    RUN_PAUSED_EVENT = 90_004, "Run Paused Event", None, "fas fa-play"
    RUN_RESUME_REQUESTED_EVENT = 90_005, "Run Resume Requested Event", None, "fas fa-play"
    RUN_RESUMED_EVENT = 90_006, "Run Resumed Event", None, "fas fa-play"
    RUN_STOP_REQUESTED_EVENT = 90_007, "Run Stop Requested Event", None, "fas fa-play"
    RUN_FAILED_EVENT = 90_008, "Run Failed Event", None, "fas fa-play"
    RUN_COMPLETED_EVENT = 90_009, "Run Completed Event", None, "fas fa-play"
    SPAN_EVENT = 90_301, "Span", None, "fas fa-ruler-horizontal"
    INTERRUPTION = 90_400, "Interruption", None, "fas fa-hand"
    # QUEUE, RUN_QUEUE, ...
    # JOB, ...
    LOG_EVENT = 91_000, "Log", None, "fas fa-file-lines"
    # metric
    GAUGE_METRIC = 92_000, "Gauge Metric", None, "fas fa-gauge"
    GAUGE_MEASUREMENT_EVENT = 92_001, "Gauge Measurement", None, "fas fa-gauge"
    COUNTER_METRIC = 92_100, "Counter Metric", None, "fas fa-gauge"
    COUNTER_MEASUREMENT_EVENT = 92_101, "Counter Measurement", None, "fas fa-gauge"
    HISTOGRAM_METRIC = 92_200, "Histogram Metric", None, "fas fa-gauge"
    HISTOGRAM_MEASUREMENT_EVENT = 92_201, "Histogram Measurement", None, "fas fa-gauge"

    # deployment [100_000-110_000]
    ENVIRONMENT = 100_000, "Environment", None, "fas fa-environment"
    # DEPLOYMENT, ...
    # PREVIEW, DRAFT, RELEASE, ROLLOUT, ...
    # INCIDENT, ESCALATION, ...

    # product [110_000-120_000]
    # SETTINGS, ...
    # VISIT, RECORDING/REPLAY, SURVEY, ...
    # ONBOARDING, TOUR, FUNNEL, COHORT, JOURNEY, ..
    # FEATURE, FEATURE_FLAG, FEATURE_GATE, ...
    # SEGMENT, EXPERIMENT, ...

    # social [120_000-130_000]
    THREAD = 120_000, "Thread", None, "fas fa-reel"
    MESSAGE = 120_100, "Message", None, "fas fa-message"
    REACTION = 120_200, "Reaction", None, "fas fa-heart"
    STAR = 120_300, "Star", None, "fas fa-star"
    FOLLOW = 120_400, "Follow", None, "fas fa-plus"
    NOTIFICATION = 120_500, "Notification", None, "fas fa-bell"
    NOTIFICATION_EVENT = 120_501, "Notification Event", None, "fas fa-bell"
    NOTIFICATION_SENT_EVENT = 120_502, "Notification Sent Event", None, "fas fa-bell"
    NOTIFICATION_RESCINDED_EVENT = 120_503, "Notification Rescinded Event", None, "fas fa-bell"
    NOTIFICATION_READ_EVENT = 120_504, "Notification Read Event", None, "fas fa-bell"
    NOTIFICATION_DISMISSED_EVENT = 120_505, "Notification Dismissed Event", None, "fas fa-bell"
    NOTIFICATION_EXPIRED_EVENT = 120_506, "Notification Expired Event", None, "fas fa-bell"
    # FEED, FEED_ITEM, ...
    # POLL, VOTE, REVIEW, RATING, RANK, ...
    # ACHIEVEMENT, BADGE, WISHLIST/WATCHLIST, ...

    # finance [130_000-140_000]
    # WALLET, BALANCE, BUDGET, TRANSFER, CREDIT, ...
    # TIER, SUBSCRIPTION, PRODUCT, PRICE, ...
    # ORDER, INVOICE, DISCOUNT, DISPUTE, REFUND, ...

    # locale [140_000-150_000]
    # LOCALE, STRING, TRANSLATION, ...

    # internet [150_000-160_000]
    # DOMAIN, ...
    # EMAIL, EMAIL_ATTEMPT, ...

    # infra [160_000-170_000]
    DATABASE = 160_000, "Database", "Database for Postgres data", "fas fa-database"
    MACHINE = 160_100, "Machine", "Machine for ephemeral computing", "fas fa-machine-classic"
    # SEARCH/INDEX, VAULT, CACHE, S3, ...
    # GALAXY, ...
    # HOST, ENDPOINT, DEPLOYMENT, NETWORK, AUTOSCALER, ...

    # intelligence [170_000-180_000]
    # MODEL, FINETUNE, ...
    # PROMPT, INFERENCE/COMPLETION/..., ...
    # RECOMMENDATION, ...

    # world [180_000-190_000]
    # PHONE_NUMBER, ADDRESS, ...

    # scene [190_000-200_000]
    WINDOW = 190_000, "Window", None, "fas fa-galaxy"
    SCENE = 190_100, "Scene", "Scene of an Application", "fas fa-masks-theater"
    SCENE_EVENT = 190_101, "Scene Event", None, "fas fa-masks-theater"
    SCENE_ENTERED_EVENT = 190_102, "Scene Entered Event", None, "fas fa-masks-theater"
    SCENE_EXITED_EVENT = 190_103, "Scene Exited Event", None, "fas fa-masks-theater"
    LAYER = 190_200, "Layer", "Layer of a Scene", "fas fa-layer-group"
    VARIANT = 190_300, "Variant", "Variant of a Scene", "fas fa-shapes"
    VIEW = 190_400, "View", "View in a Scene", "fas fa-eye"
    # VIEW_ENTER_EVENT, VIEW_EXIT_EVENT, ...
    # VIEWPORT, OVERLAY, WIDGET,
    # FORM, MENU, ...

    # interaction [200_000-210_000]
    INPUT_EVENT = 200_000, "Input Event", None, "fas fa-mouse-pointer"
    # pointer events
    POINTER_EVENT = 200_100, "Pointer Event", None, "fas fa-mouse-pointer"
    POINTER_DOWN_EVENT = 200_101, "Pointer Down Event", None, "fas fa-mouse-pointer"
    POINTER_UP_EVENT = 200_102, "Pointer Up Event", None, "fas fa-mouse-pointer"
    POINTER_MOVE_EVENT = 200_103, "Pointer Move Event", None, "fas fa-mouse-pointer"
    POINTER_ENTER_EVENT = 200_104, "Pointer Enter Event", None, "fas fa-mouse-pointer"
    POINTER_OVER_EVENT = 200_105, "Pointer Over Event", None, "fas fa-mouse-pointer"
    POINTER_LEAVE_EVENT = 200_106, "Pointer Leave Event", None, "fas fa-mouse-pointer"
    LONG_PRESS_EVENT = 200_107, "Long Press Event", None, "fas fa-mouse-pointer"
    # mouse events
    MOUSE_EVENT = 200_200, "Mouse Event", None, "fas fa-mouse-pointer"
    CLICK_EVENT = 200_201, "Click Event", None, "fas fa-mouse-pointer"
    LEFT_CLICK_EVENT = 200_202, "Left Click Event", None, "fas fa-mouse-pointer"
    RIGHT_CLICK_EVENT = 200_203, "Right Click Event", None, "fas fa-mouse-pointer"
    MIDDLE_CLICK_EVENT = 200_204, "Middle Click Event", None, "fas fa-mouse-pointer"
    DOUBLE_CLICK_EVENT = 200_205, "Double Click Event", None, "fas fa-mouse-pointer"
    WHEEL_EVENT = 200_206, "Wheel Event", None, "fas fa-mouse-pointer"
    # keyboard events
    KEYBOARD_EVENT = 200_300, "Key Event", None, "fas fa-keyboard"
    KEY_DOWN_EVENT = 200_301, "Key Down Event", None, "fas fa-keyboard"
    KEY_UP_EVENT = 200_302, "Key Up Event", None, "fas fa-keyboard"
    KEY_PRESS_EVENT = 200_303, "Key Press Event", None, "fas fa-keyboard"
    # drag events
    DRAG_EVENT = 200_400, "Drag Event", None, "fas fa-arrows-up-down-left-right"
    DRAG_START_EVENT = 200_401, "Drag Start Event", None, "fas fa-arrows-up-down-left-right"
    DRAG_END_EVENT = 200_402, "Drag End Event", None, "fas fa-arrows-up-down-left-right"
    DRAG_OVER_EVENT = 200_403, "Drag Over Event", None, "fas fa-arrows-up-down-left-right"
    DRAG_ENTER_EVENT = 200_404, "Drag Enter Event", None, "fas fa-arrows-up-down-left-right"
    DRAG_LEAVE_EVENT = 200_405, "Drag Leave Event", None, "fas fa-arrows-up-down-left-right"
    DROP_EVENT = 200_406, "Drop Event", None, "fas fa-arrows-up-down-left-right"
    # clipboard events
    CLIPBOARD_EVENT = 200_500, "Clipboard Event", None, "fas fa-clipboard"
    COPY_EVENT = 200_501, "Copy Event", None, "fas fa-clipboard"
    CUT_EVENT = 200_502, "Cut Event", None, "fas fa-clipboard"
    PASTE_EVENT = 200_503, "Paste Event", None, "fas fa-clipboard"
    # focus events
    FOCUS_EVENT = 200_600, "Focus Event", None, "fas fa-keyboard"
    FOCUS_IN_EVENT = 200_601, "Focus In Event", None, "fas fa-keyboard"
    FOCUS_OUT_EVENT = 200_602, "Focus Out Event", None, "fas fa-keyboard"
    # command
    # COMMAND,  MODE, TOOL, SHORTCUT/KEYBINDING, ...
    # GESTURE, ...
    # CLIPBOARD, ...
    # CAMERA, SPEAKER, MICROPHONE, ...

    # container views [210_000-220_000]
    CONTAINER_VIEW = 210_000, "Container View", None, "fas fa-table"
    FRAME_VIEW = 210_100, "Frame View", "Fixed Container", "fas fa-frame"
    LABEL_VIEW = 210_200, "Label View", "Label Container", "fas fa-font-case"
    SPLIT_VIEW = 210_300, "Split View", "Split Container", "fas fa-columns"
    # SLOT_DEFINITION_VIEW, SLOT_VIEW, ...
    # FORM_VIEW, MENU_VIEW, ...
    # TAB_VIEW, ...
    # DRAWER_VIEW, SPLIT_DRAWER_VIEW, GRID/GRID_ELEMENT_VIEW, ...

    # content views [220_000-230_000]
    CONTENT_VIEW = 220_000, "Content View", None, "fas fa-text"
    TEXT_VIEW = 220_100, "Text View", "Text", "fas fa-text"
    # CODE_VIEW, ICON_VIEW, IMAGE_VIEW, AUDIO_VIEW, VIDEO_VIEW, DOCUMENT_VIEW, ...

    # input views [230_000-240_000]
    INPUT_VIEW = 230_000, "Input View", None, "fas fa-hashtag"
    NUMBER_INPUT_VIEW = 230_100, "Number Input View", "Number Input", "fas fa-hashtag"
    SLIDER_INPUT_VIEW = 230_200, "Slider Input View", "Slider Input", "fas fa-slider"
    # STRING_INPUT_VIEW, TOGGLE_INPUT_VIEW, PICKER_INPUT_VIEW, COLOR_INPUT_VIEW, ...
    # ICON_INPUT_VIEW, FILE_INPUT_VIEW, DATETIME_INPUT_VIEW, DURATION_INPUT_VIEW, ...

    # NOTE :Architecture: node and internal views should probably be defined in user space?
    # node/internal views [240_000-250_000]
    INTERNAL_VIEW = 240_000, "Internal View", None, "fas fa-eye"
    # WIZARD_VIEW, ...

    # canvas [250_000-260_000]
    CANVAS = 250_000, "Canvas", None, "fas fa-canvas"
    SHAPE = 250_100, "Shape", None, "fas fa-shapes"
    LINE_SHAPE = 250_200, "Line Shape", None, "fas fa-line"
    ARROW_SHAPE = 250_300, "Arrow Shape", None, "fas fa-arrow-right"
    ANNOTATION_SHAPE = 250_400, "Annotation Shape", None, "fas fa-comment"
    # VECTOR/POINT, VECTOR_NETWORK, ...
    # BITMAP, ...

    # animation [260_000-270_000]
    # ANIMATION, TRACK, KEYFRAME, ...
    # AUDIO, AUDIO_PLAYER, VIDEO, VIDEO_PLAYER, ...

    # style [270_000-280_000]
    THEME = 270_000, "Theme", None, "fas fa-palette"
    PALETTE = 270_100, "Palette", None, "fas fa-palette"
    STYLE = 270_200, "Style", None, "fas fa-palette"
    COLOR_STYLE = 270_300, "Color Style", None, "fas fa-palette"
    FILL_STYLE = 270_400, "Fill Style", None, "fas fa-fill"
    FONT_STYLE = 270_500, "Font Style", None, "fas fa-text"
    BORDER_STYLE = 270_600, "Border Style", None, "fas fa-border-outer"
    SHADOW_STYLE = 270_700, "Shadow Style", None, "fas fa-eclipse"
    GRADIENT_STYLE = 270_800, "Gradient Style", None, "fas fa-gradient"
    TRANSITION_STYLE = 270_900, "Transition Style", None, "fas fa-bezier-curve"
    EFFECT_STYLE = 270_1000, "Effect Style", None, "fas fa-sparkle"
    STROKE_STYLE = 270_1100, "Stroke Style", None, "fas fa-stroke"
    # BRUSH_STYLE, ...
    # SHADER, MATERIAL, ...


ENUM_TYPES: tuple[EnumType, ...] = tuple(EnumType)
NODE_TYPES: tuple[NodeType, ...] = tuple(NodeType)
STRUCT_TYPES: tuple[StructType, ...] = tuple(StructType)
TRAIT_TYPES: tuple[TraitType, ...] = tuple(TraitType)


@builtin_enum(EnumType.STORE_TYPE)
class StoreType(Enum):
    GLOBAL_ENTITY_PRIMARY = 1000
    SPATIAL_ENTITY_PRIMARY = 1100
    SPATIAL_EVENT_PRIMARY = 2100


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


@builtin_enum(EnumType.MATERIALIZATION)
class Materialization(Enum):
    """The materialization level of an Entity."""

    PARTIAL_NODE = 1, "Partial Node, Partial Graph"
    PARTIAL_GRAPH = 2, "Full Node, Partial Graph"
    FULL_GRAPH = 3, "Full Node, Full Graph"


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
