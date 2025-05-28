
# ruff: noqa

from typing import TYPE_CHECKING, Union

VERSION = '2025.05.28.1'

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
AnyNodeData = Union[ClientData, OrganizationData, OrganizationInviteData, OrganizationMembershipData, UserData, BenchData, BenchInviteData, BenchMembershipData, HandleData, PackageData, PackageMembershipData, PackageInviteData, FieldData, FileData, LinkData, RecordData, SchemaData, TableData, DatabaseData, MachineData, ActionData, AgentData, CursorData, FlowData, FlowEdgeData, ServiceData, TaskData, InterruptionData, RunData, SpanData, MessageData, ThreadData, BlockData, PageData, RouteData, SceneData, SpaceData, ColorStyleData, BorderStyleData, TransitionStyleData, EffectStyleData, GradientStyleData, FontStyleData, ShadowStyleData, ThemeData, FrameViewData, LabelViewData, SplitViewData, TextViewData, NumberInputViewData, SliderInputViewData, WizardViewData, ThreadViewData]
AnyStructData = Union[ScopeData, PropertyReferenceData, NodeReferenceData, CodeData, StringConstraintData, NumberConstraintData, CollectionConstraintData, NodeConstraintData, TypeData, EditData, ChangeData, ChangeResultData, IconData, ValueData, RelationReferenceData, AttributeReferenceData, FunctionData, ConditionData, AggregationData, ExpressionData, SortData, JoinData, QueryData, QueryResultData, QueryUpdateData, SelectionData, TextSpanData, TextLineData, TextData, VariableData, OriginData, ScheduleData, ErrorData, ColorData, LengthData, PositionData, DimensionData, InsetsData, CornersData, Axis2Data, Axis3Data, Vector2Data, Vector3Data, Vector4Data, GridData, GridSpanData, BorderData, TransitionData, EffectData, GradientStopData, GradientData, FillData, FontData, ShadowData]
AnyObjectData = AnyNodeData | AnyStructData
