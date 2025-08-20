from typing import TYPE_CHECKING

from ._const import UNSET
from .enum import OptionEnum, declare_enum, declare_option

if TYPE_CHECKING:
    pass


#
# Enums
#


def _get_universe_domain(metatype: int) -> tuple["UniverseDomain", "UniverseCategory"]:
    """Find the UniverseDomain / UniverseCategory from a class's module (from the module names)."""
    if metatype < 10_000_000:
        universe_domain = UniverseDomain.CORE
    else:
        universe_domain = UniverseDomain((metatype // 10_000_000) * 10_000_000)
    if metatype < 100_000:
        universe_category = UniverseCategory.BUILTIN
    else:
        universe_category = UniverseCategory((metatype // 100_000) * 100_000)
    return universe_domain, universe_category


class UniverseDomain(OptionEnum):
    """The Destack Computational Universe is organized into domains."""

    CORE = declare_option(
        1,
        "Core",
        description="Intrinsics: Basic atoms the rest of the Universe is built on.",
    )
    BASICS = declare_option(
        10_000_000,
        "Basics",
        description="Scaffolding: Common Universe blocks that span domains.",
    )
    SIMULATION = declare_option(
        20_000_000,
        "Simulation",
        description="Modeling: The Universe as a simulation.",
    )
    IMAGINATION = declare_option(
        30_000_000,
        "Imagination",
        description="Imagining: The Universe as a canvas.",
    )
    PRESENTATION = declare_option(
        40_000_000,
        "Presentation",
        description="Presenting: The Universe on stage.",
    )
    PRODUCTION = declare_option(
        50_000_000,
        "Production",
        description="Operating: Building the Universe.",
    )
    DISTRIBUTION = declare_option(
        60_000_000,
        "Distribution",
        description="Distributing: Integrating the Universe.",
    )


class UniverseCategory(OptionEnum):
    """How the Destack Computational Universe is organized (domains > categories)."""

    #
    # CORE
    #

    BUILTIN = declare_option(1, "Core", description="Primitives and intrinsics")
    DEFINITION = declare_option(100_000, "Definition", description="Builtin definitions")
    COMMON = declare_option(200_000, "Common", description="Shared definitions")
    ENCODING = declare_option(1_000_000, "Encoding", description="Serialization and packing")
    PERSISTENCE = declare_option(
        1_100_000, "Persistence", description="Storage and synchronization"
    )
    GENERATION = declare_option(
        2_000_000, "Generation", description="Code generation and compilation"
    )
    LOCAL = declare_option(2_100_000, "Local", description="Local runtime integration")
    # ...
    UNIVERSE = declare_option(3_000_000, "Universe", description="Global computational universe")
    SPACE = declare_option(3_100_000, "Space", description="Spacetime organization")

    #
    # BASICS
    #

    ENTITY = declare_option(10_000_000, "Entity", description="Entity management")
    SCRIPT = declare_option(10_100_000, "Script", description="Logic, scripting and behavior")
    INTELLIGENCE = declare_option(10_200_000, "Intelligence", description="Artificial intelligence")
    ACCESS = declare_option(
        10_300_000, "Access", description="Identity, authentication and authorization"
    )
    QUALITY = declare_option(10_400_000, "Quality", description="Quality assurance")
    # STORAGE/STREAMING/SYNC,
    SOCIAL = declare_option(11_000_000, "Social", description="Interactions, reputation and trust")
    FINANCE = declare_option(11_100_000, "Finance", description="Accounting and finance")
    # INTERNET, ...
    EDITOR = declare_option(19_000_000, "Editor", description="Editing the Universe")

    #
    # SIMULATION
    #

    GEOMETRY = declare_option(20_000_000, "Geometry", description="Geometric representations")
    GEOGRAPHY = declare_option(20_100_000, "Geography", description="Geographic representations")
    PHYSICS = declare_option(20_200_000, "Physics", description="Physics simulation")
    PERCEPTION = declare_option(20_300_000, "Perception", description="Sensing and interaction")
    # ML/MODEL_SERVING, ...
    # CHARACTER/HUMAN?, FLESH/FUR/...?, CLOTH, FLUID?, ...
    # NAVIGATION, ACTUATION, ...
    # script, NETWORKING, STATISTICS, ...
    # MECHANICAL, MOLECULAR, ELECTRICAL, AERODYNAMIC, LIGHT, SUBSTANCE, ...
    # GEOLOGY, BIOLOGY, CHEMISTRY, ECOLOGICAL, CIVIL, ...

    #
    # IMAGINATION
    #

    AUDIO = declare_option(30_000_000, "Audio", description="Audio and sound production")
    IMAGE = declare_option(30_100_000, "Image", description="Image and photo production")
    VIDEO = declare_option(30_200_000, "Video", description="Video production")
    MODEL = declare_option(30_300_000, "Model", description="Modeling, sculpting, CSG, CAD")
    PAINT = declare_option(30_400_000, "Paint", description="Drawing and painting")
    ANIMATION = declare_option(30_500_000, "Animation", description="Motion choreography")
    # MUSIC, ...
    STYLE = declare_option(31_000_000, "Style", description="Appearance and theming")
    DOCUMENT = declare_option(32_000_000, "Document", description="Document processing")
    # NARRATIVE, GAME, ...
    # ARCHITECTURE, ...

    #
    # PRESENTATION
    #

    SCENE = declare_option(40_000_000, "Scene", description="Staging and viewing")
    VIEW = declare_option(40_100_000, "View", description="View construction")
    RENDERING = declare_option(
        40_200_000, "Rendering", description="Render pipelines, post-processing"
    )
    CAMERA = declare_option(40_300_000, "Camera", description="Camera and viewport")
    SHADERS = declare_option(40_400_000, "Shaders", description="Shader programming")
    MATERIAL = declare_option(40_500_000, "Material", description="Material rendering")
    LIGHTING = declare_option(40_600_000, "Lighting", description="Light and shadows")
    # PARTICLE?, TEXT/FONT, XR, ...

    #
    # PRODUCTION
    #

    CLOUD = declare_option(50_000_000, "Cloud", description="Cloud computing infrastructure")
    OBSERVABILITY = declare_option(
        50_100_000, "Observability", description="Telemetry on everything"
    )
    EXPERIENCE = declare_option(50_200_000, "Experience", description="User experience")
    # PRINTING, ACTUATION, ROBOTICS, ...
    # CONSTRUCTION, TRANSPORTATION/SHIPPING, ENERGY, DEFENSE, ...
    # CONSUMER/HOME, PHARMACEUTICAL, ...

    #
    # DISTRIBUTION
    #

    LOCALIZATION = declare_option(
        60_000_000, "Localization", description="Localization and internationalization"
    )
    LEGAL = declare_option(60_100_000, "Legal", description="Legal, compliance and policy")
    COMMERCE = declare_option(60_200_000, "Commerce", description="Billing and monetization")
    # ACCESSIBILITY, GOVERNANCE, ...
    # CONTENT, CRYPTO, ...


class EnumType(OptionEnum):
    #
    # CORE
    #

    # builtin [1]
    MODULE_TYPE = declare_option(2)
    ENUM_TYPE = declare_option(3)
    NODE_TYPE = declare_option(4)
    STRUCT_TYPE = declare_option(5)
    TRAIT_TYPE = declare_option(6)
    HANDLE_TYPE = declare_option(7)
    UNIVERSE_DOMAIN = declare_option(10)
    UNIVERSE_CATEGORY = declare_option(11)
    MATERIALIZATION = declare_option(20)
    PROCESS_FLAG = declare_option(21)
    EXTENSION_FLAG = declare_option(22)
    RUNTIME_PLATFORM = declare_option(30)
    RUNTIME_LANGUAGE = declare_option(31)
    RUNTIME_TYPE = declare_option(32)
    EVENT_STATUS = declare_option(40)
    STRING_CASING = declare_option(50)

    # definition [100_000]

    # common [200_000]
    PRIMITIVE_TYPE = declare_option(200_000)
    TYPE_CARDINALITY = declare_option(200_001)
    SCALAR_TYPE = declare_option(200_002)
    VALUE_FACTORY = declare_option(200_003)
    PROPERTY_ZONE = declare_option(200_006)
    REFERENCE_TYPE = declare_option(200_010)
    # text
    TEXT_SPAN_TYPE = declare_option(200_100)
    TEXT_STYLE_FLAG = declare_option(200_101)
    ICON_TYPE = declare_option(200_110)
    # change
    CHANGE_TYPE = declare_option(200_200)
    EDIT_OPERATION_TYPE = declare_option(200_201)
    # query
    CONDITIONAL_TYPE = declare_option(200_300)
    AGGREGATION_TYPE = declare_option(200_301)
    SORT_MODE = declare_option(200_302)
    SORT_TYPE = declare_option(200_303)
    JOIN_TYPE = declare_option(200_304)
    EXPRESSION_TYPE = declare_option(200_306)
    QUERY_TYPE = declare_option(200_320)

    # encoding [1_000_000]
    ENCODING = declare_option(1_000_000)
    ENCODER_FLAG = declare_option(1_000_001)
    ENCODER_STABILITY = declare_option(1_000_002)

    # persistence [1_100_000]
    # ...

    # generation [2_000_000]
    # ...

    # local [2_100_000]
    # ...

    # universe [3_000_000]
    CLIENT_TYPE = declare_option(3_000_200)

    # space [3_100_000]
    BRANCH_TYPE = declare_option(3_100_300)
    SNAPSHOT_TYPE = declare_option(3_100_400)
    SNAPSHOT_STATUS = declare_option(3_100_401)
    FOLDER_TYPE = declare_option(3_100_500)

    #
    # BASICS
    #

    # entity [10_000_000]
    INDEX_TYPE = declare_option(10_000_100)
    CONSTRAINT_TYPE = declare_option(10_000_200)
    MIGRATION_TYPE = declare_option(10_000_300)

    # script [10_100_000]
    RUN_STATUS = declare_option(10_101_000)
    FUNCTION_OPERATOR = declare_option(10_100_400)
    METHOD_TYPE = declare_option(10_100_500)
    ACTION_TYPE = declare_option(10_100_600)
    TRIGGER_TYPE = declare_option(10_100_700)
    TIMER_TYPE = declare_option(10_100_800)
    DAY_OF_WEEK = declare_option(10_100_801)
    MONTH = declare_option(10_100_802)
    SCHEDULE_FREQUENCY = declare_option(10_100_803)
    LOG_LEVEL = declare_option(10_101_100)

    # intelligence [10_200_000]
    # ...

    # access [10_300_000]
    ROLE_TYPE = declare_option(10_300_300)
    SANCTION_TYPE = declare_option(10_300_400)
    ENTITLEMENT_TYPE = declare_option(10_300_500)

    # quality [10_400_000]
    # ...

    # social [11_000_000]
    NOTIFICATION_STATUS = declare_option(11_001_301)
    # ...

    # finance [11_100_000]
    # ...

    # editor [19_000_000]
    # ...

    #
    # SIMULATION
    #

    # geometry [20_000_000]
    # ...

    # geography [20_100_000]
    # ...

    # physics [20_200_000]
    RIGID_MOTION_MODE = declare_option(20_200_100)
    JOINT_FLAG = declare_option(20_201_000)

    # perception [20_300_000]
    MOUSE_BUTTON = declare_option(20_300_000)

    #
    # IMAGINATION
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

    # animation [30_500_000]
    TRANSITION_TYPE = declare_option(30_500_000)
    SPRING_TYPE = declare_option(30_500_001)
    EFFECT_TYPE = declare_option(30_500_100)
    REPEAT_TYPE = declare_option(30_500_002)
    EASING = declare_option(30_500_003)
    TEXT_SPLIT_TYPE = declare_option(30_500_004)
    OFFSCREEN_BEHAVIOR = declare_option(30_500_005)

    # style [31_000_000]
    COLOR_TYPE = declare_option(31_000_000)
    COLOR_SHADE = declare_option(31_000_001)
    COLOR_HUE = declare_option(31_000_002)
    COLOR_INTENT = declare_option(31_000_003)
    FILL_TYPE = declare_option(31_000_400)
    FILL_POSITION = declare_option(31_000_401)
    FILL_SIZE = declare_option(31_000_402)
    FONT_TYPE = declare_option(31_000_500)
    FONT_WEIGHT = declare_option(31_000_501)
    FONT_SIZE = declare_option(31_000_502)
    TEXT_ALIGN = declare_option(31_000_503)
    TEXT_DECORATION = declare_option(31_000_504)
    TEXT_TRANSFORM = declare_option(31_000_505)
    BORDER_TYPE = declare_option(31_000_600)
    SHADOW_TYPE = declare_option(31_000_700)
    SHADOW_POSITION = declare_option(31_000_701)
    GRADIENT_TYPE = declare_option(31_000_800)
    STROKE_TYPE = declare_option(31_000_900)
    # ...

    # document [32_000_000]
    # ...

    #
    # PRESENTATION
    #

    # scene [40_000_000]
    # ...

    # view [40_100_000]
    LENGTH_TYPE = declare_option(40_100_001)
    LAYOUT = declare_option(40_100_002)
    DISTRIBUTE = declare_option(40_100_003)
    ALIGN = declare_option(40_100_004)
    DIRECTION = declare_option(40_100_005)
    OVERFLOW = declare_option(40_100_006)
    ANCHOR = declare_option(40_100_007)
    # ...

    # rendering [40_200_000]
    # ...

    # camera [40_300_000]
    # ...

    # shaders [40_400_000]
    # ...

    # material [40_500_000]
    # ...

    # lighting [40_600_000]
    # ...

    #
    # PRODUCTION
    #

    # cloud [50_000_000]
    REGION = declare_option(50_000_001)
    REGION_AREA = declare_option(50_000_002)
    REGION_CONTINENT = declare_option(50_000_003)
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

    # commerce [60_200_000]
    # ...


# circular definitions
EnumType = declare_enum(EnumType.ENUM_TYPE, domain=UNSET, category=UNSET)(EnumType)
UniverseDomain = declare_enum(EnumType.UNIVERSE_DOMAIN, domain=UNSET, category=UNSET)(
    UniverseDomain
)
UniverseCategory = declare_enum(EnumType.UNIVERSE_CATEGORY, domain=UNSET, category=UNSET)(
    UniverseCategory
)
EnumType.__declaration__.domain = UniverseDomain.CORE
EnumType.__declaration__.category = UniverseCategory.BUILTIN
UniverseDomain.__declaration__.domain = UniverseDomain.CORE
UniverseDomain.__declaration__.category = UniverseCategory.BUILTIN
UniverseCategory.__declaration__.domain = UniverseDomain.CORE
UniverseCategory.__declaration__.category = UniverseCategory.BUILTIN


@declare_enum(EnumType.MODULE_TYPE)
class ModuleType(OptionEnum):
    """Built-in module types."""

    ROOT = declare_option(
        1,
        "Root",
        description="Root module for the entire Universe",
    )
    DOMAIN = declare_option(
        2,
        "Domain",
        description="Module for an entire UniverseDomain",
    )
    CATEGORY = declare_option(
        3,
        "Category",
        description="Module for an entire UniverseCategory",
    )
    OBJECT = declare_option(
        4,
        "Object",
        description="Module for one or more Objects",
    )


@declare_enum(EnumType.TRAIT_TYPE)
class TraitType(OptionEnum):
    #
    # CORE
    #

    # builtin [1]
    ORDERED = declare_option(1, "Ordered", description="Is ordered")
    RESOURCE = declare_option(2, "Resource", description="Is a Resource")
    VARIANT = declare_option(3, "Variant", description="Is a Variant")
    TEMPLATE = declare_option(4, "Template", description="Is a Template")
    # PAUSABLE?

    # definition [100_000]
    # ...

    # common [200_000]
    # ...

    # encoding [1_000_000]
    # ...

    # persistence [1_100_000]
    # ...

    # generation [2_000_000]
    # ...

    # local [2_100_000]
    # ...

    # universe [3_000_000]
    # ...

    # space [3_100_000]
    # ...

    #
    # BASICS
    #

    # entity [10_000_000]
    # ...

    # script [10_100_000]
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

    # social [11_000_000]
    STARABLE = declare_option(11_000_100, "Starable", description="Can be starred")
    REACTABLE = declare_option(11_000_200, "Reactable", description="Can be reacted to")
    FOLLOWABLE = declare_option(11_000_300, "Followable", description="Can be followed")
    # RATEABLE, VOTABLE, ...
    # ASSIGNABLE, MESSAGEABLE, CLOSABLE, LOCKABLE, ...

    # finance [60_300_000]
    # ...

    # finance [11_100_000]
    # ...

    #
    # SIMULATION
    #

    # geometry [20_000_000]
    # ...

    # geography [20_100_000]
    # ...

    # physics [20_200_000]
    # ...

    # perception [20_300_000]
    INTERACTIVE = declare_option(20_300_000, "Interactive", description="Can be interacted with")
    DRAGGABLE = declare_option(20_300_001, "Draggable", description="Can be dragged")
    SELECTABLE = declare_option(20_300_002, "Selectable", description="Can be selected")
    # ...

    #
    # IMAGINATION
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

    # animation [30_500_000]
    # ...

    # style [31_000_000]
    # ...

    # document [32_000_000]
    # ...

    #
    # PRESENTATION
    #

    # scene [40_000_000]
    # ...

    # view [40_100_000]
    # ANIMATABLE/TWEENABLE, ...

    # rendering [40_200_000]
    # ...

    # camera [40_300_000]
    # ...

    # shaders [40_400_000]
    # ...

    # material [40_500_000]
    # ...

    # lighting [40_600_000]
    # ...

    #
    # PRODUCTION
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

    # commerce [60_200_000]
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

    # definition [100_000]
    # ...

    # common [200_000]
    CHANGE_EVENT = declare_option(201_000, "Change Event")
    # ...

    # encoding [1_000_000]
    # ...

    # persistence [1_100_000]
    # ...

    # generation [2_000_000]
    # ...

    # local [2_100_000]
    # ...

    # universe [3_000_000]
    UNIVERSE = declare_option(
        3_000_000, "Universe", description="The Destack computational universe"
    )
    # GALAXY, ...
    # HANDLE?, ...
    # user
    USER = declare_option(3_000_100, "User")
    # FRIENDSHIP, FRIENDSHIP_INVITE, ...
    CLIENT = declare_option(3_000_200, "Client")
    # CREDENTIAL, ACCOUNT, PROFILE, ...
    # organization
    ORGANIZATION = declare_option(3_000_300, "Organization")
    TEAM = declare_option(3_000_400, "Team", description="Team in an Organization")

    # space [3_100_000]
    SPACE = declare_option(3_100_000, "Space", description="Universal Space")
    # FRAGMENT (multiple disjoint trees)
    # SLOT (inside tree)
    # LINK/PORTAL (to another subtree)
    BRANCH = declare_option(3_100_300, "Branch")
    SNAPSHOT = declare_option(3_100_400, "Snapshot", description="Point in Space-time")
    FOLDER = declare_option(3_100_500, "Folder", description="Sub-space of a Space")
    # nocheckin: Folder->Module .. use Modules for everything (also see ModuleDefinition?)
    # APPLICATION (extends Folder?), ...
    # DEPENDENCY, VERSION, ...
    # HISTORY, REPLAY, TIMELINE, FORK, ...
    # LINK, PORTAL, ...

    #
    # BASICS
    #

    # entity [10_000_000]
    # nocheckin: move entity into core? also events?
    # nocheckin: CustomTraitDefinition? CustomSystemDefinition? CustomComponentDefinition?
    #  .. also revisit *_definition/*_builtin mappings..
    #   (Property vs PropertyDefinition? Module? Entity? Event? Struct?)
    CUSTOM_EVENT_DEFINITION = declare_option(
        10_000_000,
        "Custom Event",
        description="Custom Event Definition",
    )
    CUSTOM_STRUCT_DEFINITION = declare_option(
        10_000_100,
        "Custom Struct",
        description="Custom Struct Definition",
    )
    CUSTOM_ERROR_DEFINITION = declare_option(
        10_000_300,
        "Custom Error",
        description="Custom Error Definition",
    )
    CUSTOM_MESSAGE_DEFINITION = declare_option(
        10_000_200,
        "Custom Message",
        description="Custom Message Definition",
    )
    CUSTOM_ENUM_DEFINITION = declare_option(
        10_000_500,
        "Custom Enum",
        description="Custom Enum Definition",
    )
    CUSTOM_PROPERTY_DEFINITION = declare_option(
        10_001_000,
        "Custom Property",
        description="Custom Property Definition",
    )
    CUSTOM_OPTION_DEFINITION = declare_option(
        10_001_100,
        "Custom Option",
        description="Custom Option Definition",
    )
    # CUSTOM_UNION_DEFINITION, ...
    TAG = declare_option(10_010_000, "Tag")
    INDEX = declare_option(
        10_010_100,
        "Index",
        description="Index of an Entity",
    )
    CONSTRAINT = declare_option(
        10_010_200,
        "Constraint",
        description="Constraint of an Entity",
    )
    # EXPECTATION, ...
    MIGRATION = declare_option(
        10_011_000,
        "Migration",
        description="Migration of an Entity",
    )
    MIGRATION_OPERATION = declare_option(
        10_011_100,
        "Migration Operation",
        description="Migration Operation of an Entity",
    )
    FILE = declare_option(10_020_000, "File")
    # DIRECTORY, SYNC, ...
    # INDEX, CONSTRAINT, MIGRATION, ...
    # REMOTE, FOREIGN_DATA/ENTITY_WRAPPER, ...
    # SECRET, ...

    # script [10_100_000]
    ENVIRONMENT = declare_option(10_100_100, "Environment")
    SCRIPT = declare_option(10_100_200, "Script")
    CUSTOM_EVENT = declare_option(10_100_300, "Signal", description="Custom Event instance")
    FUNCTION = declare_option(10_101_000, "Function")
    METHOD = declare_option(10_101_100, "Method")
    ACTION = declare_option(10_101_200, "Action")
    # MUTATION?
    TRIGGER = declare_option(10_102_000, "Trigger")
    TRIGGER_EVENT = declare_option(10_102_001, "Trigger Event")
    TIMER = declare_option(10_102_100, "Timer")
    TIMER_EVENT = declare_option(10_102_101, "Timer Event")
    TIMER_STARTED_EVENT = declare_option(10_102_102, "Timer Started Event")
    TIMER_PAUSED_EVENT = declare_option(10_102_103, "Timer Paused Event")
    TIMER_RESUMED_EVENT = declare_option(10_102_104, "Timer Resumed Event")
    TIMER_COMPLETED_EVENT = declare_option(10_102_105, "Timer Completed Event")
    TIMER_CANCELLED_EVENT = declare_option(10_102_106, "Timer Cancelled Event")
    # EFFECT, DEPENDENCY, ...
    # BREAKPOINT, ...
    # ROOM, TOPIC, CHANNEL, ...
    # QUEUE, TASK, ...
    # SEMAPHORE, LOCK/LATCH, ...
    # RATE_LIMIT, ...
    # STATE_MACHINE, STATE, STATE_TRANSITION, ...
    # PLATFORM_VARIANT, STATE_VARIANT, ...
    # RELEASE, DEPLOYMENT, ...
    # PREVIEW, DRAFT, ROLLOUT, ...
    # TASK, TASK_GROUP/TASK_QUEUE, JOB, ...
    RUN = declare_option(10_110_000, "Run")
    RUN_EVENT = declare_option(10_110_001, "Run Event")
    RUN_STARTED_EVENT = declare_option(10_110_002, "Run Started Event")
    RUN_PAUSE_REQUESTED_EVENT = declare_option(10_110_003, "Run Pause Requested Event")
    RUN_PAUSED_EVENT = declare_option(10_110_004, "Run Paused Event")
    RUN_RESUME_REQUESTED_EVENT = declare_option(10_110_005, "Run Resume Requested Event")
    RUN_RESUMED_EVENT = declare_option(10_110_006, "Run Resumed Event")
    RUN_STOP_REQUESTED_EVENT = declare_option(10_110_007, "Run Stop Requested Event")
    RUN_FAILED_EVENT = declare_option(10_110_008, "Run Failed Event")
    RUN_COMPLETED_EVENT = declare_option(10_110_009, "Run Completed Event")
    SPAN_EVENT = declare_option(10_110_010, "Span")
    LOG_EVENT = declare_option(10_110_011, "Log")

    # intelligence [10_200_000]
    # INTELLIGENCE/AI/MODEL, FINETUNE, ...
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
    # ASSERT/ASSERTION, ...
    # FIXTURE, MOCK, ...
    # LINT, WARNING, ERROR, ...
    # DEPRECATION, ...

    # social [11_000_000]
    STAR = declare_option(11_001_100, "Star")
    STAR_EVENT = declare_option(11_001_101, "Star Event")
    STAR_ADDED_EVENT = declare_option(11_001_102, "Star Added Event")
    STAR_REMOVED_EVENT = declare_option(11_001_103, "Star Removed Event")
    REACTION = declare_option(11_001_200, "Reaction")
    REACTION_EVENT = declare_option(11_001_201, "Reaction Event")
    REACTION_ADDED_EVENT = declare_option(11_001_202, "Reaction Added Event")
    REACTION_REMOVED_EVENT = declare_option(11_001_203, "Reaction Removed Event")
    FOLLOW = declare_option(11_001_300, "Follow")
    FOLLOW_EVENT = declare_option(11_001_301, "Follow Event")
    FOLLOW_ADDED_EVENT = declare_option(11_001_302, "Follow Added Event")
    FOLLOW_REMOVED_EVENT = declare_option(11_001_303, "Follow Removed Event")
    NOTIFICATION = declare_option(11_001_400, "Notification")
    NOTIFICATION_EVENT = declare_option(11_001_401, "Notification Event")
    NOTIFICATION_SENT_EVENT = declare_option(11_001_402, "Notification Sent Event")
    NOTIFICATION_RESCINDED_EVENT = declare_option(11_001_403, "Notification Rescinded Event")
    NOTIFICATION_READ_EVENT = declare_option(11_001_404, "Notification Read Event")
    NOTIFICATION_DISMISSED_EVENT = declare_option(11_001_405, "Notification Dismissed Event")
    NOTIFICATION_EXPIRED_EVENT = declare_option(11_001_406, "Notification Expired Event")
    # FEED, FEED_ITEM, ...
    # THREAD, MESSAGE, ...
    # POLL, VOTE, REVIEW, RATING, RANK, ...
    # ACHIEVEMENT, BADGE, WISHLIST/WATCHLIST, ...

    # finance [11_100_000]
    # WALLET, BALANCE, BUDGET, TRANSFER, CREDIT, INVOICE, ...
    # PAYMENT_METHOD, PAYOUT, REFUND, ...

    #
    # SIMULATION
    #

    # geometry [20_000_000]
    # 2D
    ENTITY2D = declare_option(20_000_000, "Entity2D")
    SHAPE2D = declare_option(20_000_100, "Shape2D")
    POINT_SHAPE2D = declare_option(20_001_000, "Point Shape2D")
    CIRCLE_SHAPE2D = declare_option(20_001_100, "Circle Shape2D")
    CAPSULE_SHAPE2D = declare_option(20_001_200, "Capsule Shape2D")
    RECTANGLE_SHAPE2D = declare_option(20_001_300, "Rectangle Shape2D")
    SEGMENT_SHAPE2D = declare_option(20_001_400, "Segment Shape2D")
    PATH_SHAPE2D = declare_option(20_001_500, "Chain Shape2D")
    ELLIPSE_SHAPE2D = declare_option(20_001_600, "Ellipse Shape2D")
    HALFSPACE_SHAPE2D = declare_option(20_001_700, "Half Space Shape2D")
    CONVEX_SHAPE2D = declare_option(20_001_800, "Convex Polygon Shape2D")
    MESH_SHAPE2D = declare_option(20_001_900, "Mesh Shape2D")
    # 3D
    ENTITY3D = declare_option(20_010_000, "Entity3D")
    SHAPE3D = declare_option(20_010_100, "Shape3D")
    POINT_SHAPE3D = declare_option(20_011_000, "Point Shape3D")
    SPHERE_SHAPE3D = declare_option(20_011_100, "Sphere Shape3D")
    CAPSULE_SHAPE3D = declare_option(20_011_200, "Capsule Shape3D")
    BOX_SHAPE3D = declare_option(20_011_300, "Box Shape3D")
    SEGMENT_SHAPE3D = declare_option(20_011_400, "Segment Shape3D")
    POLYLINE_SHAPE3D = declare_option(20_011_500, "Polyline Shape3D")
    ELLIPSOID_SHAPE3D = declare_option(20_011_600, "Ellipsoid Shape3D")
    CYLINDER_SHAPE3D = declare_option(20_011_700, "Cylinder Shape3D")
    CONE_SHAPE3D = declare_option(20_011_800, "Cone Shape3D")
    PLANE_SHAPE3D = declare_option(20_011_900, "Plane Shape3D")
    CONVEX_SHAPE3D = declare_option(20_012_000, "Convex Hull Shape3D")
    MESH_SHAPE3D = declare_option(20_012_100, "Mesh Shape3D")
    # vectors
    # VECTOR_NETWORK, VECTOR_POINT, VECTOR_SEGMENT, VECTOR_REGION, ...

    # geography [20_100_000]
    # ...

    # physics [20_200_000]
    # 2D
    BODY2D = declare_option(20_200_000, "Body2D")
    BODY_EVENT = declare_option(20_200_001, "Body Event")
    BODY_SLEEP_EVENT = declare_option(20_200_002, "Body Sleep Event")
    BODY_WAKE_EVENT = declare_option(20_200_003, "Body Wake Event")
    # BODY_CONTACT_EVENT, BODY_CONTACT_STARTED, ...
    RIGID_BODY2D = declare_option(20_200_100, "Rigid Body2D")
    SOFT_BODY2D = declare_option(20_200_200, "Soft Body2D")
    JOINT2D = declare_option(20_201_000, "Joint2D")
    JOINT_EVENT = declare_option(20_201_001, "Joint Event")
    JOINT_BREAK_EVENT = declare_option(20_201_002, "Joint Break Event")
    HINGE_JOINT2D = declare_option(20_201_100, "Hinge Joint2D")
    PRISMATIC_JOINT2D = declare_option(20_201_200, "Prismatic Joint2D")
    FIXED_JOINT2D = declare_option(20_201_300, "Fixed Joint2D")
    ROPE_JOINT2D = declare_option(20_201_400, "Rope Joint2D")
    WHEEL_JOINT2D = declare_option(20_201_500, "Wheel Joint2D")
    # 3D
    BODY3D = declare_option(20_210_000, "Body3D")
    RIGID_BODY3D = declare_option(20_210_100, "Rigid Body3D")
    SOFT_BODY3D = declare_option(20_210_200, "Soft Body3D")
    JOINT3D = declare_option(20_211_000, "Joint3D")
    HINGE_JOINT3D = declare_option(20_211_100, "Hinge Joint3D")
    PRISMATIC_JOINT3D = declare_option(20_211_200, "Prismatic Joint3D")
    FIXED_JOINT3D = declare_option(20_211_300, "Fixed Joint3D")
    ROPE_JOINT3D = declare_option(20_211_400, "Rope Joint3D")
    WHEEL_JOINT3D = declare_option(20_211_500, "Wheel Joint3D")
    SPHERICAL_JOINT3D = declare_option(20_211_600, "Spherical Joint3D")
    # colliders
    COLLIDER2D = declare_option(20_220_000, "Collider2D")
    COLLIDER_EVENT = declare_option(20_220_001, "Collider Event")
    COLLIDER_CONTACT_EVENT = declare_option(20_220_002, "Collider Contact Event")
    COLLIDER3D = declare_option(20_220_100, "Collider3D")
    # COLLIDER_EVENT, COLLIDER_CONTACT_EVENT, COLLIDER_CONTACT_STARTED, ...

    # perception [20_300_000]
    INPUT_EVENT = declare_option(20_300_000, "Input Event")
    # SENSOR_EVENT, ...
    # pointer events
    POINTER2D = declare_option(20_300_100, "Pointer2D")
    POINTER_EVENT = declare_option(20_300_101, "Pointer Event")
    POINTER_DOWN_EVENT = declare_option(20_300_102, "Pointer Down Event")
    POINTER_UP_EVENT = declare_option(20_300_103, "Pointer Up Event")
    POINTER_MOVE_EVENT = declare_option(20_300_104, "Pointer Move Event")
    POINTER_ENTER_EVENT = declare_option(20_300_105, "Pointer Enter Event")
    POINTER_OVER_EVENT = declare_option(20_300_106, "Pointer Over Event")
    POINTER_LEAVE_EVENT = declare_option(20_300_107, "Pointer Leave Event")
    POINTER_LONG_PRESS_EVENT = declare_option(20_300_108, "Long Press Event")
    # mouse events
    MOUSE2D = declare_option(20_301_000, "Mouse")
    MOUSE_EVENT = declare_option(20_301_101, "Mouse Event")
    CLICK_EVENT = declare_option(20_301_102, "Click Event")
    SINGLE_CLICK_EVENT = declare_option(20_301_103, "Single Click Event")
    DOUBLE_CLICK_EVENT = declare_option(20_301_104, "Double Click Event")
    TRIPLE_CLICK_EVENT = declare_option(20_301_105, "Triple Click Event")
    WHEEL_EVENT = declare_option(20_301_106, "Wheel Event")
    # key events
    KEY_EVENT = declare_option(20_302_001, "Key Event")
    KEY_DOWN_EVENT = declare_option(20_302_002, "Key Down Event")
    KEY_UP_EVENT = declare_option(20_302_003, "Key Up Event")
    KEY_PRESS_EVENT = declare_option(20_302_004, "Key Press Event")
    # drag events
    DRAG_EVENT = declare_option(20_303_001, "Drag Event")
    DRAG_START_EVENT = declare_option(20_303_002, "Drag Start Event")
    DRAG_END_EVENT = declare_option(20_303_003, "Drag End Event")
    DRAG_OVER_EVENT = declare_option(20_303_004, "Drag Over Event")
    DRAG_ENTER_EVENT = declare_option(20_303_005, "Drag Enter Event")
    DRAG_LEAVE_EVENT = declare_option(20_303_006, "Drag Leave Event")
    DROP_EVENT = declare_option(20_303_007, "Drop Event")
    # clipboard events
    CLIPBOARD_EVENT = declare_option(20_304_001, "Clipboard Event")
    COPY_EVENT = declare_option(20_304_002, "Copy Event")
    CUT_EVENT = declare_option(20_304_003, "Cut Event")
    PASTE_EVENT = declare_option(20_304_004, "Paste Event")
    # focus events
    FOCUS_EVENT = declare_option(20_305_001, "Focus Event")
    FOCUS_IN_EVENT = declare_option(20_305_002, "Focus In Event")
    FOCUS_OUT_EVENT = declare_option(20_305_003, "Focus Out Event")
    # COMMAND, MODE, TOOL, SHORTCUT/KEYBINDING, ...
    # GESTURE, ...
    # CAMERA, SPEAKER, MICROPHONE, ...
    # ...

    #
    # IMAGINATION
    #

    # audio [30_000_000]
    AUDIO = declare_option(30_000_000, "Audio")
    # SOUND_SOURCE, ...

    # image [30_100_000]
    IMAGE = declare_option(30_100_000, "Image")
    # ...

    # video [30_200_000]
    VIDEO = declare_option(30_200_000, "Video")
    # VIDEO_STREAM, ...

    # model [30_300_000]
    MODEL = declare_option(30_300_000, "Model")
    # CSG, MODEL_ADD, MODEL_SUBTRACT, MODEL_INTERSECT, ...

    # paint [30_400_000]
    # RASTER/BITMAP, ...
    # DAB, PAINT, BRUSH, ...
    # SPRITE, SPRITE_SHEET, NINESLICE_SPRITE, TILING_SPRITE, ...
    # TEXTURE, ...

    # animation [30_500_000]
    TRANSITION_TEMPLATE = declare_option(30_500_000, "Transition Style")
    EFFECT_TEMPLATE = declare_option(30_500_100, "Effect Style")
    # ANIMATION, ANIMATION_TRACK, ANIMATION_KEYFRAME, ...
    # KEYFRAME_VARIANT, ...
    # RIG, ...

    # style [31_000_000]
    THEME = declare_option(31_000_000, "Theme")
    PALETTE = declare_option(31_000_100, "Palette")
    STYLE = declare_option(31_000_200, "Style")
    COLOR_STYLE = declare_option(31_001_000, "Color Style")
    FILL_STYLE = declare_option(31_001_100, "Fill Style")
    FONT_STYLE = declare_option(31_001_200, "Font Style")
    BORDER_STYLE = declare_option(31_001_300, "Border Style")
    SHADOW_STYLE = declare_option(31_001_400, "Shadow Style")
    GRADIENT_STYLE = declare_option(31_001_500, "Gradient Style")
    STROKE_STYLE = declare_option(31_001_600, "Stroke Style")

    # document [32_000_000]
    DOCUMENT = declare_option(32_000_000, "Document")
    # ...

    #
    # PRESENTATION
    #

    # scene [40_000_000]
    STAGE = declare_option(40_000_000, "Stage")
    SCENE = declare_option(40_000_100, "Scene", description="Scene of an Application")
    SCENE_EVENT = declare_option(40_000_101, "Scene Event")
    LAYER = declare_option(40_000_200, "Layer", description="Layer of a Scene")
    # VIEW_VARIANT, BREAKPOINT_VARIANT, ...
    # VIEWPORT, OVERLAY, WIDGET, HUD, ...
    # ROOM, ...
    # FORM, MENU, INVENTORY, ...

    # view [40_100_000]
    # container views (2D)
    VIEW2D = declare_option(40_100_000, "View", description="View in a Scene")
    LAYOUT_VIEW2D = declare_option(40_100_100, "Container View")
    FRAME_VIEW2D = declare_option(40_100_200, "Frame View", description="Fixed Container")
    LABEL_VIEW2D = declare_option(40_100_300, "Label View", description="Label Container")
    SPLIT_VIEW2D = declare_option(40_100_400, "Split View", description="Split Container")
    # SLOT_DEFINITION_VIEW2D, SLOT_VIEW2D, ...
    # FORM_VIEW2D, MENU_VIEW2D, ...
    # TAB_VIEW2D, ...
    # DRAWER_VIEW2D, SPLIT_DRAWER_VIEW2D, GRID/GRID_ELEMENT_VIEW2D, ...
    # POPOVER_VIEW2D, SHEET_VIEW2D, ALERT_VIEW2D, HUD_VIEW2D, ...
    # content views (2D)
    CONTENT_VIEW2D = declare_option(40_100_500, "Content View")
    TEXT_VIEW2D = declare_option(40_100_501, "Text View", description="Text")
    # CODE_VIEW2D, DOCUMENT_VIEW2D, ...
    # IMAGE_VIEW2D, AUDIO_VIEW2D, VIDEO_VIEW2D, ...
    # input views (2D)
    INPUT_VIEW2D = declare_option(40_100_600, "Input View")
    NUMBER_INPUT_VIEW2D = declare_option(
        40_100_601, "Number Input View", description="Number Input"
    )
    SLIDER_INPUT_VIEW2D = declare_option(
        40_100_602, "Slider Input View", description="Slider Input"
    )
    # TEXT_INPUT_VIEW2D, TOGGLE_INPUT_VIEW2D, PICKER_INPUT_VIEW2D, COLOR_INPUT_VIEW2D, ...
    # FILE_INPUT_VIEW2D, DATETIME_INPUT_VIEW2D, DURATION_INPUT_VIEW2D, ...

    # rendering [40_200_000]
    # ...

    # camera [40_300_000]
    # ...

    # shaders [40_400_000]
    # ...

    # material [40_500_000]
    # ...

    # lighting [40_600_000]
    # LIGHT, LIGHT2D, ...
    # POINT_LIGHT2D, DIRECTIONAL_LIGHT2D, SPOT_LIGHT2D, AMBIENT_LIGHT2D, ...
    # OCCLUDER, ...
    # ...

    #
    # PRODUCTION
    #

    # cloud [50_000_000]
    MACHINE = declare_option(50_000_000, "Machine", description="Machine for ephemeral computing")
    # DATABASE, SEARCH, VAULT, CACHE, S3, ...
    # HOST, ENDPOINT, NETWORK, AUTOSCALER, ...

    # observability [50_100_000]
    # metric
    METRIC = declare_option(50_100_000, "Metric")
    MEASUREMENT_EVENT = declare_option(50_100_001, "Measurement of a Metric")
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
    LOCALE = declare_option(60_000_000, "Locale")
    # LOCALIZATION, STRING, TRANSLATION, ...
    # LOCALIZATION_VARIANT, GEO_VARIANT, ...

    # legal [60_100_000]
    # ...

    # commerce [60_200_000]
    # TIER, SUBSCRIPTION, PRODUCT, PRICE, ...
    # ORDER, DISCOUNT, DISPUTE, ...


@declare_enum(EnumType.STRUCT_TYPE)
class StructType(OptionEnum):
    #
    # CORE
    #

    # builtin [1]
    # root
    STRUCT = declare_option(1, "Struct", description="Root of all Structs")
    ERROR = declare_option(2, "Error", description="Error")
    MESSAGE = declare_option(3, "Message", description="Message")

    # error [10_000]

    # definition [100_000]
    # nocheckin: remove definition, spread definitions into relevant custom stuff?
    #  (should map for everything? if not do we need those Nodes?)
    #  (also maybe invert *_definition and *_custom_definition so that custom has no prefix
    #    but *_builtin does?)
    #  (where does Entity belong anyway?)
    DEFINITION = declare_option(100_000)
    SCHEMA_DEFINITION = declare_option(100_001)
    MODULE_DEFINITION = declare_option(100_002)
    OBJECT_DEFINITION = declare_option(100_003)
    NODE_DEFINITION = declare_option(100_004)
    STRUCT_DEFINITION = declare_option(100_005)
    HANDLE_DEFINITION = declare_option(100_006)
    ENUM_DEFINITION = declare_option(100_007)
    PROPERTY_DEFINITION = declare_option(100_100)
    CONSTANT_DEFINITION = declare_option(100_101)
    OPTION_DEFINITION = declare_option(100_102)
    # ALIAS_DEFINITION, UNION_DEFINITION, ...
    # EXPECTATION_DEFINITION, ...
    # MUTATION_DEFINITION, ...

    # common [200_000]
    # type/value
    TYPE = declare_option(200_000)
    NUMBER_CONSTRAINT = declare_option(200_001)
    STRING_CONSTRAINT = declare_option(200_002)
    COLLECTION_CONSTRAINT = declare_option(200_003)
    VALUE = declare_option(200_050)
    NAMED_VALUE = declare_option(200_051)
    # expressions
    EXPRESSION = declare_option(200_100)
    JOIN = declare_option(200_101)
    AGGREGATION = declare_option(200_102)
    CONDITION = declare_option(200_103)
    SORT = declare_option(200_104)
    SELECT = declare_option(200_105)
    # query
    QUERY = declare_option(200_200)
    # references
    NODE_IDENTITY_REFERENCE = declare_option(200_300)
    NODE_SPATIAL_REFERENCE = declare_option(200_301)
    NODE_TEMPORAL_REFERENCE = declare_option(200_302)
    # NODE_PATH, NODE_PATH_TOKEN, ...
    PROPERTY_REFERENCE = declare_option(200_360)
    # text
    TEXT = declare_option(200_400)
    TEXT_SPAN = declare_option(200_401)
    # icon
    ICON = declare_option(200_500)
    # change
    EDIT_OPERATION = declare_option(201_100)

    # encoding [1_000_000]
    # ...

    # persistence [1_100_000]
    # ...

    # generation [2_000_000]
    # ...

    # local [2_100_000]
    # ...

    # universe [3_000_000]
    UNIVERSE_SIGNUP_REQUEST = declare_option(3_000_000)
    UNIVERSE_SIGNUP_RESPONSE = declare_option(3_000_001)
    UNIVERSE_SPAWN_REQUEST = declare_option(3_000_002)
    UNIVERSE_SPAWN_RESPONSE = declare_option(3_000_003)
    # ...

    # space [3_100_000]
    # ...

    #
    # BASICS
    #

    # entity [10_000_000]
    # custom
    CUSTOM_STRUCT = declare_option(
        10_000_100,
        "Custom Struct",
        description="Custom Struct Instance",
    )
    CUSTOM_ERROR = declare_option(
        10_000_200,
        "Custom Error",
        description="Custom Error Instance",
    )
    CUSTOM_MESSAGE = declare_option(
        10_000_300,
        "Custom Message",
        description="Custom Message Instance",
    )
    TAG_DEFINITION = declare_option(10_010_000)
    INDEX_DEFINITION = declare_option(10_010_100)
    CONSTRAINT_DEFINITION = declare_option(10_010_200)
    MIGRATION_DEFINITION = declare_option(10_011_000)
    MIGRATION_OPERATION_DEFINITION = declare_option(10_011_100)
    # ...

    # script [10_100_000]
    SCHEDULE = declare_option(10_100_810)
    FUNCTION_DEFINITION = declare_option(10_101_000)
    METHOD_DEFINITION = declare_option(10_101_100)
    ACTION_DEFINITION = declare_option(10_101_200)
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
    MESH2 = declare_option(20_000_030)
    MESH3 = declare_option(20_000_031)
    # 2D
    FORM2D = declare_option(20_000_100, "Form2D")
    POINT2D = declare_option(20_001_000, "Point2D")
    CIRCLE2D = declare_option(20_001_100, "Circle2D")
    CAPSULE2D = declare_option(20_001_200, "Capsule2D")
    RECTANGLE2D = declare_option(20_001_300, "Rectangle2D")
    SEGMENT2D = declare_option(20_001_400, "Segment2D")
    PATH2D = declare_option(20_001_500, "Path2D")
    ELLIPSE2D = declare_option(20_001_600, "Ellipse2D")
    HALFSPACE2D = declare_option(20_001_700, "Half Space2D")
    # 3D
    FORM3D = declare_option(20_010_300, "Form3D")
    POINT3D = declare_option(20_011_000, "Point3D")
    SPHERE3D = declare_option(20_011_100, "Sphere3D")
    CAPSULE3D = declare_option(20_011_200, "Capsule3D")
    BOX3D = declare_option(20_011_300, "Box3D")
    SEGMENT3D = declare_option(20_011_400, "Segment3D")
    POLYLINE3D = declare_option(20_011_500, "Polyline3D")
    ELLIPSOID3D = declare_option(20_011_600, "Ellipsoid3D")
    CYLINDER3D = declare_option(20_011_700, "Cylinder3D")
    CONE3D = declare_option(20_011_800, "Cone3D")
    PLANE3D = declare_option(20_011_900, "Plane3D")

    # geography [20_100_000]
    # ...

    # physics [20_200_000]
    JOINT_SPRING = declare_option(20_201_001, "Joint Spring")
    JOINT_SCALAR_LIMIT = declare_option(20_201_002, "Joint Scalar Limit")
    JOINT_CONE_LIMIT = declare_option(20_201_003, "Joint Cone Limit")
    JOINT_TWIST_LIMIT = declare_option(20_201_004, "Joint Cone Limit")
    JOINT_BREAK_LIMIT = declare_option(20_201_005, "Joint Break Limit")
    JOINT_MOTOR = declare_option(20_201_006, "Joint Motor")
    JOINT_FRAME2D = declare_option(20_201_007, "Joint Frame2D")
    JOINT_FRAME3D = declare_option(20_201_008, "Joint Frame3D")

    # perception [20_300_000]
    # ...

    #
    # IMAGINATION
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

    # animation [30_500_000]
    TRANSITION = declare_option(30_500_000, "Transition")
    EFFECT = declare_option(30_500_100, "Effect")

    # style [31_000_000]
    COLOR = declare_option(31_001_000, "Color")
    FILL = declare_option(31_001_100, "Fill")
    FONT = declare_option(31_001_200, "Font")
    BORDER = declare_option(31_001_300, "Border")
    SHADOW = declare_option(31_001_400, "Shadow")
    GRADIENT = declare_option(31_001_500, "Gradient")
    GRADIENT_STOP = declare_option(31_001_501, "Gradient Stop")
    STROKE = declare_option(31_001_600, "Stroke")
    STROKE_CAP = declare_option(31_001_601, "Stroke Cap")
    STROKE_PATH = declare_option(31_001_602, "Stroke Path")
    STROKE_POINT = declare_option(31_001_603, "Stroke Point")

    # document [32_000_000]
    # ...

    #
    # PRESENTATION
    #

    # scene [40_000_000]
    # ...

    # view [40_100_000]
    LENGTH = declare_option(40_100_000, "Length")
    OFFSET2 = declare_option(40_100_001, "Position")
    GRID2 = declare_option(40_100_002, "Grid")
    GRID_SPAN2 = declare_option(40_100_003, "Grid Span")
    INSET2 = declare_option(40_100_004, "Insets")
    CORNER2 = declare_option(40_100_005, "Corners")
    AXIS2 = declare_option(40_100_006, "Axis2")
    AXIS3 = declare_option(40_100_007, "Axis3")

    # rendering [40_200_000]
    # ...

    # camera [40_300_000]
    # ...

    # shaders [40_400_000]
    # ...

    # material [40_500_000]
    # ...

    # lighting [40_600_000]
    # ...

    #
    # PRODUCTION
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

    # commerce [60_200_000]
    # ...


@declare_enum(EnumType.HANDLE_TYPE)
class HandleType(OptionEnum):
    """The type of a Handle."""

    #
    # CORE
    #

    HANDLE = declare_option(1)

    # persistence [1_100_000]
    GRAPH = declare_option(1_100_000)
    # CONNECTION, STREAM, TRANSPORT, CHANNEL, ...?

    # local [2_100_000]
    SESSION = declare_option(2_100_000)
    CONTEXT = declare_option(2_100_100)
    LOGGER = declare_option(2_100_200)
    TRACER = declare_option(2_100_300)
