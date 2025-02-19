# ruff: noqa

from typing import TYPE_CHECKING, Union

VERSION = "2025.02.19.1"

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
    BenchData,
    HandleData,
    UserData,
    OrganizationData,
    TeamData,
    ClientData,
    ScalerData,
    StoreData,
    MachineData,
    BrowserData,
    FileData,
    StreamData,
    SecretData,
    PackageData,
    DependencyData,
    PageData,
    BlockData,
    ChoiceData,
    ClassData,
    FieldData,
    FlowData,
    ActionData,
    PipeData,
    TriggerData,
    ViewData,
    DatabaseData,
    ChannelData,
    RoleData,
    IdentityData,
    SpaceData,
    ThreadData,
    MessageData,
    NotificationData,
    RecordData,
    MembershipData,
    InviteData,
    SessionData,
    RunData,
    RunSpanData,
    InterruptionData,
    LogData,
    PlanData,
    SkipData,
    EmptyData,
]
AnyStructData = Union[
    ContextData,
    EditContextData,
    EditData,
    EditOperationData,
    ChangeData,
    ChangeVignetteData,
    GraphScopeData,
    ClientOriginData,
    NodeReferenceData,
    PropertyReferenceData,
    PolicyData,
    PolicyRuleData,
    SubjectData,
    AccessZoneData,
    AccessMatrixData,
    AccessData,
    MachineImageData,
    TypeData,
    TypeConstraintData,
    FileInfoData,
    IconData,
    ScheduleData,
    TextData,
    TextLineData,
    TextSpanData,
    TextTableData,
    TextCellData,
    CodeData,
    CodeLineData,
    PathData,
    PathElementData,
    ExpressionData,
    AggregationResultData,
    SelectionData,
    SelectOptionsData,
    ValueData,
    ComputedValueData,
    ErrorData,
    RunOptionsData,
    RunTraceData,
    RunFrameData,
    CallData,
    CallPlanData,
    BreakpointData,
    TextOptionsData,
    AudioOptionsData,
    ImageOptionsData,
    VideoOptionsData,
    ToolSelectionData,
    ColorData,
    FontData,
    RectangleData,
    OffsetData,
    TransformData,
    Vector2Data,
    Vector3Data,
    Vector4Data,
    LineData,
    RectangleConstraintData,
    DomNodeData,
]
AnyObjectData = AnyNodeData | AnyStructData
BenchNodeData = Union[
    BenchData,
    HandleData,
    ScalerData,
    StoreData,
    MachineData,
    BrowserData,
    FileData,
    StreamData,
    SecretData,
    PackageData,
    DependencyData,
    PageData,
    BlockData,
    ChoiceData,
    ClassData,
    FieldData,
    FlowData,
    ActionData,
    PipeData,
    TriggerData,
    ViewData,
    DatabaseData,
    ChannelData,
    RoleData,
    IdentityData,
    SpaceData,
    ThreadData,
    MessageData,
    NotificationData,
    RecordData,
    MembershipData,
    InviteData,
    SessionData,
    RunData,
    RunSpanData,
    InterruptionData,
    LogData,
    PlanData,
]
ResourceNodeData = Union[
    ScalerData, StoreData, MachineData, BrowserData, FileData, StreamData, SecretData
]
DynamicResourceNodeData = Union[MachineData, BrowserData, FileData, StreamData, SecretData]
StaticResourceNodeData = Union[ScalerData, StoreData]
SourceNodeData = Union[
    PackageData,
    DependencyData,
    PageData,
    BlockData,
    ChoiceData,
    ClassData,
    FieldData,
    FlowData,
    ActionData,
    PipeData,
    TriggerData,
    ViewData,
    DatabaseData,
    ChannelData,
    RoleData,
    IdentityData,
    SpaceData,
]
InlineSourceNodeData = Union[
    PageData,
    ChoiceData,
    ClassData,
    FlowData,
    ViewData,
    DatabaseData,
    ChannelData,
    RoleData,
    IdentityData,
]
TypeBaseNodeData = Union[ChoiceData, ClassData, FlowData, ActionData, DatabaseData]
StateNodeData = Union[ThreadData, MessageData, NotificationData, RecordData]
RuntimeNodeData = Union[SessionData, RunData, RunSpanData, InterruptionData, LogData, PlanData]
