from datetime import date, datetime, time, timedelta
from decimal import Decimal
from typing import (
    TYPE_CHECKING,
)

from destack.utils.uuid import UUID

from .enum import Enum, builtin_enum

if TYPE_CHECKING:
    pass


#
# Enums
#


class EnumType(Enum):
    # space [1-500]
    SPACE_STATUS = 1
    USER_STATUS = 20
    ORGANIZATION_STATUS = 40
    CLIENT_TYPE = 100
    # query
    CONDITIONAL_TYPE = 103
    AGGREGATION_TYPE = 104
    SORT_MODE = 105
    SORT_TYPE = 106
    JOIN_TYPE = 107
    FUNCTION_TYPE = 108
    EXPRESSION_TYPE = 109
    QUERY_TYPE = 120
    QUERY_UPDATE_TYPE = 121

    # access [500-1000]
    MEMBERSHIP_PERMISSION = 501
    ROLE_TYPE = 520
    PERMISSION_TYPE = 530
    SANCTION_TYPE = 540
    ENTITLEMENT_TYPE = 550
    # ...

    # folder [1000-1500]
    FOLDER_TYPE = 1000

    # history [1500-2000]
    # ...

    # entity [2000-2500]
    # ...

    # data [2500-3000]
    TEXT_SPAN_TYPE = 2521
    FILE_RETENTION_MODE = 2540
    FILE_SOURCE = 2541
    FILE_TYPE = 2542
    FILE_FORMAT = 2543
    ICON_TYPE = 2531
    LINK_TYPE = 2550
    PRIMITIVE_TYPE = 2560
    TYPE_CARDINALITY = 2561
    SCALAR_TYPE = 2562
    DEFAULT_FACTORY = 2563
    STRING_FORMAT = 2570
    NUMBER_FORMAT = 2571
    CUSTOM_PROPERTY_TYPE = 2580
    EDGE_TYPE = 2581
    EDGE_DIRECTION = 2582
    CASCADE_ACTION = 2583
    RESOURCE_STATUS = 2590
    # ...

    # logic [3000-3500]
    ACTION_CARDINALITY = 3020
    CURSOR_STATUS = 3100
    SCHEDULE_FREQUENCY = 3050
    DAY_OF_WEEK = 3051
    MONTH = 3052
    TIMER_TYPE = 3053
    TRIGGER_TYPE = 3040
    # ...

    # test [3500-4000]
    # ...

    # runtime [4000-4500]
    RUN_STATUS = 4000
    INTERRUPTION_TYPE = 4020
    INTERRUPTION_STATUS = 4021
    INTERRUPTION_RESPONSE = 4022
    LOG_LEVEL = 4100
    # ...

    # deployment [4500-5000]
    ENVIRONMENT_TYPE = 4500

    # product [5000-5500]
    # ...

    # social [5500-6000]
    THREAD_STATUS = 5500
    NOTIFICATION_STATUS = 5600

    # finance [6000-6500]
    # ...

    # locale [6500-7000]
    # ...

    # internet [7000-7500]
    # ...

    # infra [7500-8000]
    CLOUD = 7500
    REGION = 7501
    REGION_AREA = 7502
    REGION_CONTINENT = 7503
    TENANCY = 7504
    DATABASE_TYPE = 7505
    MACHINE_TYPE = 7600
    # SEARCH_TYPE, WAREHOUSE_TYPE, ...
    # ...

    # intelligence [8000-8500]
    MODEL_DEVELOPER = 8000
    MODEL_PROVIDER = 8001
    # ...

    # world [8500-9000]
    # ...

    # scene [9000-9500]
    WINDOW_TYPE = 9000
    LAYER_TYPE = 9020
    VARIANT_TYPE = 9030
    VARIANT_STATE_TYPE = 9031

    # interaction [9500-10000]
    MODE_TYPE = 9500
    TOOL_TYPE = 9501
    MOUSE_BUTTON = 9510

    # container views [10000-10200]
    # ...

    # content views [10200-10400]
    # ...

    # input views [10400-10600]
    # ...

    # node/internal views [10600-10800]
    # ...

    # canvas [11000-11500]
    CANVAS_TYPE = 11000
    POLYGON_SHAPE_TYPE = 11010
    ARROW_HEAD_TYPE = 11011

    # animation [11500-12000]
    # ...

    # style [12000-12500]
    COLOR_TYPE = 12020
    COLOR_SHADE = 12021
    COLOR_HUE = 12022
    COLOR_INTENT = 12023
    FILL_TYPE = 12030
    FILL_POSITION = 12031
    FILL_SIZE = 12032
    FONT_TYPE = 12040
    FONT_WEIGHT = 12041
    FONT_SIZE = 12042
    TEXT_ALIGN = 12043
    TEXT_DECORATION = 12044
    TEXT_TRANSFORM = 12045
    BORDER_TYPE = 12050
    SHADOW_TYPE = 12060
    SHADOW_POSITION = 12061
    GRADIENT_TYPE = 12070
    TRANSITION_TYPE = 12080
    SPRING_TYPE = 12081
    EFFECT_TYPE = 12090
    STROKE_TYPE = 12100
    POSITION_TYPE = 12110
    LENGTH_UNIT = 12111
    LAYOUT = 12112
    DISTRIBUTE = 12113
    ALIGN = 12114
    DIRECTION = 12115
    OVERFLOW = 12116
    DIMENSION_TYPE = 12117
    REPEAT_TYPE = 12118
    TEXT_SPLIT_TYPE = 12119
    OFFSCREEN_BEHAVIOR = 12120
    EASING = 12121

    # meta [50000-51000]
    ENUM_TYPE = 50000
    NODE_TYPE = 50001
    STRUCT_TYPE = 50002
    TRAIT_TYPE = 50003
    RELATION_TYPE = 50010
    OBJECT_TYPE = 50011
    PROPERTY_REFERENCE_TYPE = 50012
    MATERIALIZATION_TYPE = 50013
    STORE_ZONE = 50020
    STORE_TYPE = 50021
    STORE_IMPLEMENTATION = 50022
    PLATFORM_TYPE = 50030
    RUNTIME_TYPE = 50031
    OPERATING_SYSTEM = 50040
    EDIT_TYPE = 50050
    EDIT_OPERATION = 50051
    CHANGE_STATUS = 50052
    CHANGE_DEBOUNCE = 50053
    NODE_PERMISSION = 50100
    JOINABLE_PERMISSION = 50101


builtin_enum(EnumType.ENUM_TYPE)(EnumType)


@builtin_enum(EnumType.STRUCT_TYPE)
class StructType(Enum):
    # space [1-500]
    # ...

    # access [500-1000]
    # ...

    # folder [1000-1500]
    # ...

    # spacetime [1500-2000]
    # ...

    # entity [2000-2500]
    # ...

    # data [2500-3000]
    VALUE = 2500
    TYPE = 2501
    NUMBER_CONSTRAINT = 2502
    STRING_CONSTRAINT = 2503
    COLLECTION_CONSTRAINT = 2504
    NODE_CONSTRAINT = 2505
    TEXT = 2520, None, None, "fas fa-text"
    TEXT_SPAN = 2521, None, None, "fas fa-text"
    ICON = 2531
    SELECTION = 2571
    # SCHEMA, UNION, TAG, ...

    # logic [3000-3500]
    SCHEDULE = 3001
    # ...

    # test [3500-4000]
    # ...

    # runtime [4000-4500]
    # ...

    # deployment [4500-5000]
    # ...

    # product [5000-5500]
    # ...

    # social [5500-6000]
    # ...

    # finance [6000-6500]
    # ...

    # locale [6500-7000]
    # ...

    # internet [7000-7500]
    # ...

    # infra [7500-8000]
    DATABASE_INFO = 7501
    GALAXY_INFO = 7601

    # intelligence [8000-8500]
    # ...

    # world [8500-9000]
    # ...

    # scene [9000-9500]
    # ...

    # interaction [9500-10000]
    # ...

    # container views [10000-10200]
    # ...

    # content views [10200-10400]
    # ...

    # input views [10400-10600]
    # ...

    # node/internal views [10600-10800]
    # ...

    # canvas [11000-11500]
    LINE = 11010, "Line", None, "fas fa-line"
    POLYGON = 11011, "Polygon", None, "fas fa-polygon"

    # animation [11500-12000]
    # ...

    # style [12000-12500]
    COLOR = 12011, None, None, "fas fa-palette"
    SHADOW = 12012, None, None, "fas fa-eclipse"
    BORDER = 12013, None, None, "fas fa-border-outer"
    FONT = 12014, None, None, "fas fa-text"
    GRADIENT_STOP = 12015, None, None, "fas fa-gradient"
    GRADIENT = 12016, None, None, "fas fa-gradient"
    FILL = 12017, None, None, "fas fa-fill"
    LENGTH = 12018, None, None, "fas fa-ruler"
    POSITION = 12020, None, None, "fas fa-location-crosshair"
    DIMENSION = 12022, None, None, "fas fa-ruler"
    TRANSITION = 12024, None, None, "fas fa-bezier-curve"
    EFFECT = 12025, None, None, "fas fa-sparkle"
    GRID = 12026, None, None, "fas fa-grid-2"
    GRID_SPAN = 12028, None, None, "fas fa-grid-2"
    INSETS = 12030, None, None, "fas fa-corner"
    CORNERS = 12032, None, None, "fas fa-corner"
    STROKE = 12100, None, None, "fas fa-stroke"
    STROKE_CAP = 12101, None, None, "fas fa-stroke"
    STROKE_PATH = 12102, None, None, "fas fa-stroke"
    STROKE_POINT = 12103, None, None, "fas fa-stroke"

    # meta [50000-51000]
    SCOPE = 50000
    ORIGIN = 50001
    NODE_REFERENCE = 50002
    PROPERTY_REFERENCE = 50003
    PROPERTY_DEFINITION = 50004
    TRAIT_DEFINITION = 50005
    NODE_DEFINITION = 50006
    STRUCT_DEFINITION = 50007
    ENUM_DEFINITION = 50008
    OPTION_DEFINITION = 50009
    PERMISSION_DEFINITION = 50010
    CONSTANT_DEFINITION = 50011
    # ACTION_DEFINITION, ...?
    EDIT = 50020
    CHANGE = 50021
    CHANGE_RESULT = 50022
    EXPRESSION = 50100
    FUNCTION = 50101
    JOIN = 50102
    AGGREGATION = 50103
    CONDITION = 50104
    SORT = 50105
    SELECT = 50106
    RELATION_REFERENCE = 50107
    OBJECT_REFERENCE = 50108
    QUERY = 50110
    QUERY_RESULT = 50111
    QUERY_RESULT_GROUP = 50112
    QUERY_UPDATE = 50113
    HISTOGRAM = 50114
    VECTOR2 = 50200, None, None, "fas fa-vector-square"
    VECTOR3 = 50201, None, None, "fas fa-vector-square"
    VECTOR4 = 50202, None, None, "fas fa-vector-square"
    VECTOR2I = 50203, None, None, "fas fa-vector-square"
    VECTOR3I = 50204, None, None, "fas fa-vector-square"
    VECTOR4I = 50205, None, None, "fas fa-vector-square"
    AXIS2 = 50207, None, None, "fas fa-vector-square"
    AXIS3 = 50209, None, None, "fas fa-vector-square"


@builtin_enum(EnumType.TRAIT_TYPE)
class TraitType(Enum):
    # destack [1-400]
    # where
    GLOBAL = 1, "Global", "Is global", "fas fa-globe"
    SPATIAL = 2, "Spatial", "Is in a Space", "fas fa-solar-system"
    # LOCAL?
    # storage
    # RELATIONAL/OLTP, INDEXED; ANALYTIC, ...?

    # kind
    ENTITY = 20, "Entity", "Is an Entity", "fas fa-hexagon"
    EVENT = 21, "Event", "Is an Event", "fas fa-bolt"

    # type
    RESOURCE = 30, "Resource", "Is a Resource", "fas fa-server"
    CUSTOM_NODE_DEFINITION = (
        31,
        "Custom Node Definition",
        "Is a Custom Node Definition",
        "fas fa-table",
    )
    CUSTOM_NODE = 32, "Custom Node", "Is a Custom Node", "fas fa-database"
    # behavior
    TRACKED = 51, "Tracked", "Is tracked", "fas fa-clock"
    ARCHIVABLE = 52, "Archivable", "Can be archived", "fas fa-box-archive"
    DELETABLE = 53, "Deletable", "Can be deleted", "fas fa-trash"
    EXTENSIBLE = 55, "Extensible", "Is extensible", "fas fa-expand"
    ORDERED = 56, "Ordered", "Is ordered", "fas fa-sort"
    # attribute
    HAS_NAME = 100, "Name", "Has a name", "fas fa-font-case"
    HAS_SLUG = 101, "Slug", "Has a slug", "fas fa-hashtag"
    HAS_ICON = 102, "Icon", "Has an icon", "fas fa-icons"

    # access [500-1000]
    OWNABLE = 500, "Ownable", "Is ownable", "fas fa-user"
    JOINABLE = 502, "Joinable", "Is joinable", "fas fa-users"
    SUBJECT = 505, "Subject", "Is a Subject", "fas fa-user"
    OWNER = 506, "Owner", "Is an Owner", "fas fa-user"
    MEMBERSHIP = 510, "Membership", "Is a Membership", "fas fa-users"
    INVITE = 511, "Invite", "Is an Invite", "fas fa-envelope"

    # folder [1000-1500]
    TAGGABLE = 1000, "Taggable", "Can be tagged", "fas fa-tag"
    TAG = 1001, "Tag", "Is a Tag", "fas fa-tag"

    # spacetime [1500-2000]
    # ...

    # entity [2000-2500]
    # ...

    # data [2500-3000]
    # ...

    # logic [3000-3500]
    ACTIONABLE = 3000, "Actionable", "Can define an Action", "fas fa-play"
    RUNNABLE = 3001, "Runnable", "Can be run", "fas fa-play"
    SCRIPTABLE = 3002, "Scriptable", "Can be scripted", "fas fa-code"
    SOURCEABLE = 3003, "Sourcable", "Can be defined in a Script", "fas fa-code"
    # PAUSEABLE?
    CURSOR = 3012, "Cursor", "Is a Cursor", "fas fa-mouse-pointer"

    # test [3500-4000]
    # ...

    # runtime [4000-4500]
    METRIC = 4010, "Instrument", "Is an Instrument", "fas fa-microscope"
    MEASUREMENT = 4011, "Measurement", "Is a Measurement", "fas fa-microscope"
    # ...

    # deployment [4500-5000]
    # ...

    # product [5000-5500]
    SETTINGS = 5000, "Settings", "Defines Settings", "fas fa-cog"

    # social [5500-6000]
    # MESSAGE, THREAD, ...
    STARABLE = 5530, "Starable", "Can be starred", "fas fa-star"
    REACTABLE = 5532, "Reactable", "Can be reacted to", "fas fa-heart"
    FOLLOWABLE = 5534, "Followable", "Can be followed", "fas fa-plus"
    FOLLOW = 5535, "Follow", "Follow", "fas fa-plus"
    # RATEABLE, VOTABLE, ...
    # ASSIGNABLE, MESSAGEABLE, CLOSABLE, LOCKABLE, ...

    # finance [6000-6500]
    # ...

    # locale [6500-7000]
    # ...

    # internet [7000-7500]
    # ...

    # infra [7500-8000]
    # ...

    # intelligence [8000-8500]
    # ...

    # world [8500-9000]
    # ...

    # scene [9000-9500]
    VISUAL = 9000, "Visual", "Is a Visual", "fas fa-eye"
    VIEW = 9001, "View", "Is a View", "fas fa-eye"
    # VIEW_ENTER_EVENT, VIEW_EXIT_EVENT, ...

    # interaction [9500-10000]
    INPUT_EVENT = 9500, "Input Event", "Is an Input Event", "fas fa-mouse-pointer"
    POINTER_EVENT = 9501, "Pointer Event", "Is a Pointer Event", "fas fa-mouse-pointer"
    MOUSE_EVENT = 9510, "Mouse Event", "Is a Mouse Event", "fas fa-mouse-pointer"
    CLICK_EVENT = 9511, "Click Event", "Is a Click Event", "fas fa-mouse-pointer"
    KEYBOARD_EVENT = 9520, "Keyboard Event", "Is a Keyboard Event", "fas fa-keyboard"
    DRAG_EVENT = 9530, "Drag Event", "Is a Drag Event", "fas fa-arrows-up-down-left-right"
    CLIPBOARD_EVENT = 9540, "Clipboard Event", "Is a Clipboard Event", "fas fa-clipboard"
    FOCUS_EVENT = 9550, "Focus Event", "Is a Focus Event", "fas fa-focus"

    # container views [10000-10200]
    CONTAINER_VIEW = 10000, "Container View", "Is a Container View", "fas fa-container"

    # content views [10200-10400]
    CONTENT_VIEW = 10200, "Content View", "Is a Content View", "fas fa-content"

    # input views [10400-10600]
    INPUT_VIEW = 10400, "Input View", "Is an Input View", "fas fa-input"

    # node/internal views [10600-10800]
    NODE_VIEW = 10600, "Node View", "Is a Node View", "fas fa-node"
    INTERNAL_VIEW = 10650, "Internal View", "Is an Internal View", "fas fa-internal"

    # canvas [11000-11500]
    SHAPE = 11000, "Shape", "Is a Shape", "fas fa-shapes"

    # animation [11500-12000]
    # ...

    # style [12000-12500]
    STYLE = 12000, "Style", "Is a Style", "fas fa-palette"

    # meta [50000-51000]
    # ...


@builtin_enum(EnumType.NODE_TYPE)
class NodeType(Enum):
    # space [1-500]
    SPACE = 1, "Space", "Universal Space", "https://heydestack.com/favicon.ico"
    HANDLE = 20, "Handle", "Unique @handle", "fas fa-at"
    USER = 40, "User", None, "fas fa-user"
    FRIENDSHIP = 60, "Friendship", "Friendship between two Users", "fas fa-user-friends"
    FRIENDSHIP_INVITE = (
        80,
        "Friendship Invite",
        "Invite to be friends with another User",
        "fas fa-user-plus",
    )
    FRIENDSHIP_INVITE_SENT_EVENT = 90, "Friendship Invite Sent Event", None, "fas fa-user-plus"
    FRIENDSHIP_INVITE_RESCINDED_EVENT = (
        91,
        "Friendship Invite Rescinded Event",
        None,
        "fas fa-user-plus",
    )
    FRIENDSHIP_INVITE_ACCEPTED_EVENT = (
        92,
        "Friendship Invite Accepted Event",
        None,
        "fas fa-user-plus",
    )
    FRIENDSHIP_INVITE_REJECTED_EVENT = (
        93,
        "Friendship Invite Rejected Event",
        None,
        "fas fa-user-plus",
    )
    ORGANIZATION = 100, "Organization", None, "fas fa-building"
    TEAM = 120, "Team", "Team in an Organization", "fas fa-users"
    # CREDENTIAL, ACCOUNT, PROFILE, ...
    CLIENT = 200, "Client", None, "fas fa-desktop"

    # access [500-1000]
    MEMBERSHIP = 500, "Membership", "Membership in a Space/Folder", "fas fa-user-group"
    MEMBERSHIP_JOINED_EVENT = 510, "Membership Join Event", None, "fas fa-user-group"
    MEMBERSHIP_LEFT_EVENT = 511, "Membership Leave Event", None, "fas fa-user-group"
    INVITE = 520, "Invite", "Invite to a Space/Folder", "fas fa-user-plus"
    INVITE_SENT_EVENT = 521, "Invite Sent Event", None, "fas fa-user-plus"
    INVITE_RESCINDED_EVENT = 522, "Invite Rescinded Event", None, "fas fa-user-plus"
    INVITE_ACCEPTED_EVENT = 523, "Invite Accepted Event", None, "fas fa-user-plus"
    INVITE_REJECTED_EVENT = 524, "Invite Rejected Event", None, "fas fa-user-plus"
    ROLE = 540, "Role", "Role in something", "fas fa-user-tag"
    ROLE_ASSIGNED_EVENT = 550, "Role Assigned Event", None, "fas fa-user-tag"
    ROLE_UNASSIGNED_EVENT = 551, "Role Unassigned Event", None, "fas fa-user-tag"
    PERMISSION = 560, "Permission", "Permission for something", "fas fa-user-shield"
    SANCTION = 580, "Sanction", "Temporary or permanent restriction", "fas fa-user-minus"
    SANCTION_REQUESTED_EVENT = 590, "Sanction Requested Event", None, "fas fa-user-minus"
    SANCTION_GRANTED_EVENT = 591, "Sanction Granted Event", None, "fas fa-user-minus"
    SANCTION_REVOKED_EVENT = 592, "Sanction Revoked Event", None, "fas fa-user-minus"
    SANCTION_EXPIRED_EVENT = 593, "Sanction Expired Event", None, "fas fa-user-minus"
    ENTITLEMENT = 600, "Entitlement", "Temporary or permanent grant", "fas fa-user-check"
    ENTITLEMENT_REQUESTED_EVENT = 610, "Entitlement Requested Event", None, "fas fa-user-check"
    ENTITLEMENT_GRANTED_EVENT = 611, "Entitlement Granted Event", None, "fas fa-user-check"
    ENTITLEMENT_REVOKED_EVENT = 612, "Entitlement Revoked Event", None, "fas fa-user-check"
    ENTITLEMENT_EXPIRED_EVENT = 613, "Entitlement Expired Event", None, "fas fa-user-check"
    AGENT = 620, "Agent", None, "fas fa-robot"
    # CHALLENGE, ...

    # folder [1000-1500]
    FOLDER = 1000, "Folder", "Sub-space of a Space", "fas fa-folder-open"
    # DEPENDENCY, ...
    TAG = 1010, "Tag", None, "fas fa-tag"
    TAGGING = 1011, "Tagging", None, "fas fa-tag"

    # spacetime [1500-2000]
    SNAPSHOT = 1500, "Snapshot", None, "fas fa-save"
    BRANCH = 1510, "Branch", None, "fas fa-code-branch"
    # HISTORY, REPLAY, ...
    # FORK, ...

    # entity [2000-2500]
    CUSTOM_ENTITY_DEFINITION = 2000, "Custom Node Definition", None, "fas fa-table"
    CUSTOM_ENTITY = 2001, "Custom Node Instance", None, "fas fa-database"
    # INDEX, CONSTRAINT, MIGRATION, ...
    # MIRROR/SYNC, ...
    # TRAIT_DEFINITION/TRAIT_IMPLEMENTATION, INTERFACE, ...

    # data [2500-3000]
    CUSTOM_STRUCT_DEFINITION = 2500, "Struct", None, "fas fa-shapes"
    CUSTOM_ENUM_DEFINITION = 2510, "Enum", None, "fas fa-shapes"
    CUSTOM_PROPERTY = 2520, "Field", None, "fas fa-triangle"
    CUSTOM_OPTION = 2530, "Option", None, "fas fa-circle"
    FILE = 2540, "File", None, "fas fa-file"
    LINK = 2550, "Link", "Link to something", "fas fa-link"
    # STREAM, SECRET, ...

    # logic [3000-3500]
    SCRIPT = 3000, "Script", None, "fas fa-code"
    SERVICE = 3020, "Service", None, "fas fa-screwdriver-wrench"
    ACTION = 3040, "Action", None, "fas fa-step-forward"
    ROUTE = 3060, "Route", None, "fas fa-route"
    TRIGGER = 3080, "Trigger", None, "fas fa-bolt"
    TRIGGER_STARTED_EVENT = 3090, "Trigger Started Event", None, "fas fa-bolt"
    TRIGGER_STOPPED_EVENT = 3091, "Trigger Stopped Event", None, "fas fa-bolt"
    TIMER = 3100, "Timer", None, "fas fa-clock"
    TIMER_STARTED_EVENT = 3110, "Timer Started Event", None, "fas fa-clock"
    TIMER_STOPPED_EVENT = 3111, "Timer Stopped Event", None, "fas fa-clock"
    # BREAKPOINT, ...
    EVENT_CURSOR = 3200, "Event Cursor", None, "fas fa-signal"
    SCREEN_CURSOR = 3201, "Mouse Cursor", None, "fas fa-mouse"
    THREAD_CURSOR = 3202, "Query Cursor", None, "fas fa-magnifying-glass"
    # QUERY_CURSOR, WEB_CURSOR, ...
    # ROOM, CHANNEL, LOCK, ...
    # TASK, ...
    # RATE_LIMIT, ...

    # test [3500-4000]
    # TEST, TEST_SUITE, TEST_CASE, TEST_RESULT, ...
    # FIXTURE, MOCK, ...
    # LINT, WARNING, ERROR, ...

    # runtime [4000-4500]
    RUN = 4000, "Run", None, "fas fa-play"
    RUN_STARTED_EVENT = 4011, "Run Started Event", None, "fas fa-play"
    RUN_PAUSE_REQUESTED_EVENT = 4012, "Run Pause Requested Event", None, "fas fa-play"
    RUN_PAUSED_EVENT = 4013, "Run Paused Event", None, "fas fa-play"
    RUN_RESUME_REQUESTED_EVENT = 4014, "Run Resume Requested Event", None, "fas fa-play"
    RUN_RESUMED_EVENT = 4015, "Run Resumed Event", None, "fas fa-play"
    RUN_STOP_REQUESTED_EVENT = 4016, "Run Stop Requested Event", None, "fas fa-play"
    RUN_FAILED_EVENT = 4017, "Run Failed Event", None, "fas fa-play"
    RUN_COMPLETED_EVENT = 4018, "Run Completed Event", None, "fas fa-play"
    # RUN_QUEUE = 4001, "Run Queue", "Run Queue", "fas fa-list-check"
    SPAN = 4020, "Span", None, "fas fa-ruler-horizontal"
    INTERRUPTION = 4040, "Interruption", None, "fas fa-hand"
    # JOB, ...
    LOG = 4100, "Log", None, "fas fa-file-lines"
    GAUGE_METRIC = 4110, "Gauge Metric", None, "fas fa-gauge"
    GAUGE_MEASUREMENT = 4111, "Gauge Measurement", None, "fas fa-gauge"
    COUNTER_METRIC = 4112, "Counter Metric", None, "fas fa-gauge"
    COUNTER_MEASUREMENT = 4113, "Counter Measurement", None, "fas fa-gauge"
    HISTOGRAM_METRIC = 4114, "Histogram Metric", None, "fas fa-gauge"
    HISTOGRAM_MEASUREMENT = 4115, "Histogram Measurement", None, "fas fa-gauge"
    CUSTOM_EVENT_DEFINITION = 4200, "Custom Event Definition", None, "fas fa-signal"
    CUSTOM_EVENT = 4201, "Custom Event", None, "fas fa-signal"
    EDIT_EVENT = 4202, "Edit Event", None, "fas fa-file-lines"
    # CHANGE_EVENT, QUERY_EVENT, ...

    # deployment [4500-5000]
    ENVIRONMENT = 4500, "Environment", None, "fas fa-environment"
    # DEPLOYMENT, ...
    # PREVIEW, DRAFT, RELEASE, ROLLOUT, ...
    # INCIDENT, ESCALATION, ...

    # product [5000-5500]
    # SETTINGS, ...
    # VISIT, RECORDING/REPLAY, SURVEY, ...
    # ONBOARDING, TOUR, FUNNEL, COHORT, JOURNEY, ..
    # FEATURE, FEATURE_FLAG, FEATURE_GATE, ...
    # SEGMENT, EXPERIMENT, ...

    # social [5500-6000]
    THREAD = 5500, "Thread", None, "fas fa-reel"
    MESSAGE = 5520, "Message", None, "fas fa-message"
    REACTION = 5540, "Reaction", None, "fas fa-heart"
    STAR = 5560, "Star", None, "fas fa-star"
    FOLLOW = 5580, "Follow", None, "fas fa-plus"
    NOTIFICATION = 5600, "Notification", None, "fas fa-bell"
    NOTIFICATION_SENT_EVENT = 5610, "Notification Sent Event", None, "fas fa-bell"
    NOTIFICATION_RESCINDED_EVENT = 5620, "Notification Rescinded Event", None, "fas fa-bell"
    NOTIFICATION_READ_EVENT = 5630, "Notification Read Event", None, "fas fa-bell"
    NOTIFICATION_DISMISSED_EVENT = 5640, "Notification Dismissed Event", None, "fas fa-bell"
    NOTIFICATION_EXPIRED_EVENT = 5650, "Notification Expired Event", None, "fas fa-bell"
    # FEED, FEED_ITEM, ...
    # POLL, VOTE, REVIEW, RATING, RANK, ...
    # ACHIEVEMENT, BADGE, WISHLIST/WATCHLIST, ...

    # finance [6000-6500]
    # WALLET, BALANCE, BUDGET, TRANSFER, CREDIT, ...
    # TIER, SUBSCRIPTION, PRODUCT, PRICE, ...
    # ORDER, INVOICE, DISCOUNT, DISPUTE, REFUND, ...

    # locale [6500-7000]
    # LOCALE, STRING, TRANSLATION, ...

    # internet [7000-7500]
    # DOMAIN, ...
    # EMAIL, EMAIL_ATTEMPT, ...

    # infra [7500-8000]
    DATABASE = 7500, "Database", "Database for Postgres data", "fas fa-database"
    # SEARCH/INDEX, VAULT, CACHE, S3, ...
    # GALAXY = 5010, "Galaxy", "Galaxy", "fas fa-galaxy"
    MACHINE = 7600, "Machine", "Machine for ephemeral computing", "fas fa-machine-classic"
    # HOST, ENDPOINT, DEPLOYMENT, NETWORK, AUTOSCALER, ...

    # intelligence [8000-8500]
    # MODEL, FINETUNE, ...
    # INFERENCE, PROMPT, ...
    # RECOMMENDATION, ...

    # world [8500-9000]
    # PHONE_NUMBER, ADDRESS, ...

    # scene [9000-9500]
    WINDOW = 9000, "Window", None, "fas fa-galaxy"
    SCENE = 9020, "Scene", "Scene of an Application", "fas fa-masks-theater"
    SCENE_ENTERED_EVENT = 9030, "Scene Entered Event", None, "fas fa-masks-theater"
    SCENE_EXITED_EVENT = 9031, "Scene Exited Event", None, "fas fa-masks-theater"
    LAYER = 9040, "Layer", "Layer of a Scene", "fas fa-layer-group"
    VARIANT = 9060, "Variant", "Variant of a Scene", "fas fa-shapes"
    # VIEWPORT, OVERLAY, WIDGET,
    # FORM, MENU, ...

    # interaction [9500-10000]
    # pointer events
    POINTER_DOWN_EVENT = 9500, "Pointer Down Event", None, "fas fa-mouse-pointer"
    POINTER_UP_EVENT = 9501, "Pointer Up Event", None, "fas fa-mouse-pointer"
    POINTER_MOVE_EVENT = 9502, "Pointer Move Event", None, "fas fa-mouse-pointer"
    POINTER_ENTER_EVENT = 9503, "Pointer Enter Event", None, "fas fa-mouse-pointer"
    POINTER_OVER_EVENT = 9504, "Pointer Over Event", None, "fas fa-mouse-pointer"
    POINTER_LEAVE_EVENT = 9505, "Pointer Leave Event", None, "fas fa-mouse-pointer"
    LONG_PRESS_EVENT = 9506, "Long Press Event", None, "fas fa-mouse-pointer"
    # mouse events
    LEFT_CLICK_EVENT = 9510, "Left Click Event", None, "fas fa-mouse-pointer"
    RIGHT_CLICK_EVENT = 9511, "Right Click Event", None, "fas fa-mouse-pointer"
    MIDDLE_CLICK_EVENT = 9512, "Middle Click Event", None, "fas fa-mouse-pointer"
    DOUBLE_CLICK_EVENT = 9513, "Double Click Event", None, "fas fa-mouse-pointer"
    WHEEL_EVENT = 9514, "Wheel Event", None, "fas fa-mouse-pointer"
    # keyboard events
    KEY_DOWN_EVENT = 9520, "Key Down Event", None, "fas fa-keyboard"
    KEY_UP_EVENT = 9521, "Key Up Event", None, "fas fa-keyboard"
    KEY_PRESS_EVENT = 9522, "Key Press Event", None, "fas fa-keyboard"
    # drag events
    DRAG_START_EVENT = 9530, "Drag Start Event", None, "fas fa-arrows-up-down-left-right"
    DRAG_END_EVENT = 9531, "Drag End Event", None, "fas fa-arrows-up-down-left-right"
    DRAG_OVER_EVENT = 9532, "Drag Over Event", None, "fas fa-arrows-up-down-left-right"
    DRAG_ENTER_EVENT = 9533, "Drag Enter Event", None, "fas fa-arrows-up-down-left-right"
    DRAG_LEAVE_EVENT = 9534, "Drag Leave Event", None, "fas fa-arrows-up-down-left-right"
    DROP_EVENT = 9535, "Drop Event", None, "fas fa-arrows-up-down-left-right"
    # clipboard events
    COPY_EVENT = 9540, "Copy Event", None, "fas fa-clipboard"
    CUT_EVENT = 9541, "Cut Event", None, "fas fa-clipboard"
    PASTE_EVENT = 9542, "Paste Event", None, "fas fa-clipboard"
    # focus events
    FOCUS_IN_EVENT = 9552, "Focus In Event", None, "fas fa-keyboard"
    FOCUS_OUT_EVENT = 9553, "Focus Out Event", None, "fas fa-keyboard"
    # command
    # COMMAND, MODE, TOOL, SHORTCUT/KEYBINDING, ...
    # GESTURE, ...
    # CLIPBOARD, ...
    # CAMERA, SPEAKER, MICROPHONE, ...

    # container views [10000-10200]
    CUSTOM_VIEW_DEFINITION = 10000, "Custom View Definition", None, "fas fa-table"
    CUSTOM_VIEW = 10001, "Custom View", None, "fas fa-table"
    # SLOT_DEFINITION_VIEW, SLOT_VIEW, ...
    FRAME_VIEW = 10020, "Frame View", "Fixed Container", "fas fa-frame"
    LABEL_VIEW = 10030, "Label View", "Label Container", "fas fa-font-case"
    # FORM_VIEW, MENU_VIEW, ...
    SPLIT_VIEW = 10040, "Split View", "Split Container", "fas fa-columns"
    # TAB_VIEW, ...
    # DRAWER_VIEW, SPLIT_DRAWER_VIEW, GRID/GRID_ELEMENT_VIEW, ...

    # content views [10200-10400]
    TEXT_VIEW = 10200, "Text View", "Text", "fas fa-text"
    # CODE_VIEW, ICON_VIEW, IMAGE_VIEW, AUDIO_VIEW, VIDEO_VIEW, DOCUMENT_VIEW, ...

    # input views [10400-10600]
    NUMBER_INPUT_VIEW = 10400, "Number Input View", "Number Input", "fas fa-hashtag"
    SLIDER_INPUT_VIEW = 10401, "Slider Input View", "Slider Input", "fas fa-slider"
    # STRING_INPUT_VIEW, TOGGLE_INPUT_VIEW, PICKER_INPUT_VIEW, COLOR_INPUT_VIEW, ...
    # ICON_INPUT_VIEW, FILE_INPUT_VIEW, DATETIME_INPUT_VIEW, DURATION_INPUT_VIEW, ...

    # NOTE :Architecture: node and internal views should probably be defined in user space?
    # node/internal views [10600-10800]
    THREAD_VIEW = 10600, "Thread View", "Thread", "fas fa-reel"
    WIZARD_VIEW = 10650, "Wizard View", "Wizard", "fas fa-wand-sparkles"

    # canvas [11000-11500]
    CANVAS = 11000, "Canvas", None, "fas fa-canvas"
    LINE_SHAPE = 11010, "Line Shape", None, "fas fa-line"
    POLYGON_SHAPE = 11011, "Plane Shape", None, "fas fa-shapes"
    ARROW_SHAPE = 11012, "Arrow Shape", None, "fas fa-arrow-right"
    ANNOTATION_SHAPE = 11013, "Annotation Shape", None, "fas fa-comment"
    # VECTOR/POINT, VECTOR_NETWORK, ...
    # BITMAP, ...

    # animation [11500-12000]
    # ANIMATION, TRACK, KEYFRAME, ...
    # AUDIO, AUDIO_PLAYER, VIDEO, VIDEO_PLAYER, ...

    # style [12000-12500]
    THEME = 12000, "Theme", None, "fas fa-palette"
    PALETTE = 12010, "Palette", None, "fas fa-palette"
    COLOR_STYLE = 12020, "Color Style", None, "fas fa-palette"
    FILL_STYLE = 12030, "Fill Style", None, "fas fa-fill"
    FONT_STYLE = 12040, "Font Style", None, "fas fa-text"
    BORDER_STYLE = 12050, "Border Style", None, "fas fa-border-outer"
    SHADOW_STYLE = 12060, "Shadow Style", None, "fas fa-eclipse"
    GRADIENT_STYLE = 12070, "Gradient Style", None, "fas fa-gradient"
    TRANSITION_STYLE = 12080, "Transition Style", None, "fas fa-bezier-curve"
    EFFECT_STYLE = 12090, "Effect Style", None, "fas fa-sparkle"
    STROKE_STYLE = 12100, "Stroke Style", None, "fas fa-stroke"
    # BRUSH_STYLE, ...
    # SHADER, MATERIAL, ...

    # meta [50000-51000]
    # ...


ENUM_TYPES: tuple[EnumType, ...] = tuple(EnumType)
NODE_TYPES: tuple[NodeType, ...] = tuple(NodeType)
STRUCT_TYPES: tuple[StructType, ...] = tuple(StructType)
TRAIT_TYPES: tuple[TraitType, ...] = tuple(TraitType)


@builtin_enum(EnumType.STORE_ZONE)
class StoreZone(Enum):
    GLOBAL = 1
    SPATIAL = 2
    LOCAL = 3


@builtin_enum(EnumType.STORE_TYPE)
class StoreType(Enum):
    GLOBAL_ENTITY = 100
    # GLOBAL_SEARCH?
    SPATIAL_ENTITY = 200
    # SPATIAL_PARTICLE, SPATIAL_ANALYTIC, ...
    # SPATIAL_SEARCH, SPATIAL_CACHE, ...
    LOCAL_MEMORY = 300

    @property
    def zone(self) -> StoreZone:
        return StoreZone(self.value // 100)


@builtin_enum(EnumType.STORE_IMPLEMENTATION)
class StoreImplementation(Enum):
    MEMORY = 1
    POSTGRES = 10
    # CASSANDRA, ELASTICSEARCH, REDIS, ...


@builtin_enum(EnumType.RUNTIME_TYPE)
class RuntimeType(Enum):
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


@builtin_enum(EnumType.NODE_PERMISSION)
class NodePermission(Enum):
    # read
    READ = 1, "Read"
    # write
    ADD = 10, "Create, Upsert, Unarchive, Restore"
    UPDATE = 11, "Update"
    REMOVE = 12, "Archive, Delete, Erase"


@builtin_enum(EnumType.MATERIALIZATION_TYPE)
class MaterializationType(Enum):
    PARTIAL_NODE = 1, "Partial Node"
    PARTIAL_GRAPH = 2, "Full Node, Partial Graph"
    FULL_GRAPH = 3, "Full"


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

    EUROPE = 1000, "Europe", None, "🇪🇺"
    NORTH_AMERICA = 2000, "North America", None, "🇺🇸"
    SOUTH_AMERICA = 3000, "South America", None, "🇧🇷"
    MIDDLE_EAST = 4000, "Middle East", None, "🇸🇦"
    AFRICA = 5000, "Africa", None, "🇿🇦"
    ASIA = 6000, "Asia", None, "🇮🇳"
    AUSTRALIA = 7000, "Australia", None, "🇦🇺"
    PRIVATE = 9000

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

    EUROPE_CENTRAL = 1000, None, None, "🇪🇺"
    NORTH_AMERICA_EAST = 2000, None, None, "🇺🇸"
    NORTH_AMERICA_WEST = 2200, None, None, "🇺🇸"
    SOUTH_AMERICA_EAST = 3000, None, None, "🇧🇷"
    MIDDLE_EAST_CENTRAL = 4000, None, None, "🇸🇦"
    MIDDLE_EAST_WEST = 4200, None, None, "🇸🇦"
    AFRICA_SOUTH = 5000, None, None, "🇿🇦"
    ASIA_WEST = 6000
    ASIA_SOUTH = 6200
    ASIA_EAST = 6400
    AUSTRALIA_SOUTH = 7000, None, None, "🇦🇺"

    @property
    def continent(self) -> RegionContinent:
        return RegionContinent((self.id // 1000) * 1000)

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


@builtin_enum(EnumType.REGION)
class Region(Enum):
    """Regions in an Area on a Continent."""

    # eu-central
    ZURICH = 1000, None, None, "🇨🇭"
    FRANKFURT = 1010, None, None, "🇩🇪"

    # na-east
    VIRGINIA = 2000, None, None, "🇺🇸"
    OHIO = 2010, None, None, "🇺🇸"

    # na-west
    OREGON = 2200, None, None, "🇺🇸"

    # sa-east
    SAO_PAULO = 3000, None, None, "🇧🇷"

    ...

    # af-south
    CAPE_TOWN = 5000, None, None, "🇿🇦"

    # as-east
    MUMBAI = 6000, None, None, "🇮🇳"

    # as-south
    SINGAPORE = 6200, None, None, "🇸🇬"

    # as-east
    TOKYO = 6400, None, None, "🇯🇵"

    # au-south
    SYDNEY = 7000, None, None, "🇦🇺"

    @property
    def continent(self) -> RegionContinent:
        return RegionContinent((self.id // 1000) * 1000)

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
    # range: -32768 to 32767
    INT16 = 4, "Integer", "Very small integer", "fas fa-tally"
    # range: -2147483648 to 2147483647
    INT32 = 5, "Integer", "Small integer", "fas fa-tally"
    # range: -9223372036854775808 to 9223372036854775807
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
    # CUSTOM_ENUM, CUSTOM_STRUCT, ...


@builtin_enum(EnumType.DEFAULT_FACTORY)
class DefaultFactory(Enum):
    """The factory to use for default values."""

    UUID = 1
    NOW = 2
    REGION = 3


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
