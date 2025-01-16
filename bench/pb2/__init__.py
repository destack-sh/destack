# ruff: noqa

from typing import TYPE_CHECKING, Union

VERSION = "2025.01.16.1"

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
    UserData,
    HandleData,
    ClientData,
    OrganizationData,
    MembershipData,
    InviteData,
    ScalerData,
    StoreData,
    MachineData,
    BrowserData,
    FileData,
    StreamData,
    SecretData,
    PackageData,
    DependencyData,
    SpaceData,
    BlockData,
    FieldData,
    ViewData,
    ActionData,
    PipeData,
    MessageData,
    RecordData,
    SessionData,
    RunData,
    InterruptionData,
    LogData,
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
    ScheduleData,
    FileInfoData,
    IconData,
    TextData,
    TextLineData,
    TextSpanData,
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
    ObjectMappingData,
    RunErrorData,
    RunOptionsData,
    RunAttemptData,
    RunTraceData,
    RunFrameData,
    RunSpanData,
    RunEventData,
    TextOptionsData,
    AudioOptionsData,
    ImageOptionsData,
    VideoOptionsData,
    CallData,
    BreakpointData,
    LogInfoData,
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
    ClientData,
    MembershipData,
    InviteData,
    ScalerData,
    StoreData,
    MachineData,
    BrowserData,
    FileData,
    StreamData,
    SecretData,
    PackageData,
    DependencyData,
    SpaceData,
    BlockData,
    FieldData,
    ViewData,
    ActionData,
    PipeData,
    MessageData,
    RecordData,
    SessionData,
    RunData,
    InterruptionData,
    LogData,
]
PackageNodeData = Union[
    DependencyData, SpaceData, BlockData, FieldData, ViewData, ActionData, PipeData
]
ResourceNodeData = Union[
    ScalerData, StoreData, MachineData, BrowserData, FileData, StreamData, SecretData
]
DynamicResourceNodeData = Union[MachineData, BrowserData, FileData, StreamData, SecretData]
StaticResourceNodeData = Union[ScalerData, StoreData]
SourceNodeData = Union[
    DependencyData, SpaceData, BlockData, FieldData, ViewData, ActionData, PipeData
]
StateNodeData = Union[MessageData, RecordData]
RuntimeNodeData = Union[SessionData, RunData, InterruptionData, LogData]
