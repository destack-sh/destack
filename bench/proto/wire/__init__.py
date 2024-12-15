# ruff: noqa

from typing import TYPE_CHECKING, Union

VERSION = "2024.12.15.2"

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
    ServerData,
    StoreData,
    DriveData,
    VaultData,
    CacheData,
    MachineData,
    BrowserData,
    FileData,
    StreamData,
    SecretData,
    PackageData,
    DependencyData,
    SpaceData,
    BlockData,
    TriggerData,
    FieldData,
    QueryData,
    ViewData,
    StepData,
    PipeData,
    BadgeData,
    MessageData,
    RecordData,
    SessionData,
    RunData,
    InterruptData,
    LogData,
    SkipData,
]
AnyStructData = Union[
    RuntimeContextData,
    EditContextData,
    EditData,
    EditInfoData,
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
    TypeInfoData,
    TypeConstraintData,
    VariableObjectData,
    MemberObjectData,
    InputObjectData,
    OutputObjectData,
    ScheduleData,
    FileInfoData,
    IconData,
    TextData,
    TextLineData,
    TextSpanData,
    CodeData,
    CodeLineData,
    PathData,
    PathTokenData,
    ExpressionData,
    AggregationResultData,
    SelectionData,
    SelectOptionsData,
    ValueData,
    ObjectMappingData,
    FieldMappingData,
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
    ContinueData,
    CallData,
    ContextData,
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
]
AnyObjectData = AnyNodeData | AnyStructData
