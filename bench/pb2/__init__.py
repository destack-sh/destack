# ruff: noqa

from typing import TYPE_CHECKING, Union

VERSION = "2025.05.19.0"

# import from all generated files
from .runtime_pb2 import *
from .health_pb2 import *
from .common_pb2 import *
from .common_grpc import *
from .lang_grpc import *
from .runtime_grpc import *
from .system_grpc import *
from .health_grpc import *
from .lang_pb2 import *
from .system_pb2 import *
from .google.type.date_pb2 import *
from .google.type.timeofday_pb2 import *
from .google.type.datetime_pb2 import *

# extra utility types
AnyNodeData = Union[
    ClaimData,
    RoleData,
    TeamData,
    BenchData,
    BenchInviteData,
    BenchMembershipData,
    ClientData,
    HandleData,
    OrganizationData,
    OrganizationInviteData,
    OrganizationMembershipData,
    UserData,
    FieldData,
    FileData,
    LinkData,
    RecordData,
    SchemaData,
    TableData,
    ComputerData,
    DatabaseData,
    ActionData,
    AgentData,
    CursorData,
    FlowData,
    FlowEdgeData,
    ServiceData,
    TaskData,
    ApplicationData,
    BlockData,
    DependencyData,
    PackageData,
    PageData,
    InterruptionData,
    RunData,
    SpanData,
    ChannelData,
    MessageData,
    NotificationData,
    ThreadData,
    RouteData,
    SceneData,
    SpaceData,
    ColorStyleData,
    BorderStyleData,
    TransitionStyleData,
    EffectStyleData,
    GradientStyleData,
    FontStyleData,
    ShadowStyleData,
    ThemeData,
    FrameViewData,
    LabelViewData,
    SplitViewData,
    TextViewData,
    NumberInputViewData,
    SliderInputViewData,
    WizardViewData,
    ThreadViewData,
]
AnyStructData = Union[
    ScopeData,
    PropertyReferenceData,
    CodeData,
    StringConstraintData,
    NumberConstraintData,
    CollectionConstraintData,
    NodeConstraintData,
    TypeData,
    NodeReferenceData,
    EditData,
    IconData,
    ValueData,
    RelationReferenceData,
    AttributeReferenceData,
    FunctionData,
    ConditionData,
    AggregationData,
    ExpressionData,
    SortData,
    JoinData,
    QueryData,
    SelectionData,
    TextSpanData,
    TextLineData,
    TextData,
    VariableData,
    OriginData,
    ScheduleData,
    RunTraceData,
    RunFrameData,
    ErrorData,
    ColorData,
    LengthData,
    PositionData,
    DimensionData,
    InsetsData,
    CornersData,
    Axis2Data,
    Axis3Data,
    Vector2Data,
    Vector3Data,
    Vector4Data,
    GridData,
    GridSpanData,
    BorderData,
    TransitionData,
    EffectData,
    GradientStopData,
    GradientData,
    FillData,
    FontData,
    ShadowData,
]
AnyObjectData = AnyNodeData | AnyStructData
