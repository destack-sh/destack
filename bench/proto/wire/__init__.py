# ruff: noqa

from typing import TYPE_CHECKING, Union

VERSION = "2024.11.18.0"

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
    OrganizationData,
    HandleData,
    ClientData,
    ServerData,
    StoreData,
    MachineData,
    DriveData,
    VaultData,
    CacheData,
    FileData,
    SecretData,
    BrowserData,
    MembershipData,
    InviteData,
    BranchData,
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
    SessionContextData,
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
    PathData,
    PathTokenData,
    PolicyData,
    PolicyRuleData,
    SubjectData,
    AccessZoneData,
    AccessMatrixData,
    AccessData,
    TextData,
    TextLineData,
    TextSpanData,
    TypeInfoData,
    TypeConstraintData,
    VariableObjectData,
    MemberObjectData,
    InputObjectData,
    OutputObjectData,
    ScheduleData,
    FileInfoData,
    FileReferenceData,
    IconData,
    SecretReferenceData,
    ExpressionData,
    AggregationResultData,
    SelectionData,
    SelectOptionsData,
    ValueData,
    ComputedValueData,
    ObjectMappingData,
    FieldMappingData,
    CodeData,
    CodeLineData,
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
