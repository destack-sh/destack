# ruff: noqa

from typing import TYPE_CHECKING, Union

VERSION = "2024.09.24.1"

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
from .common_pb2 import *
from .lang_pb2 import *
from .health_pb2 import *
from .system_pb2 import *
from .runtime_pb2 import *

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
    SignalData,
    LogData,
    NotificationData,
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
    ScheduleData,
    FileInfoData,
    FileReferenceData,
    IconData,
    SecretReferenceData,
    TriggerInfoData,
    ExpressionData,
    AggregationData,
    SelectionData,
    QueryInfoData,
    ReadOptionsData,
    ValueData,
    ComputedValueData,
    CodeData,
    CodeLineData,
    PortKeyData,
    PortData,
    RunErrorData,
    RunOptionsData,
    RunAttemptData,
    RunTraceData,
    RunFrameData,
    RunSpanData,
    RunEventData,
    BreakpointData,
    LogInfoData,
    ColorData,
    FontData,
    BoxData,
    OffsetData,
    TransformData,
    Vector2Data,
    Vector3Data,
    Vector4Data,
    LineData,
    StartViewStateData,
    FeedViewStateData,
    UserWizardViewStateData,
    TreeViewStateData,
]
