# type: ignore
# ruff: noqa

from typing import TYPE_CHECKING, Union, AsyncIterator, Mapping

from google.protobuf import timestamp_pb2 as _timestamp_pb2
from google.protobuf import duration_pb2 as _duration_pb2
from google.protobuf import struct_pb2 as _struct_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import (
    ClassVar as _ClassVar,
    Iterable as _Iterable,
    Mapping as _Mapping,
    Optional as _Optional,
    Union as _Union,
)

DESCRIPTOR: _descriptor.FileDescriptor

class AccessKind(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ACCESS_KIND_UNSPECIFIED: _ClassVar[AccessKind]
    ACCESS_KIND_READ: _ClassVar[AccessKind]
    ACCESS_KIND_EDIT: _ClassVar[AccessKind]
    ACCESS_KIND_USE: _ClassVar[AccessKind]

class AccessMode(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ACCESS_MODE_UNSPECIFIED: _ClassVar[AccessMode]
    ACCESS_MODE_ADAPTIVE: _ClassVar[AccessMode]
    ACCESS_MODE_ATOMIC: _ClassVar[AccessMode]

class AccessType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ACCESS_TYPE_UNSPECIFIED: _ClassVar[AccessType]
    ACCESS_TYPE_GET: _ClassVar[AccessType]
    ACCESS_TYPE_SEARCH: _ClassVar[AccessType]
    ACCESS_TYPE_AGGREGATE: _ClassVar[AccessType]
    ACCESS_TYPE_CREATE: _ClassVar[AccessType]
    ACCESS_TYPE_UPSERT: _ClassVar[AccessType]
    ACCESS_TYPE_UPDATE: _ClassVar[AccessType]
    ACCESS_TYPE_MOVE: _ClassVar[AccessType]
    ACCESS_TYPE_ARCHIVE: _ClassVar[AccessType]
    ACCESS_TYPE_UNARCHIVE: _ClassVar[AccessType]
    ACCESS_TYPE_DELETE: _ClassVar[AccessType]
    ACCESS_TYPE_RESTORE: _ClassVar[AccessType]
    ACCESS_TYPE_ERASE: _ClassVar[AccessType]
    ACCESS_TYPE_START: _ClassVar[AccessType]
    ACCESS_TYPE_PAUSE: _ClassVar[AccessType]
    ACCESS_TYPE_RESUME: _ClassVar[AccessType]
    ACCESS_TYPE_STOP: _ClassVar[AccessType]
    ACCESS_TYPE_KILL: _ClassVar[AccessType]
    ACCESS_TYPE_SEND: _ClassVar[AccessType]
    ACCESS_TYPE_RECEIVE: _ClassVar[AccessType]

class AggregationOp(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    AGGREGATION_OP_UNSPECIFIED: _ClassVar[AggregationOp]
    AGGREGATION_OP_EXISTS: _ClassVar[AggregationOp]
    AGGREGATION_OP_COUNT: _ClassVar[AggregationOp]
    AGGREGATION_OP_SUM: _ClassVar[AggregationOp]
    AGGREGATION_OP_MIN: _ClassVar[AggregationOp]
    AGGREGATION_OP_MAX: _ClassVar[AggregationOp]
    AGGREGATION_OP_AVERAGE: _ClassVar[AggregationOp]
    AGGREGATION_OP_MEDIAN: _ClassVar[AggregationOp]
    AGGREGATION_OP_HISTOGRAM: _ClassVar[AggregationOp]

class Alignment(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ALIGNMENT_UNSPECIFIED: _ClassVar[Alignment]
    ALIGNMENT_START: _ClassVar[Alignment]
    ALIGNMENT_MIDDLE: _ClassVar[Alignment]
    ALIGNMENT_END: _ClassVar[Alignment]
    ALIGNMENT_SPACE_BETWEEN: _ClassVar[Alignment]

class Anchor(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ANCHOR_UNSPECIFIED: _ClassVar[Anchor]
    ANCHOR_TOP: _ClassVar[Anchor]
    ANCHOR_TOP_LEFT: _ClassVar[Anchor]
    ANCHOR_TOP_RIGHT: _ClassVar[Anchor]
    ANCHOR_RIGHT: _ClassVar[Anchor]
    ANCHOR_RIGHT_TOP: _ClassVar[Anchor]
    ANCHOR_RIGHT_BOTTOM: _ClassVar[Anchor]
    ANCHOR_BOTTOM: _ClassVar[Anchor]
    ANCHOR_BOTTOM_LEFT: _ClassVar[Anchor]
    ANCHOR_BOTTOM_RIGHT: _ClassVar[Anchor]
    ANCHOR_LEFT: _ClassVar[Anchor]
    ANCHOR_LEFT_TOP: _ClassVar[Anchor]
    ANCHOR_LEFT_BOTTOM: _ClassVar[Anchor]

class BenchType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    BENCH_TYPE_UNSPECIFIED: _ClassVar[BenchType]
    BENCH_TYPE_BENCH: _ClassVar[BenchType]
    BENCH_TYPE_USER: _ClassVar[BenchType]
    BENCH_TYPE_ORGANIZATION: _ClassVar[BenchType]
    BENCH_TYPE_HANDLE: _ClassVar[BenchType]
    BENCH_TYPE_CLIENT: _ClassVar[BenchType]
    BENCH_TYPE_SERVER: _ClassVar[BenchType]
    BENCH_TYPE_STORE: _ClassVar[BenchType]
    BENCH_TYPE_MACHINE: _ClassVar[BenchType]
    BENCH_TYPE_DRIVE: _ClassVar[BenchType]
    BENCH_TYPE_VAULT: _ClassVar[BenchType]
    BENCH_TYPE_CACHE: _ClassVar[BenchType]
    BENCH_TYPE_FILE: _ClassVar[BenchType]
    BENCH_TYPE_SECRET: _ClassVar[BenchType]
    BENCH_TYPE_MEMBERSHIP: _ClassVar[BenchType]
    BENCH_TYPE_INVITE: _ClassVar[BenchType]
    BENCH_TYPE_BRANCH: _ClassVar[BenchType]
    BENCH_TYPE_PACKAGE: _ClassVar[BenchType]
    BENCH_TYPE_DEPENDENCY: _ClassVar[BenchType]
    BENCH_TYPE_SPACE: _ClassVar[BenchType]
    BENCH_TYPE_BLOCK: _ClassVar[BenchType]
    BENCH_TYPE_TRIGGER: _ClassVar[BenchType]
    BENCH_TYPE_FIELD: _ClassVar[BenchType]
    BENCH_TYPE_QUERY: _ClassVar[BenchType]
    BENCH_TYPE_VIEW: _ClassVar[BenchType]
    BENCH_TYPE_STEP: _ClassVar[BenchType]
    BENCH_TYPE_PIPE: _ClassVar[BenchType]
    BENCH_TYPE_BADGE: _ClassVar[BenchType]
    BENCH_TYPE_MESSAGE: _ClassVar[BenchType]
    BENCH_TYPE_RECORD: _ClassVar[BenchType]
    BENCH_TYPE_SESSION: _ClassVar[BenchType]
    BENCH_TYPE_RUN: _ClassVar[BenchType]
    BENCH_TYPE_SIGNAL: _ClassVar[BenchType]
    BENCH_TYPE_LOG: _ClassVar[BenchType]
    BENCH_TYPE_NOTIFICATION: _ClassVar[BenchType]
    BENCH_TYPE_SKIP: _ClassVar[BenchType]
    BENCH_TYPE_SESSION_CONTEXT: _ClassVar[BenchType]
    BENCH_TYPE_EDIT_CONTEXT: _ClassVar[BenchType]
    BENCH_TYPE_EDIT: _ClassVar[BenchType]
    BENCH_TYPE_EDIT_INFO: _ClassVar[BenchType]
    BENCH_TYPE_EDIT_OPERATION: _ClassVar[BenchType]
    BENCH_TYPE_CHANGE: _ClassVar[BenchType]
    BENCH_TYPE_CHANGE_VIGNETTE: _ClassVar[BenchType]
    BENCH_TYPE_GRAPH_SCOPE: _ClassVar[BenchType]
    BENCH_TYPE_CLIENT_ORIGIN: _ClassVar[BenchType]
    BENCH_TYPE_NODE_REFERENCE: _ClassVar[BenchType]
    BENCH_TYPE_PROPERTY_REFERENCE: _ClassVar[BenchType]
    BENCH_TYPE_PATH: _ClassVar[BenchType]
    BENCH_TYPE_PATH_TOKEN: _ClassVar[BenchType]
    BENCH_TYPE_POLICY: _ClassVar[BenchType]
    BENCH_TYPE_POLICY_RULE: _ClassVar[BenchType]
    BENCH_TYPE_SUBJECT: _ClassVar[BenchType]
    BENCH_TYPE_ACCESS_ZONE: _ClassVar[BenchType]
    BENCH_TYPE_ACCESS_MATRIX: _ClassVar[BenchType]
    BENCH_TYPE_ACCESS: _ClassVar[BenchType]
    BENCH_TYPE_TEXT: _ClassVar[BenchType]
    BENCH_TYPE_TEXT_LINE: _ClassVar[BenchType]
    BENCH_TYPE_TEXT_SPAN: _ClassVar[BenchType]
    BENCH_TYPE_TYPE_INFO: _ClassVar[BenchType]
    BENCH_TYPE_TYPE_CONSTRAINT: _ClassVar[BenchType]
    BENCH_TYPE_SCHEDULE: _ClassVar[BenchType]
    BENCH_TYPE_FILE_INFO: _ClassVar[BenchType]
    BENCH_TYPE_FILE_REFERENCE: _ClassVar[BenchType]
    BENCH_TYPE_ICON: _ClassVar[BenchType]
    BENCH_TYPE_SECRET_REFERENCE: _ClassVar[BenchType]
    BENCH_TYPE_TRIGGER_INFO: _ClassVar[BenchType]
    BENCH_TYPE_EXPRESSION: _ClassVar[BenchType]
    BENCH_TYPE_AGGREGATION: _ClassVar[BenchType]
    BENCH_TYPE_SELECTION: _ClassVar[BenchType]
    BENCH_TYPE_QUERY_INFO: _ClassVar[BenchType]
    BENCH_TYPE_READ_OPTIONS: _ClassVar[BenchType]
    BENCH_TYPE_VALUE: _ClassVar[BenchType]
    BENCH_TYPE_COMPUTED_VALUE: _ClassVar[BenchType]
    BENCH_TYPE_CODE: _ClassVar[BenchType]
    BENCH_TYPE_CODE_LINE: _ClassVar[BenchType]
    BENCH_TYPE_PORT_KEY: _ClassVar[BenchType]
    BENCH_TYPE_PORT: _ClassVar[BenchType]
    BENCH_TYPE_RUN_ERROR: _ClassVar[BenchType]
    BENCH_TYPE_RUN_OPTIONS: _ClassVar[BenchType]
    BENCH_TYPE_RUN_ATTEMPT: _ClassVar[BenchType]
    BENCH_TYPE_RUN_TRACE: _ClassVar[BenchType]
    BENCH_TYPE_RUN_FRAME: _ClassVar[BenchType]
    BENCH_TYPE_RUN_SPAN: _ClassVar[BenchType]
    BENCH_TYPE_RUN_EVENT: _ClassVar[BenchType]
    BENCH_TYPE_BREAKPOINT: _ClassVar[BenchType]
    BENCH_TYPE_LOG_INFO: _ClassVar[BenchType]
    BENCH_TYPE_COLOR: _ClassVar[BenchType]
    BENCH_TYPE_FONT: _ClassVar[BenchType]
    BENCH_TYPE_BOX: _ClassVar[BenchType]
    BENCH_TYPE_OFFSET: _ClassVar[BenchType]
    BENCH_TYPE_TRANSFORM: _ClassVar[BenchType]
    BENCH_TYPE_VECTOR2: _ClassVar[BenchType]
    BENCH_TYPE_VECTOR3: _ClassVar[BenchType]
    BENCH_TYPE_VECTOR4: _ClassVar[BenchType]
    BENCH_TYPE_LINE: _ClassVar[BenchType]
    BENCH_TYPE_START_VIEW_STATE: _ClassVar[BenchType]
    BENCH_TYPE_FEED_VIEW_STATE: _ClassVar[BenchType]
    BENCH_TYPE_USER_WIZARD_VIEW_STATE: _ClassVar[BenchType]
    BENCH_TYPE_TREE_VIEW_STATE: _ClassVar[BenchType]
    BENCH_TYPE_ENUM_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_NODE_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_STRUCT_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_OBJECT_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_BENCH_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_CHANGE_KIND: _ClassVar[BenchType]
    BENCH_TYPE_ACCESS_MODE: _ClassVar[BenchType]
    BENCH_TYPE_ACCESS_KIND: _ClassVar[BenchType]
    BENCH_TYPE_READ_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_EDIT_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_USE_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_ACCESS_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_CHANGE_CATEGORY: _ClassVar[BenchType]
    BENCH_TYPE_EDIT_OPERATION_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_POLICY_EFFECT: _ClassVar[BenchType]
    BENCH_TYPE_CLOUD: _ClassVar[BenchType]
    BENCH_TYPE_REGION: _ClassVar[BenchType]
    BENCH_TYPE_REGION_ZONE: _ClassVar[BenchType]
    BENCH_TYPE_REGION_AREA: _ClassVar[BenchType]
    BENCH_TYPE_RESOURCE_STATUS: _ClassVar[BenchType]
    BENCH_TYPE_FILE_RETENTION_MODE: _ClassVar[BenchType]
    BENCH_TYPE_FILE_KIND: _ClassVar[BenchType]
    BENCH_TYPE_FILE_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_FILE_FORMAT: _ClassVar[BenchType]
    BENCH_TYPE_CLIENT_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_PRIMITIVE_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_FIELD_ZONE: _ClassVar[BenchType]
    BENCH_TYPE_TYPE_KIND: _ClassVar[BenchType]
    BENCH_TYPE_TYPE_FORMAT: _ClassVar[BenchType]
    BENCH_TYPE_BLOCK_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_SCHEDULE_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_TIME_INTERVAL: _ClassVar[BenchType]
    BENCH_TYPE_DAY: _ClassVar[BenchType]
    BENCH_TYPE_MONTH: _ClassVar[BenchType]
    BENCH_TYPE_TEXT_LINE_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_ICON_KIND: _ClassVar[BenchType]
    BENCH_TYPE_EXPRESSION_KIND: _ClassVar[BenchType]
    BENCH_TYPE_EXPRESSION_OP: _ClassVar[BenchType]
    BENCH_TYPE_LITERAL_OP: _ClassVar[BenchType]
    BENCH_TYPE_FUNCTIONAL_OP: _ClassVar[BenchType]
    BENCH_TYPE_CONDITIONAL_OP: _ClassVar[BenchType]
    BENCH_TYPE_AGGREGATION_OP: _ClassVar[BenchType]
    BENCH_TYPE_SORT_MODE: _ClassVar[BenchType]
    BENCH_TYPE_SORT_OP: _ClassVar[BenchType]
    BENCH_TYPE_SELECTION_KIND: _ClassVar[BenchType]
    BENCH_TYPE_SELECTION_TARGET: _ClassVar[BenchType]
    BENCH_TYPE_PATH_TOKEN_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_LOG_KIND: _ClassVar[BenchType]
    BENCH_TYPE_LOG_LEVEL: _ClassVar[BenchType]
    BENCH_TYPE_RUN_STATUS: _ClassVar[BenchType]
    BENCH_TYPE_RUN_KIND: _ClassVar[BenchType]
    BENCH_TYPE_RUN_ERROR_KIND: _ClassVar[BenchType]
    BENCH_TYPE_RUN_ERROR_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_RUN_SPAN_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_RUN_EVENT_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_SESSION_STATUS: _ClassVar[BenchType]
    BENCH_TYPE_TRIGGER_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_BREAKPOINT_KIND: _ClassVar[BenchType]
    BENCH_TYPE_BREAKPOINT_ACTION: _ClassVar[BenchType]
    BENCH_TYPE_CACHE_MODE: _ClassVar[BenchType]
    BENCH_TYPE_CODE_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_MODEL_PROVIDER: _ClassVar[BenchType]
    BENCH_TYPE_MODEL_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_STEP_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_PIPE_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_PIPE_FILTER_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_PORT_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_PORT_SIDE: _ClassVar[BenchType]
    BENCH_TYPE_NOTIFICATION_LEVEL: _ClassVar[BenchType]
    BENCH_TYPE_SPACE_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_VIEW_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_VARIANT: _ClassVar[BenchType]
    BENCH_TYPE_COLOR_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_COLOR_SHADE: _ClassVar[BenchType]
    BENCH_TYPE_FONT_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_FONT_WEIGHT: _ClassVar[BenchType]
    BENCH_TYPE_FONT_SIZE: _ClassVar[BenchType]
    BENCH_TYPE_SPACING: _ClassVar[BenchType]
    BENCH_TYPE_ANCHOR: _ClassVar[BenchType]
    BENCH_TYPE_ORIENTATION: _ClassVar[BenchType]
    BENCH_TYPE_ALIGNMENT: _ClassVar[BenchType]
    BENCH_TYPE_USER_WIZARD_STAGE: _ClassVar[BenchType]
    BENCH_TYPE_TREE_VIEW_PRESET: _ClassVar[BenchType]
    BENCH_TYPE_USER_STATUS: _ClassVar[BenchType]
    BENCH_TYPE_ORGANIZATION_STATUS: _ClassVar[BenchType]

class BlockType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    BLOCK_TYPE_UNSPECIFIED: _ClassVar[BlockType]
    BLOCK_TYPE_PAGE: _ClassVar[BlockType]
    BLOCK_TYPE_CLASS: _ClassVar[BlockType]
    BLOCK_TYPE_CHOICE: _ClassVar[BlockType]
    BLOCK_TYPE_SIGNAL: _ClassVar[BlockType]
    BLOCK_TYPE_NOTIFICATION: _ClassVar[BlockType]
    BLOCK_TYPE_TEXT: _ClassVar[BlockType]
    BLOCK_TYPE_CODE: _ClassVar[BlockType]
    BLOCK_TYPE_FLOW: _ClassVar[BlockType]
    BLOCK_TYPE_VALUE: _ClassVar[BlockType]
    BLOCK_TYPE_DATABASE: _ClassVar[BlockType]
    BLOCK_TYPE_QUERY: _ClassVar[BlockType]
    BLOCK_TYPE_VIEW: _ClassVar[BlockType]
    BLOCK_TYPE_ROLE: _ClassVar[BlockType]
    BLOCK_TYPE_IDENTITY: _ClassVar[BlockType]

class BreakpointAction(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    BREAKPOINT_ACTION_UNSPECIFIED: _ClassVar[BreakpointAction]
    BREAKPOINT_ACTION_SUSPEND: _ClassVar[BreakpointAction]

class BreakpointKind(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    BREAKPOINT_KIND_UNSPECIFIED: _ClassVar[BreakpointKind]
    BREAKPOINT_KIND_START_RUN: _ClassVar[BreakpointKind]
    BREAKPOINT_KIND_FAIL_RUN: _ClassVar[BreakpointKind]
    BREAKPOINT_KIND_COMPLETE_RUN: _ClassVar[BreakpointKind]
    BREAKPOINT_KIND_CODE_LINE: _ClassVar[BreakpointKind]

class CacheMode(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CACHE_MODE_UNSPECIFIED: _ClassVar[CacheMode]
    CACHE_MODE_NEVER: _ClassVar[CacheMode]
    CACHE_MODE_ALWAYS: _ClassVar[CacheMode]

class ChangeCategory(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CHANGE_CATEGORY_UNSPECIFIED: _ClassVar[ChangeCategory]
    CHANGE_CATEGORY_SPACE: _ClassVar[ChangeCategory]
    CHANGE_CATEGORY_SESSION: _ClassVar[ChangeCategory]

class ChangeKind(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CHANGE_KIND_UNSPECIFIED: _ClassVar[ChangeKind]
    CHANGE_KIND_CODE: _ClassVar[ChangeKind]
    CHANGE_KIND_LOGS: _ClassVar[ChangeKind]
    CHANGE_KIND_LOGS_QUERY: _ClassVar[ChangeKind]

class ClientType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CLIENT_TYPE_UNSPECIFIED: _ClassVar[ClientType]
    CLIENT_TYPE_BENCH_WEB: _ClassVar[ClientType]
    CLIENT_TYPE_BENCH_BROWSER_PLUGIN: _ClassVar[ClientType]
    CLIENT_TYPE_BENCH_DESKTOP: _ClassVar[ClientType]
    CLIENT_TYPE_BENCH_MOBILE: _ClassVar[ClientType]
    CLIENT_TYPE_BENCH_MACHINE: _ClassVar[ClientType]

class Cloud(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CLOUD_UNSPECIFIED: _ClassVar[Cloud]
    CLOUD_AWS: _ClassVar[Cloud]
    CLOUD_AZURE: _ClassVar[Cloud]
    CLOUD_GCP: _ClassVar[Cloud]
    CLOUD_OCI: _ClassVar[Cloud]
    CLOUD_ALIBABA: _ClassVar[Cloud]
    CLOUD_HETZNER: _ClassVar[Cloud]
    CLOUD_NEON: _ClassVar[Cloud]
    CLOUD_PRIVATE: _ClassVar[Cloud]

class CodeType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CODE_TYPE_UNSPECIFIED: _ClassVar[CodeType]
    CODE_TYPE_SNIPPET: _ClassVar[CodeType]
    CODE_TYPE_SCRIPT: _ClassVar[CodeType]
    CODE_TYPE_FUNCTION: _ClassVar[CodeType]

class ColorShade(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    COLOR_SHADE_UNSPECIFIED: _ClassVar[ColorShade]
    COLOR_SHADE_S50: _ClassVar[ColorShade]
    COLOR_SHADE_S100: _ClassVar[ColorShade]
    COLOR_SHADE_S200: _ClassVar[ColorShade]
    COLOR_SHADE_S300: _ClassVar[ColorShade]
    COLOR_SHADE_S400: _ClassVar[ColorShade]
    COLOR_SHADE_S500: _ClassVar[ColorShade]
    COLOR_SHADE_S600: _ClassVar[ColorShade]
    COLOR_SHADE_S700: _ClassVar[ColorShade]
    COLOR_SHADE_S800: _ClassVar[ColorShade]
    COLOR_SHADE_S900: _ClassVar[ColorShade]
    COLOR_SHADE_S950: _ClassVar[ColorShade]

class ColorType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    COLOR_TYPE_UNSPECIFIED: _ClassVar[ColorType]
    COLOR_TYPE_PRIMARY: _ClassVar[ColorType]
    COLOR_TYPE_SECONDARY: _ClassVar[ColorType]
    COLOR_TYPE_ACCENT: _ClassVar[ColorType]
    COLOR_TYPE_CANVAS: _ClassVar[ColorType]
    COLOR_TYPE_SUCCESS: _ClassVar[ColorType]
    COLOR_TYPE_HINT: _ClassVar[ColorType]
    COLOR_TYPE_WARNING: _ClassVar[ColorType]
    COLOR_TYPE_DANGER: _ClassVar[ColorType]
    COLOR_TYPE_GRAY: _ClassVar[ColorType]
    COLOR_TYPE_RED: _ClassVar[ColorType]
    COLOR_TYPE_ORANGE: _ClassVar[ColorType]
    COLOR_TYPE_AMBER: _ClassVar[ColorType]
    COLOR_TYPE_YELLOW: _ClassVar[ColorType]
    COLOR_TYPE_LIME: _ClassVar[ColorType]
    COLOR_TYPE_GREEN: _ClassVar[ColorType]
    COLOR_TYPE_EMERALD: _ClassVar[ColorType]
    COLOR_TYPE_TEAL: _ClassVar[ColorType]
    COLOR_TYPE_CYAN: _ClassVar[ColorType]
    COLOR_TYPE_SKY: _ClassVar[ColorType]
    COLOR_TYPE_BLUE: _ClassVar[ColorType]
    COLOR_TYPE_INDIGO: _ClassVar[ColorType]
    COLOR_TYPE_VIOLET: _ClassVar[ColorType]
    COLOR_TYPE_PURPLE: _ClassVar[ColorType]
    COLOR_TYPE_FUCHSIA: _ClassVar[ColorType]
    COLOR_TYPE_PINK: _ClassVar[ColorType]
    COLOR_TYPE_ROSE: _ClassVar[ColorType]

class ConditionalOp(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CONDITIONAL_OP_UNSPECIFIED: _ClassVar[ConditionalOp]
    CONDITIONAL_OP_NOT: _ClassVar[ConditionalOp]
    CONDITIONAL_OP_AND: _ClassVar[ConditionalOp]
    CONDITIONAL_OP_OR: _ClassVar[ConditionalOp]
    CONDITIONAL_OP_EQUALS: _ClassVar[ConditionalOp]
    CONDITIONAL_OP_NOT_EQUALS: _ClassVar[ConditionalOp]
    CONDITIONAL_OP_GREATER_THAN: _ClassVar[ConditionalOp]
    CONDITIONAL_OP_GREATER_THAN_OR_EQUALS: _ClassVar[ConditionalOp]
    CONDITIONAL_OP_LESS_THAN: _ClassVar[ConditionalOp]
    CONDITIONAL_OP_LESS_THAN_OR_EQUALS: _ClassVar[ConditionalOp]
    CONDITIONAL_OP_MATCHES_REGEX: _ClassVar[ConditionalOp]
    CONDITIONAL_OP_STARTS_WITH: _ClassVar[ConditionalOp]
    CONDITIONAL_OP_ENDS_WITH: _ClassVar[ConditionalOp]
    CONDITIONAL_OP_CONTAINS: _ClassVar[ConditionalOp]
    CONDITIONAL_OP_NOT_CONTAINS: _ClassVar[ConditionalOp]
    CONDITIONAL_OP_IN: _ClassVar[ConditionalOp]
    CONDITIONAL_OP_NOT_IN: _ClassVar[ConditionalOp]
    CONDITIONAL_OP_EXISTS: _ClassVar[ConditionalOp]
    CONDITIONAL_OP_NOT_EXISTS: _ClassVar[ConditionalOp]
    CONDITIONAL_OP_NEAR: _ClassVar[ConditionalOp]

class Day(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    DAY_UNSPECIFIED: _ClassVar[Day]
    DAY_MONDAY: _ClassVar[Day]
    DAY_TUESDAY: _ClassVar[Day]
    DAY_WEDNESDAY: _ClassVar[Day]
    DAY_THURSDAY: _ClassVar[Day]
    DAY_FRIDAY: _ClassVar[Day]
    DAY_SATURDAY: _ClassVar[Day]
    DAY_SUNDAY: _ClassVar[Day]

class EditOperationType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    EDIT_OPERATION_TYPE_UNSPECIFIED: _ClassVar[EditOperationType]
    EDIT_OPERATION_TYPE_SET: _ClassVar[EditOperationType]
    EDIT_OPERATION_TYPE_CLEAR: _ClassVar[EditOperationType]
    EDIT_OPERATION_TYPE_APPEND: _ClassVar[EditOperationType]
    EDIT_OPERATION_TYPE_REMOVE: _ClassVar[EditOperationType]

class EditType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    EDIT_TYPE_UNSPECIFIED: _ClassVar[EditType]
    EDIT_TYPE_CREATE: _ClassVar[EditType]
    EDIT_TYPE_UPSERT: _ClassVar[EditType]
    EDIT_TYPE_UPDATE: _ClassVar[EditType]
    EDIT_TYPE_MOVE: _ClassVar[EditType]
    EDIT_TYPE_ARCHIVE: _ClassVar[EditType]
    EDIT_TYPE_UNARCHIVE: _ClassVar[EditType]
    EDIT_TYPE_DELETE: _ClassVar[EditType]
    EDIT_TYPE_RESTORE: _ClassVar[EditType]
    EDIT_TYPE_ERASE: _ClassVar[EditType]

class EnumType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ENUM_TYPE_UNSPECIFIED: _ClassVar[EnumType]
    ENUM_TYPE_ENUM_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_NODE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_STRUCT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_OBJECT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_BENCH_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_CHANGE_KIND: _ClassVar[EnumType]
    ENUM_TYPE_ACCESS_MODE: _ClassVar[EnumType]
    ENUM_TYPE_ACCESS_KIND: _ClassVar[EnumType]
    ENUM_TYPE_READ_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_EDIT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_USE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_ACCESS_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_CHANGE_CATEGORY: _ClassVar[EnumType]
    ENUM_TYPE_EDIT_OPERATION_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_POLICY_EFFECT: _ClassVar[EnumType]
    ENUM_TYPE_CLOUD: _ClassVar[EnumType]
    ENUM_TYPE_REGION: _ClassVar[EnumType]
    ENUM_TYPE_REGION_ZONE: _ClassVar[EnumType]
    ENUM_TYPE_REGION_AREA: _ClassVar[EnumType]
    ENUM_TYPE_RESOURCE_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_FILE_RETENTION_MODE: _ClassVar[EnumType]
    ENUM_TYPE_FILE_KIND: _ClassVar[EnumType]
    ENUM_TYPE_FILE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_FILE_FORMAT: _ClassVar[EnumType]
    ENUM_TYPE_CLIENT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_PRIMITIVE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_FIELD_ZONE: _ClassVar[EnumType]
    ENUM_TYPE_TYPE_KIND: _ClassVar[EnumType]
    ENUM_TYPE_TYPE_FORMAT: _ClassVar[EnumType]
    ENUM_TYPE_BLOCK_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_SCHEDULE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_TIME_INTERVAL: _ClassVar[EnumType]
    ENUM_TYPE_DAY: _ClassVar[EnumType]
    ENUM_TYPE_MONTH: _ClassVar[EnumType]
    ENUM_TYPE_TEXT_LINE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_ICON_KIND: _ClassVar[EnumType]
    ENUM_TYPE_EXPRESSION_KIND: _ClassVar[EnumType]
    ENUM_TYPE_EXPRESSION_OP: _ClassVar[EnumType]
    ENUM_TYPE_LITERAL_OP: _ClassVar[EnumType]
    ENUM_TYPE_FUNCTIONAL_OP: _ClassVar[EnumType]
    ENUM_TYPE_CONDITIONAL_OP: _ClassVar[EnumType]
    ENUM_TYPE_AGGREGATION_OP: _ClassVar[EnumType]
    ENUM_TYPE_SORT_MODE: _ClassVar[EnumType]
    ENUM_TYPE_SORT_OP: _ClassVar[EnumType]
    ENUM_TYPE_SELECTION_KIND: _ClassVar[EnumType]
    ENUM_TYPE_SELECTION_TARGET: _ClassVar[EnumType]
    ENUM_TYPE_PATH_TOKEN_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_LOG_KIND: _ClassVar[EnumType]
    ENUM_TYPE_LOG_LEVEL: _ClassVar[EnumType]
    ENUM_TYPE_RUN_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_RUN_KIND: _ClassVar[EnumType]
    ENUM_TYPE_RUN_ERROR_KIND: _ClassVar[EnumType]
    ENUM_TYPE_RUN_ERROR_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_RUN_SPAN_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_RUN_EVENT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_SESSION_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_TRIGGER_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_BREAKPOINT_KIND: _ClassVar[EnumType]
    ENUM_TYPE_BREAKPOINT_ACTION: _ClassVar[EnumType]
    ENUM_TYPE_CACHE_MODE: _ClassVar[EnumType]
    ENUM_TYPE_CODE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_MODEL_PROVIDER: _ClassVar[EnumType]
    ENUM_TYPE_MODEL_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_STEP_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_PIPE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_PIPE_FILTER_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_PORT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_PORT_SIDE: _ClassVar[EnumType]
    ENUM_TYPE_NOTIFICATION_LEVEL: _ClassVar[EnumType]
    ENUM_TYPE_SPACE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_VIEW_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_VARIANT: _ClassVar[EnumType]
    ENUM_TYPE_COLOR_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_COLOR_SHADE: _ClassVar[EnumType]
    ENUM_TYPE_FONT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_FONT_WEIGHT: _ClassVar[EnumType]
    ENUM_TYPE_FONT_SIZE: _ClassVar[EnumType]
    ENUM_TYPE_SPACING: _ClassVar[EnumType]
    ENUM_TYPE_ANCHOR: _ClassVar[EnumType]
    ENUM_TYPE_ORIENTATION: _ClassVar[EnumType]
    ENUM_TYPE_ALIGNMENT: _ClassVar[EnumType]
    ENUM_TYPE_USER_WIZARD_STAGE: _ClassVar[EnumType]
    ENUM_TYPE_TREE_VIEW_PRESET: _ClassVar[EnumType]
    ENUM_TYPE_USER_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_ORGANIZATION_STATUS: _ClassVar[EnumType]

class ExpressionKind(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    EXPRESSION_KIND_UNSPECIFIED: _ClassVar[ExpressionKind]
    EXPRESSION_KIND_LITERAL: _ClassVar[ExpressionKind]
    EXPRESSION_KIND_FUNCTIONAL: _ClassVar[ExpressionKind]
    EXPRESSION_KIND_CONDITIONAL: _ClassVar[ExpressionKind]
    EXPRESSION_KIND_SORT: _ClassVar[ExpressionKind]
    EXPRESSION_KIND_AGGREGATION: _ClassVar[ExpressionKind]

class ExpressionOp(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    EXPRESSION_OP_UNSPECIFIED: _ClassVar[ExpressionOp]
    EXPRESSION_OP_VALUE: _ClassVar[ExpressionOp]
    EXPRESSION_OP_NONE: _ClassVar[ExpressionOp]
    EXPRESSION_OP_TRUE: _ClassVar[ExpressionOp]
    EXPRESSION_OP_FALSE: _ClassVar[ExpressionOp]
    EXPRESSION_OP_ADD: _ClassVar[ExpressionOp]
    EXPRESSION_OP_SUBTRACT: _ClassVar[ExpressionOp]
    EXPRESSION_OP_MULTIPLY: _ClassVar[ExpressionOp]
    EXPRESSION_OP_DIVIDE: _ClassVar[ExpressionOp]
    EXPRESSION_OP_MODULO: _ClassVar[ExpressionOp]
    EXPRESSION_OP_POWER: _ClassVar[ExpressionOp]
    EXPRESSION_OP_NOT: _ClassVar[ExpressionOp]
    EXPRESSION_OP_AND: _ClassVar[ExpressionOp]
    EXPRESSION_OP_OR: _ClassVar[ExpressionOp]
    EXPRESSION_OP_EQUALS: _ClassVar[ExpressionOp]
    EXPRESSION_OP_NOT_EQUALS: _ClassVar[ExpressionOp]
    EXPRESSION_OP_GREATER_THAN: _ClassVar[ExpressionOp]
    EXPRESSION_OP_GREATER_THAN_OR_EQUALS: _ClassVar[ExpressionOp]
    EXPRESSION_OP_LESS_THAN: _ClassVar[ExpressionOp]
    EXPRESSION_OP_LESS_THAN_OR_EQUALS: _ClassVar[ExpressionOp]
    EXPRESSION_OP_MATCHES_REGEX: _ClassVar[ExpressionOp]
    EXPRESSION_OP_STARTS_WITH: _ClassVar[ExpressionOp]
    EXPRESSION_OP_ENDS_WITH: _ClassVar[ExpressionOp]
    EXPRESSION_OP_CONTAINS: _ClassVar[ExpressionOp]
    EXPRESSION_OP_NOT_CONTAINS: _ClassVar[ExpressionOp]
    EXPRESSION_OP_IN: _ClassVar[ExpressionOp]
    EXPRESSION_OP_NOT_IN: _ClassVar[ExpressionOp]
    EXPRESSION_OP_EXISTS: _ClassVar[ExpressionOp]
    EXPRESSION_OP_NOT_EXISTS: _ClassVar[ExpressionOp]
    EXPRESSION_OP_NEAR: _ClassVar[ExpressionOp]
    EXPRESSION_OP_COUNT: _ClassVar[ExpressionOp]
    EXPRESSION_OP_SUM: _ClassVar[ExpressionOp]
    EXPRESSION_OP_MIN: _ClassVar[ExpressionOp]
    EXPRESSION_OP_MAX: _ClassVar[ExpressionOp]
    EXPRESSION_OP_AVERAGE: _ClassVar[ExpressionOp]
    EXPRESSION_OP_MEDIAN: _ClassVar[ExpressionOp]
    EXPRESSION_OP_HISTOGRAM: _ClassVar[ExpressionOp]
    EXPRESSION_OP_ASCENDING: _ClassVar[ExpressionOp]
    EXPRESSION_OP_DESCENDING: _ClassVar[ExpressionOp]

class FieldZone(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FIELD_ZONE_UNSPECIFIED: _ClassVar[FieldZone]
    FIELD_ZONE_VARIABLE: _ClassVar[FieldZone]
    FIELD_ZONE_MEMBER: _ClassVar[FieldZone]
    FIELD_ZONE_INPUT: _ClassVar[FieldZone]
    FIELD_ZONE_OUTPUT: _ClassVar[FieldZone]
    FIELD_ZONE_OPTION: _ClassVar[FieldZone]

class FileFormat(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FILE_FORMAT_UNSPECIFIED: _ClassVar[FileFormat]
    FILE_FORMAT_TXT: _ClassVar[FileFormat]
    FILE_FORMAT_MARKDOWN: _ClassVar[FileFormat]
    FILE_FORMAT_RTF: _ClassVar[FileFormat]
    FILE_FORMAT_INI: _ClassVar[FileFormat]
    FILE_FORMAT_LOG: _ClassVar[FileFormat]
    FILE_FORMAT_PYTHON: _ClassVar[FileFormat]
    FILE_FORMAT_JAVASCRIPT: _ClassVar[FileFormat]
    FILE_FORMAT_TYPESCRIPT: _ClassVar[FileFormat]
    FILE_FORMAT_GO: _ClassVar[FileFormat]
    FILE_FORMAT_C_LANG: _ClassVar[FileFormat]
    FILE_FORMAT_CPP: _ClassVar[FileFormat]
    FILE_FORMAT_OBJECTIVE_C: _ClassVar[FileFormat]
    FILE_FORMAT_SWIFT: _ClassVar[FileFormat]
    FILE_FORMAT_RUBY: _ClassVar[FileFormat]
    FILE_FORMAT_PHP: _ClassVar[FileFormat]
    FILE_FORMAT_CSS: _ClassVar[FileFormat]
    FILE_FORMAT_JAVA: _ClassVar[FileFormat]
    FILE_FORMAT_KOTLIN: _ClassVar[FileFormat]
    FILE_FORMAT_RUST: _ClassVar[FileFormat]
    FILE_FORMAT_SCALA: _ClassVar[FileFormat]
    FILE_FORMAT_SHELL: _ClassVar[FileFormat]
    FILE_FORMAT_SQL: _ClassVar[FileFormat]
    FILE_FORMAT_POWERSHELL: _ClassVar[FileFormat]
    FILE_FORMAT_ASSEMBLY: _ClassVar[FileFormat]
    FILE_FORMAT_LATEX: _ClassVar[FileFormat]
    FILE_FORMAT_JPEG: _ClassVar[FileFormat]
    FILE_FORMAT_PNG: _ClassVar[FileFormat]
    FILE_FORMAT_GIF: _ClassVar[FileFormat]
    FILE_FORMAT_BMP: _ClassVar[FileFormat]
    FILE_FORMAT_TIFF: _ClassVar[FileFormat]
    FILE_FORMAT_WEBP: _ClassVar[FileFormat]
    FILE_FORMAT_SVG: _ClassVar[FileFormat]
    FILE_FORMAT_ICO: _ClassVar[FileFormat]
    FILE_FORMAT_RAW: _ClassVar[FileFormat]
    FILE_FORMAT_HEIC: _ClassVar[FileFormat]
    FILE_FORMAT_HEIF: _ClassVar[FileFormat]
    FILE_FORMAT_MP3: _ClassVar[FileFormat]
    FILE_FORMAT_WAV: _ClassVar[FileFormat]
    FILE_FORMAT_FLAC: _ClassVar[FileFormat]
    FILE_FORMAT_AAC: _ClassVar[FileFormat]
    FILE_FORMAT_OGG: _ClassVar[FileFormat]
    FILE_FORMAT_M4A: _ClassVar[FileFormat]
    FILE_FORMAT_WMA: _ClassVar[FileFormat]
    FILE_FORMAT_MP4: _ClassVar[FileFormat]
    FILE_FORMAT_WEBM: _ClassVar[FileFormat]
    FILE_FORMAT_AVI: _ClassVar[FileFormat]
    FILE_FORMAT_MOV: _ClassVar[FileFormat]
    FILE_FORMAT_WMV: _ClassVar[FileFormat]
    FILE_FORMAT_FLV: _ClassVar[FileFormat]
    FILE_FORMAT_MKV: _ClassVar[FileFormat]
    FILE_FORMAT_PDF: _ClassVar[FileFormat]
    FILE_FORMAT_DOCX: _ClassVar[FileFormat]
    FILE_FORMAT_PPTX: _ClassVar[FileFormat]
    FILE_FORMAT_ODT: _ClassVar[FileFormat]
    FILE_FORMAT_XLSX: _ClassVar[FileFormat]
    FILE_FORMAT_ODS: _ClassVar[FileFormat]
    FILE_FORMAT_EPUB: _ClassVar[FileFormat]
    FILE_FORMAT_MOBI: _ClassVar[FileFormat]
    FILE_FORMAT_CHM: _ClassVar[FileFormat]
    FILE_FORMAT_DOC: _ClassVar[FileFormat]
    FILE_FORMAT_XLS: _ClassVar[FileFormat]
    FILE_FORMAT_PPT: _ClassVar[FileFormat]
    FILE_FORMAT_HTML: _ClassVar[FileFormat]
    FILE_FORMAT_JSON: _ClassVar[FileFormat]
    FILE_FORMAT_YAML: _ClassVar[FileFormat]
    FILE_FORMAT_CSV: _ClassVar[FileFormat]
    FILE_FORMAT_XML: _ClassVar[FileFormat]
    FILE_FORMAT_TOML: _ClassVar[FileFormat]
    FILE_FORMAT_SQLITE: _ClassVar[FileFormat]
    FILE_FORMAT_PARQUET: _ClassVar[FileFormat]
    FILE_FORMAT_ZIP: _ClassVar[FileFormat]
    FILE_FORMAT_RAR: _ClassVar[FileFormat]
    FILE_FORMAT_TAR: _ClassVar[FileFormat]
    FILE_FORMAT_SEVENZIP: _ClassVar[FileFormat]
    FILE_FORMAT_CAB: _ClassVar[FileFormat]
    FILE_FORMAT_GZIP: _ClassVar[FileFormat]
    FILE_FORMAT_BZIP2: _ClassVar[FileFormat]
    FILE_FORMAT_XZ: _ClassVar[FileFormat]
    FILE_FORMAT_EXE: _ClassVar[FileFormat]
    FILE_FORMAT_APP_IMAGE: _ClassVar[FileFormat]
    FILE_FORMAT_APK: _ClassVar[FileFormat]
    FILE_FORMAT_DMG: _ClassVar[FileFormat]
    FILE_FORMAT_JAR: _ClassVar[FileFormat]
    FILE_FORMAT_MSI: _ClassVar[FileFormat]
    FILE_FORMAT_DEB: _ClassVar[FileFormat]
    FILE_FORMAT_RPM: _ClassVar[FileFormat]

class FileKind(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FILE_KIND_UNSPECIFIED: _ClassVar[FileKind]
    FILE_KIND_DRIVE: _ClassVar[FileKind]
    FILE_KIND_DRIVE_INLINE: _ClassVar[FileKind]
    FILE_KIND_INLINE: _ClassVar[FileKind]
    FILE_KIND_EXTERNAL: _ClassVar[FileKind]

class FileRetentionMode(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FILE_RETENTION_MODE_UNSPECIFIED: _ClassVar[FileRetentionMode]
    FILE_RETENTION_MODE_AUTOMATIC: _ClassVar[FileRetentionMode]
    FILE_RETENTION_MODE_MANUAL: _ClassVar[FileRetentionMode]
    FILE_RETENTION_MODE_TIMED: _ClassVar[FileRetentionMode]

class FileType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FILE_TYPE_UNSPECIFIED: _ClassVar[FileType]
    FILE_TYPE_TEXT: _ClassVar[FileType]
    FILE_TYPE_CODE: _ClassVar[FileType]
    FILE_TYPE_IMAGE: _ClassVar[FileType]
    FILE_TYPE_AUDIO: _ClassVar[FileType]
    FILE_TYPE_VIDEO: _ClassVar[FileType]
    FILE_TYPE_DOCUMENT: _ClassVar[FileType]
    FILE_TYPE_DATA: _ClassVar[FileType]
    FILE_TYPE_ARCHIVE: _ClassVar[FileType]
    FILE_TYPE_EXECUTABLE: _ClassVar[FileType]
    FILE_TYPE_GENERIC: _ClassVar[FileType]

class FontSize(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FONT_SIZE_UNSPECIFIED: _ClassVar[FontSize]
    FONT_SIZE_XS: _ClassVar[FontSize]
    FONT_SIZE_SM: _ClassVar[FontSize]
    FONT_SIZE_BASE: _ClassVar[FontSize]
    FONT_SIZE_LG: _ClassVar[FontSize]
    FONT_SIZE_XL: _ClassVar[FontSize]
    FONT_SIZE_XL2: _ClassVar[FontSize]
    FONT_SIZE_XL3: _ClassVar[FontSize]
    FONT_SIZE_XL4: _ClassVar[FontSize]
    FONT_SIZE_XL5: _ClassVar[FontSize]
    FONT_SIZE_XL6: _ClassVar[FontSize]
    FONT_SIZE_XL7: _ClassVar[FontSize]

class FontType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FONT_TYPE_UNSPECIFIED: _ClassVar[FontType]
    FONT_TYPE_SERIF: _ClassVar[FontType]
    FONT_TYPE_SANS: _ClassVar[FontType]
    FONT_TYPE_MONO: _ClassVar[FontType]

class FontWeight(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FONT_WEIGHT_UNSPECIFIED: _ClassVar[FontWeight]
    FONT_WEIGHT_THIN: _ClassVar[FontWeight]
    FONT_WEIGHT_EXTRA_LIGHT: _ClassVar[FontWeight]
    FONT_WEIGHT_LIGHT: _ClassVar[FontWeight]
    FONT_WEIGHT_NORMAL: _ClassVar[FontWeight]
    FONT_WEIGHT_MEDIUM: _ClassVar[FontWeight]
    FONT_WEIGHT_SEMI_BOLD: _ClassVar[FontWeight]
    FONT_WEIGHT_BOLD: _ClassVar[FontWeight]
    FONT_WEIGHT_EXTRA_BOLD: _ClassVar[FontWeight]
    FONT_WEIGHT_BLACK: _ClassVar[FontWeight]

class FunctionalOp(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FUNCTIONAL_OP_UNSPECIFIED: _ClassVar[FunctionalOp]
    FUNCTIONAL_OP_ADD: _ClassVar[FunctionalOp]
    FUNCTIONAL_OP_SUBTRACT: _ClassVar[FunctionalOp]
    FUNCTIONAL_OP_MULTIPLY: _ClassVar[FunctionalOp]
    FUNCTIONAL_OP_DIVIDE: _ClassVar[FunctionalOp]
    FUNCTIONAL_OP_MODULO: _ClassVar[FunctionalOp]
    FUNCTIONAL_OP_POWER: _ClassVar[FunctionalOp]

class IconKind(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ICON_KIND_UNSPECIFIED: _ClassVar[IconKind]
    ICON_KIND_EMOJI: _ClassVar[IconKind]
    ICON_KIND_FONT_AWESOME: _ClassVar[IconKind]
    ICON_KIND_VS_CODE: _ClassVar[IconKind]

class IdEnum(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ID_ENUM_UNSPECIFIED: _ClassVar[IdEnum]

class LiteralOp(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    LITERAL_OP_UNSPECIFIED: _ClassVar[LiteralOp]
    LITERAL_OP_VALUE: _ClassVar[LiteralOp]
    LITERAL_OP_NONE: _ClassVar[LiteralOp]
    LITERAL_OP_TRUE: _ClassVar[LiteralOp]
    LITERAL_OP_FALSE: _ClassVar[LiteralOp]

class LogKind(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    LOG_KIND_UNSPECIFIED: _ClassVar[LogKind]
    LOG_KIND_CHANGE: _ClassVar[LogKind]
    LOG_KIND_EDIT: _ClassVar[LogKind]

class LogLevel(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    LOG_LEVEL_UNSPECIFIED: _ClassVar[LogLevel]
    LOG_LEVEL_TRACE: _ClassVar[LogLevel]
    LOG_LEVEL_DEBUG: _ClassVar[LogLevel]
    LOG_LEVEL_INFO: _ClassVar[LogLevel]
    LOG_LEVEL_WARNING: _ClassVar[LogLevel]
    LOG_LEVEL_ERROR: _ClassVar[LogLevel]
    LOG_LEVEL_CRITICAL: _ClassVar[LogLevel]

class ModelProvider(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MODEL_PROVIDER_UNSPECIFIED: _ClassVar[ModelProvider]
    MODEL_PROVIDER_OPENAI: _ClassVar[ModelProvider]
    MODEL_PROVIDER_ANTHROPIC: _ClassVar[ModelProvider]
    MODEL_PROVIDER_GOOGLE: _ClassVar[ModelProvider]
    MODEL_PROVIDER_META: _ClassVar[ModelProvider]

class ModelType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MODEL_TYPE_UNSPECIFIED: _ClassVar[ModelType]
    MODEL_TYPE_OPENAI_GPT4_0: _ClassVar[ModelType]
    MODEL_TYPE_OPENAI_GPT4_O_MINI: _ClassVar[ModelType]
    MODEL_TYPE_OPENAI_O1_PREVIEW: _ClassVar[ModelType]
    MODEL_TYPE_OPENAI_O1_MINI: _ClassVar[ModelType]
    MODEL_TYPE_ANTHROPIC_CLAUDE_3_5_SONNET: _ClassVar[ModelType]
    MODEL_TYPE_GOOGLE_GEMINI_1_5_PRO: _ClassVar[ModelType]
    MODEL_TYPE_META_LLAMA_3_1_80B: _ClassVar[ModelType]
    MODEL_TYPE_META_LLAMA_3_1_400B: _ClassVar[ModelType]

class Month(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MONTH_UNSPECIFIED: _ClassVar[Month]
    MONTH_JANUARY: _ClassVar[Month]
    MONTH_FEBRUARY: _ClassVar[Month]
    MONTH_MARCH: _ClassVar[Month]
    MONTH_APRIL: _ClassVar[Month]
    MONTH_MAY: _ClassVar[Month]
    MONTH_JUNE: _ClassVar[Month]
    MONTH_JULY: _ClassVar[Month]
    MONTH_AUGUST: _ClassVar[Month]
    MONTH_SEPTEMBER: _ClassVar[Month]
    MONTH_OCTOBER: _ClassVar[Month]
    MONTH_NOVEMBER: _ClassVar[Month]
    MONTH_DECEMBER: _ClassVar[Month]

class NodeType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    NODE_TYPE_UNSPECIFIED: _ClassVar[NodeType]
    NODE_TYPE_BENCH: _ClassVar[NodeType]
    NODE_TYPE_USER: _ClassVar[NodeType]
    NODE_TYPE_ORGANIZATION: _ClassVar[NodeType]
    NODE_TYPE_HANDLE: _ClassVar[NodeType]
    NODE_TYPE_CLIENT: _ClassVar[NodeType]
    NODE_TYPE_SERVER: _ClassVar[NodeType]
    NODE_TYPE_STORE: _ClassVar[NodeType]
    NODE_TYPE_MACHINE: _ClassVar[NodeType]
    NODE_TYPE_DRIVE: _ClassVar[NodeType]
    NODE_TYPE_VAULT: _ClassVar[NodeType]
    NODE_TYPE_CACHE: _ClassVar[NodeType]
    NODE_TYPE_FILE: _ClassVar[NodeType]
    NODE_TYPE_SECRET: _ClassVar[NodeType]
    NODE_TYPE_MEMBERSHIP: _ClassVar[NodeType]
    NODE_TYPE_INVITE: _ClassVar[NodeType]
    NODE_TYPE_BRANCH: _ClassVar[NodeType]
    NODE_TYPE_PACKAGE: _ClassVar[NodeType]
    NODE_TYPE_DEPENDENCY: _ClassVar[NodeType]
    NODE_TYPE_SPACE: _ClassVar[NodeType]
    NODE_TYPE_BLOCK: _ClassVar[NodeType]
    NODE_TYPE_TRIGGER: _ClassVar[NodeType]
    NODE_TYPE_FIELD: _ClassVar[NodeType]
    NODE_TYPE_QUERY: _ClassVar[NodeType]
    NODE_TYPE_VIEW: _ClassVar[NodeType]
    NODE_TYPE_STEP: _ClassVar[NodeType]
    NODE_TYPE_PIPE: _ClassVar[NodeType]
    NODE_TYPE_BADGE: _ClassVar[NodeType]
    NODE_TYPE_MESSAGE: _ClassVar[NodeType]
    NODE_TYPE_RECORD: _ClassVar[NodeType]
    NODE_TYPE_SESSION: _ClassVar[NodeType]
    NODE_TYPE_RUN: _ClassVar[NodeType]
    NODE_TYPE_SIGNAL: _ClassVar[NodeType]
    NODE_TYPE_LOG: _ClassVar[NodeType]
    NODE_TYPE_NOTIFICATION: _ClassVar[NodeType]
    NODE_TYPE_SKIP: _ClassVar[NodeType]

class NotificationLevel(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    NOTIFICATION_LEVEL_UNSPECIFIED: _ClassVar[NotificationLevel]
    NOTIFICATION_LEVEL_PASSIVE: _ClassVar[NotificationLevel]
    NOTIFICATION_LEVEL_ACTIVE: _ClassVar[NotificationLevel]
    NOTIFICATION_LEVEL_URGENT: _ClassVar[NotificationLevel]

class ObjectType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    OBJECT_TYPE_UNSPECIFIED: _ClassVar[ObjectType]
    OBJECT_TYPE_BENCH: _ClassVar[ObjectType]
    OBJECT_TYPE_USER: _ClassVar[ObjectType]
    OBJECT_TYPE_ORGANIZATION: _ClassVar[ObjectType]
    OBJECT_TYPE_HANDLE: _ClassVar[ObjectType]
    OBJECT_TYPE_CLIENT: _ClassVar[ObjectType]
    OBJECT_TYPE_SERVER: _ClassVar[ObjectType]
    OBJECT_TYPE_STORE: _ClassVar[ObjectType]
    OBJECT_TYPE_MACHINE: _ClassVar[ObjectType]
    OBJECT_TYPE_DRIVE: _ClassVar[ObjectType]
    OBJECT_TYPE_VAULT: _ClassVar[ObjectType]
    OBJECT_TYPE_CACHE: _ClassVar[ObjectType]
    OBJECT_TYPE_FILE: _ClassVar[ObjectType]
    OBJECT_TYPE_SECRET: _ClassVar[ObjectType]
    OBJECT_TYPE_MEMBERSHIP: _ClassVar[ObjectType]
    OBJECT_TYPE_INVITE: _ClassVar[ObjectType]
    OBJECT_TYPE_BRANCH: _ClassVar[ObjectType]
    OBJECT_TYPE_PACKAGE: _ClassVar[ObjectType]
    OBJECT_TYPE_DEPENDENCY: _ClassVar[ObjectType]
    OBJECT_TYPE_SPACE: _ClassVar[ObjectType]
    OBJECT_TYPE_BLOCK: _ClassVar[ObjectType]
    OBJECT_TYPE_TRIGGER: _ClassVar[ObjectType]
    OBJECT_TYPE_FIELD: _ClassVar[ObjectType]
    OBJECT_TYPE_QUERY: _ClassVar[ObjectType]
    OBJECT_TYPE_VIEW: _ClassVar[ObjectType]
    OBJECT_TYPE_STEP: _ClassVar[ObjectType]
    OBJECT_TYPE_PIPE: _ClassVar[ObjectType]
    OBJECT_TYPE_BADGE: _ClassVar[ObjectType]
    OBJECT_TYPE_MESSAGE: _ClassVar[ObjectType]
    OBJECT_TYPE_RECORD: _ClassVar[ObjectType]
    OBJECT_TYPE_SESSION: _ClassVar[ObjectType]
    OBJECT_TYPE_RUN: _ClassVar[ObjectType]
    OBJECT_TYPE_SIGNAL: _ClassVar[ObjectType]
    OBJECT_TYPE_LOG: _ClassVar[ObjectType]
    OBJECT_TYPE_NOTIFICATION: _ClassVar[ObjectType]
    OBJECT_TYPE_SKIP: _ClassVar[ObjectType]
    OBJECT_TYPE_SESSION_CONTEXT: _ClassVar[ObjectType]
    OBJECT_TYPE_EDIT_CONTEXT: _ClassVar[ObjectType]
    OBJECT_TYPE_EDIT: _ClassVar[ObjectType]
    OBJECT_TYPE_EDIT_INFO: _ClassVar[ObjectType]
    OBJECT_TYPE_EDIT_OPERATION: _ClassVar[ObjectType]
    OBJECT_TYPE_CHANGE: _ClassVar[ObjectType]
    OBJECT_TYPE_CHANGE_VIGNETTE: _ClassVar[ObjectType]
    OBJECT_TYPE_GRAPH_SCOPE: _ClassVar[ObjectType]
    OBJECT_TYPE_CLIENT_ORIGIN: _ClassVar[ObjectType]
    OBJECT_TYPE_NODE_REFERENCE: _ClassVar[ObjectType]
    OBJECT_TYPE_PROPERTY_REFERENCE: _ClassVar[ObjectType]
    OBJECT_TYPE_PATH: _ClassVar[ObjectType]
    OBJECT_TYPE_PATH_TOKEN: _ClassVar[ObjectType]
    OBJECT_TYPE_POLICY: _ClassVar[ObjectType]
    OBJECT_TYPE_POLICY_RULE: _ClassVar[ObjectType]
    OBJECT_TYPE_SUBJECT: _ClassVar[ObjectType]
    OBJECT_TYPE_ACCESS_ZONE: _ClassVar[ObjectType]
    OBJECT_TYPE_ACCESS_MATRIX: _ClassVar[ObjectType]
    OBJECT_TYPE_ACCESS: _ClassVar[ObjectType]
    OBJECT_TYPE_TEXT: _ClassVar[ObjectType]
    OBJECT_TYPE_TEXT_LINE: _ClassVar[ObjectType]
    OBJECT_TYPE_TEXT_SPAN: _ClassVar[ObjectType]
    OBJECT_TYPE_TYPE_INFO: _ClassVar[ObjectType]
    OBJECT_TYPE_TYPE_CONSTRAINT: _ClassVar[ObjectType]
    OBJECT_TYPE_SCHEDULE: _ClassVar[ObjectType]
    OBJECT_TYPE_FILE_INFO: _ClassVar[ObjectType]
    OBJECT_TYPE_FILE_REFERENCE: _ClassVar[ObjectType]
    OBJECT_TYPE_ICON: _ClassVar[ObjectType]
    OBJECT_TYPE_SECRET_REFERENCE: _ClassVar[ObjectType]
    OBJECT_TYPE_TRIGGER_INFO: _ClassVar[ObjectType]
    OBJECT_TYPE_EXPRESSION: _ClassVar[ObjectType]
    OBJECT_TYPE_AGGREGATION: _ClassVar[ObjectType]
    OBJECT_TYPE_SELECTION: _ClassVar[ObjectType]
    OBJECT_TYPE_QUERY_INFO: _ClassVar[ObjectType]
    OBJECT_TYPE_READ_OPTIONS: _ClassVar[ObjectType]
    OBJECT_TYPE_VALUE: _ClassVar[ObjectType]
    OBJECT_TYPE_COMPUTED_VALUE: _ClassVar[ObjectType]
    OBJECT_TYPE_CODE: _ClassVar[ObjectType]
    OBJECT_TYPE_CODE_LINE: _ClassVar[ObjectType]
    OBJECT_TYPE_PORT_KEY: _ClassVar[ObjectType]
    OBJECT_TYPE_PORT: _ClassVar[ObjectType]
    OBJECT_TYPE_RUN_ERROR: _ClassVar[ObjectType]
    OBJECT_TYPE_RUN_OPTIONS: _ClassVar[ObjectType]
    OBJECT_TYPE_RUN_ATTEMPT: _ClassVar[ObjectType]
    OBJECT_TYPE_RUN_TRACE: _ClassVar[ObjectType]
    OBJECT_TYPE_RUN_FRAME: _ClassVar[ObjectType]
    OBJECT_TYPE_RUN_SPAN: _ClassVar[ObjectType]
    OBJECT_TYPE_RUN_EVENT: _ClassVar[ObjectType]
    OBJECT_TYPE_BREAKPOINT: _ClassVar[ObjectType]
    OBJECT_TYPE_LOG_INFO: _ClassVar[ObjectType]
    OBJECT_TYPE_COLOR: _ClassVar[ObjectType]
    OBJECT_TYPE_FONT: _ClassVar[ObjectType]
    OBJECT_TYPE_BOX: _ClassVar[ObjectType]
    OBJECT_TYPE_OFFSET: _ClassVar[ObjectType]
    OBJECT_TYPE_TRANSFORM: _ClassVar[ObjectType]
    OBJECT_TYPE_VECTOR2: _ClassVar[ObjectType]
    OBJECT_TYPE_VECTOR3: _ClassVar[ObjectType]
    OBJECT_TYPE_VECTOR4: _ClassVar[ObjectType]
    OBJECT_TYPE_LINE: _ClassVar[ObjectType]
    OBJECT_TYPE_START_VIEW_STATE: _ClassVar[ObjectType]
    OBJECT_TYPE_FEED_VIEW_STATE: _ClassVar[ObjectType]
    OBJECT_TYPE_USER_WIZARD_VIEW_STATE: _ClassVar[ObjectType]
    OBJECT_TYPE_TREE_VIEW_STATE: _ClassVar[ObjectType]

class OrganizationStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ORGANIZATION_STATUS_UNSPECIFIED: _ClassVar[OrganizationStatus]
    ORGANIZATION_STATUS_REGISTERED: _ClassVar[OrganizationStatus]
    ORGANIZATION_STATUS_ACTIVATED: _ClassVar[OrganizationStatus]

class Orientation(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ORIENTATION_UNSPECIFIED: _ClassVar[Orientation]
    ORIENTATION_HORIZONTAL: _ClassVar[Orientation]
    ORIENTATION_HORIZONTAL_REVERSED: _ClassVar[Orientation]
    ORIENTATION_VERTICAL: _ClassVar[Orientation]
    ORIENTATION_VERTICAL_REVERSED: _ClassVar[Orientation]

class PathTokenType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PATH_TOKEN_TYPE_UNSPECIFIED: _ClassVar[PathTokenType]
    PATH_TOKEN_TYPE_ROOT: _ClassVar[PathTokenType]
    PATH_TOKEN_TYPE_CURRENT: _ClassVar[PathTokenType]
    PATH_TOKEN_TYPE_PARENT: _ClassVar[PathTokenType]
    PATH_TOKEN_TYPE_BENCH: _ClassVar[PathTokenType]
    PATH_TOKEN_TYPE_CHILD: _ClassVar[PathTokenType]
    PATH_TOKEN_TYPE_CONTAINER: _ClassVar[PathTokenType]
    PATH_TOKEN_TYPE_UNIQUE: _ClassVar[PathTokenType]
    PATH_TOKEN_TYPE_FIELD: _ClassVar[PathTokenType]

class PipeFilterType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PIPE_FILTER_TYPE_UNSPECIFIED: _ClassVar[PipeFilterType]
    PIPE_FILTER_TYPE_IS_NON_EMPTY: _ClassVar[PipeFilterType]
    PIPE_FILTER_TYPE_IS_TRUTHY: _ClassVar[PipeFilterType]
    PIPE_FILTER_TYPE_IS_EMPTY: _ClassVar[PipeFilterType]
    PIPE_FILTER_TYPE_IS_FALSY: _ClassVar[PipeFilterType]
    PIPE_FILTER_TYPE_HAS_ERROR: _ClassVar[PipeFilterType]

class PipeType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PIPE_TYPE_UNSPECIFIED: _ClassVar[PipeType]
    PIPE_TYPE_CONTROL_AND_DATA: _ClassVar[PipeType]
    PIPE_TYPE_DATA: _ClassVar[PipeType]

class PolicyEffect(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    POLICY_EFFECT_UNSPECIFIED: _ClassVar[PolicyEffect]
    POLICY_EFFECT_ALLOW: _ClassVar[PolicyEffect]
    POLICY_EFFECT_DENY: _ClassVar[PolicyEffect]

class PortSide(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PORT_SIDE_UNSPECIFIED: _ClassVar[PortSide]
    PORT_SIDE_INCOMING: _ClassVar[PortSide]
    PORT_SIDE_OUTGOING: _ClassVar[PortSide]

class PortType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PORT_TYPE_UNSPECIFIED: _ClassVar[PortType]
    PORT_TYPE_RUN: _ClassVar[PortType]
    PORT_TYPE_OBJECT: _ClassVar[PortType]
    PORT_TYPE_FIELD: _ClassVar[PortType]

class PrimitiveType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PRIMITIVE_TYPE_UNSPECIFIED: _ClassVar[PrimitiveType]
    PRIMITIVE_TYPE_BOOLEAN: _ClassVar[PrimitiveType]
    PRIMITIVE_TYPE_INT16: _ClassVar[PrimitiveType]
    PRIMITIVE_TYPE_INT32: _ClassVar[PrimitiveType]
    PRIMITIVE_TYPE_INT64: _ClassVar[PrimitiveType]
    PRIMITIVE_TYPE_DECIMAL: _ClassVar[PrimitiveType]
    PRIMITIVE_TYPE_FLOAT32: _ClassVar[PrimitiveType]
    PRIMITIVE_TYPE_FLOAT64: _ClassVar[PrimitiveType]
    PRIMITIVE_TYPE_STRING: _ClassVar[PrimitiveType]
    PRIMITIVE_TYPE_UUID: _ClassVar[PrimitiveType]
    PRIMITIVE_TYPE_JSON: _ClassVar[PrimitiveType]
    PRIMITIVE_TYPE_BYTES: _ClassVar[PrimitiveType]
    PRIMITIVE_TYPE_VECTOR: _ClassVar[PrimitiveType]
    PRIMITIVE_TYPE_DATETIME: _ClassVar[PrimitiveType]
    PRIMITIVE_TYPE_DATE: _ClassVar[PrimitiveType]
    PRIMITIVE_TYPE_TIME: _ClassVar[PrimitiveType]
    PRIMITIVE_TYPE_INTERVAL: _ClassVar[PrimitiveType]

class ReadType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    READ_TYPE_UNSPECIFIED: _ClassVar[ReadType]
    READ_TYPE_GET: _ClassVar[ReadType]
    READ_TYPE_SEARCH: _ClassVar[ReadType]
    READ_TYPE_AGGREGATE: _ClassVar[ReadType]

class ReferenceKind(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    REFERENCE_KIND_UNSPECIFIED: _ClassVar[ReferenceKind]
    REFERENCE_KIND_NODE_ANCESTOR: _ClassVar[ReferenceKind]
    REFERENCE_KIND_NODE_ANCESTOR_OR_SELF: _ClassVar[ReferenceKind]
    REFERENCE_KIND_NODE_PARENT: _ClassVar[ReferenceKind]
    REFERENCE_KIND_NODE_CHILDREN: _ClassVar[ReferenceKind]
    REFERENCE_KIND_NODE_REGULAR: _ClassVar[ReferenceKind]
    REFERENCE_KIND_NODE_TEMPLATE: _ClassVar[ReferenceKind]
    REFERENCE_KIND_STRUCT_PARENT: _ClassVar[ReferenceKind]
    REFERENCE_KIND_STRUCT_CHILD: _ClassVar[ReferenceKind]
    REFERENCE_KIND_PROPERTY: _ClassVar[ReferenceKind]

class Region(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    REGION_UNSPECIFIED: _ClassVar[Region]
    REGION_ZURICH: _ClassVar[Region]
    REGION_FRANKFURT: _ClassVar[Region]
    REGION_VIRGINIA: _ClassVar[Region]
    REGION_OHIO: _ClassVar[Region]
    REGION_OREGON: _ClassVar[Region]
    REGION_SAO_PAULO: _ClassVar[Region]
    REGION_CAPE_TOWN: _ClassVar[Region]
    REGION_MUMBAI: _ClassVar[Region]
    REGION_SINGAPORE: _ClassVar[Region]
    REGION_TOKYO: _ClassVar[Region]
    REGION_SYDNEY: _ClassVar[Region]

class RegionArea(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    REGION_AREA_UNSPECIFIED: _ClassVar[RegionArea]
    REGION_AREA_EUROPE: _ClassVar[RegionArea]
    REGION_AREA_NORTH_AMERICA: _ClassVar[RegionArea]
    REGION_AREA_SOUTH_AMERICA: _ClassVar[RegionArea]
    REGION_AREA_MIDDLE_EAST: _ClassVar[RegionArea]
    REGION_AREA_AFRICA: _ClassVar[RegionArea]
    REGION_AREA_ASIA: _ClassVar[RegionArea]
    REGION_AREA_AUSTRALIA: _ClassVar[RegionArea]
    REGION_AREA_PRIVATE: _ClassVar[RegionArea]
    REGION_AREA_GLOBAL: _ClassVar[RegionArea]

class RegionZone(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    REGION_ZONE_UNSPECIFIED: _ClassVar[RegionZone]
    REGION_ZONE_EUROPE_CENTRAL: _ClassVar[RegionZone]
    REGION_ZONE_NORTH_AMERICA_EAST: _ClassVar[RegionZone]
    REGION_ZONE_NORTH_AMERICA_WEST: _ClassVar[RegionZone]
    REGION_ZONE_SOUTH_AMERICA_EAST: _ClassVar[RegionZone]
    REGION_ZONE_MIDDLE_EAST_CENTRAL: _ClassVar[RegionZone]
    REGION_ZONE_MIDDLE_EAST_WEST: _ClassVar[RegionZone]
    REGION_ZONE_AFRICA_SOUTH: _ClassVar[RegionZone]
    REGION_ZONE_ASIA_WEST: _ClassVar[RegionZone]
    REGION_ZONE_ASIA_SOUTH: _ClassVar[RegionZone]
    REGION_ZONE_ASIA_EAST: _ClassVar[RegionZone]
    REGION_ZONE_AUSTRALIA_SOUTH: _ClassVar[RegionZone]

class ResourceStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RESOURCE_STATUS_UNSPECIFIED: _ClassVar[ResourceStatus]
    RESOURCE_STATUS_DECLARED: _ClassVar[ResourceStatus]
    RESOURCE_STATUS_UP: _ClassVar[ResourceStatus]
    RESOURCE_STATUS_SLEEPING: _ClassVar[ResourceStatus]
    RESOURCE_STATUS_DOWN: _ClassVar[ResourceStatus]
    RESOURCE_STATUS_DEGRADED: _ClassVar[ResourceStatus]
    RESOURCE_STATUS_GONE: _ClassVar[ResourceStatus]

class RunErrorKind(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RUN_ERROR_KIND_UNSPECIFIED: _ClassVar[RunErrorKind]
    RUN_ERROR_KIND_INTERNAL: _ClassVar[RunErrorKind]
    RUN_ERROR_KIND_RUNTIME: _ClassVar[RunErrorKind]

class RunErrorType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RUN_ERROR_TYPE_UNSPECIFIED: _ClassVar[RunErrorType]
    RUN_ERROR_TYPE_REPLAY: _ClassVar[RunErrorType]
    RUN_ERROR_TYPE_ABORTED: _ClassVar[RunErrorType]
    RUN_ERROR_TYPE_RUNTIME_UNAVAILABLE: _ClassVar[RunErrorType]
    RUN_ERROR_TYPE_RUN_IMPOSSIBLE: _ClassVar[RunErrorType]
    RUN_ERROR_TYPE_INVALID_VALUE: _ClassVar[RunErrorType]
    RUN_ERROR_TYPE_CODE_INVALID: _ClassVar[RunErrorType]
    RUN_ERROR_TYPE_TEXT_INVALID: _ClassVar[RunErrorType]
    RUN_ERROR_TYPE_MODEL_INCAPABLE: _ClassVar[RunErrorType]
    RUN_ERROR_TYPE_MANUAL_NONRETRYABLE: _ClassVar[RunErrorType]
    RUN_ERROR_TYPE_UNKNOWN_NONRETRYABLE: _ClassVar[RunErrorType]
    RUN_ERROR_TYPE_MODEL_FAILED: _ClassVar[RunErrorType]
    RUN_ERROR_TYPE_MANUAL_RETRYABLE: _ClassVar[RunErrorType]
    RUN_ERROR_TYPE_UNKNOWN_RETRYABLE: _ClassVar[RunErrorType]

class RunEventType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RUN_EVENT_TYPE_UNSPECIFIED: _ClassVar[RunEventType]
    RUN_EVENT_TYPE_PAUSED: _ClassVar[RunEventType]
    RUN_EVENT_TYPE_RESUMED: _ClassVar[RunEventType]
    RUN_EVENT_TYPE_HALTED: _ClassVar[RunEventType]
    RUN_EVENT_TYPE_CUSTOM: _ClassVar[RunEventType]

class RunKind(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RUN_KIND_UNSPECIFIED: _ClassVar[RunKind]
    RUN_KIND_CODE: _ClassVar[RunKind]
    RUN_KIND_TEXT: _ClassVar[RunKind]
    RUN_KIND_STEP: _ClassVar[RunKind]
    RUN_KIND_FLOW: _ClassVar[RunKind]

class RunSpanType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RUN_SPAN_TYPE_UNSPECIFIED: _ClassVar[RunSpanType]
    RUN_SPAN_TYPE_CUSTOM: _ClassVar[RunSpanType]

class RunStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RUN_STATUS_UNSPECIFIED: _ClassVar[RunStatus]
    RUN_STATUS_SCHEDULED: _ClassVar[RunStatus]
    RUN_STATUS_QUEUED: _ClassVar[RunStatus]
    RUN_STATUS_RUNNING: _ClassVar[RunStatus]
    RUN_STATUS_PAUSED: _ClassVar[RunStatus]
    RUN_STATUS_SUSPENDED: _ClassVar[RunStatus]
    RUN_STATUS_CANCELLED: _ClassVar[RunStatus]
    RUN_STATUS_ABORTED: _ClassVar[RunStatus]
    RUN_STATUS_FAILED: _ClassVar[RunStatus]
    RUN_STATUS_COMPLETED: _ClassVar[RunStatus]

class ScheduleType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SCHEDULE_TYPE_UNSPECIFIED: _ClassVar[ScheduleType]
    SCHEDULE_TYPE_INTERVAL: _ClassVar[ScheduleType]
    SCHEDULE_TYPE_CRON: _ClassVar[ScheduleType]

class SelectionKind(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SELECTION_KIND_UNSPECIFIED: _ClassVar[SelectionKind]
    SELECTION_KIND_RANGE: _ClassVar[SelectionKind]
    SELECTION_KIND_LIST: _ClassVar[SelectionKind]

class SelectionTarget(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SELECTION_TARGET_UNSPECIFIED: _ClassVar[SelectionTarget]
    SELECTION_TARGET_NODE: _ClassVar[SelectionTarget]
    SELECTION_TARGET_VALUE: _ClassVar[SelectionTarget]

class SessionStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SESSION_STATUS_UNSPECIFIED: _ClassVar[SessionStatus]
    SESSION_STATUS_PENDING: _ClassVar[SessionStatus]
    SESSION_STATUS_OPEN: _ClassVar[SessionStatus]
    SESSION_STATUS_CLOSED: _ClassVar[SessionStatus]

class SortMode(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SORT_MODE_UNSPECIFIED: _ClassVar[SortMode]
    SORT_MODE_MAX: _ClassVar[SortMode]
    SORT_MODE_MIN: _ClassVar[SortMode]
    SORT_MODE_AVERAGE: _ClassVar[SortMode]
    SORT_MODE_SUM: _ClassVar[SortMode]
    SORT_MODE_MEDIAN: _ClassVar[SortMode]

class SortOp(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SORT_OP_UNSPECIFIED: _ClassVar[SortOp]
    SORT_OP_ASCENDING: _ClassVar[SortOp]
    SORT_OP_DESCENDING: _ClassVar[SortOp]

class SpaceType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SPACE_TYPE_UNSPECIFIED: _ClassVar[SpaceType]
    SPACE_TYPE_DESKTOP: _ClassVar[SpaceType]
    SPACE_TYPE_BROWSER: _ClassVar[SpaceType]
    SPACE_TYPE_MOBILE: _ClassVar[SpaceType]

class Spacing(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SPACING_UNSPECIFIED: _ClassVar[Spacing]
    SPACING_S1: _ClassVar[Spacing]
    SPACING_S2: _ClassVar[Spacing]
    SPACING_S3: _ClassVar[Spacing]
    SPACING_S4: _ClassVar[Spacing]
    SPACING_S6: _ClassVar[Spacing]
    SPACING_S8: _ClassVar[Spacing]
    SPACING_S10: _ClassVar[Spacing]
    SPACING_S12: _ClassVar[Spacing]
    SPACING_S14: _ClassVar[Spacing]
    SPACING_S16: _ClassVar[Spacing]
    SPACING_S20: _ClassVar[Spacing]
    SPACING_S24: _ClassVar[Spacing]
    SPACING_S28: _ClassVar[Spacing]
    SPACING_S32: _ClassVar[Spacing]
    SPACING_S36: _ClassVar[Spacing]
    SPACING_S40: _ClassVar[Spacing]
    SPACING_S44: _ClassVar[Spacing]
    SPACING_S48: _ClassVar[Spacing]
    SPACING_S52: _ClassVar[Spacing]
    SPACING_S56: _ClassVar[Spacing]
    SPACING_S60: _ClassVar[Spacing]
    SPACING_S64: _ClassVar[Spacing]
    SPACING_S72: _ClassVar[Spacing]
    SPACING_S80: _ClassVar[Spacing]
    SPACING_S96: _ClassVar[Spacing]
    SPACING_S128: _ClassVar[Spacing]
    SPACING_S160: _ClassVar[Spacing]
    SPACING_S192: _ClassVar[Spacing]
    SPACING_S224: _ClassVar[Spacing]
    SPACING_S256: _ClassVar[Spacing]

class StepType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    STEP_TYPE_UNSPECIFIED: _ClassVar[StepType]
    STEP_TYPE_START: _ClassVar[StepType]
    STEP_TYPE_COMPLETE: _ClassVar[StepType]
    STEP_TYPE_FAIL: _ClassVar[StepType]
    STEP_TYPE_TRIGGER: _ClassVar[StepType]
    STEP_TYPE_BLOCK: _ClassVar[StepType]
    STEP_TYPE_TEXT: _ClassVar[StepType]
    STEP_TYPE_CODE: _ClassVar[StepType]
    STEP_TYPE_SEND: _ClassVar[StepType]
    STEP_TYPE_VALUE: _ClassVar[StepType]
    STEP_TYPE_GROUP: _ClassVar[StepType]
    STEP_TYPE_LOOP: _ClassVar[StepType]
    STEP_TYPE_REPEAT: _ClassVar[StepType]

class StructType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    STRUCT_TYPE_UNSPECIFIED: _ClassVar[StructType]
    STRUCT_TYPE_SESSION_CONTEXT: _ClassVar[StructType]
    STRUCT_TYPE_EDIT_CONTEXT: _ClassVar[StructType]
    STRUCT_TYPE_EDIT: _ClassVar[StructType]
    STRUCT_TYPE_EDIT_INFO: _ClassVar[StructType]
    STRUCT_TYPE_EDIT_OPERATION: _ClassVar[StructType]
    STRUCT_TYPE_CHANGE: _ClassVar[StructType]
    STRUCT_TYPE_CHANGE_VIGNETTE: _ClassVar[StructType]
    STRUCT_TYPE_GRAPH_SCOPE: _ClassVar[StructType]
    STRUCT_TYPE_CLIENT_ORIGIN: _ClassVar[StructType]
    STRUCT_TYPE_NODE_REFERENCE: _ClassVar[StructType]
    STRUCT_TYPE_PROPERTY_REFERENCE: _ClassVar[StructType]
    STRUCT_TYPE_PATH: _ClassVar[StructType]
    STRUCT_TYPE_PATH_TOKEN: _ClassVar[StructType]
    STRUCT_TYPE_POLICY: _ClassVar[StructType]
    STRUCT_TYPE_POLICY_RULE: _ClassVar[StructType]
    STRUCT_TYPE_SUBJECT: _ClassVar[StructType]
    STRUCT_TYPE_ACCESS_ZONE: _ClassVar[StructType]
    STRUCT_TYPE_ACCESS_MATRIX: _ClassVar[StructType]
    STRUCT_TYPE_ACCESS: _ClassVar[StructType]
    STRUCT_TYPE_TEXT: _ClassVar[StructType]
    STRUCT_TYPE_TEXT_LINE: _ClassVar[StructType]
    STRUCT_TYPE_TEXT_SPAN: _ClassVar[StructType]
    STRUCT_TYPE_TYPE_INFO: _ClassVar[StructType]
    STRUCT_TYPE_TYPE_CONSTRAINT: _ClassVar[StructType]
    STRUCT_TYPE_SCHEDULE: _ClassVar[StructType]
    STRUCT_TYPE_FILE_INFO: _ClassVar[StructType]
    STRUCT_TYPE_FILE_REFERENCE: _ClassVar[StructType]
    STRUCT_TYPE_ICON: _ClassVar[StructType]
    STRUCT_TYPE_SECRET_REFERENCE: _ClassVar[StructType]
    STRUCT_TYPE_TRIGGER_INFO: _ClassVar[StructType]
    STRUCT_TYPE_EXPRESSION: _ClassVar[StructType]
    STRUCT_TYPE_AGGREGATION: _ClassVar[StructType]
    STRUCT_TYPE_SELECTION: _ClassVar[StructType]
    STRUCT_TYPE_QUERY_INFO: _ClassVar[StructType]
    STRUCT_TYPE_READ_OPTIONS: _ClassVar[StructType]
    STRUCT_TYPE_VALUE: _ClassVar[StructType]
    STRUCT_TYPE_COMPUTED_VALUE: _ClassVar[StructType]
    STRUCT_TYPE_CODE: _ClassVar[StructType]
    STRUCT_TYPE_CODE_LINE: _ClassVar[StructType]
    STRUCT_TYPE_PORT_KEY: _ClassVar[StructType]
    STRUCT_TYPE_PORT: _ClassVar[StructType]
    STRUCT_TYPE_RUN_ERROR: _ClassVar[StructType]
    STRUCT_TYPE_RUN_OPTIONS: _ClassVar[StructType]
    STRUCT_TYPE_RUN_ATTEMPT: _ClassVar[StructType]
    STRUCT_TYPE_RUN_TRACE: _ClassVar[StructType]
    STRUCT_TYPE_RUN_FRAME: _ClassVar[StructType]
    STRUCT_TYPE_RUN_SPAN: _ClassVar[StructType]
    STRUCT_TYPE_RUN_EVENT: _ClassVar[StructType]
    STRUCT_TYPE_BREAKPOINT: _ClassVar[StructType]
    STRUCT_TYPE_LOG_INFO: _ClassVar[StructType]
    STRUCT_TYPE_COLOR: _ClassVar[StructType]
    STRUCT_TYPE_FONT: _ClassVar[StructType]
    STRUCT_TYPE_BOX: _ClassVar[StructType]
    STRUCT_TYPE_OFFSET: _ClassVar[StructType]
    STRUCT_TYPE_TRANSFORM: _ClassVar[StructType]
    STRUCT_TYPE_VECTOR2: _ClassVar[StructType]
    STRUCT_TYPE_VECTOR3: _ClassVar[StructType]
    STRUCT_TYPE_VECTOR4: _ClassVar[StructType]
    STRUCT_TYPE_LINE: _ClassVar[StructType]
    STRUCT_TYPE_START_VIEW_STATE: _ClassVar[StructType]
    STRUCT_TYPE_FEED_VIEW_STATE: _ClassVar[StructType]
    STRUCT_TYPE_USER_WIZARD_VIEW_STATE: _ClassVar[StructType]
    STRUCT_TYPE_TREE_VIEW_STATE: _ClassVar[StructType]

class TextLineType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TEXT_LINE_TYPE_UNSPECIFIED: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_PLAIN: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_HEADING_SMALL: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_HEADING_MEDIUM: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_HEADING_LARGE: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_CALLOUT: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_QUOTE: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_LIST_BULLET: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_LIST_NUMBERED: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_LIST_UNCHECKED: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_LIST_CHECKED: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_DIVIDER: _ClassVar[TextLineType]

class TimeInterval(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TIME_INTERVAL_UNSPECIFIED: _ClassVar[TimeInterval]
    TIME_INTERVAL_SECOND: _ClassVar[TimeInterval]
    TIME_INTERVAL_MINUTE: _ClassVar[TimeInterval]
    TIME_INTERVAL_HOUR: _ClassVar[TimeInterval]
    TIME_INTERVAL_DAY: _ClassVar[TimeInterval]
    TIME_INTERVAL_WEEK: _ClassVar[TimeInterval]
    TIME_INTERVAL_MONTH: _ClassVar[TimeInterval]
    TIME_INTERVAL_YEAR: _ClassVar[TimeInterval]

class TreeViewPreset(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TREE_VIEW_PRESET_UNSPECIFIED: _ClassVar[TreeViewPreset]
    TREE_VIEW_PRESET_EXPLORE: _ClassVar[TreeViewPreset]
    TREE_VIEW_PRESET_OUTLINE: _ClassVar[TreeViewPreset]

class TriggerType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TRIGGER_TYPE_UNSPECIFIED: _ClassVar[TriggerType]
    TRIGGER_TYPE_SCHEDULE: _ClassVar[TriggerType]
    TRIGGER_TYPE_SIGNAL: _ClassVar[TriggerType]

class TypeFormat(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TYPE_FORMAT_UNSPECIFIED: _ClassVar[TypeFormat]
    TYPE_FORMAT_URL: _ClassVar[TypeFormat]
    TYPE_FORMAT_EMAIL: _ClassVar[TypeFormat]
    TYPE_FORMAT_EMOJI: _ClassVar[TypeFormat]
    TYPE_FORMAT_PHONE_NUMBER: _ClassVar[TypeFormat]
    TYPE_FORMAT_SLUG: _ClassVar[TypeFormat]

class TypeKind(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TYPE_KIND_UNSPECIFIED: _ClassVar[TypeKind]
    TYPE_KIND_PRIMITIVE: _ClassVar[TypeKind]
    TYPE_KIND_STRUCT: _ClassVar[TypeKind]
    TYPE_KIND_NODE: _ClassVar[TypeKind]
    TYPE_KIND_ENUM: _ClassVar[TypeKind]
    TYPE_KIND_BASED_NODE: _ClassVar[TypeKind]
    TYPE_KIND_OBJECT: _ClassVar[TypeKind]
    TYPE_KIND_LITERAL: _ClassVar[TypeKind]
    TYPE_KIND_UNION: _ClassVar[TypeKind]
    TYPE_KIND_ALIAS: _ClassVar[TypeKind]

class UseType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    USE_TYPE_UNSPECIFIED: _ClassVar[UseType]
    USE_TYPE_START: _ClassVar[UseType]
    USE_TYPE_PAUSE: _ClassVar[UseType]
    USE_TYPE_RESUME: _ClassVar[UseType]
    USE_TYPE_STOP: _ClassVar[UseType]
    USE_TYPE_KILL: _ClassVar[UseType]
    USE_TYPE_SEND: _ClassVar[UseType]
    USE_TYPE_RECEIVE: _ClassVar[UseType]

class UserStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    USER_STATUS_UNSPECIFIED: _ClassVar[UserStatus]
    USER_STATUS_INVITED: _ClassVar[UserStatus]
    USER_STATUS_RESERVED: _ClassVar[UserStatus]
    USER_STATUS_WAITLISTED: _ClassVar[UserStatus]
    USER_STATUS_REGISTERED: _ClassVar[UserStatus]
    USER_STATUS_ACTIVATED: _ClassVar[UserStatus]

class UserWizardViewStage(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    USER_WIZARD_VIEW_STAGE_UNSPECIFIED: _ClassVar[UserWizardViewStage]
    USER_WIZARD_VIEW_STAGE_SIGN_UP: _ClassVar[UserWizardViewStage]
    USER_WIZARD_VIEW_STAGE_LOG_IN: _ClassVar[UserWizardViewStage]

class Variant(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    VARIANT_UNSPECIFIED: _ClassVar[Variant]
    VARIANT_PRIMARY: _ClassVar[Variant]
    VARIANT_SECONDARY: _ClassVar[Variant]
    VARIANT_COMPACT: _ClassVar[Variant]
    VARIANT_STEALTH: _ClassVar[Variant]

class ViewType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    VIEW_TYPE_UNSPECIFIED: _ClassVar[ViewType]
    VIEW_TYPE_USER_WIZARD: _ClassVar[ViewType]
    VIEW_TYPE_BENCH_WIZARD: _ClassVar[ViewType]
    VIEW_TYPE_EMPTY: _ClassVar[ViewType]
    VIEW_TYPE_PAGE: _ClassVar[ViewType]
    VIEW_TYPE_BLOCK: _ClassVar[ViewType]
    VIEW_TYPE_FIELD: _ClassVar[ViewType]
    VIEW_TYPE_DATABASE: _ClassVar[ViewType]
    VIEW_TYPE_VIEW: _ClassVar[ViewType]
    VIEW_TYPE_FLOW: _ClassVar[ViewType]
    VIEW_TYPE_STEP: _ClassVar[ViewType]
    VIEW_TYPE_TYPE: _ClassVar[ViewType]
    VIEW_TYPE_VARIABLE: _ClassVar[ViewType]
    VIEW_TYPE_OBJECT: _ClassVar[ViewType]
    VIEW_TYPE_RUN: _ClassVar[ViewType]
    VIEW_TYPE_LOG: _ClassVar[ViewType]
    VIEW_TYPE_PIPE: _ClassVar[ViewType]
    VIEW_TYPE_TREE: _ClassVar[ViewType]
    VIEW_TYPE_INSPECT: _ClassVar[ViewType]
    VIEW_TYPE_CREATE: _ClassVar[ViewType]
    VIEW_TYPE_CHAT: _ClassVar[ViewType]
    VIEW_TYPE_START: _ClassVar[ViewType]
    VIEW_TYPE_FEED: _ClassVar[ViewType]
    VIEW_TYPE_TIMELINE: _ClassVar[ViewType]
    VIEW_TYPE_HISTORY: _ClassVar[ViewType]
    VIEW_TYPE_WINDOW: _ClassVar[ViewType]
    VIEW_TYPE_TAB: _ClassVar[ViewType]
    VIEW_TYPE_SPLIT: _ClassVar[ViewType]
    VIEW_TYPE_SPLIT_DRAWER: _ClassVar[ViewType]
    VIEW_TYPE_STACK: _ClassVar[ViewType]
    VIEW_TYPE_DRAWER: _ClassVar[ViewType]
    VIEW_TYPE_SCROLL: _ClassVar[ViewType]
    VIEW_TYPE_GRID: _ClassVar[ViewType]
    VIEW_TYPE_GROUP: _ClassVar[ViewType]
    VIEW_TYPE_SECTION: _ClassVar[ViewType]
    VIEW_TYPE_FORM: _ClassVar[ViewType]
    VIEW_TYPE_SPACER: _ClassVar[ViewType]
    VIEW_TYPE_DIVIDER: _ClassVar[ViewType]
    VIEW_TYPE_LIST: _ClassVar[ViewType]
    VIEW_TYPE_TABLE: _ClassVar[ViewType]
    VIEW_TYPE_BREADCRUMB: _ClassVar[ViewType]
    VIEW_TYPE_PROGRESS: _ClassVar[ViewType]
    VIEW_TYPE_AVATAR: _ClassVar[ViewType]
    VIEW_TYPE_BADGE: _ClassVar[ViewType]
    VIEW_TYPE_SHAPE: _ClassVar[ViewType]
    VIEW_TYPE_CHART: _ClassVar[ViewType]
    VIEW_TYPE_BUTTON: _ClassVar[ViewType]
    VIEW_TYPE_MULTI_BUTTON: _ClassVar[ViewType]
    VIEW_TYPE_LINK: _ClassVar[ViewType]
    VIEW_TYPE_VALUE: _ClassVar[ViewType]
    VIEW_TYPE_NUMBER: _ClassVar[ViewType]
    VIEW_TYPE_SLIDER: _ClassVar[ViewType]
    VIEW_TYPE_STRING: _ClassVar[ViewType]
    VIEW_TYPE_TEXT: _ClassVar[ViewType]
    VIEW_TYPE_CODE: _ClassVar[ViewType]
    VIEW_TYPE_JSON: _ClassVar[ViewType]
    VIEW_TYPE_TOGGLE: _ClassVar[ViewType]
    VIEW_TYPE_PICKER: _ClassVar[ViewType]
    VIEW_TYPE_CALENDAR: _ClassVar[ViewType]
    VIEW_TYPE_MAP: _ClassVar[ViewType]
    VIEW_TYPE_COLOR: _ClassVar[ViewType]
    VIEW_TYPE_ICON: _ClassVar[ViewType]
    VIEW_TYPE_FILE: _ClassVar[ViewType]
    VIEW_TYPE_IMAGE: _ClassVar[ViewType]
    VIEW_TYPE_AUDIO: _ClassVar[ViewType]
    VIEW_TYPE_VIDEO: _ClassVar[ViewType]
    VIEW_TYPE_DOCUMENT: _ClassVar[ViewType]

ACCESS_KIND_UNSPECIFIED: AccessKind
ACCESS_KIND_READ: AccessKind
ACCESS_KIND_EDIT: AccessKind
ACCESS_KIND_USE: AccessKind
ACCESS_MODE_UNSPECIFIED: AccessMode
ACCESS_MODE_ADAPTIVE: AccessMode
ACCESS_MODE_ATOMIC: AccessMode
ACCESS_TYPE_UNSPECIFIED: AccessType
ACCESS_TYPE_GET: AccessType
ACCESS_TYPE_SEARCH: AccessType
ACCESS_TYPE_AGGREGATE: AccessType
ACCESS_TYPE_CREATE: AccessType
ACCESS_TYPE_UPSERT: AccessType
ACCESS_TYPE_UPDATE: AccessType
ACCESS_TYPE_MOVE: AccessType
ACCESS_TYPE_ARCHIVE: AccessType
ACCESS_TYPE_UNARCHIVE: AccessType
ACCESS_TYPE_DELETE: AccessType
ACCESS_TYPE_RESTORE: AccessType
ACCESS_TYPE_ERASE: AccessType
ACCESS_TYPE_START: AccessType
ACCESS_TYPE_PAUSE: AccessType
ACCESS_TYPE_RESUME: AccessType
ACCESS_TYPE_STOP: AccessType
ACCESS_TYPE_KILL: AccessType
ACCESS_TYPE_SEND: AccessType
ACCESS_TYPE_RECEIVE: AccessType
AGGREGATION_OP_UNSPECIFIED: AggregationOp
AGGREGATION_OP_EXISTS: AggregationOp
AGGREGATION_OP_COUNT: AggregationOp
AGGREGATION_OP_SUM: AggregationOp
AGGREGATION_OP_MIN: AggregationOp
AGGREGATION_OP_MAX: AggregationOp
AGGREGATION_OP_AVERAGE: AggregationOp
AGGREGATION_OP_MEDIAN: AggregationOp
AGGREGATION_OP_HISTOGRAM: AggregationOp
ALIGNMENT_UNSPECIFIED: Alignment
ALIGNMENT_START: Alignment
ALIGNMENT_MIDDLE: Alignment
ALIGNMENT_END: Alignment
ALIGNMENT_SPACE_BETWEEN: Alignment
ANCHOR_UNSPECIFIED: Anchor
ANCHOR_TOP: Anchor
ANCHOR_TOP_LEFT: Anchor
ANCHOR_TOP_RIGHT: Anchor
ANCHOR_RIGHT: Anchor
ANCHOR_RIGHT_TOP: Anchor
ANCHOR_RIGHT_BOTTOM: Anchor
ANCHOR_BOTTOM: Anchor
ANCHOR_BOTTOM_LEFT: Anchor
ANCHOR_BOTTOM_RIGHT: Anchor
ANCHOR_LEFT: Anchor
ANCHOR_LEFT_TOP: Anchor
ANCHOR_LEFT_BOTTOM: Anchor
BENCH_TYPE_UNSPECIFIED: BenchType
BENCH_TYPE_BENCH: BenchType
BENCH_TYPE_USER: BenchType
BENCH_TYPE_ORGANIZATION: BenchType
BENCH_TYPE_HANDLE: BenchType
BENCH_TYPE_CLIENT: BenchType
BENCH_TYPE_SERVER: BenchType
BENCH_TYPE_STORE: BenchType
BENCH_TYPE_MACHINE: BenchType
BENCH_TYPE_DRIVE: BenchType
BENCH_TYPE_VAULT: BenchType
BENCH_TYPE_CACHE: BenchType
BENCH_TYPE_FILE: BenchType
BENCH_TYPE_SECRET: BenchType
BENCH_TYPE_MEMBERSHIP: BenchType
BENCH_TYPE_INVITE: BenchType
BENCH_TYPE_BRANCH: BenchType
BENCH_TYPE_PACKAGE: BenchType
BENCH_TYPE_DEPENDENCY: BenchType
BENCH_TYPE_SPACE: BenchType
BENCH_TYPE_BLOCK: BenchType
BENCH_TYPE_TRIGGER: BenchType
BENCH_TYPE_FIELD: BenchType
BENCH_TYPE_QUERY: BenchType
BENCH_TYPE_VIEW: BenchType
BENCH_TYPE_STEP: BenchType
BENCH_TYPE_PIPE: BenchType
BENCH_TYPE_BADGE: BenchType
BENCH_TYPE_MESSAGE: BenchType
BENCH_TYPE_RECORD: BenchType
BENCH_TYPE_SESSION: BenchType
BENCH_TYPE_RUN: BenchType
BENCH_TYPE_SIGNAL: BenchType
BENCH_TYPE_LOG: BenchType
BENCH_TYPE_NOTIFICATION: BenchType
BENCH_TYPE_SKIP: BenchType
BENCH_TYPE_SESSION_CONTEXT: BenchType
BENCH_TYPE_EDIT_CONTEXT: BenchType
BENCH_TYPE_EDIT: BenchType
BENCH_TYPE_EDIT_INFO: BenchType
BENCH_TYPE_EDIT_OPERATION: BenchType
BENCH_TYPE_CHANGE: BenchType
BENCH_TYPE_CHANGE_VIGNETTE: BenchType
BENCH_TYPE_GRAPH_SCOPE: BenchType
BENCH_TYPE_CLIENT_ORIGIN: BenchType
BENCH_TYPE_NODE_REFERENCE: BenchType
BENCH_TYPE_PROPERTY_REFERENCE: BenchType
BENCH_TYPE_PATH: BenchType
BENCH_TYPE_PATH_TOKEN: BenchType
BENCH_TYPE_POLICY: BenchType
BENCH_TYPE_POLICY_RULE: BenchType
BENCH_TYPE_SUBJECT: BenchType
BENCH_TYPE_ACCESS_ZONE: BenchType
BENCH_TYPE_ACCESS_MATRIX: BenchType
BENCH_TYPE_ACCESS: BenchType
BENCH_TYPE_TEXT: BenchType
BENCH_TYPE_TEXT_LINE: BenchType
BENCH_TYPE_TEXT_SPAN: BenchType
BENCH_TYPE_TYPE_INFO: BenchType
BENCH_TYPE_TYPE_CONSTRAINT: BenchType
BENCH_TYPE_SCHEDULE: BenchType
BENCH_TYPE_FILE_INFO: BenchType
BENCH_TYPE_FILE_REFERENCE: BenchType
BENCH_TYPE_ICON: BenchType
BENCH_TYPE_SECRET_REFERENCE: BenchType
BENCH_TYPE_TRIGGER_INFO: BenchType
BENCH_TYPE_EXPRESSION: BenchType
BENCH_TYPE_AGGREGATION: BenchType
BENCH_TYPE_SELECTION: BenchType
BENCH_TYPE_QUERY_INFO: BenchType
BENCH_TYPE_READ_OPTIONS: BenchType
BENCH_TYPE_VALUE: BenchType
BENCH_TYPE_COMPUTED_VALUE: BenchType
BENCH_TYPE_CODE: BenchType
BENCH_TYPE_CODE_LINE: BenchType
BENCH_TYPE_PORT_KEY: BenchType
BENCH_TYPE_PORT: BenchType
BENCH_TYPE_RUN_ERROR: BenchType
BENCH_TYPE_RUN_OPTIONS: BenchType
BENCH_TYPE_RUN_ATTEMPT: BenchType
BENCH_TYPE_RUN_TRACE: BenchType
BENCH_TYPE_RUN_FRAME: BenchType
BENCH_TYPE_RUN_SPAN: BenchType
BENCH_TYPE_RUN_EVENT: BenchType
BENCH_TYPE_BREAKPOINT: BenchType
BENCH_TYPE_LOG_INFO: BenchType
BENCH_TYPE_COLOR: BenchType
BENCH_TYPE_FONT: BenchType
BENCH_TYPE_BOX: BenchType
BENCH_TYPE_OFFSET: BenchType
BENCH_TYPE_TRANSFORM: BenchType
BENCH_TYPE_VECTOR2: BenchType
BENCH_TYPE_VECTOR3: BenchType
BENCH_TYPE_VECTOR4: BenchType
BENCH_TYPE_LINE: BenchType
BENCH_TYPE_START_VIEW_STATE: BenchType
BENCH_TYPE_FEED_VIEW_STATE: BenchType
BENCH_TYPE_USER_WIZARD_VIEW_STATE: BenchType
BENCH_TYPE_TREE_VIEW_STATE: BenchType
BENCH_TYPE_ENUM_TYPE: BenchType
BENCH_TYPE_NODE_TYPE: BenchType
BENCH_TYPE_STRUCT_TYPE: BenchType
BENCH_TYPE_OBJECT_TYPE: BenchType
BENCH_TYPE_BENCH_TYPE: BenchType
BENCH_TYPE_CHANGE_KIND: BenchType
BENCH_TYPE_ACCESS_MODE: BenchType
BENCH_TYPE_ACCESS_KIND: BenchType
BENCH_TYPE_READ_TYPE: BenchType
BENCH_TYPE_EDIT_TYPE: BenchType
BENCH_TYPE_USE_TYPE: BenchType
BENCH_TYPE_ACCESS_TYPE: BenchType
BENCH_TYPE_CHANGE_CATEGORY: BenchType
BENCH_TYPE_EDIT_OPERATION_TYPE: BenchType
BENCH_TYPE_POLICY_EFFECT: BenchType
BENCH_TYPE_CLOUD: BenchType
BENCH_TYPE_REGION: BenchType
BENCH_TYPE_REGION_ZONE: BenchType
BENCH_TYPE_REGION_AREA: BenchType
BENCH_TYPE_RESOURCE_STATUS: BenchType
BENCH_TYPE_FILE_RETENTION_MODE: BenchType
BENCH_TYPE_FILE_KIND: BenchType
BENCH_TYPE_FILE_TYPE: BenchType
BENCH_TYPE_FILE_FORMAT: BenchType
BENCH_TYPE_CLIENT_TYPE: BenchType
BENCH_TYPE_PRIMITIVE_TYPE: BenchType
BENCH_TYPE_FIELD_ZONE: BenchType
BENCH_TYPE_TYPE_KIND: BenchType
BENCH_TYPE_TYPE_FORMAT: BenchType
BENCH_TYPE_BLOCK_TYPE: BenchType
BENCH_TYPE_SCHEDULE_TYPE: BenchType
BENCH_TYPE_TIME_INTERVAL: BenchType
BENCH_TYPE_DAY: BenchType
BENCH_TYPE_MONTH: BenchType
BENCH_TYPE_TEXT_LINE_TYPE: BenchType
BENCH_TYPE_ICON_KIND: BenchType
BENCH_TYPE_EXPRESSION_KIND: BenchType
BENCH_TYPE_EXPRESSION_OP: BenchType
BENCH_TYPE_LITERAL_OP: BenchType
BENCH_TYPE_FUNCTIONAL_OP: BenchType
BENCH_TYPE_CONDITIONAL_OP: BenchType
BENCH_TYPE_AGGREGATION_OP: BenchType
BENCH_TYPE_SORT_MODE: BenchType
BENCH_TYPE_SORT_OP: BenchType
BENCH_TYPE_SELECTION_KIND: BenchType
BENCH_TYPE_SELECTION_TARGET: BenchType
BENCH_TYPE_PATH_TOKEN_TYPE: BenchType
BENCH_TYPE_LOG_KIND: BenchType
BENCH_TYPE_LOG_LEVEL: BenchType
BENCH_TYPE_RUN_STATUS: BenchType
BENCH_TYPE_RUN_KIND: BenchType
BENCH_TYPE_RUN_ERROR_KIND: BenchType
BENCH_TYPE_RUN_ERROR_TYPE: BenchType
BENCH_TYPE_RUN_SPAN_TYPE: BenchType
BENCH_TYPE_RUN_EVENT_TYPE: BenchType
BENCH_TYPE_SESSION_STATUS: BenchType
BENCH_TYPE_TRIGGER_TYPE: BenchType
BENCH_TYPE_BREAKPOINT_KIND: BenchType
BENCH_TYPE_BREAKPOINT_ACTION: BenchType
BENCH_TYPE_CACHE_MODE: BenchType
BENCH_TYPE_CODE_TYPE: BenchType
BENCH_TYPE_MODEL_PROVIDER: BenchType
BENCH_TYPE_MODEL_TYPE: BenchType
BENCH_TYPE_STEP_TYPE: BenchType
BENCH_TYPE_PIPE_TYPE: BenchType
BENCH_TYPE_PIPE_FILTER_TYPE: BenchType
BENCH_TYPE_PORT_TYPE: BenchType
BENCH_TYPE_PORT_SIDE: BenchType
BENCH_TYPE_NOTIFICATION_LEVEL: BenchType
BENCH_TYPE_SPACE_TYPE: BenchType
BENCH_TYPE_VIEW_TYPE: BenchType
BENCH_TYPE_VARIANT: BenchType
BENCH_TYPE_COLOR_TYPE: BenchType
BENCH_TYPE_COLOR_SHADE: BenchType
BENCH_TYPE_FONT_TYPE: BenchType
BENCH_TYPE_FONT_WEIGHT: BenchType
BENCH_TYPE_FONT_SIZE: BenchType
BENCH_TYPE_SPACING: BenchType
BENCH_TYPE_ANCHOR: BenchType
BENCH_TYPE_ORIENTATION: BenchType
BENCH_TYPE_ALIGNMENT: BenchType
BENCH_TYPE_USER_WIZARD_STAGE: BenchType
BENCH_TYPE_TREE_VIEW_PRESET: BenchType
BENCH_TYPE_USER_STATUS: BenchType
BENCH_TYPE_ORGANIZATION_STATUS: BenchType
BLOCK_TYPE_UNSPECIFIED: BlockType
BLOCK_TYPE_PAGE: BlockType
BLOCK_TYPE_CLASS: BlockType
BLOCK_TYPE_CHOICE: BlockType
BLOCK_TYPE_SIGNAL: BlockType
BLOCK_TYPE_NOTIFICATION: BlockType
BLOCK_TYPE_TEXT: BlockType
BLOCK_TYPE_CODE: BlockType
BLOCK_TYPE_FLOW: BlockType
BLOCK_TYPE_VALUE: BlockType
BLOCK_TYPE_DATABASE: BlockType
BLOCK_TYPE_QUERY: BlockType
BLOCK_TYPE_VIEW: BlockType
BLOCK_TYPE_ROLE: BlockType
BLOCK_TYPE_IDENTITY: BlockType
BREAKPOINT_ACTION_UNSPECIFIED: BreakpointAction
BREAKPOINT_ACTION_SUSPEND: BreakpointAction
BREAKPOINT_KIND_UNSPECIFIED: BreakpointKind
BREAKPOINT_KIND_START_RUN: BreakpointKind
BREAKPOINT_KIND_FAIL_RUN: BreakpointKind
BREAKPOINT_KIND_COMPLETE_RUN: BreakpointKind
BREAKPOINT_KIND_CODE_LINE: BreakpointKind
CACHE_MODE_UNSPECIFIED: CacheMode
CACHE_MODE_NEVER: CacheMode
CACHE_MODE_ALWAYS: CacheMode
CHANGE_CATEGORY_UNSPECIFIED: ChangeCategory
CHANGE_CATEGORY_SPACE: ChangeCategory
CHANGE_CATEGORY_SESSION: ChangeCategory
CHANGE_KIND_UNSPECIFIED: ChangeKind
CHANGE_KIND_CODE: ChangeKind
CHANGE_KIND_LOGS: ChangeKind
CHANGE_KIND_LOGS_QUERY: ChangeKind
CLIENT_TYPE_UNSPECIFIED: ClientType
CLIENT_TYPE_BENCH_WEB: ClientType
CLIENT_TYPE_BENCH_BROWSER_PLUGIN: ClientType
CLIENT_TYPE_BENCH_DESKTOP: ClientType
CLIENT_TYPE_BENCH_MOBILE: ClientType
CLIENT_TYPE_BENCH_MACHINE: ClientType
CLOUD_UNSPECIFIED: Cloud
CLOUD_AWS: Cloud
CLOUD_AZURE: Cloud
CLOUD_GCP: Cloud
CLOUD_OCI: Cloud
CLOUD_ALIBABA: Cloud
CLOUD_HETZNER: Cloud
CLOUD_NEON: Cloud
CLOUD_PRIVATE: Cloud
CODE_TYPE_UNSPECIFIED: CodeType
CODE_TYPE_SNIPPET: CodeType
CODE_TYPE_SCRIPT: CodeType
CODE_TYPE_FUNCTION: CodeType
COLOR_SHADE_UNSPECIFIED: ColorShade
COLOR_SHADE_S50: ColorShade
COLOR_SHADE_S100: ColorShade
COLOR_SHADE_S200: ColorShade
COLOR_SHADE_S300: ColorShade
COLOR_SHADE_S400: ColorShade
COLOR_SHADE_S500: ColorShade
COLOR_SHADE_S600: ColorShade
COLOR_SHADE_S700: ColorShade
COLOR_SHADE_S800: ColorShade
COLOR_SHADE_S900: ColorShade
COLOR_SHADE_S950: ColorShade
COLOR_TYPE_UNSPECIFIED: ColorType
COLOR_TYPE_PRIMARY: ColorType
COLOR_TYPE_SECONDARY: ColorType
COLOR_TYPE_ACCENT: ColorType
COLOR_TYPE_CANVAS: ColorType
COLOR_TYPE_SUCCESS: ColorType
COLOR_TYPE_HINT: ColorType
COLOR_TYPE_WARNING: ColorType
COLOR_TYPE_DANGER: ColorType
COLOR_TYPE_GRAY: ColorType
COLOR_TYPE_RED: ColorType
COLOR_TYPE_ORANGE: ColorType
COLOR_TYPE_AMBER: ColorType
COLOR_TYPE_YELLOW: ColorType
COLOR_TYPE_LIME: ColorType
COLOR_TYPE_GREEN: ColorType
COLOR_TYPE_EMERALD: ColorType
COLOR_TYPE_TEAL: ColorType
COLOR_TYPE_CYAN: ColorType
COLOR_TYPE_SKY: ColorType
COLOR_TYPE_BLUE: ColorType
COLOR_TYPE_INDIGO: ColorType
COLOR_TYPE_VIOLET: ColorType
COLOR_TYPE_PURPLE: ColorType
COLOR_TYPE_FUCHSIA: ColorType
COLOR_TYPE_PINK: ColorType
COLOR_TYPE_ROSE: ColorType
CONDITIONAL_OP_UNSPECIFIED: ConditionalOp
CONDITIONAL_OP_NOT: ConditionalOp
CONDITIONAL_OP_AND: ConditionalOp
CONDITIONAL_OP_OR: ConditionalOp
CONDITIONAL_OP_EQUALS: ConditionalOp
CONDITIONAL_OP_NOT_EQUALS: ConditionalOp
CONDITIONAL_OP_GREATER_THAN: ConditionalOp
CONDITIONAL_OP_GREATER_THAN_OR_EQUALS: ConditionalOp
CONDITIONAL_OP_LESS_THAN: ConditionalOp
CONDITIONAL_OP_LESS_THAN_OR_EQUALS: ConditionalOp
CONDITIONAL_OP_MATCHES_REGEX: ConditionalOp
CONDITIONAL_OP_STARTS_WITH: ConditionalOp
CONDITIONAL_OP_ENDS_WITH: ConditionalOp
CONDITIONAL_OP_CONTAINS: ConditionalOp
CONDITIONAL_OP_NOT_CONTAINS: ConditionalOp
CONDITIONAL_OP_IN: ConditionalOp
CONDITIONAL_OP_NOT_IN: ConditionalOp
CONDITIONAL_OP_EXISTS: ConditionalOp
CONDITIONAL_OP_NOT_EXISTS: ConditionalOp
CONDITIONAL_OP_NEAR: ConditionalOp
DAY_UNSPECIFIED: Day
DAY_MONDAY: Day
DAY_TUESDAY: Day
DAY_WEDNESDAY: Day
DAY_THURSDAY: Day
DAY_FRIDAY: Day
DAY_SATURDAY: Day
DAY_SUNDAY: Day
EDIT_OPERATION_TYPE_UNSPECIFIED: EditOperationType
EDIT_OPERATION_TYPE_SET: EditOperationType
EDIT_OPERATION_TYPE_CLEAR: EditOperationType
EDIT_OPERATION_TYPE_APPEND: EditOperationType
EDIT_OPERATION_TYPE_REMOVE: EditOperationType
EDIT_TYPE_UNSPECIFIED: EditType
EDIT_TYPE_CREATE: EditType
EDIT_TYPE_UPSERT: EditType
EDIT_TYPE_UPDATE: EditType
EDIT_TYPE_MOVE: EditType
EDIT_TYPE_ARCHIVE: EditType
EDIT_TYPE_UNARCHIVE: EditType
EDIT_TYPE_DELETE: EditType
EDIT_TYPE_RESTORE: EditType
EDIT_TYPE_ERASE: EditType
ENUM_TYPE_UNSPECIFIED: EnumType
ENUM_TYPE_ENUM_TYPE: EnumType
ENUM_TYPE_NODE_TYPE: EnumType
ENUM_TYPE_STRUCT_TYPE: EnumType
ENUM_TYPE_OBJECT_TYPE: EnumType
ENUM_TYPE_BENCH_TYPE: EnumType
ENUM_TYPE_CHANGE_KIND: EnumType
ENUM_TYPE_ACCESS_MODE: EnumType
ENUM_TYPE_ACCESS_KIND: EnumType
ENUM_TYPE_READ_TYPE: EnumType
ENUM_TYPE_EDIT_TYPE: EnumType
ENUM_TYPE_USE_TYPE: EnumType
ENUM_TYPE_ACCESS_TYPE: EnumType
ENUM_TYPE_CHANGE_CATEGORY: EnumType
ENUM_TYPE_EDIT_OPERATION_TYPE: EnumType
ENUM_TYPE_POLICY_EFFECT: EnumType
ENUM_TYPE_CLOUD: EnumType
ENUM_TYPE_REGION: EnumType
ENUM_TYPE_REGION_ZONE: EnumType
ENUM_TYPE_REGION_AREA: EnumType
ENUM_TYPE_RESOURCE_STATUS: EnumType
ENUM_TYPE_FILE_RETENTION_MODE: EnumType
ENUM_TYPE_FILE_KIND: EnumType
ENUM_TYPE_FILE_TYPE: EnumType
ENUM_TYPE_FILE_FORMAT: EnumType
ENUM_TYPE_CLIENT_TYPE: EnumType
ENUM_TYPE_PRIMITIVE_TYPE: EnumType
ENUM_TYPE_FIELD_ZONE: EnumType
ENUM_TYPE_TYPE_KIND: EnumType
ENUM_TYPE_TYPE_FORMAT: EnumType
ENUM_TYPE_BLOCK_TYPE: EnumType
ENUM_TYPE_SCHEDULE_TYPE: EnumType
ENUM_TYPE_TIME_INTERVAL: EnumType
ENUM_TYPE_DAY: EnumType
ENUM_TYPE_MONTH: EnumType
ENUM_TYPE_TEXT_LINE_TYPE: EnumType
ENUM_TYPE_ICON_KIND: EnumType
ENUM_TYPE_EXPRESSION_KIND: EnumType
ENUM_TYPE_EXPRESSION_OP: EnumType
ENUM_TYPE_LITERAL_OP: EnumType
ENUM_TYPE_FUNCTIONAL_OP: EnumType
ENUM_TYPE_CONDITIONAL_OP: EnumType
ENUM_TYPE_AGGREGATION_OP: EnumType
ENUM_TYPE_SORT_MODE: EnumType
ENUM_TYPE_SORT_OP: EnumType
ENUM_TYPE_SELECTION_KIND: EnumType
ENUM_TYPE_SELECTION_TARGET: EnumType
ENUM_TYPE_PATH_TOKEN_TYPE: EnumType
ENUM_TYPE_LOG_KIND: EnumType
ENUM_TYPE_LOG_LEVEL: EnumType
ENUM_TYPE_RUN_STATUS: EnumType
ENUM_TYPE_RUN_KIND: EnumType
ENUM_TYPE_RUN_ERROR_KIND: EnumType
ENUM_TYPE_RUN_ERROR_TYPE: EnumType
ENUM_TYPE_RUN_SPAN_TYPE: EnumType
ENUM_TYPE_RUN_EVENT_TYPE: EnumType
ENUM_TYPE_SESSION_STATUS: EnumType
ENUM_TYPE_TRIGGER_TYPE: EnumType
ENUM_TYPE_BREAKPOINT_KIND: EnumType
ENUM_TYPE_BREAKPOINT_ACTION: EnumType
ENUM_TYPE_CACHE_MODE: EnumType
ENUM_TYPE_CODE_TYPE: EnumType
ENUM_TYPE_MODEL_PROVIDER: EnumType
ENUM_TYPE_MODEL_TYPE: EnumType
ENUM_TYPE_STEP_TYPE: EnumType
ENUM_TYPE_PIPE_TYPE: EnumType
ENUM_TYPE_PIPE_FILTER_TYPE: EnumType
ENUM_TYPE_PORT_TYPE: EnumType
ENUM_TYPE_PORT_SIDE: EnumType
ENUM_TYPE_NOTIFICATION_LEVEL: EnumType
ENUM_TYPE_SPACE_TYPE: EnumType
ENUM_TYPE_VIEW_TYPE: EnumType
ENUM_TYPE_VARIANT: EnumType
ENUM_TYPE_COLOR_TYPE: EnumType
ENUM_TYPE_COLOR_SHADE: EnumType
ENUM_TYPE_FONT_TYPE: EnumType
ENUM_TYPE_FONT_WEIGHT: EnumType
ENUM_TYPE_FONT_SIZE: EnumType
ENUM_TYPE_SPACING: EnumType
ENUM_TYPE_ANCHOR: EnumType
ENUM_TYPE_ORIENTATION: EnumType
ENUM_TYPE_ALIGNMENT: EnumType
ENUM_TYPE_USER_WIZARD_STAGE: EnumType
ENUM_TYPE_TREE_VIEW_PRESET: EnumType
ENUM_TYPE_USER_STATUS: EnumType
ENUM_TYPE_ORGANIZATION_STATUS: EnumType
EXPRESSION_KIND_UNSPECIFIED: ExpressionKind
EXPRESSION_KIND_LITERAL: ExpressionKind
EXPRESSION_KIND_FUNCTIONAL: ExpressionKind
EXPRESSION_KIND_CONDITIONAL: ExpressionKind
EXPRESSION_KIND_SORT: ExpressionKind
EXPRESSION_KIND_AGGREGATION: ExpressionKind
EXPRESSION_OP_UNSPECIFIED: ExpressionOp
EXPRESSION_OP_VALUE: ExpressionOp
EXPRESSION_OP_NONE: ExpressionOp
EXPRESSION_OP_TRUE: ExpressionOp
EXPRESSION_OP_FALSE: ExpressionOp
EXPRESSION_OP_ADD: ExpressionOp
EXPRESSION_OP_SUBTRACT: ExpressionOp
EXPRESSION_OP_MULTIPLY: ExpressionOp
EXPRESSION_OP_DIVIDE: ExpressionOp
EXPRESSION_OP_MODULO: ExpressionOp
EXPRESSION_OP_POWER: ExpressionOp
EXPRESSION_OP_NOT: ExpressionOp
EXPRESSION_OP_AND: ExpressionOp
EXPRESSION_OP_OR: ExpressionOp
EXPRESSION_OP_EQUALS: ExpressionOp
EXPRESSION_OP_NOT_EQUALS: ExpressionOp
EXPRESSION_OP_GREATER_THAN: ExpressionOp
EXPRESSION_OP_GREATER_THAN_OR_EQUALS: ExpressionOp
EXPRESSION_OP_LESS_THAN: ExpressionOp
EXPRESSION_OP_LESS_THAN_OR_EQUALS: ExpressionOp
EXPRESSION_OP_MATCHES_REGEX: ExpressionOp
EXPRESSION_OP_STARTS_WITH: ExpressionOp
EXPRESSION_OP_ENDS_WITH: ExpressionOp
EXPRESSION_OP_CONTAINS: ExpressionOp
EXPRESSION_OP_NOT_CONTAINS: ExpressionOp
EXPRESSION_OP_IN: ExpressionOp
EXPRESSION_OP_NOT_IN: ExpressionOp
EXPRESSION_OP_EXISTS: ExpressionOp
EXPRESSION_OP_NOT_EXISTS: ExpressionOp
EXPRESSION_OP_NEAR: ExpressionOp
EXPRESSION_OP_COUNT: ExpressionOp
EXPRESSION_OP_SUM: ExpressionOp
EXPRESSION_OP_MIN: ExpressionOp
EXPRESSION_OP_MAX: ExpressionOp
EXPRESSION_OP_AVERAGE: ExpressionOp
EXPRESSION_OP_MEDIAN: ExpressionOp
EXPRESSION_OP_HISTOGRAM: ExpressionOp
EXPRESSION_OP_ASCENDING: ExpressionOp
EXPRESSION_OP_DESCENDING: ExpressionOp
FIELD_ZONE_UNSPECIFIED: FieldZone
FIELD_ZONE_VARIABLE: FieldZone
FIELD_ZONE_MEMBER: FieldZone
FIELD_ZONE_INPUT: FieldZone
FIELD_ZONE_OUTPUT: FieldZone
FIELD_ZONE_OPTION: FieldZone
FILE_FORMAT_UNSPECIFIED: FileFormat
FILE_FORMAT_TXT: FileFormat
FILE_FORMAT_MARKDOWN: FileFormat
FILE_FORMAT_RTF: FileFormat
FILE_FORMAT_INI: FileFormat
FILE_FORMAT_LOG: FileFormat
FILE_FORMAT_PYTHON: FileFormat
FILE_FORMAT_JAVASCRIPT: FileFormat
FILE_FORMAT_TYPESCRIPT: FileFormat
FILE_FORMAT_GO: FileFormat
FILE_FORMAT_C_LANG: FileFormat
FILE_FORMAT_CPP: FileFormat
FILE_FORMAT_OBJECTIVE_C: FileFormat
FILE_FORMAT_SWIFT: FileFormat
FILE_FORMAT_RUBY: FileFormat
FILE_FORMAT_PHP: FileFormat
FILE_FORMAT_CSS: FileFormat
FILE_FORMAT_JAVA: FileFormat
FILE_FORMAT_KOTLIN: FileFormat
FILE_FORMAT_RUST: FileFormat
FILE_FORMAT_SCALA: FileFormat
FILE_FORMAT_SHELL: FileFormat
FILE_FORMAT_SQL: FileFormat
FILE_FORMAT_POWERSHELL: FileFormat
FILE_FORMAT_ASSEMBLY: FileFormat
FILE_FORMAT_LATEX: FileFormat
FILE_FORMAT_JPEG: FileFormat
FILE_FORMAT_PNG: FileFormat
FILE_FORMAT_GIF: FileFormat
FILE_FORMAT_BMP: FileFormat
FILE_FORMAT_TIFF: FileFormat
FILE_FORMAT_WEBP: FileFormat
FILE_FORMAT_SVG: FileFormat
FILE_FORMAT_ICO: FileFormat
FILE_FORMAT_RAW: FileFormat
FILE_FORMAT_HEIC: FileFormat
FILE_FORMAT_HEIF: FileFormat
FILE_FORMAT_MP3: FileFormat
FILE_FORMAT_WAV: FileFormat
FILE_FORMAT_FLAC: FileFormat
FILE_FORMAT_AAC: FileFormat
FILE_FORMAT_OGG: FileFormat
FILE_FORMAT_M4A: FileFormat
FILE_FORMAT_WMA: FileFormat
FILE_FORMAT_MP4: FileFormat
FILE_FORMAT_WEBM: FileFormat
FILE_FORMAT_AVI: FileFormat
FILE_FORMAT_MOV: FileFormat
FILE_FORMAT_WMV: FileFormat
FILE_FORMAT_FLV: FileFormat
FILE_FORMAT_MKV: FileFormat
FILE_FORMAT_PDF: FileFormat
FILE_FORMAT_DOCX: FileFormat
FILE_FORMAT_PPTX: FileFormat
FILE_FORMAT_ODT: FileFormat
FILE_FORMAT_XLSX: FileFormat
FILE_FORMAT_ODS: FileFormat
FILE_FORMAT_EPUB: FileFormat
FILE_FORMAT_MOBI: FileFormat
FILE_FORMAT_CHM: FileFormat
FILE_FORMAT_DOC: FileFormat
FILE_FORMAT_XLS: FileFormat
FILE_FORMAT_PPT: FileFormat
FILE_FORMAT_HTML: FileFormat
FILE_FORMAT_JSON: FileFormat
FILE_FORMAT_YAML: FileFormat
FILE_FORMAT_CSV: FileFormat
FILE_FORMAT_XML: FileFormat
FILE_FORMAT_TOML: FileFormat
FILE_FORMAT_SQLITE: FileFormat
FILE_FORMAT_PARQUET: FileFormat
FILE_FORMAT_ZIP: FileFormat
FILE_FORMAT_RAR: FileFormat
FILE_FORMAT_TAR: FileFormat
FILE_FORMAT_SEVENZIP: FileFormat
FILE_FORMAT_CAB: FileFormat
FILE_FORMAT_GZIP: FileFormat
FILE_FORMAT_BZIP2: FileFormat
FILE_FORMAT_XZ: FileFormat
FILE_FORMAT_EXE: FileFormat
FILE_FORMAT_APP_IMAGE: FileFormat
FILE_FORMAT_APK: FileFormat
FILE_FORMAT_DMG: FileFormat
FILE_FORMAT_JAR: FileFormat
FILE_FORMAT_MSI: FileFormat
FILE_FORMAT_DEB: FileFormat
FILE_FORMAT_RPM: FileFormat
FILE_KIND_UNSPECIFIED: FileKind
FILE_KIND_DRIVE: FileKind
FILE_KIND_DRIVE_INLINE: FileKind
FILE_KIND_INLINE: FileKind
FILE_KIND_EXTERNAL: FileKind
FILE_RETENTION_MODE_UNSPECIFIED: FileRetentionMode
FILE_RETENTION_MODE_AUTOMATIC: FileRetentionMode
FILE_RETENTION_MODE_MANUAL: FileRetentionMode
FILE_RETENTION_MODE_TIMED: FileRetentionMode
FILE_TYPE_UNSPECIFIED: FileType
FILE_TYPE_TEXT: FileType
FILE_TYPE_CODE: FileType
FILE_TYPE_IMAGE: FileType
FILE_TYPE_AUDIO: FileType
FILE_TYPE_VIDEO: FileType
FILE_TYPE_DOCUMENT: FileType
FILE_TYPE_DATA: FileType
FILE_TYPE_ARCHIVE: FileType
FILE_TYPE_EXECUTABLE: FileType
FILE_TYPE_GENERIC: FileType
FONT_SIZE_UNSPECIFIED: FontSize
FONT_SIZE_XS: FontSize
FONT_SIZE_SM: FontSize
FONT_SIZE_BASE: FontSize
FONT_SIZE_LG: FontSize
FONT_SIZE_XL: FontSize
FONT_SIZE_XL2: FontSize
FONT_SIZE_XL3: FontSize
FONT_SIZE_XL4: FontSize
FONT_SIZE_XL5: FontSize
FONT_SIZE_XL6: FontSize
FONT_SIZE_XL7: FontSize
FONT_TYPE_UNSPECIFIED: FontType
FONT_TYPE_SERIF: FontType
FONT_TYPE_SANS: FontType
FONT_TYPE_MONO: FontType
FONT_WEIGHT_UNSPECIFIED: FontWeight
FONT_WEIGHT_THIN: FontWeight
FONT_WEIGHT_EXTRA_LIGHT: FontWeight
FONT_WEIGHT_LIGHT: FontWeight
FONT_WEIGHT_NORMAL: FontWeight
FONT_WEIGHT_MEDIUM: FontWeight
FONT_WEIGHT_SEMI_BOLD: FontWeight
FONT_WEIGHT_BOLD: FontWeight
FONT_WEIGHT_EXTRA_BOLD: FontWeight
FONT_WEIGHT_BLACK: FontWeight
FUNCTIONAL_OP_UNSPECIFIED: FunctionalOp
FUNCTIONAL_OP_ADD: FunctionalOp
FUNCTIONAL_OP_SUBTRACT: FunctionalOp
FUNCTIONAL_OP_MULTIPLY: FunctionalOp
FUNCTIONAL_OP_DIVIDE: FunctionalOp
FUNCTIONAL_OP_MODULO: FunctionalOp
FUNCTIONAL_OP_POWER: FunctionalOp
ICON_KIND_UNSPECIFIED: IconKind
ICON_KIND_EMOJI: IconKind
ICON_KIND_FONT_AWESOME: IconKind
ICON_KIND_VS_CODE: IconKind
ID_ENUM_UNSPECIFIED: IdEnum
LITERAL_OP_UNSPECIFIED: LiteralOp
LITERAL_OP_VALUE: LiteralOp
LITERAL_OP_NONE: LiteralOp
LITERAL_OP_TRUE: LiteralOp
LITERAL_OP_FALSE: LiteralOp
LOG_KIND_UNSPECIFIED: LogKind
LOG_KIND_CHANGE: LogKind
LOG_KIND_EDIT: LogKind
LOG_LEVEL_UNSPECIFIED: LogLevel
LOG_LEVEL_TRACE: LogLevel
LOG_LEVEL_DEBUG: LogLevel
LOG_LEVEL_INFO: LogLevel
LOG_LEVEL_WARNING: LogLevel
LOG_LEVEL_ERROR: LogLevel
LOG_LEVEL_CRITICAL: LogLevel
MODEL_PROVIDER_UNSPECIFIED: ModelProvider
MODEL_PROVIDER_OPENAI: ModelProvider
MODEL_PROVIDER_ANTHROPIC: ModelProvider
MODEL_PROVIDER_GOOGLE: ModelProvider
MODEL_PROVIDER_META: ModelProvider
MODEL_TYPE_UNSPECIFIED: ModelType
MODEL_TYPE_OPENAI_GPT4_0: ModelType
MODEL_TYPE_OPENAI_GPT4_O_MINI: ModelType
MODEL_TYPE_OPENAI_O1_PREVIEW: ModelType
MODEL_TYPE_OPENAI_O1_MINI: ModelType
MODEL_TYPE_ANTHROPIC_CLAUDE_3_5_SONNET: ModelType
MODEL_TYPE_GOOGLE_GEMINI_1_5_PRO: ModelType
MODEL_TYPE_META_LLAMA_3_1_80B: ModelType
MODEL_TYPE_META_LLAMA_3_1_400B: ModelType
MONTH_UNSPECIFIED: Month
MONTH_JANUARY: Month
MONTH_FEBRUARY: Month
MONTH_MARCH: Month
MONTH_APRIL: Month
MONTH_MAY: Month
MONTH_JUNE: Month
MONTH_JULY: Month
MONTH_AUGUST: Month
MONTH_SEPTEMBER: Month
MONTH_OCTOBER: Month
MONTH_NOVEMBER: Month
MONTH_DECEMBER: Month
NODE_TYPE_UNSPECIFIED: NodeType
NODE_TYPE_BENCH: NodeType
NODE_TYPE_USER: NodeType
NODE_TYPE_ORGANIZATION: NodeType
NODE_TYPE_HANDLE: NodeType
NODE_TYPE_CLIENT: NodeType
NODE_TYPE_SERVER: NodeType
NODE_TYPE_STORE: NodeType
NODE_TYPE_MACHINE: NodeType
NODE_TYPE_DRIVE: NodeType
NODE_TYPE_VAULT: NodeType
NODE_TYPE_CACHE: NodeType
NODE_TYPE_FILE: NodeType
NODE_TYPE_SECRET: NodeType
NODE_TYPE_MEMBERSHIP: NodeType
NODE_TYPE_INVITE: NodeType
NODE_TYPE_BRANCH: NodeType
NODE_TYPE_PACKAGE: NodeType
NODE_TYPE_DEPENDENCY: NodeType
NODE_TYPE_SPACE: NodeType
NODE_TYPE_BLOCK: NodeType
NODE_TYPE_TRIGGER: NodeType
NODE_TYPE_FIELD: NodeType
NODE_TYPE_QUERY: NodeType
NODE_TYPE_VIEW: NodeType
NODE_TYPE_STEP: NodeType
NODE_TYPE_PIPE: NodeType
NODE_TYPE_BADGE: NodeType
NODE_TYPE_MESSAGE: NodeType
NODE_TYPE_RECORD: NodeType
NODE_TYPE_SESSION: NodeType
NODE_TYPE_RUN: NodeType
NODE_TYPE_SIGNAL: NodeType
NODE_TYPE_LOG: NodeType
NODE_TYPE_NOTIFICATION: NodeType
NODE_TYPE_SKIP: NodeType
NOTIFICATION_LEVEL_UNSPECIFIED: NotificationLevel
NOTIFICATION_LEVEL_PASSIVE: NotificationLevel
NOTIFICATION_LEVEL_ACTIVE: NotificationLevel
NOTIFICATION_LEVEL_URGENT: NotificationLevel
OBJECT_TYPE_UNSPECIFIED: ObjectType
OBJECT_TYPE_BENCH: ObjectType
OBJECT_TYPE_USER: ObjectType
OBJECT_TYPE_ORGANIZATION: ObjectType
OBJECT_TYPE_HANDLE: ObjectType
OBJECT_TYPE_CLIENT: ObjectType
OBJECT_TYPE_SERVER: ObjectType
OBJECT_TYPE_STORE: ObjectType
OBJECT_TYPE_MACHINE: ObjectType
OBJECT_TYPE_DRIVE: ObjectType
OBJECT_TYPE_VAULT: ObjectType
OBJECT_TYPE_CACHE: ObjectType
OBJECT_TYPE_FILE: ObjectType
OBJECT_TYPE_SECRET: ObjectType
OBJECT_TYPE_MEMBERSHIP: ObjectType
OBJECT_TYPE_INVITE: ObjectType
OBJECT_TYPE_BRANCH: ObjectType
OBJECT_TYPE_PACKAGE: ObjectType
OBJECT_TYPE_DEPENDENCY: ObjectType
OBJECT_TYPE_SPACE: ObjectType
OBJECT_TYPE_BLOCK: ObjectType
OBJECT_TYPE_TRIGGER: ObjectType
OBJECT_TYPE_FIELD: ObjectType
OBJECT_TYPE_QUERY: ObjectType
OBJECT_TYPE_VIEW: ObjectType
OBJECT_TYPE_STEP: ObjectType
OBJECT_TYPE_PIPE: ObjectType
OBJECT_TYPE_BADGE: ObjectType
OBJECT_TYPE_MESSAGE: ObjectType
OBJECT_TYPE_RECORD: ObjectType
OBJECT_TYPE_SESSION: ObjectType
OBJECT_TYPE_RUN: ObjectType
OBJECT_TYPE_SIGNAL: ObjectType
OBJECT_TYPE_LOG: ObjectType
OBJECT_TYPE_NOTIFICATION: ObjectType
OBJECT_TYPE_SKIP: ObjectType
OBJECT_TYPE_SESSION_CONTEXT: ObjectType
OBJECT_TYPE_EDIT_CONTEXT: ObjectType
OBJECT_TYPE_EDIT: ObjectType
OBJECT_TYPE_EDIT_INFO: ObjectType
OBJECT_TYPE_EDIT_OPERATION: ObjectType
OBJECT_TYPE_CHANGE: ObjectType
OBJECT_TYPE_CHANGE_VIGNETTE: ObjectType
OBJECT_TYPE_GRAPH_SCOPE: ObjectType
OBJECT_TYPE_CLIENT_ORIGIN: ObjectType
OBJECT_TYPE_NODE_REFERENCE: ObjectType
OBJECT_TYPE_PROPERTY_REFERENCE: ObjectType
OBJECT_TYPE_PATH: ObjectType
OBJECT_TYPE_PATH_TOKEN: ObjectType
OBJECT_TYPE_POLICY: ObjectType
OBJECT_TYPE_POLICY_RULE: ObjectType
OBJECT_TYPE_SUBJECT: ObjectType
OBJECT_TYPE_ACCESS_ZONE: ObjectType
OBJECT_TYPE_ACCESS_MATRIX: ObjectType
OBJECT_TYPE_ACCESS: ObjectType
OBJECT_TYPE_TEXT: ObjectType
OBJECT_TYPE_TEXT_LINE: ObjectType
OBJECT_TYPE_TEXT_SPAN: ObjectType
OBJECT_TYPE_TYPE_INFO: ObjectType
OBJECT_TYPE_TYPE_CONSTRAINT: ObjectType
OBJECT_TYPE_SCHEDULE: ObjectType
OBJECT_TYPE_FILE_INFO: ObjectType
OBJECT_TYPE_FILE_REFERENCE: ObjectType
OBJECT_TYPE_ICON: ObjectType
OBJECT_TYPE_SECRET_REFERENCE: ObjectType
OBJECT_TYPE_TRIGGER_INFO: ObjectType
OBJECT_TYPE_EXPRESSION: ObjectType
OBJECT_TYPE_AGGREGATION: ObjectType
OBJECT_TYPE_SELECTION: ObjectType
OBJECT_TYPE_QUERY_INFO: ObjectType
OBJECT_TYPE_READ_OPTIONS: ObjectType
OBJECT_TYPE_VALUE: ObjectType
OBJECT_TYPE_COMPUTED_VALUE: ObjectType
OBJECT_TYPE_CODE: ObjectType
OBJECT_TYPE_CODE_LINE: ObjectType
OBJECT_TYPE_PORT_KEY: ObjectType
OBJECT_TYPE_PORT: ObjectType
OBJECT_TYPE_RUN_ERROR: ObjectType
OBJECT_TYPE_RUN_OPTIONS: ObjectType
OBJECT_TYPE_RUN_ATTEMPT: ObjectType
OBJECT_TYPE_RUN_TRACE: ObjectType
OBJECT_TYPE_RUN_FRAME: ObjectType
OBJECT_TYPE_RUN_SPAN: ObjectType
OBJECT_TYPE_RUN_EVENT: ObjectType
OBJECT_TYPE_BREAKPOINT: ObjectType
OBJECT_TYPE_LOG_INFO: ObjectType
OBJECT_TYPE_COLOR: ObjectType
OBJECT_TYPE_FONT: ObjectType
OBJECT_TYPE_BOX: ObjectType
OBJECT_TYPE_OFFSET: ObjectType
OBJECT_TYPE_TRANSFORM: ObjectType
OBJECT_TYPE_VECTOR2: ObjectType
OBJECT_TYPE_VECTOR3: ObjectType
OBJECT_TYPE_VECTOR4: ObjectType
OBJECT_TYPE_LINE: ObjectType
OBJECT_TYPE_START_VIEW_STATE: ObjectType
OBJECT_TYPE_FEED_VIEW_STATE: ObjectType
OBJECT_TYPE_USER_WIZARD_VIEW_STATE: ObjectType
OBJECT_TYPE_TREE_VIEW_STATE: ObjectType
ORGANIZATION_STATUS_UNSPECIFIED: OrganizationStatus
ORGANIZATION_STATUS_REGISTERED: OrganizationStatus
ORGANIZATION_STATUS_ACTIVATED: OrganizationStatus
ORIENTATION_UNSPECIFIED: Orientation
ORIENTATION_HORIZONTAL: Orientation
ORIENTATION_HORIZONTAL_REVERSED: Orientation
ORIENTATION_VERTICAL: Orientation
ORIENTATION_VERTICAL_REVERSED: Orientation
PATH_TOKEN_TYPE_UNSPECIFIED: PathTokenType
PATH_TOKEN_TYPE_ROOT: PathTokenType
PATH_TOKEN_TYPE_CURRENT: PathTokenType
PATH_TOKEN_TYPE_PARENT: PathTokenType
PATH_TOKEN_TYPE_BENCH: PathTokenType
PATH_TOKEN_TYPE_CHILD: PathTokenType
PATH_TOKEN_TYPE_CONTAINER: PathTokenType
PATH_TOKEN_TYPE_UNIQUE: PathTokenType
PATH_TOKEN_TYPE_FIELD: PathTokenType
PIPE_FILTER_TYPE_UNSPECIFIED: PipeFilterType
PIPE_FILTER_TYPE_IS_NON_EMPTY: PipeFilterType
PIPE_FILTER_TYPE_IS_TRUTHY: PipeFilterType
PIPE_FILTER_TYPE_IS_EMPTY: PipeFilterType
PIPE_FILTER_TYPE_IS_FALSY: PipeFilterType
PIPE_FILTER_TYPE_HAS_ERROR: PipeFilterType
PIPE_TYPE_UNSPECIFIED: PipeType
PIPE_TYPE_CONTROL_AND_DATA: PipeType
PIPE_TYPE_DATA: PipeType
POLICY_EFFECT_UNSPECIFIED: PolicyEffect
POLICY_EFFECT_ALLOW: PolicyEffect
POLICY_EFFECT_DENY: PolicyEffect
PORT_SIDE_UNSPECIFIED: PortSide
PORT_SIDE_INCOMING: PortSide
PORT_SIDE_OUTGOING: PortSide
PORT_TYPE_UNSPECIFIED: PortType
PORT_TYPE_RUN: PortType
PORT_TYPE_OBJECT: PortType
PORT_TYPE_FIELD: PortType
PRIMITIVE_TYPE_UNSPECIFIED: PrimitiveType
PRIMITIVE_TYPE_BOOLEAN: PrimitiveType
PRIMITIVE_TYPE_INT16: PrimitiveType
PRIMITIVE_TYPE_INT32: PrimitiveType
PRIMITIVE_TYPE_INT64: PrimitiveType
PRIMITIVE_TYPE_DECIMAL: PrimitiveType
PRIMITIVE_TYPE_FLOAT32: PrimitiveType
PRIMITIVE_TYPE_FLOAT64: PrimitiveType
PRIMITIVE_TYPE_STRING: PrimitiveType
PRIMITIVE_TYPE_UUID: PrimitiveType
PRIMITIVE_TYPE_JSON: PrimitiveType
PRIMITIVE_TYPE_BYTES: PrimitiveType
PRIMITIVE_TYPE_VECTOR: PrimitiveType
PRIMITIVE_TYPE_DATETIME: PrimitiveType
PRIMITIVE_TYPE_DATE: PrimitiveType
PRIMITIVE_TYPE_TIME: PrimitiveType
PRIMITIVE_TYPE_INTERVAL: PrimitiveType
READ_TYPE_UNSPECIFIED: ReadType
READ_TYPE_GET: ReadType
READ_TYPE_SEARCH: ReadType
READ_TYPE_AGGREGATE: ReadType
REFERENCE_KIND_UNSPECIFIED: ReferenceKind
REFERENCE_KIND_NODE_ANCESTOR: ReferenceKind
REFERENCE_KIND_NODE_ANCESTOR_OR_SELF: ReferenceKind
REFERENCE_KIND_NODE_PARENT: ReferenceKind
REFERENCE_KIND_NODE_CHILDREN: ReferenceKind
REFERENCE_KIND_NODE_REGULAR: ReferenceKind
REFERENCE_KIND_NODE_TEMPLATE: ReferenceKind
REFERENCE_KIND_STRUCT_PARENT: ReferenceKind
REFERENCE_KIND_STRUCT_CHILD: ReferenceKind
REFERENCE_KIND_PROPERTY: ReferenceKind
REGION_UNSPECIFIED: Region
REGION_ZURICH: Region
REGION_FRANKFURT: Region
REGION_VIRGINIA: Region
REGION_OHIO: Region
REGION_OREGON: Region
REGION_SAO_PAULO: Region
REGION_CAPE_TOWN: Region
REGION_MUMBAI: Region
REGION_SINGAPORE: Region
REGION_TOKYO: Region
REGION_SYDNEY: Region
REGION_AREA_UNSPECIFIED: RegionArea
REGION_AREA_EUROPE: RegionArea
REGION_AREA_NORTH_AMERICA: RegionArea
REGION_AREA_SOUTH_AMERICA: RegionArea
REGION_AREA_MIDDLE_EAST: RegionArea
REGION_AREA_AFRICA: RegionArea
REGION_AREA_ASIA: RegionArea
REGION_AREA_AUSTRALIA: RegionArea
REGION_AREA_PRIVATE: RegionArea
REGION_AREA_GLOBAL: RegionArea
REGION_ZONE_UNSPECIFIED: RegionZone
REGION_ZONE_EUROPE_CENTRAL: RegionZone
REGION_ZONE_NORTH_AMERICA_EAST: RegionZone
REGION_ZONE_NORTH_AMERICA_WEST: RegionZone
REGION_ZONE_SOUTH_AMERICA_EAST: RegionZone
REGION_ZONE_MIDDLE_EAST_CENTRAL: RegionZone
REGION_ZONE_MIDDLE_EAST_WEST: RegionZone
REGION_ZONE_AFRICA_SOUTH: RegionZone
REGION_ZONE_ASIA_WEST: RegionZone
REGION_ZONE_ASIA_SOUTH: RegionZone
REGION_ZONE_ASIA_EAST: RegionZone
REGION_ZONE_AUSTRALIA_SOUTH: RegionZone
RESOURCE_STATUS_UNSPECIFIED: ResourceStatus
RESOURCE_STATUS_DECLARED: ResourceStatus
RESOURCE_STATUS_UP: ResourceStatus
RESOURCE_STATUS_SLEEPING: ResourceStatus
RESOURCE_STATUS_DOWN: ResourceStatus
RESOURCE_STATUS_DEGRADED: ResourceStatus
RESOURCE_STATUS_GONE: ResourceStatus
RUN_ERROR_KIND_UNSPECIFIED: RunErrorKind
RUN_ERROR_KIND_INTERNAL: RunErrorKind
RUN_ERROR_KIND_RUNTIME: RunErrorKind
RUN_ERROR_TYPE_UNSPECIFIED: RunErrorType
RUN_ERROR_TYPE_REPLAY: RunErrorType
RUN_ERROR_TYPE_ABORTED: RunErrorType
RUN_ERROR_TYPE_RUNTIME_UNAVAILABLE: RunErrorType
RUN_ERROR_TYPE_RUN_IMPOSSIBLE: RunErrorType
RUN_ERROR_TYPE_INVALID_VALUE: RunErrorType
RUN_ERROR_TYPE_CODE_INVALID: RunErrorType
RUN_ERROR_TYPE_TEXT_INVALID: RunErrorType
RUN_ERROR_TYPE_MODEL_INCAPABLE: RunErrorType
RUN_ERROR_TYPE_MANUAL_NONRETRYABLE: RunErrorType
RUN_ERROR_TYPE_UNKNOWN_NONRETRYABLE: RunErrorType
RUN_ERROR_TYPE_MODEL_FAILED: RunErrorType
RUN_ERROR_TYPE_MANUAL_RETRYABLE: RunErrorType
RUN_ERROR_TYPE_UNKNOWN_RETRYABLE: RunErrorType
RUN_EVENT_TYPE_UNSPECIFIED: RunEventType
RUN_EVENT_TYPE_PAUSED: RunEventType
RUN_EVENT_TYPE_RESUMED: RunEventType
RUN_EVENT_TYPE_HALTED: RunEventType
RUN_EVENT_TYPE_CUSTOM: RunEventType
RUN_KIND_UNSPECIFIED: RunKind
RUN_KIND_CODE: RunKind
RUN_KIND_TEXT: RunKind
RUN_KIND_STEP: RunKind
RUN_KIND_FLOW: RunKind
RUN_SPAN_TYPE_UNSPECIFIED: RunSpanType
RUN_SPAN_TYPE_CUSTOM: RunSpanType
RUN_STATUS_UNSPECIFIED: RunStatus
RUN_STATUS_SCHEDULED: RunStatus
RUN_STATUS_QUEUED: RunStatus
RUN_STATUS_RUNNING: RunStatus
RUN_STATUS_PAUSED: RunStatus
RUN_STATUS_SUSPENDED: RunStatus
RUN_STATUS_CANCELLED: RunStatus
RUN_STATUS_ABORTED: RunStatus
RUN_STATUS_FAILED: RunStatus
RUN_STATUS_COMPLETED: RunStatus
SCHEDULE_TYPE_UNSPECIFIED: ScheduleType
SCHEDULE_TYPE_INTERVAL: ScheduleType
SCHEDULE_TYPE_CRON: ScheduleType
SELECTION_KIND_UNSPECIFIED: SelectionKind
SELECTION_KIND_RANGE: SelectionKind
SELECTION_KIND_LIST: SelectionKind
SELECTION_TARGET_UNSPECIFIED: SelectionTarget
SELECTION_TARGET_NODE: SelectionTarget
SELECTION_TARGET_VALUE: SelectionTarget
SESSION_STATUS_UNSPECIFIED: SessionStatus
SESSION_STATUS_PENDING: SessionStatus
SESSION_STATUS_OPEN: SessionStatus
SESSION_STATUS_CLOSED: SessionStatus
SORT_MODE_UNSPECIFIED: SortMode
SORT_MODE_MAX: SortMode
SORT_MODE_MIN: SortMode
SORT_MODE_AVERAGE: SortMode
SORT_MODE_SUM: SortMode
SORT_MODE_MEDIAN: SortMode
SORT_OP_UNSPECIFIED: SortOp
SORT_OP_ASCENDING: SortOp
SORT_OP_DESCENDING: SortOp
SPACE_TYPE_UNSPECIFIED: SpaceType
SPACE_TYPE_DESKTOP: SpaceType
SPACE_TYPE_BROWSER: SpaceType
SPACE_TYPE_MOBILE: SpaceType
SPACING_UNSPECIFIED: Spacing
SPACING_S1: Spacing
SPACING_S2: Spacing
SPACING_S3: Spacing
SPACING_S4: Spacing
SPACING_S6: Spacing
SPACING_S8: Spacing
SPACING_S10: Spacing
SPACING_S12: Spacing
SPACING_S14: Spacing
SPACING_S16: Spacing
SPACING_S20: Spacing
SPACING_S24: Spacing
SPACING_S28: Spacing
SPACING_S32: Spacing
SPACING_S36: Spacing
SPACING_S40: Spacing
SPACING_S44: Spacing
SPACING_S48: Spacing
SPACING_S52: Spacing
SPACING_S56: Spacing
SPACING_S60: Spacing
SPACING_S64: Spacing
SPACING_S72: Spacing
SPACING_S80: Spacing
SPACING_S96: Spacing
SPACING_S128: Spacing
SPACING_S160: Spacing
SPACING_S192: Spacing
SPACING_S224: Spacing
SPACING_S256: Spacing
STEP_TYPE_UNSPECIFIED: StepType
STEP_TYPE_START: StepType
STEP_TYPE_COMPLETE: StepType
STEP_TYPE_FAIL: StepType
STEP_TYPE_TRIGGER: StepType
STEP_TYPE_BLOCK: StepType
STEP_TYPE_TEXT: StepType
STEP_TYPE_CODE: StepType
STEP_TYPE_SEND: StepType
STEP_TYPE_VALUE: StepType
STEP_TYPE_GROUP: StepType
STEP_TYPE_LOOP: StepType
STEP_TYPE_REPEAT: StepType
STRUCT_TYPE_UNSPECIFIED: StructType
STRUCT_TYPE_SESSION_CONTEXT: StructType
STRUCT_TYPE_EDIT_CONTEXT: StructType
STRUCT_TYPE_EDIT: StructType
STRUCT_TYPE_EDIT_INFO: StructType
STRUCT_TYPE_EDIT_OPERATION: StructType
STRUCT_TYPE_CHANGE: StructType
STRUCT_TYPE_CHANGE_VIGNETTE: StructType
STRUCT_TYPE_GRAPH_SCOPE: StructType
STRUCT_TYPE_CLIENT_ORIGIN: StructType
STRUCT_TYPE_NODE_REFERENCE: StructType
STRUCT_TYPE_PROPERTY_REFERENCE: StructType
STRUCT_TYPE_PATH: StructType
STRUCT_TYPE_PATH_TOKEN: StructType
STRUCT_TYPE_POLICY: StructType
STRUCT_TYPE_POLICY_RULE: StructType
STRUCT_TYPE_SUBJECT: StructType
STRUCT_TYPE_ACCESS_ZONE: StructType
STRUCT_TYPE_ACCESS_MATRIX: StructType
STRUCT_TYPE_ACCESS: StructType
STRUCT_TYPE_TEXT: StructType
STRUCT_TYPE_TEXT_LINE: StructType
STRUCT_TYPE_TEXT_SPAN: StructType
STRUCT_TYPE_TYPE_INFO: StructType
STRUCT_TYPE_TYPE_CONSTRAINT: StructType
STRUCT_TYPE_SCHEDULE: StructType
STRUCT_TYPE_FILE_INFO: StructType
STRUCT_TYPE_FILE_REFERENCE: StructType
STRUCT_TYPE_ICON: StructType
STRUCT_TYPE_SECRET_REFERENCE: StructType
STRUCT_TYPE_TRIGGER_INFO: StructType
STRUCT_TYPE_EXPRESSION: StructType
STRUCT_TYPE_AGGREGATION: StructType
STRUCT_TYPE_SELECTION: StructType
STRUCT_TYPE_QUERY_INFO: StructType
STRUCT_TYPE_READ_OPTIONS: StructType
STRUCT_TYPE_VALUE: StructType
STRUCT_TYPE_COMPUTED_VALUE: StructType
STRUCT_TYPE_CODE: StructType
STRUCT_TYPE_CODE_LINE: StructType
STRUCT_TYPE_PORT_KEY: StructType
STRUCT_TYPE_PORT: StructType
STRUCT_TYPE_RUN_ERROR: StructType
STRUCT_TYPE_RUN_OPTIONS: StructType
STRUCT_TYPE_RUN_ATTEMPT: StructType
STRUCT_TYPE_RUN_TRACE: StructType
STRUCT_TYPE_RUN_FRAME: StructType
STRUCT_TYPE_RUN_SPAN: StructType
STRUCT_TYPE_RUN_EVENT: StructType
STRUCT_TYPE_BREAKPOINT: StructType
STRUCT_TYPE_LOG_INFO: StructType
STRUCT_TYPE_COLOR: StructType
STRUCT_TYPE_FONT: StructType
STRUCT_TYPE_BOX: StructType
STRUCT_TYPE_OFFSET: StructType
STRUCT_TYPE_TRANSFORM: StructType
STRUCT_TYPE_VECTOR2: StructType
STRUCT_TYPE_VECTOR3: StructType
STRUCT_TYPE_VECTOR4: StructType
STRUCT_TYPE_LINE: StructType
STRUCT_TYPE_START_VIEW_STATE: StructType
STRUCT_TYPE_FEED_VIEW_STATE: StructType
STRUCT_TYPE_USER_WIZARD_VIEW_STATE: StructType
STRUCT_TYPE_TREE_VIEW_STATE: StructType
TEXT_LINE_TYPE_UNSPECIFIED: TextLineType
TEXT_LINE_TYPE_PLAIN: TextLineType
TEXT_LINE_TYPE_HEADING_SMALL: TextLineType
TEXT_LINE_TYPE_HEADING_MEDIUM: TextLineType
TEXT_LINE_TYPE_HEADING_LARGE: TextLineType
TEXT_LINE_TYPE_CALLOUT: TextLineType
TEXT_LINE_TYPE_QUOTE: TextLineType
TEXT_LINE_TYPE_LIST_BULLET: TextLineType
TEXT_LINE_TYPE_LIST_NUMBERED: TextLineType
TEXT_LINE_TYPE_LIST_UNCHECKED: TextLineType
TEXT_LINE_TYPE_LIST_CHECKED: TextLineType
TEXT_LINE_TYPE_DIVIDER: TextLineType
TIME_INTERVAL_UNSPECIFIED: TimeInterval
TIME_INTERVAL_SECOND: TimeInterval
TIME_INTERVAL_MINUTE: TimeInterval
TIME_INTERVAL_HOUR: TimeInterval
TIME_INTERVAL_DAY: TimeInterval
TIME_INTERVAL_WEEK: TimeInterval
TIME_INTERVAL_MONTH: TimeInterval
TIME_INTERVAL_YEAR: TimeInterval
TREE_VIEW_PRESET_UNSPECIFIED: TreeViewPreset
TREE_VIEW_PRESET_EXPLORE: TreeViewPreset
TREE_VIEW_PRESET_OUTLINE: TreeViewPreset
TRIGGER_TYPE_UNSPECIFIED: TriggerType
TRIGGER_TYPE_SCHEDULE: TriggerType
TRIGGER_TYPE_SIGNAL: TriggerType
TYPE_FORMAT_UNSPECIFIED: TypeFormat
TYPE_FORMAT_URL: TypeFormat
TYPE_FORMAT_EMAIL: TypeFormat
TYPE_FORMAT_EMOJI: TypeFormat
TYPE_FORMAT_PHONE_NUMBER: TypeFormat
TYPE_FORMAT_SLUG: TypeFormat
TYPE_KIND_UNSPECIFIED: TypeKind
TYPE_KIND_PRIMITIVE: TypeKind
TYPE_KIND_STRUCT: TypeKind
TYPE_KIND_NODE: TypeKind
TYPE_KIND_ENUM: TypeKind
TYPE_KIND_BASED_NODE: TypeKind
TYPE_KIND_OBJECT: TypeKind
TYPE_KIND_LITERAL: TypeKind
TYPE_KIND_UNION: TypeKind
TYPE_KIND_ALIAS: TypeKind
USE_TYPE_UNSPECIFIED: UseType
USE_TYPE_START: UseType
USE_TYPE_PAUSE: UseType
USE_TYPE_RESUME: UseType
USE_TYPE_STOP: UseType
USE_TYPE_KILL: UseType
USE_TYPE_SEND: UseType
USE_TYPE_RECEIVE: UseType
USER_STATUS_UNSPECIFIED: UserStatus
USER_STATUS_INVITED: UserStatus
USER_STATUS_RESERVED: UserStatus
USER_STATUS_WAITLISTED: UserStatus
USER_STATUS_REGISTERED: UserStatus
USER_STATUS_ACTIVATED: UserStatus
USER_WIZARD_VIEW_STAGE_UNSPECIFIED: UserWizardViewStage
USER_WIZARD_VIEW_STAGE_SIGN_UP: UserWizardViewStage
USER_WIZARD_VIEW_STAGE_LOG_IN: UserWizardViewStage
VARIANT_UNSPECIFIED: Variant
VARIANT_PRIMARY: Variant
VARIANT_SECONDARY: Variant
VARIANT_COMPACT: Variant
VARIANT_STEALTH: Variant
VIEW_TYPE_UNSPECIFIED: ViewType
VIEW_TYPE_USER_WIZARD: ViewType
VIEW_TYPE_BENCH_WIZARD: ViewType
VIEW_TYPE_EMPTY: ViewType
VIEW_TYPE_PAGE: ViewType
VIEW_TYPE_BLOCK: ViewType
VIEW_TYPE_FIELD: ViewType
VIEW_TYPE_DATABASE: ViewType
VIEW_TYPE_VIEW: ViewType
VIEW_TYPE_FLOW: ViewType
VIEW_TYPE_STEP: ViewType
VIEW_TYPE_TYPE: ViewType
VIEW_TYPE_VARIABLE: ViewType
VIEW_TYPE_OBJECT: ViewType
VIEW_TYPE_RUN: ViewType
VIEW_TYPE_LOG: ViewType
VIEW_TYPE_PIPE: ViewType
VIEW_TYPE_TREE: ViewType
VIEW_TYPE_INSPECT: ViewType
VIEW_TYPE_CREATE: ViewType
VIEW_TYPE_CHAT: ViewType
VIEW_TYPE_START: ViewType
VIEW_TYPE_FEED: ViewType
VIEW_TYPE_TIMELINE: ViewType
VIEW_TYPE_HISTORY: ViewType
VIEW_TYPE_WINDOW: ViewType
VIEW_TYPE_TAB: ViewType
VIEW_TYPE_SPLIT: ViewType
VIEW_TYPE_SPLIT_DRAWER: ViewType
VIEW_TYPE_STACK: ViewType
VIEW_TYPE_DRAWER: ViewType
VIEW_TYPE_SCROLL: ViewType
VIEW_TYPE_GRID: ViewType
VIEW_TYPE_GROUP: ViewType
VIEW_TYPE_SECTION: ViewType
VIEW_TYPE_FORM: ViewType
VIEW_TYPE_SPACER: ViewType
VIEW_TYPE_DIVIDER: ViewType
VIEW_TYPE_LIST: ViewType
VIEW_TYPE_TABLE: ViewType
VIEW_TYPE_BREADCRUMB: ViewType
VIEW_TYPE_PROGRESS: ViewType
VIEW_TYPE_AVATAR: ViewType
VIEW_TYPE_BADGE: ViewType
VIEW_TYPE_SHAPE: ViewType
VIEW_TYPE_CHART: ViewType
VIEW_TYPE_BUTTON: ViewType
VIEW_TYPE_MULTI_BUTTON: ViewType
VIEW_TYPE_LINK: ViewType
VIEW_TYPE_VALUE: ViewType
VIEW_TYPE_NUMBER: ViewType
VIEW_TYPE_SLIDER: ViewType
VIEW_TYPE_STRING: ViewType
VIEW_TYPE_TEXT: ViewType
VIEW_TYPE_CODE: ViewType
VIEW_TYPE_JSON: ViewType
VIEW_TYPE_TOGGLE: ViewType
VIEW_TYPE_PICKER: ViewType
VIEW_TYPE_CALENDAR: ViewType
VIEW_TYPE_MAP: ViewType
VIEW_TYPE_COLOR: ViewType
VIEW_TYPE_ICON: ViewType
VIEW_TYPE_FILE: ViewType
VIEW_TYPE_IMAGE: ViewType
VIEW_TYPE_AUDIO: ViewType
VIEW_TYPE_VIDEO: ViewType
VIEW_TYPE_DOCUMENT: ViewType

class AccessData(_message.Message):
    __slots__ = ("metatype", "mode", "decision", "verb", "node_type", "allowed_properties_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    DECISION_FIELD_NUMBER: _ClassVar[int]
    VERB_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    ALLOWED_PROPERTIES_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    mode: AccessMode
    decision: PolicyEffect
    verb: AccessType
    node_type: NodeType
    allowed_properties_ptr: _containers.RepeatedCompositeFieldContainer[PropertyReferenceData]
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        mode: _Optional[_Union[AccessMode, str]] = ...,
        decision: _Optional[_Union[PolicyEffect, str]] = ...,
        verb: _Optional[_Union[AccessType, str]] = ...,
        node_type: _Optional[_Union[NodeType, str]] = ...,
        allowed_properties_ptr: _Optional[_Iterable[_Union[PropertyReferenceData, _Mapping]]] = ...,
    ) -> None: ...

class AccessMatrixData(_message.Message):
    __slots__ = ("metatype", "subject", "identities", "scoped_zones", "base_zones")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    SUBJECT_FIELD_NUMBER: _ClassVar[int]
    IDENTITIES_FIELD_NUMBER: _ClassVar[int]
    SCOPED_ZONES_FIELD_NUMBER: _ClassVar[int]
    BASE_ZONES_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    subject: SubjectData
    identities: _containers.RepeatedCompositeFieldContainer[SubjectData]
    scoped_zones: _containers.RepeatedCompositeFieldContainer[AccessZoneData]
    base_zones: _containers.RepeatedCompositeFieldContainer[AccessZoneData]
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        subject: _Optional[_Union[SubjectData, _Mapping]] = ...,
        identities: _Optional[_Iterable[_Union[SubjectData, _Mapping]]] = ...,
        scoped_zones: _Optional[_Iterable[_Union[AccessZoneData, _Mapping]]] = ...,
        base_zones: _Optional[_Iterable[_Union[AccessZoneData, _Mapping]]] = ...,
    ) -> None: ...

class AccessZoneData(_message.Message):
    __slots__ = ("metatype", "id", "parent_id", "scope_id", "identity_id", "rules")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_ID_FIELD_NUMBER: _ClassVar[int]
    SCOPE_ID_FIELD_NUMBER: _ClassVar[int]
    IDENTITY_ID_FIELD_NUMBER: _ClassVar[int]
    RULES_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: int
    parent_id: int
    scope_id: str
    identity_id: int
    rules: _containers.RepeatedCompositeFieldContainer[PolicyRuleData]
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[int] = ...,
        parent_id: _Optional[int] = ...,
        scope_id: _Optional[str] = ...,
        identity_id: _Optional[int] = ...,
        rules: _Optional[_Iterable[_Union[PolicyRuleData, _Mapping]]] = ...,
    ) -> None: ...

class AggregationData(_message.Message):
    __slots__ = ("metatype", "op", "exists", "count", "scalar")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    OP_FIELD_NUMBER: _ClassVar[int]
    EXISTS_FIELD_NUMBER: _ClassVar[int]
    COUNT_FIELD_NUMBER: _ClassVar[int]
    SCALAR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    op: AggregationOp
    exists: bool
    count: int
    scalar: float
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        op: _Optional[_Union[AggregationOp, str]] = ...,
        exists: bool = ...,
        count: _Optional[int] = ...,
        scalar: _Optional[float] = ...,
    ) -> None: ...

class BoxData(_message.Message):
    __slots__ = ("metatype", "width", "height", "width_relative", "height_relative")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    WIDTH_RELATIVE_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_RELATIVE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    width: int
    height: int
    width_relative: float
    height_relative: float
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        width: _Optional[int] = ...,
        height: _Optional[int] = ...,
        width_relative: _Optional[float] = ...,
        height_relative: _Optional[float] = ...,
    ) -> None: ...

class BreakpointData(_message.Message):
    __slots__ = ("metatype", "kind", "action", "condition")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    KIND_FIELD_NUMBER: _ClassVar[int]
    ACTION_FIELD_NUMBER: _ClassVar[int]
    CONDITION_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    kind: BreakpointKind
    action: BreakpointAction
    condition: ExpressionData
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        kind: _Optional[_Union[BreakpointKind, str]] = ...,
        action: _Optional[_Union[BreakpointAction, str]] = ...,
        condition: _Optional[_Union[ExpressionData, _Mapping]] = ...,
    ) -> None: ...

class ChangeData(_message.Message):
    __slots__ = ("metatype", "key", "kind", "scope_ptr", "code", "edits", "logs_ptr", "logs_query")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    KIND_FIELD_NUMBER: _ClassVar[int]
    SCOPE_PTR_FIELD_NUMBER: _ClassVar[int]
    CODE_FIELD_NUMBER: _ClassVar[int]
    EDITS_FIELD_NUMBER: _ClassVar[int]
    LOGS_PTR_FIELD_NUMBER: _ClassVar[int]
    LOGS_QUERY_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    key: str
    kind: ChangeKind
    scope_ptr: NodeReferenceData
    code: CodeData
    edits: _containers.RepeatedCompositeFieldContainer[EditData]
    logs_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    logs_query: QueryInfoData
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        key: _Optional[str] = ...,
        kind: _Optional[_Union[ChangeKind, str]] = ...,
        scope_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        code: _Optional[_Union[CodeData, _Mapping]] = ...,
        edits: _Optional[_Iterable[_Union[EditData, _Mapping]]] = ...,
        logs_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...,
        logs_query: _Optional[_Union[QueryInfoData, _Mapping]] = ...,
    ) -> None: ...

class ChangeVignetteData(_message.Message):
    __slots__ = ("metatype", "name", "title", "subtype", "subsubtype", "icon")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    SUBTYPE_FIELD_NUMBER: _ClassVar[int]
    SUBSUBTYPE_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    name: str
    title: str
    subtype: int
    subsubtype: int
    icon: IconData
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        name: _Optional[str] = ...,
        title: _Optional[str] = ...,
        subtype: _Optional[int] = ...,
        subsubtype: _Optional[int] = ...,
        icon: _Optional[_Union[IconData, _Mapping]] = ...,
    ) -> None: ...

class ClientOriginData(_message.Message):
    __slots__ = ("metatype", "type", "id", "nonce")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    NONCE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    type: ClientType
    id: str
    nonce: str
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        type: _Optional[_Union[ClientType, str]] = ...,
        id: _Optional[str] = ...,
        nonce: _Optional[str] = ...,
    ) -> None: ...

class CodeData(_message.Message):
    __slots__ = ("metatype", "lines")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    LINES_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    lines: _containers.RepeatedCompositeFieldContainer[CodeLineData]
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        lines: _Optional[_Iterable[_Union[CodeLineData, _Mapping]]] = ...,
    ) -> None: ...

class CodeLineData(_message.Message):
    __slots__ = ("metatype", "content")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    CONTENT_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    content: str
    def __init__(
        self, metatype: _Optional[_Union[ObjectType, str]] = ..., content: _Optional[str] = ...
    ) -> None: ...

class ColorData(_message.Message):
    __slots__ = ("metatype", "type", "shade", "hex")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    SHADE_FIELD_NUMBER: _ClassVar[int]
    HEX_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    type: ColorType
    shade: ColorShade
    hex: str
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        type: _Optional[_Union[ColorType, str]] = ...,
        shade: _Optional[_Union[ColorShade, str]] = ...,
        hex: _Optional[str] = ...,
    ) -> None: ...

class ComputedValueData(_message.Message):
    __slots__ = ("metatype", "expression", "code")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    EXPRESSION_FIELD_NUMBER: _ClassVar[int]
    CODE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    expression: ExpressionData
    code: CodeData
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        expression: _Optional[_Union[ExpressionData, _Mapping]] = ...,
        code: _Optional[_Union[CodeData, _Mapping]] = ...,
    ) -> None: ...

class EditData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "type",
        "node_ptr",
        "vignette",
        "properties",
        "old_node",
        "new_node",
        "scope",
        "change_key",
        "category",
        "subject_ptr",
        "origin",
        "context",
        "edited_at",
        "revision",
        "epoch",
        "undo_of_ptr",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    VIGNETTE_FIELD_NUMBER: _ClassVar[int]
    PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    OLD_NODE_FIELD_NUMBER: _ClassVar[int]
    NEW_NODE_FIELD_NUMBER: _ClassVar[int]
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    CHANGE_KEY_FIELD_NUMBER: _ClassVar[int]
    CATEGORY_FIELD_NUMBER: _ClassVar[int]
    SUBJECT_PTR_FIELD_NUMBER: _ClassVar[int]
    ORIGIN_FIELD_NUMBER: _ClassVar[int]
    CONTEXT_FIELD_NUMBER: _ClassVar[int]
    EDITED_AT_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    EPOCH_FIELD_NUMBER: _ClassVar[int]
    UNDO_OF_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    type: EditType
    node_ptr: NodeReferenceData
    vignette: ChangeVignetteData
    properties: _containers.RepeatedScalarFieldContainer[int]
    old_node: SomeNodeData
    new_node: SomeNodeData
    scope: GraphScopeData
    change_key: str
    category: ChangeCategory
    subject_ptr: NodeReferenceData
    origin: ClientOriginData
    context: EditContextData
    edited_at: _timestamp_pb2.Timestamp
    revision: int
    epoch: int
    undo_of_ptr: NodeReferenceData
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        type: _Optional[_Union[EditType, str]] = ...,
        node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        vignette: _Optional[_Union[ChangeVignetteData, _Mapping]] = ...,
        properties: _Optional[_Iterable[int]] = ...,
        old_node: _Optional[_Union[SomeNodeData, _Mapping]] = ...,
        new_node: _Optional[_Union[SomeNodeData, _Mapping]] = ...,
        scope: _Optional[_Union[GraphScopeData, _Mapping]] = ...,
        change_key: _Optional[str] = ...,
        category: _Optional[_Union[ChangeCategory, str]] = ...,
        subject_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        origin: _Optional[_Union[ClientOriginData, _Mapping]] = ...,
        context: _Optional[_Union[EditContextData, _Mapping]] = ...,
        edited_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        epoch: _Optional[int] = ...,
        undo_of_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
    ) -> None: ...

class EditContextData(_message.Message):
    __slots__ = (
        "metatype",
        "block_ptr",
        "step_ptr",
        "session_ptr",
        "run_ptr",
        "run_root_ptr",
        "identity_ptr",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    STEP_PTR_FIELD_NUMBER: _ClassVar[int]
    SESSION_PTR_FIELD_NUMBER: _ClassVar[int]
    RUN_PTR_FIELD_NUMBER: _ClassVar[int]
    RUN_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    IDENTITY_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    block_ptr: NodeReferenceData
    step_ptr: NodeReferenceData
    session_ptr: NodeReferenceData
    run_ptr: NodeReferenceData
    run_root_ptr: NodeReferenceData
    identity_ptr: NodeReferenceData
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        step_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        session_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        run_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        run_root_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        identity_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
    ) -> None: ...

class EditInfoData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "type",
        "node_ptr",
        "vignette",
        "properties",
        "old_node",
        "new_node",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    VIGNETTE_FIELD_NUMBER: _ClassVar[int]
    PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    OLD_NODE_FIELD_NUMBER: _ClassVar[int]
    NEW_NODE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    type: EditType
    node_ptr: NodeReferenceData
    vignette: ChangeVignetteData
    properties: _containers.RepeatedScalarFieldContainer[int]
    old_node: SomeNodeData
    new_node: SomeNodeData
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        type: _Optional[_Union[EditType, str]] = ...,
        node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        vignette: _Optional[_Union[ChangeVignetteData, _Mapping]] = ...,
        properties: _Optional[_Iterable[int]] = ...,
        old_node: _Optional[_Union[SomeNodeData, _Mapping]] = ...,
        new_node: _Optional[_Union[SomeNodeData, _Mapping]] = ...,
    ) -> None: ...

class EditOperationData(_message.Message):
    __slots__ = ("metatype", "type", "path", "value_packed")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    PATH_FIELD_NUMBER: _ClassVar[int]
    VALUE_PACKED_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    type: EditOperationType
    path: _containers.RepeatedScalarFieldContainer[str]
    value_packed: _struct_pb2.Struct
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        type: _Optional[_Union[EditOperationType, str]] = ...,
        path: _Optional[_Iterable[str]] = ...,
        value_packed: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
    ) -> None: ...

class ExpressionData(_message.Message):
    __slots__ = (
        "metatype",
        "op",
        "property_ptr",
        "clauses",
        "value_packed",
        "sort_mode",
        "tolerance",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    OP_FIELD_NUMBER: _ClassVar[int]
    PROPERTY_PTR_FIELD_NUMBER: _ClassVar[int]
    CLAUSES_FIELD_NUMBER: _ClassVar[int]
    VALUE_PACKED_FIELD_NUMBER: _ClassVar[int]
    SORT_MODE_FIELD_NUMBER: _ClassVar[int]
    TOLERANCE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    op: ExpressionOp
    property_ptr: PropertyReferenceData
    clauses: _containers.RepeatedCompositeFieldContainer[ExpressionData]
    value_packed: _struct_pb2.Struct
    sort_mode: SortMode
    tolerance: float
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        op: _Optional[_Union[ExpressionOp, str]] = ...,
        property_ptr: _Optional[_Union[PropertyReferenceData, _Mapping]] = ...,
        clauses: _Optional[_Iterable[_Union[ExpressionData, _Mapping]]] = ...,
        value_packed: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
        sort_mode: _Optional[_Union[SortMode, str]] = ...,
        tolerance: _Optional[float] = ...,
    ) -> None: ...

class FeedViewStateData(_message.Message):
    __slots__ = ("metatype", "node_type", "filter", "filter_pills")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    FILTER_FIELD_NUMBER: _ClassVar[int]
    FILTER_PILLS_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    node_type: NodeType
    filter: ExpressionData
    filter_pills: _containers.RepeatedScalarFieldContainer[str]
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        node_type: _Optional[_Union[NodeType, str]] = ...,
        filter: _Optional[_Union[ExpressionData, _Mapping]] = ...,
        filter_pills: _Optional[_Iterable[str]] = ...,
    ) -> None: ...

class FileInfoData(_message.Message):
    __slots__ = (
        "metatype",
        "kind",
        "title",
        "external_url",
        "inline_content",
        "coarse_type",
        "mime_type",
        "format",
        "size",
        "sha256",
        "width",
        "height",
        "aspect_ratio",
        "codec",
        "duration",
        "bitrate",
        "channels",
        "sample_rate",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    KIND_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    EXTERNAL_URL_FIELD_NUMBER: _ClassVar[int]
    INLINE_CONTENT_FIELD_NUMBER: _ClassVar[int]
    COARSE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MIME_TYPE_FIELD_NUMBER: _ClassVar[int]
    FORMAT_FIELD_NUMBER: _ClassVar[int]
    SIZE_FIELD_NUMBER: _ClassVar[int]
    SHA256_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    ASPECT_RATIO_FIELD_NUMBER: _ClassVar[int]
    CODEC_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    BITRATE_FIELD_NUMBER: _ClassVar[int]
    CHANNELS_FIELD_NUMBER: _ClassVar[int]
    SAMPLE_RATE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    kind: FileKind
    title: str
    external_url: str
    inline_content: bytes
    coarse_type: FileType
    mime_type: str
    format: FileFormat
    size: int
    sha256: str
    width: int
    height: int
    aspect_ratio: float
    codec: str
    duration: float
    bitrate: int
    channels: int
    sample_rate: int
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        kind: _Optional[_Union[FileKind, str]] = ...,
        title: _Optional[str] = ...,
        external_url: _Optional[str] = ...,
        inline_content: _Optional[bytes] = ...,
        coarse_type: _Optional[_Union[FileType, str]] = ...,
        mime_type: _Optional[str] = ...,
        format: _Optional[_Union[FileFormat, str]] = ...,
        size: _Optional[int] = ...,
        sha256: _Optional[str] = ...,
        width: _Optional[int] = ...,
        height: _Optional[int] = ...,
        aspect_ratio: _Optional[float] = ...,
        codec: _Optional[str] = ...,
        duration: _Optional[float] = ...,
        bitrate: _Optional[int] = ...,
        channels: _Optional[int] = ...,
        sample_rate: _Optional[int] = ...,
    ) -> None: ...

class FileReferenceData(_message.Message):
    __slots__ = (
        "metatype",
        "type",
        "id",
        "ck",
        "bench_id",
        "base_ck",
        "base_bench_id",
        "kind",
        "title",
        "external_url",
        "inline_content",
        "coarse_type",
        "mime_type",
        "format",
        "size",
        "sha256",
        "width",
        "height",
        "aspect_ratio",
        "codec",
        "duration",
        "bitrate",
        "channels",
        "sample_rate",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    BENCH_ID_FIELD_NUMBER: _ClassVar[int]
    BASE_CK_FIELD_NUMBER: _ClassVar[int]
    BASE_BENCH_ID_FIELD_NUMBER: _ClassVar[int]
    KIND_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    EXTERNAL_URL_FIELD_NUMBER: _ClassVar[int]
    INLINE_CONTENT_FIELD_NUMBER: _ClassVar[int]
    COARSE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MIME_TYPE_FIELD_NUMBER: _ClassVar[int]
    FORMAT_FIELD_NUMBER: _ClassVar[int]
    SIZE_FIELD_NUMBER: _ClassVar[int]
    SHA256_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    ASPECT_RATIO_FIELD_NUMBER: _ClassVar[int]
    CODEC_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    BITRATE_FIELD_NUMBER: _ClassVar[int]
    CHANNELS_FIELD_NUMBER: _ClassVar[int]
    SAMPLE_RATE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    type: NodeType
    id: str
    ck: str
    bench_id: str
    base_ck: str
    base_bench_id: str
    kind: FileKind
    title: str
    external_url: str
    inline_content: bytes
    coarse_type: FileType
    mime_type: str
    format: FileFormat
    size: int
    sha256: str
    width: int
    height: int
    aspect_ratio: float
    codec: str
    duration: float
    bitrate: int
    channels: int
    sample_rate: int
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        type: _Optional[_Union[NodeType, str]] = ...,
        id: _Optional[str] = ...,
        ck: _Optional[str] = ...,
        bench_id: _Optional[str] = ...,
        base_ck: _Optional[str] = ...,
        base_bench_id: _Optional[str] = ...,
        kind: _Optional[_Union[FileKind, str]] = ...,
        title: _Optional[str] = ...,
        external_url: _Optional[str] = ...,
        inline_content: _Optional[bytes] = ...,
        coarse_type: _Optional[_Union[FileType, str]] = ...,
        mime_type: _Optional[str] = ...,
        format: _Optional[_Union[FileFormat, str]] = ...,
        size: _Optional[int] = ...,
        sha256: _Optional[str] = ...,
        width: _Optional[int] = ...,
        height: _Optional[int] = ...,
        aspect_ratio: _Optional[float] = ...,
        codec: _Optional[str] = ...,
        duration: _Optional[float] = ...,
        bitrate: _Optional[int] = ...,
        channels: _Optional[int] = ...,
        sample_rate: _Optional[int] = ...,
    ) -> None: ...

class FontData(_message.Message):
    __slots__ = ("metatype", "type", "weight", "size")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    WEIGHT_FIELD_NUMBER: _ClassVar[int]
    SIZE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    type: FontType
    weight: FontWeight
    size: FontSize
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        type: _Optional[_Union[FontType, str]] = ...,
        weight: _Optional[_Union[FontWeight, str]] = ...,
        size: _Optional[_Union[FontSize, str]] = ...,
    ) -> None: ...

class GraphScopeData(_message.Message):
    __slots__ = ("metatype", "bench_id", "package_id")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    BENCH_ID_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_ID_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    bench_id: str
    package_id: str
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        bench_id: _Optional[str] = ...,
        package_id: _Optional[str] = ...,
    ) -> None: ...

class IconData(_message.Message):
    __slots__ = ("metatype", "kind", "emoji", "fa_name", "vsc_name", "color")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    KIND_FIELD_NUMBER: _ClassVar[int]
    EMOJI_FIELD_NUMBER: _ClassVar[int]
    FA_NAME_FIELD_NUMBER: _ClassVar[int]
    VSC_NAME_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    kind: IconKind
    emoji: str
    fa_name: str
    vsc_name: str
    color: ColorData
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        kind: _Optional[_Union[IconKind, str]] = ...,
        emoji: _Optional[str] = ...,
        fa_name: _Optional[str] = ...,
        vsc_name: _Optional[str] = ...,
        color: _Optional[_Union[ColorData, _Mapping]] = ...,
    ) -> None: ...

class LineData(_message.Message):
    __slots__ = ("metatype", "points")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    POINTS_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    points: _containers.RepeatedCompositeFieldContainer[Vector2Data]
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        points: _Optional[_Iterable[_Union[Vector2Data, _Mapping]]] = ...,
    ) -> None: ...

class LogInfoData(_message.Message):
    __slots__ = ("metatype", "created_at", "level", "text", "text_plain", "value_packed")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    LEVEL_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    TEXT_PLAIN_FIELD_NUMBER: _ClassVar[int]
    VALUE_PACKED_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    created_at: _timestamp_pb2.Timestamp
    level: LogLevel
    text: TextData
    text_plain: str
    value_packed: _struct_pb2.Struct
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        level: _Optional[_Union[LogLevel, str]] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        text_plain: _Optional[str] = ...,
        value_packed: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
    ) -> None: ...

class NodeReferenceData(_message.Message):
    __slots__ = ("metatype", "type", "id", "ck", "bench_id", "base_ck", "base_bench_id")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    BENCH_ID_FIELD_NUMBER: _ClassVar[int]
    BASE_CK_FIELD_NUMBER: _ClassVar[int]
    BASE_BENCH_ID_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    type: NodeType
    id: str
    ck: str
    bench_id: str
    base_ck: str
    base_bench_id: str
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        type: _Optional[_Union[NodeType, str]] = ...,
        id: _Optional[str] = ...,
        ck: _Optional[str] = ...,
        bench_id: _Optional[str] = ...,
        base_ck: _Optional[str] = ...,
        base_bench_id: _Optional[str] = ...,
    ) -> None: ...

class OffsetData(_message.Message):
    __slots__ = (
        "metatype",
        "top",
        "right",
        "bottom",
        "left",
        "top_relative",
        "right_relative",
        "bottom_relative",
        "left_relative",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TOP_FIELD_NUMBER: _ClassVar[int]
    RIGHT_FIELD_NUMBER: _ClassVar[int]
    BOTTOM_FIELD_NUMBER: _ClassVar[int]
    LEFT_FIELD_NUMBER: _ClassVar[int]
    TOP_RELATIVE_FIELD_NUMBER: _ClassVar[int]
    RIGHT_RELATIVE_FIELD_NUMBER: _ClassVar[int]
    BOTTOM_RELATIVE_FIELD_NUMBER: _ClassVar[int]
    LEFT_RELATIVE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    top: int
    right: int
    bottom: int
    left: int
    top_relative: float
    right_relative: float
    bottom_relative: float
    left_relative: float
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        top: _Optional[int] = ...,
        right: _Optional[int] = ...,
        bottom: _Optional[int] = ...,
        left: _Optional[int] = ...,
        top_relative: _Optional[float] = ...,
        right_relative: _Optional[float] = ...,
        bottom_relative: _Optional[float] = ...,
        left_relative: _Optional[float] = ...,
    ) -> None: ...

class PathData(_message.Message):
    __slots__ = ("metatype", "tokens")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TOKENS_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    tokens: _containers.RepeatedCompositeFieldContainer[PathTokenData]
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        tokens: _Optional[_Iterable[_Union[PathTokenData, _Mapping]]] = ...,
    ) -> None: ...

class PathTokenData(_message.Message):
    __slots__ = ("metatype", "type", "name")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    type: PathTokenType
    name: str
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        type: _Optional[_Union[PathTokenType, str]] = ...,
        name: _Optional[str] = ...,
    ) -> None: ...

class PolicyData(_message.Message):
    __slots__ = ("metatype", "name", "text", "rules", "scopes_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    RULES_FIELD_NUMBER: _ClassVar[int]
    SCOPES_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    name: str
    text: TextData
    rules: _containers.RepeatedCompositeFieldContainer[PolicyRuleData]
    scopes_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        name: _Optional[str] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        rules: _Optional[_Iterable[_Union[PolicyRuleData, _Mapping]]] = ...,
        scopes_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...,
    ) -> None: ...

class PolicyRuleData(_message.Message):
    __slots__ = (
        "metatype",
        "name",
        "text",
        "subject_is_delegated",
        "subject_is_authenticated",
        "subject_is_staff",
        "subject_is_member",
        "subject_is_owner",
        "effect",
        "verbs",
        "verb_kinds",
        "object_node_types",
        "object_properties_ptr",
        "object_properties_is_system",
        "object_properties_is_sensitive",
        "object_properties_is_kernel",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    SUBJECT_IS_DELEGATED_FIELD_NUMBER: _ClassVar[int]
    SUBJECT_IS_AUTHENTICATED_FIELD_NUMBER: _ClassVar[int]
    SUBJECT_IS_STAFF_FIELD_NUMBER: _ClassVar[int]
    SUBJECT_IS_MEMBER_FIELD_NUMBER: _ClassVar[int]
    SUBJECT_IS_OWNER_FIELD_NUMBER: _ClassVar[int]
    EFFECT_FIELD_NUMBER: _ClassVar[int]
    VERBS_FIELD_NUMBER: _ClassVar[int]
    VERB_KINDS_FIELD_NUMBER: _ClassVar[int]
    OBJECT_NODE_TYPES_FIELD_NUMBER: _ClassVar[int]
    OBJECT_PROPERTIES_PTR_FIELD_NUMBER: _ClassVar[int]
    OBJECT_PROPERTIES_IS_SYSTEM_FIELD_NUMBER: _ClassVar[int]
    OBJECT_PROPERTIES_IS_SENSITIVE_FIELD_NUMBER: _ClassVar[int]
    OBJECT_PROPERTIES_IS_KERNEL_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    name: str
    text: TextData
    subject_is_delegated: bool
    subject_is_authenticated: bool
    subject_is_staff: bool
    subject_is_member: bool
    subject_is_owner: bool
    effect: PolicyEffect
    verbs: _containers.RepeatedScalarFieldContainer[AccessType]
    verb_kinds: _containers.RepeatedScalarFieldContainer[AccessKind]
    object_node_types: _containers.RepeatedScalarFieldContainer[NodeType]
    object_properties_ptr: _containers.RepeatedCompositeFieldContainer[PropertyReferenceData]
    object_properties_is_system: bool
    object_properties_is_sensitive: bool
    object_properties_is_kernel: bool
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        name: _Optional[str] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        subject_is_delegated: bool = ...,
        subject_is_authenticated: bool = ...,
        subject_is_staff: bool = ...,
        subject_is_member: bool = ...,
        subject_is_owner: bool = ...,
        effect: _Optional[_Union[PolicyEffect, str]] = ...,
        verbs: _Optional[_Iterable[_Union[AccessType, str]]] = ...,
        verb_kinds: _Optional[_Iterable[_Union[AccessKind, str]]] = ...,
        object_node_types: _Optional[_Iterable[_Union[NodeType, str]]] = ...,
        object_properties_ptr: _Optional[_Iterable[_Union[PropertyReferenceData, _Mapping]]] = ...,
        object_properties_is_system: bool = ...,
        object_properties_is_sensitive: bool = ...,
        object_properties_is_kernel: bool = ...,
    ) -> None: ...

class PortData(_message.Message):
    __slots__ = ("metatype", "type", "side", "field_ptr", "value_type", "value_packed")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    SIDE_FIELD_NUMBER: _ClassVar[int]
    FIELD_PTR_FIELD_NUMBER: _ClassVar[int]
    VALUE_TYPE_FIELD_NUMBER: _ClassVar[int]
    VALUE_PACKED_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    type: PortType
    side: PortSide
    field_ptr: NodeReferenceData
    value_type: TypeInfoData
    value_packed: _struct_pb2.Struct
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        type: _Optional[_Union[PortType, str]] = ...,
        side: _Optional[_Union[PortSide, str]] = ...,
        field_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        value_type: _Optional[_Union[TypeInfoData, _Mapping]] = ...,
        value_packed: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
    ) -> None: ...

class PortKeyData(_message.Message):
    __slots__ = ("metatype", "type", "side", "field_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    SIDE_FIELD_NUMBER: _ClassVar[int]
    FIELD_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    type: PortType
    side: PortSide
    field_ptr: NodeReferenceData
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        type: _Optional[_Union[PortType, str]] = ...,
        side: _Optional[_Union[PortSide, str]] = ...,
        field_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
    ) -> None: ...

class PropertyReferenceData(_message.Message):
    __slots__ = ("metatype", "type", "id", "references_node", "references_meta")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    REFERENCES_NODE_FIELD_NUMBER: _ClassVar[int]
    REFERENCES_META_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    type: ObjectType
    id: int
    references_node: NodeType
    references_meta: str
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        type: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[int] = ...,
        references_node: _Optional[_Union[NodeType, str]] = ...,
        references_meta: _Optional[str] = ...,
    ) -> None: ...

class QueryInfoData(_message.Message):
    __slots__ = ("metatype", "read_type", "node_type", "base_ptr", "filter", "sort")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    READ_TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    BASE_PTR_FIELD_NUMBER: _ClassVar[int]
    FILTER_FIELD_NUMBER: _ClassVar[int]
    SORT_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    read_type: ReadType
    node_type: NodeType
    base_ptr: NodeReferenceData
    filter: ExpressionData
    sort: _containers.RepeatedCompositeFieldContainer[ExpressionData]
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        read_type: _Optional[_Union[ReadType, str]] = ...,
        node_type: _Optional[_Union[NodeType, str]] = ...,
        base_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        filter: _Optional[_Union[ExpressionData, _Mapping]] = ...,
        sort: _Optional[_Iterable[_Union[ExpressionData, _Mapping]]] = ...,
    ) -> None: ...

class ReadOptionsData(_message.Message):
    __slots__ = (
        "metatype",
        "ancestor_types",
        "descendant_types",
        "include_properties_ptr",
        "exclude_properties_ptr",
        "select_properties_ptr",
        "select_all_properties",
        "include_hidden",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ANCESTOR_TYPES_FIELD_NUMBER: _ClassVar[int]
    DESCENDANT_TYPES_FIELD_NUMBER: _ClassVar[int]
    INCLUDE_PROPERTIES_PTR_FIELD_NUMBER: _ClassVar[int]
    EXCLUDE_PROPERTIES_PTR_FIELD_NUMBER: _ClassVar[int]
    SELECT_PROPERTIES_PTR_FIELD_NUMBER: _ClassVar[int]
    SELECT_ALL_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    INCLUDE_HIDDEN_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    ancestor_types: _containers.RepeatedScalarFieldContainer[NodeType]
    descendant_types: _containers.RepeatedScalarFieldContainer[NodeType]
    include_properties_ptr: _containers.RepeatedCompositeFieldContainer[PropertyReferenceData]
    exclude_properties_ptr: _containers.RepeatedCompositeFieldContainer[PropertyReferenceData]
    select_properties_ptr: _containers.RepeatedCompositeFieldContainer[PropertyReferenceData]
    select_all_properties: bool
    include_hidden: bool
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        ancestor_types: _Optional[_Iterable[_Union[NodeType, str]]] = ...,
        descendant_types: _Optional[_Iterable[_Union[NodeType, str]]] = ...,
        include_properties_ptr: _Optional[_Iterable[_Union[PropertyReferenceData, _Mapping]]] = ...,
        exclude_properties_ptr: _Optional[_Iterable[_Union[PropertyReferenceData, _Mapping]]] = ...,
        select_properties_ptr: _Optional[_Iterable[_Union[PropertyReferenceData, _Mapping]]] = ...,
        select_all_properties: bool = ...,
        include_hidden: bool = ...,
    ) -> None: ...

class RunAttemptData(_message.Message):
    __slots__ = (
        "metatype",
        "status",
        "duration",
        "started_at",
        "started_epoch",
        "terminated_at",
        "terminated_epoch",
        "error",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    STARTED_AT_FIELD_NUMBER: _ClassVar[int]
    STARTED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    TERMINATED_AT_FIELD_NUMBER: _ClassVar[int]
    TERMINATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    ERROR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    status: RunStatus
    duration: float
    started_at: _timestamp_pb2.Timestamp
    started_epoch: int
    terminated_at: _timestamp_pb2.Timestamp
    terminated_epoch: int
    error: RunErrorData
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        status: _Optional[_Union[RunStatus, str]] = ...,
        duration: _Optional[float] = ...,
        started_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        started_epoch: _Optional[int] = ...,
        terminated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        terminated_epoch: _Optional[int] = ...,
        error: _Optional[_Union[RunErrorData, _Mapping]] = ...,
    ) -> None: ...

class RunErrorData(_message.Message):
    __slots__ = ("metatype", "kind", "type", "title", "text", "node_ptr", "trace")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    KIND_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    TRACE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    kind: RunErrorKind
    type: RunErrorType
    title: str
    text: TextData
    node_ptr: NodeReferenceData
    trace: RunTraceData
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        kind: _Optional[_Union[RunErrorKind, str]] = ...,
        type: _Optional[_Union[RunErrorType, str]] = ...,
        title: _Optional[str] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        trace: _Optional[_Union[RunTraceData, _Mapping]] = ...,
    ) -> None: ...

class RunEventData(_message.Message):
    __slots__ = ("metatype", "type", "name", "title", "text", "text_plain", "created_at", "level")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    TEXT_PLAIN_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    LEVEL_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    type: RunEventType
    name: str
    title: str
    text: TextData
    text_plain: str
    created_at: _timestamp_pb2.Timestamp
    level: LogLevel
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        type: _Optional[_Union[RunEventType, str]] = ...,
        name: _Optional[str] = ...,
        title: _Optional[str] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        text_plain: _Optional[str] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        level: _Optional[_Union[LogLevel, str]] = ...,
    ) -> None: ...

class RunFrameData(_message.Message):
    __slots__ = ("metatype",)
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ...) -> None: ...

class RunOptionsData(_message.Message):
    __slots__ = (
        "metatype",
        "max_attempts",
        "max_concurrency",
        "max_runs",
        "timeout",
        "retry_interval",
        "backoff",
        "max_retry_interval",
        "retry_on",
        "suppress_fail",
        "suppress_abort",
        "breakpoints",
        "cache_mode",
        "cache_expiry",
        "model_provider",
        "model_type",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    MAX_ATTEMPTS_FIELD_NUMBER: _ClassVar[int]
    MAX_CONCURRENCY_FIELD_NUMBER: _ClassVar[int]
    MAX_RUNS_FIELD_NUMBER: _ClassVar[int]
    TIMEOUT_FIELD_NUMBER: _ClassVar[int]
    RETRY_INTERVAL_FIELD_NUMBER: _ClassVar[int]
    BACKOFF_FIELD_NUMBER: _ClassVar[int]
    MAX_RETRY_INTERVAL_FIELD_NUMBER: _ClassVar[int]
    RETRY_ON_FIELD_NUMBER: _ClassVar[int]
    SUPPRESS_FAIL_FIELD_NUMBER: _ClassVar[int]
    SUPPRESS_ABORT_FIELD_NUMBER: _ClassVar[int]
    BREAKPOINTS_FIELD_NUMBER: _ClassVar[int]
    CACHE_MODE_FIELD_NUMBER: _ClassVar[int]
    CACHE_EXPIRY_FIELD_NUMBER: _ClassVar[int]
    MODEL_PROVIDER_FIELD_NUMBER: _ClassVar[int]
    MODEL_TYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    max_attempts: int
    max_concurrency: int
    max_runs: int
    timeout: float
    retry_interval: float
    backoff: float
    max_retry_interval: float
    retry_on: _containers.RepeatedScalarFieldContainer[RunErrorType]
    suppress_fail: bool
    suppress_abort: bool
    breakpoints: _containers.RepeatedCompositeFieldContainer[BreakpointData]
    cache_mode: CacheMode
    cache_expiry: _duration_pb2.Duration
    model_provider: ModelProvider
    model_type: ModelType
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        max_attempts: _Optional[int] = ...,
        max_concurrency: _Optional[int] = ...,
        max_runs: _Optional[int] = ...,
        timeout: _Optional[float] = ...,
        retry_interval: _Optional[float] = ...,
        backoff: _Optional[float] = ...,
        max_retry_interval: _Optional[float] = ...,
        retry_on: _Optional[_Iterable[_Union[RunErrorType, str]]] = ...,
        suppress_fail: bool = ...,
        suppress_abort: bool = ...,
        breakpoints: _Optional[_Iterable[_Union[BreakpointData, _Mapping]]] = ...,
        cache_mode: _Optional[_Union[CacheMode, str]] = ...,
        cache_expiry: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ...,
        model_provider: _Optional[_Union[ModelProvider, str]] = ...,
        model_type: _Optional[_Union[ModelType, str]] = ...,
    ) -> None: ...

class RunSpanData(_message.Message):
    __slots__ = (
        "metatype",
        "type",
        "name",
        "title",
        "text",
        "text_plain",
        "started_at",
        "ended_at",
        "duration",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    TEXT_PLAIN_FIELD_NUMBER: _ClassVar[int]
    STARTED_AT_FIELD_NUMBER: _ClassVar[int]
    ENDED_AT_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    type: RunSpanType
    name: str
    title: str
    text: TextData
    text_plain: str
    started_at: _timestamp_pb2.Timestamp
    ended_at: _timestamp_pb2.Timestamp
    duration: float
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        type: _Optional[_Union[RunSpanType, str]] = ...,
        name: _Optional[str] = ...,
        title: _Optional[str] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        text_plain: _Optional[str] = ...,
        started_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        ended_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        duration: _Optional[float] = ...,
    ) -> None: ...

class RunTraceData(_message.Message):
    __slots__ = ("metatype", "frames")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    FRAMES_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    frames: _containers.RepeatedCompositeFieldContainer[RunFrameData]
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        frames: _Optional[_Iterable[_Union[RunFrameData, _Mapping]]] = ...,
    ) -> None: ...

class ScheduleData(_message.Message):
    __slots__ = ("metatype", "type", "timezone", "every", "interval", "offset", "cron")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    TIMEZONE_FIELD_NUMBER: _ClassVar[int]
    EVERY_FIELD_NUMBER: _ClassVar[int]
    INTERVAL_FIELD_NUMBER: _ClassVar[int]
    OFFSET_FIELD_NUMBER: _ClassVar[int]
    CRON_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    type: ScheduleType
    timezone: str
    every: int
    interval: TimeInterval
    offset: _duration_pb2.Duration
    cron: str
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        type: _Optional[_Union[ScheduleType, str]] = ...,
        timezone: _Optional[str] = ...,
        every: _Optional[int] = ...,
        interval: _Optional[_Union[TimeInterval, str]] = ...,
        offset: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ...,
        cron: _Optional[str] = ...,
    ) -> None: ...

class SecretReferenceData(_message.Message):
    __slots__ = ("metatype", "type", "id", "ck", "bench_id", "base_ck", "base_bench_id", "title")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    BENCH_ID_FIELD_NUMBER: _ClassVar[int]
    BASE_CK_FIELD_NUMBER: _ClassVar[int]
    BASE_BENCH_ID_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    type: NodeType
    id: str
    ck: str
    bench_id: str
    base_ck: str
    base_bench_id: str
    title: str
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        type: _Optional[_Union[NodeType, str]] = ...,
        id: _Optional[str] = ...,
        ck: _Optional[str] = ...,
        bench_id: _Optional[str] = ...,
        base_ck: _Optional[str] = ...,
        base_bench_id: _Optional[str] = ...,
        title: _Optional[str] = ...,
    ) -> None: ...

class SelectionData(_message.Message):
    __slots__ = ("metatype", "kind", "target", "nodes_ptr", "from_node_ptr", "to_node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    KIND_FIELD_NUMBER: _ClassVar[int]
    TARGET_FIELD_NUMBER: _ClassVar[int]
    NODES_PTR_FIELD_NUMBER: _ClassVar[int]
    FROM_NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    TO_NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    kind: SelectionKind
    target: SelectionTarget
    nodes_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    from_node_ptr: NodeReferenceData
    to_node_ptr: NodeReferenceData
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        kind: _Optional[_Union[SelectionKind, str]] = ...,
        target: _Optional[_Union[SelectionTarget, str]] = ...,
        nodes_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...,
        from_node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        to_node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
    ) -> None: ...

class SessionContextData(_message.Message):
    __slots__ = (
        "metatype",
        "block_ptr",
        "step_ptr",
        "pipe_ptr",
        "view_ptr",
        "session_ptr",
        "run_ptr",
        "run_root_ptr",
        "client_ptr",
        "machine_ptr",
        "server_ptr",
        "user_ptr",
        "identity_ptr",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    STEP_PTR_FIELD_NUMBER: _ClassVar[int]
    PIPE_PTR_FIELD_NUMBER: _ClassVar[int]
    VIEW_PTR_FIELD_NUMBER: _ClassVar[int]
    SESSION_PTR_FIELD_NUMBER: _ClassVar[int]
    RUN_PTR_FIELD_NUMBER: _ClassVar[int]
    RUN_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CLIENT_PTR_FIELD_NUMBER: _ClassVar[int]
    MACHINE_PTR_FIELD_NUMBER: _ClassVar[int]
    SERVER_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_PTR_FIELD_NUMBER: _ClassVar[int]
    IDENTITY_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    block_ptr: NodeReferenceData
    step_ptr: NodeReferenceData
    pipe_ptr: NodeReferenceData
    view_ptr: NodeReferenceData
    session_ptr: NodeReferenceData
    run_ptr: NodeReferenceData
    run_root_ptr: NodeReferenceData
    client_ptr: NodeReferenceData
    machine_ptr: NodeReferenceData
    server_ptr: NodeReferenceData
    user_ptr: NodeReferenceData
    identity_ptr: NodeReferenceData
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        step_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        pipe_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        view_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        session_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        run_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        run_root_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        client_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        machine_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        server_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        user_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        identity_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
    ) -> None: ...

class StartViewStateData(_message.Message):
    __slots__ = ("metatype", "inputs_packed", "run_ptr", "feed")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    INPUTS_PACKED_FIELD_NUMBER: _ClassVar[int]
    RUN_PTR_FIELD_NUMBER: _ClassVar[int]
    FEED_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    inputs_packed: _struct_pb2.Struct
    run_ptr: NodeReferenceData
    feed: FeedViewStateData
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        inputs_packed: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
        run_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        feed: _Optional[_Union[FeedViewStateData, _Mapping]] = ...,
    ) -> None: ...

class SubjectData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "is_authenticated",
        "is_staff",
        "is_system",
        "client_ptr",
        "user_ptr",
        "server_ptr",
        "identity_ptr",
        "badges_ptr",
        "owned_ptr",
        "memberships_ptr",
        "roles_ptr",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    IS_AUTHENTICATED_FIELD_NUMBER: _ClassVar[int]
    IS_STAFF_FIELD_NUMBER: _ClassVar[int]
    IS_SYSTEM_FIELD_NUMBER: _ClassVar[int]
    CLIENT_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_PTR_FIELD_NUMBER: _ClassVar[int]
    SERVER_PTR_FIELD_NUMBER: _ClassVar[int]
    IDENTITY_PTR_FIELD_NUMBER: _ClassVar[int]
    BADGES_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_PTR_FIELD_NUMBER: _ClassVar[int]
    MEMBERSHIPS_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLES_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: int
    is_authenticated: bool
    is_staff: bool
    is_system: bool
    client_ptr: NodeReferenceData
    user_ptr: NodeReferenceData
    server_ptr: NodeReferenceData
    identity_ptr: NodeReferenceData
    badges_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    owned_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    memberships_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    roles_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[int] = ...,
        is_authenticated: bool = ...,
        is_staff: bool = ...,
        is_system: bool = ...,
        client_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        user_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        server_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        identity_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        badges_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...,
        owned_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...,
        memberships_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...,
        roles_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...,
    ) -> None: ...

class TextData(_message.Message):
    __slots__ = ("metatype", "lines")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    LINES_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    lines: _containers.RepeatedCompositeFieldContainer[TextLineData]
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        lines: _Optional[_Iterable[_Union[TextLineData, _Mapping]]] = ...,
    ) -> None: ...

class TextLineData(_message.Message):
    __slots__ = (
        "metatype",
        "type",
        "spans",
        "color",
        "is_bold",
        "is_italic",
        "is_strikethrough",
        "is_underline",
        "is_code",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    SPANS_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    IS_BOLD_FIELD_NUMBER: _ClassVar[int]
    IS_ITALIC_FIELD_NUMBER: _ClassVar[int]
    IS_STRIKETHROUGH_FIELD_NUMBER: _ClassVar[int]
    IS_UNDERLINE_FIELD_NUMBER: _ClassVar[int]
    IS_CODE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    type: TextLineType
    spans: _containers.RepeatedCompositeFieldContainer[TextSpanData]
    color: ColorType
    is_bold: bool
    is_italic: bool
    is_strikethrough: bool
    is_underline: bool
    is_code: bool
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        type: _Optional[_Union[TextLineType, str]] = ...,
        spans: _Optional[_Iterable[_Union[TextSpanData, _Mapping]]] = ...,
        color: _Optional[_Union[ColorType, str]] = ...,
        is_bold: bool = ...,
        is_italic: bool = ...,
        is_strikethrough: bool = ...,
        is_underline: bool = ...,
        is_code: bool = ...,
    ) -> None: ...

class TextSpanData(_message.Message):
    __slots__ = (
        "metatype",
        "content",
        "node_ptr_node",
        "node_ptr_file",
        "node_ptr_secret",
        "color",
        "is_bold",
        "is_italic",
        "is_strikethrough",
        "is_underline",
        "is_code",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    CONTENT_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_NODE_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FILE_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_SECRET_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    IS_BOLD_FIELD_NUMBER: _ClassVar[int]
    IS_ITALIC_FIELD_NUMBER: _ClassVar[int]
    IS_STRIKETHROUGH_FIELD_NUMBER: _ClassVar[int]
    IS_UNDERLINE_FIELD_NUMBER: _ClassVar[int]
    IS_CODE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    content: str
    node_ptr_node: NodeReferenceData
    node_ptr_file: FileReferenceData
    node_ptr_secret: SecretReferenceData
    color: ColorType
    is_bold: bool
    is_italic: bool
    is_strikethrough: bool
    is_underline: bool
    is_code: bool
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        content: _Optional[str] = ...,
        node_ptr_node: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        node_ptr_file: _Optional[_Union[FileReferenceData, _Mapping]] = ...,
        node_ptr_secret: _Optional[_Union[SecretReferenceData, _Mapping]] = ...,
        color: _Optional[_Union[ColorType, str]] = ...,
        is_bold: bool = ...,
        is_italic: bool = ...,
        is_strikethrough: bool = ...,
        is_underline: bool = ...,
        is_code: bool = ...,
    ) -> None: ...

class TransformData(_message.Message):
    __slots__ = (
        "metatype",
        "translate_x",
        "translate_y",
        "scale_x",
        "scale_y",
        "skew_x",
        "skew_y",
        "rotate_x",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TRANSLATE_X_FIELD_NUMBER: _ClassVar[int]
    TRANSLATE_Y_FIELD_NUMBER: _ClassVar[int]
    SCALE_X_FIELD_NUMBER: _ClassVar[int]
    SCALE_Y_FIELD_NUMBER: _ClassVar[int]
    SKEW_X_FIELD_NUMBER: _ClassVar[int]
    SKEW_Y_FIELD_NUMBER: _ClassVar[int]
    ROTATE_X_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    translate_x: float
    translate_y: float
    scale_x: float
    scale_y: float
    skew_x: float
    skew_y: float
    rotate_x: float
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        translate_x: _Optional[float] = ...,
        translate_y: _Optional[float] = ...,
        scale_x: _Optional[float] = ...,
        scale_y: _Optional[float] = ...,
        skew_x: _Optional[float] = ...,
        skew_y: _Optional[float] = ...,
        rotate_x: _Optional[float] = ...,
    ) -> None: ...

class TreeViewStateData(_message.Message):
    __slots__ = ("metatype", "node_types", "filter_is_page", "is_default_expanded", "preset")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPES_FIELD_NUMBER: _ClassVar[int]
    FILTER_IS_PAGE_FIELD_NUMBER: _ClassVar[int]
    IS_DEFAULT_EXPANDED_FIELD_NUMBER: _ClassVar[int]
    PRESET_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    node_types: _containers.RepeatedScalarFieldContainer[NodeType]
    filter_is_page: bool
    is_default_expanded: bool
    preset: TreeViewPreset
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        node_types: _Optional[_Iterable[_Union[NodeType, str]]] = ...,
        filter_is_page: bool = ...,
        is_default_expanded: bool = ...,
        preset: _Optional[_Union[TreeViewPreset, str]] = ...,
    ) -> None: ...

class TriggerInfoData(_message.Message):
    __slots__ = ("metatype", "processed_epoch", "schedule", "signal_ptr", "condition")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    PROCESSED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    SCHEDULE_FIELD_NUMBER: _ClassVar[int]
    SIGNAL_PTR_FIELD_NUMBER: _ClassVar[int]
    CONDITION_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    processed_epoch: int
    schedule: ScheduleData
    signal_ptr: NodeReferenceData
    condition: ExpressionData
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        processed_epoch: _Optional[int] = ...,
        schedule: _Optional[_Union[ScheduleData, _Mapping]] = ...,
        signal_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        condition: _Optional[_Union[ExpressionData, _Mapping]] = ...,
    ) -> None: ...

class TypeConstraintData(_message.Message):
    __slots__ = (
        "metatype",
        "min_value",
        "max_value",
        "step_value",
        "min_length",
        "max_length",
        "regex",
        "starts_with",
        "ends_with",
        "node_is_attached",
        "node_types",
        "block_types",
        "step_types",
        "file_types",
        "file_formats",
        "view_types",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    MIN_VALUE_FIELD_NUMBER: _ClassVar[int]
    MAX_VALUE_FIELD_NUMBER: _ClassVar[int]
    STEP_VALUE_FIELD_NUMBER: _ClassVar[int]
    MIN_LENGTH_FIELD_NUMBER: _ClassVar[int]
    MAX_LENGTH_FIELD_NUMBER: _ClassVar[int]
    REGEX_FIELD_NUMBER: _ClassVar[int]
    STARTS_WITH_FIELD_NUMBER: _ClassVar[int]
    ENDS_WITH_FIELD_NUMBER: _ClassVar[int]
    NODE_IS_ATTACHED_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPES_FIELD_NUMBER: _ClassVar[int]
    BLOCK_TYPES_FIELD_NUMBER: _ClassVar[int]
    STEP_TYPES_FIELD_NUMBER: _ClassVar[int]
    FILE_TYPES_FIELD_NUMBER: _ClassVar[int]
    FILE_FORMATS_FIELD_NUMBER: _ClassVar[int]
    VIEW_TYPES_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    min_value: float
    max_value: float
    step_value: float
    min_length: int
    max_length: int
    regex: str
    starts_with: str
    ends_with: str
    node_is_attached: bool
    node_types: _containers.RepeatedScalarFieldContainer[NodeType]
    block_types: _containers.RepeatedScalarFieldContainer[BlockType]
    step_types: _containers.RepeatedScalarFieldContainer[StepType]
    file_types: _containers.RepeatedScalarFieldContainer[FileType]
    file_formats: _containers.RepeatedScalarFieldContainer[FileFormat]
    view_types: _containers.RepeatedScalarFieldContainer[ViewType]
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        min_value: _Optional[float] = ...,
        max_value: _Optional[float] = ...,
        step_value: _Optional[float] = ...,
        min_length: _Optional[int] = ...,
        max_length: _Optional[int] = ...,
        regex: _Optional[str] = ...,
        starts_with: _Optional[str] = ...,
        ends_with: _Optional[str] = ...,
        node_is_attached: bool = ...,
        node_types: _Optional[_Iterable[_Union[NodeType, str]]] = ...,
        block_types: _Optional[_Iterable[_Union[BlockType, str]]] = ...,
        step_types: _Optional[_Iterable[_Union[StepType, str]]] = ...,
        file_types: _Optional[_Iterable[_Union[FileType, str]]] = ...,
        file_formats: _Optional[_Iterable[_Union[FileFormat, str]]] = ...,
        view_types: _Optional[_Iterable[_Union[ViewType, str]]] = ...,
    ) -> None: ...

class TypeInfoData(_message.Message):
    __slots__ = (
        "metatype",
        "kind",
        "primitive_type",
        "bench_type",
        "base_type_ptr",
        "base_field_zone",
        "oneof_ptr",
        "default_packed",
        "format",
        "condition",
        "constraint",
        "is_required",
        "is_list",
        "is_secret",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    KIND_FIELD_NUMBER: _ClassVar[int]
    PRIMITIVE_TYPE_FIELD_NUMBER: _ClassVar[int]
    BENCH_TYPE_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_FIELD_ZONE_FIELD_NUMBER: _ClassVar[int]
    ONEOF_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFAULT_PACKED_FIELD_NUMBER: _ClassVar[int]
    FORMAT_FIELD_NUMBER: _ClassVar[int]
    CONDITION_FIELD_NUMBER: _ClassVar[int]
    CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    IS_REQUIRED_FIELD_NUMBER: _ClassVar[int]
    IS_LIST_FIELD_NUMBER: _ClassVar[int]
    IS_SECRET_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    kind: TypeKind
    primitive_type: PrimitiveType
    bench_type: BenchType
    base_type_ptr: NodeReferenceData
    base_field_zone: FieldZone
    oneof_ptr: NodeReferenceData
    default_packed: _struct_pb2.Struct
    format: TypeFormat
    condition: ExpressionData
    constraint: TypeConstraintData
    is_required: bool
    is_list: bool
    is_secret: bool
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        kind: _Optional[_Union[TypeKind, str]] = ...,
        primitive_type: _Optional[_Union[PrimitiveType, str]] = ...,
        bench_type: _Optional[_Union[BenchType, str]] = ...,
        base_type_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        base_field_zone: _Optional[_Union[FieldZone, str]] = ...,
        oneof_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        default_packed: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
        format: _Optional[_Union[TypeFormat, str]] = ...,
        condition: _Optional[_Union[ExpressionData, _Mapping]] = ...,
        constraint: _Optional[_Union[TypeConstraintData, _Mapping]] = ...,
        is_required: bool = ...,
        is_list: bool = ...,
        is_secret: bool = ...,
    ) -> None: ...

class UserWizardViewStateData(_message.Message):
    __slots__ = ("metatype", "stage")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    STAGE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    stage: UserWizardViewStage
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        stage: _Optional[_Union[UserWizardViewStage, str]] = ...,
    ) -> None: ...

class ValueData(_message.Message):
    __slots__ = ("metatype", "type", "name", "text", "value_packed")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    VALUE_PACKED_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    type: TypeInfoData
    name: str
    text: TextData
    value_packed: _struct_pb2.Struct
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        type: _Optional[_Union[TypeInfoData, _Mapping]] = ...,
        name: _Optional[str] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        value_packed: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
    ) -> None: ...

class Vector2Data(_message.Message):
    __slots__ = ("metatype", "x", "y")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    x: float
    y: float
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        x: _Optional[float] = ...,
        y: _Optional[float] = ...,
    ) -> None: ...

class Vector3Data(_message.Message):
    __slots__ = ("metatype", "x", "y", "z")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    Z_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    x: float
    y: float
    z: float
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        x: _Optional[float] = ...,
        y: _Optional[float] = ...,
        z: _Optional[float] = ...,
    ) -> None: ...

class Vector4Data(_message.Message):
    __slots__ = ("metatype", "x", "y", "z", "w")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    Z_FIELD_NUMBER: _ClassVar[int]
    W_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    x: float
    y: float
    z: float
    w: float
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        x: _Optional[float] = ...,
        y: _Optional[float] = ...,
        z: _Optional[float] = ...,
        w: _Optional[float] = ...,
    ) -> None: ...

class BadgeData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "ck",
        "parent_ptr",
        "package_ptr",
        "bench_ptr",
        "template_ptr",
        "templated_epoch",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "name",
        "delegated_policies",
        "expires_at",
        "key",
        "key_hash",
        "password",
        "password_hash",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    DELEGATED_POLICIES_FIELD_NUMBER: _ClassVar[int]
    EXPIRES_AT_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    KEY_HASH_FIELD_NUMBER: _ClassVar[int]
    PASSWORD_FIELD_NUMBER: _ClassVar[int]
    PASSWORD_HASH_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    templated_epoch: int
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    name: str
    delegated_policies: _containers.RepeatedCompositeFieldContainer[PolicyData]
    expires_at: _timestamp_pb2.Timestamp
    key: str
    key_hash: str
    password: str
    password_hash: str
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        ck: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        templated_epoch: _Optional[int] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        name: _Optional[str] = ...,
        delegated_policies: _Optional[_Iterable[_Union[PolicyData, _Mapping]]] = ...,
        expires_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        key: _Optional[str] = ...,
        key_hash: _Optional[str] = ...,
        password: _Optional[str] = ...,
        password_hash: _Optional[str] = ...,
    ) -> None: ...

class BenchData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "bench_ptr",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "main_handle_ptr",
        "slug",
        "name",
        "text",
        "icon",
        "owner_ptr",
        "region",
        "encryption_key",
        "policies",
        "main_store_ptr",
        "main_server_ptr",
        "main_drive_ptr",
        "main_vault_ptr",
        "main_cache_ptr",
        "main_branch_ptr",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    MAIN_HANDLE_PTR_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    OWNER_PTR_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    ENCRYPTION_KEY_FIELD_NUMBER: _ClassVar[int]
    POLICIES_FIELD_NUMBER: _ClassVar[int]
    MAIN_STORE_PTR_FIELD_NUMBER: _ClassVar[int]
    MAIN_SERVER_PTR_FIELD_NUMBER: _ClassVar[int]
    MAIN_DRIVE_PTR_FIELD_NUMBER: _ClassVar[int]
    MAIN_VAULT_PTR_FIELD_NUMBER: _ClassVar[int]
    MAIN_CACHE_PTR_FIELD_NUMBER: _ClassVar[int]
    MAIN_BRANCH_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    main_handle_ptr: NodeReferenceData
    slug: str
    name: str
    text: TextData
    icon: IconData
    owner_ptr: NodeReferenceData
    region: Region
    encryption_key: str
    policies: _containers.RepeatedCompositeFieldContainer[PolicyData]
    main_store_ptr: NodeReferenceData
    main_server_ptr: NodeReferenceData
    main_drive_ptr: NodeReferenceData
    main_vault_ptr: NodeReferenceData
    main_cache_ptr: NodeReferenceData
    main_branch_ptr: NodeReferenceData
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        main_handle_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        slug: _Optional[str] = ...,
        name: _Optional[str] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        icon: _Optional[_Union[IconData, _Mapping]] = ...,
        owner_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        region: _Optional[_Union[Region, str]] = ...,
        encryption_key: _Optional[str] = ...,
        policies: _Optional[_Iterable[_Union[PolicyData, _Mapping]]] = ...,
        main_store_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        main_server_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        main_drive_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        main_vault_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        main_cache_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        main_branch_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
    ) -> None: ...

class BlockData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "ck",
        "parent_ptr",
        "package_ptr",
        "bench_ptr",
        "template_ptr",
        "templated_epoch",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "type",
        "name",
        "order_key",
        "bases_ptr",
        "text",
        "icon",
        "value_type",
        "value_packed",
        "code",
        "run_options",
        "roles_ptr",
        "identity_ptr",
        "policies",
        "delegated_policies",
        "is_builtin",
        "is_owned",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    BASES_PTR_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    VALUE_TYPE_FIELD_NUMBER: _ClassVar[int]
    VALUE_PACKED_FIELD_NUMBER: _ClassVar[int]
    CODE_FIELD_NUMBER: _ClassVar[int]
    RUN_OPTIONS_FIELD_NUMBER: _ClassVar[int]
    ROLES_PTR_FIELD_NUMBER: _ClassVar[int]
    IDENTITY_PTR_FIELD_NUMBER: _ClassVar[int]
    POLICIES_FIELD_NUMBER: _ClassVar[int]
    DELEGATED_POLICIES_FIELD_NUMBER: _ClassVar[int]
    IS_BUILTIN_FIELD_NUMBER: _ClassVar[int]
    IS_OWNED_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    templated_epoch: int
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    type: BlockType
    name: str
    order_key: str
    bases_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    text: TextData
    icon: IconData
    value_type: TypeInfoData
    value_packed: _struct_pb2.Struct
    code: CodeData
    run_options: RunOptionsData
    roles_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    identity_ptr: NodeReferenceData
    policies: _containers.RepeatedCompositeFieldContainer[PolicyData]
    delegated_policies: _containers.RepeatedCompositeFieldContainer[PolicyData]
    is_builtin: bool
    is_owned: bool
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        ck: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        templated_epoch: _Optional[int] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        type: _Optional[_Union[BlockType, str]] = ...,
        name: _Optional[str] = ...,
        order_key: _Optional[str] = ...,
        bases_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        icon: _Optional[_Union[IconData, _Mapping]] = ...,
        value_type: _Optional[_Union[TypeInfoData, _Mapping]] = ...,
        value_packed: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
        code: _Optional[_Union[CodeData, _Mapping]] = ...,
        run_options: _Optional[_Union[RunOptionsData, _Mapping]] = ...,
        roles_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...,
        identity_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        policies: _Optional[_Iterable[_Union[PolicyData, _Mapping]]] = ...,
        delegated_policies: _Optional[_Iterable[_Union[PolicyData, _Mapping]]] = ...,
        is_builtin: bool = ...,
        is_owned: bool = ...,
    ) -> None: ...

class BranchData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "bench_ptr",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "name",
        "slug",
        "text",
        "icon",
        "policies",
        "main_package_ptr",
        "base_ptr",
        "is_overlay",
        "is_light",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    POLICIES_FIELD_NUMBER: _ClassVar[int]
    MAIN_PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_PTR_FIELD_NUMBER: _ClassVar[int]
    IS_OVERLAY_FIELD_NUMBER: _ClassVar[int]
    IS_LIGHT_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    name: str
    slug: str
    text: TextData
    icon: IconData
    policies: _containers.RepeatedCompositeFieldContainer[PolicyData]
    main_package_ptr: NodeReferenceData
    base_ptr: NodeReferenceData
    is_overlay: bool
    is_light: bool
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        name: _Optional[str] = ...,
        slug: _Optional[str] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        icon: _Optional[_Union[IconData, _Mapping]] = ...,
        policies: _Optional[_Iterable[_Union[PolicyData, _Mapping]]] = ...,
        main_package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        base_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        is_overlay: bool = ...,
        is_light: bool = ...,
    ) -> None: ...

class CacheData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "bench_ptr",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "name",
        "text",
        "region",
        "status",
        "current_status",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    CURRENT_STATUS_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    name: str
    text: TextData
    region: Region
    status: ResourceStatus
    current_status: ResourceStatus
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        name: _Optional[str] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        region: _Optional[_Union[Region, str]] = ...,
        status: _Optional[_Union[ResourceStatus, str]] = ...,
        current_status: _Optional[_Union[ResourceStatus, str]] = ...,
    ) -> None: ...

class ClientData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "bench_ptr",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "type",
        "name",
        "device_type",
        "device_name",
        "operating_system",
        "browser_name",
        "browser_version",
        "place_id",
        "access_token",
        "seen_at",
        "logged_in_at",
        "space_ptr",
        "machine_ptr",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    DEVICE_TYPE_FIELD_NUMBER: _ClassVar[int]
    DEVICE_NAME_FIELD_NUMBER: _ClassVar[int]
    OPERATING_SYSTEM_FIELD_NUMBER: _ClassVar[int]
    BROWSER_NAME_FIELD_NUMBER: _ClassVar[int]
    BROWSER_VERSION_FIELD_NUMBER: _ClassVar[int]
    PLACE_ID_FIELD_NUMBER: _ClassVar[int]
    ACCESS_TOKEN_FIELD_NUMBER: _ClassVar[int]
    SEEN_AT_FIELD_NUMBER: _ClassVar[int]
    LOGGED_IN_AT_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MACHINE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    type: ClientType
    name: str
    device_type: str
    device_name: str
    operating_system: str
    browser_name: str
    browser_version: str
    place_id: str
    access_token: str
    seen_at: _timestamp_pb2.Timestamp
    logged_in_at: _timestamp_pb2.Timestamp
    space_ptr: NodeReferenceData
    machine_ptr: NodeReferenceData
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        type: _Optional[_Union[ClientType, str]] = ...,
        name: _Optional[str] = ...,
        device_type: _Optional[str] = ...,
        device_name: _Optional[str] = ...,
        operating_system: _Optional[str] = ...,
        browser_name: _Optional[str] = ...,
        browser_version: _Optional[str] = ...,
        place_id: _Optional[str] = ...,
        access_token: _Optional[str] = ...,
        seen_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        logged_in_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        machine_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
    ) -> None: ...

class DependencyData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "ck",
        "parent_ptr",
        "package_ptr",
        "bench_ptr",
        "template_ptr",
        "templated_epoch",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "scopes_ptr",
        "dependency_ptr",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    SCOPES_PTR_FIELD_NUMBER: _ClassVar[int]
    DEPENDENCY_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    templated_epoch: int
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    scopes_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    dependency_ptr: NodeReferenceData
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        ck: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        templated_epoch: _Optional[int] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        scopes_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...,
        dependency_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
    ) -> None: ...

class DriveData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "bench_ptr",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "name",
        "text",
        "region",
        "status",
        "current_status",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    CURRENT_STATUS_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    name: str
    text: TextData
    region: Region
    status: ResourceStatus
    current_status: ResourceStatus
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        name: _Optional[str] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        region: _Optional[_Union[Region, str]] = ...,
        status: _Optional[_Union[ResourceStatus, str]] = ...,
        current_status: _Optional[_Union[ResourceStatus, str]] = ...,
    ) -> None: ...

class FieldData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "ck",
        "parent_ptr",
        "package_ptr",
        "bench_ptr",
        "template_ptr",
        "templated_epoch",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "name",
        "order_key",
        "zone",
        "text",
        "icon",
        "value_packed",
        "kind",
        "primitive_type",
        "bench_type",
        "base_type_ptr",
        "base_field_zone",
        "oneof_ptr",
        "default_packed",
        "format",
        "condition",
        "constraint",
        "is_required",
        "is_list",
        "is_secret",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    ZONE_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    VALUE_PACKED_FIELD_NUMBER: _ClassVar[int]
    KIND_FIELD_NUMBER: _ClassVar[int]
    PRIMITIVE_TYPE_FIELD_NUMBER: _ClassVar[int]
    BENCH_TYPE_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_FIELD_ZONE_FIELD_NUMBER: _ClassVar[int]
    ONEOF_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFAULT_PACKED_FIELD_NUMBER: _ClassVar[int]
    FORMAT_FIELD_NUMBER: _ClassVar[int]
    CONDITION_FIELD_NUMBER: _ClassVar[int]
    CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    IS_REQUIRED_FIELD_NUMBER: _ClassVar[int]
    IS_LIST_FIELD_NUMBER: _ClassVar[int]
    IS_SECRET_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    templated_epoch: int
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    name: str
    order_key: str
    zone: FieldZone
    text: TextData
    icon: IconData
    value_packed: _struct_pb2.Struct
    kind: TypeKind
    primitive_type: PrimitiveType
    bench_type: BenchType
    base_type_ptr: NodeReferenceData
    base_field_zone: FieldZone
    oneof_ptr: NodeReferenceData
    default_packed: _struct_pb2.Struct
    format: TypeFormat
    condition: ExpressionData
    constraint: TypeConstraintData
    is_required: bool
    is_list: bool
    is_secret: bool
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        ck: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        templated_epoch: _Optional[int] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        name: _Optional[str] = ...,
        order_key: _Optional[str] = ...,
        zone: _Optional[_Union[FieldZone, str]] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        icon: _Optional[_Union[IconData, _Mapping]] = ...,
        value_packed: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
        kind: _Optional[_Union[TypeKind, str]] = ...,
        primitive_type: _Optional[_Union[PrimitiveType, str]] = ...,
        bench_type: _Optional[_Union[BenchType, str]] = ...,
        base_type_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        base_field_zone: _Optional[_Union[FieldZone, str]] = ...,
        oneof_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        default_packed: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
        format: _Optional[_Union[TypeFormat, str]] = ...,
        condition: _Optional[_Union[ExpressionData, _Mapping]] = ...,
        constraint: _Optional[_Union[TypeConstraintData, _Mapping]] = ...,
        is_required: bool = ...,
        is_list: bool = ...,
        is_secret: bool = ...,
    ) -> None: ...

class FileData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "bench_ptr",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "text",
        "region",
        "status",
        "current_status",
        "kind",
        "title",
        "external_url",
        "inline_content",
        "coarse_type",
        "mime_type",
        "format",
        "size",
        "sha256",
        "width",
        "height",
        "aspect_ratio",
        "codec",
        "duration",
        "bitrate",
        "channels",
        "sample_rate",
        "retention",
        "expires_at",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    CURRENT_STATUS_FIELD_NUMBER: _ClassVar[int]
    KIND_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    EXTERNAL_URL_FIELD_NUMBER: _ClassVar[int]
    INLINE_CONTENT_FIELD_NUMBER: _ClassVar[int]
    COARSE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MIME_TYPE_FIELD_NUMBER: _ClassVar[int]
    FORMAT_FIELD_NUMBER: _ClassVar[int]
    SIZE_FIELD_NUMBER: _ClassVar[int]
    SHA256_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    ASPECT_RATIO_FIELD_NUMBER: _ClassVar[int]
    CODEC_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    BITRATE_FIELD_NUMBER: _ClassVar[int]
    CHANNELS_FIELD_NUMBER: _ClassVar[int]
    SAMPLE_RATE_FIELD_NUMBER: _ClassVar[int]
    RETENTION_FIELD_NUMBER: _ClassVar[int]
    EXPIRES_AT_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    text: TextData
    region: Region
    status: ResourceStatus
    current_status: ResourceStatus
    kind: FileKind
    title: str
    external_url: str
    inline_content: bytes
    coarse_type: FileType
    mime_type: str
    format: FileFormat
    size: int
    sha256: str
    width: int
    height: int
    aspect_ratio: float
    codec: str
    duration: float
    bitrate: int
    channels: int
    sample_rate: int
    retention: FileRetentionMode
    expires_at: _timestamp_pb2.Timestamp
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        region: _Optional[_Union[Region, str]] = ...,
        status: _Optional[_Union[ResourceStatus, str]] = ...,
        current_status: _Optional[_Union[ResourceStatus, str]] = ...,
        kind: _Optional[_Union[FileKind, str]] = ...,
        title: _Optional[str] = ...,
        external_url: _Optional[str] = ...,
        inline_content: _Optional[bytes] = ...,
        coarse_type: _Optional[_Union[FileType, str]] = ...,
        mime_type: _Optional[str] = ...,
        format: _Optional[_Union[FileFormat, str]] = ...,
        size: _Optional[int] = ...,
        sha256: _Optional[str] = ...,
        width: _Optional[int] = ...,
        height: _Optional[int] = ...,
        aspect_ratio: _Optional[float] = ...,
        codec: _Optional[str] = ...,
        duration: _Optional[float] = ...,
        bitrate: _Optional[int] = ...,
        channels: _Optional[int] = ...,
        sample_rate: _Optional[int] = ...,
        retention: _Optional[_Union[FileRetentionMode, str]] = ...,
        expires_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
    ) -> None: ...

class HandleData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "bench_ptr",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "slug",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    slug: str
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        slug: _Optional[str] = ...,
    ) -> None: ...

class InviteData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "bench_ptr",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "user_ptr",
        "user_email",
        "is_owner",
        "roles_ptr",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    USER_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_EMAIL_FIELD_NUMBER: _ClassVar[int]
    IS_OWNER_FIELD_NUMBER: _ClassVar[int]
    ROLES_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    user_ptr: NodeReferenceData
    user_email: str
    is_owner: bool
    roles_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        user_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        user_email: _Optional[str] = ...,
        is_owner: bool = ...,
        roles_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...,
    ) -> None: ...

class LogData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "package_ptr",
        "bench_ptr",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "kind",
        "level",
        "change_ptr",
        "undo_of_ptr",
        "type",
        "node_ptr",
        "properties",
        "old_node_packed",
        "old_node_secret_packed",
        "new_node_packed",
        "new_node_secret_packed",
        "new_revision",
        "category",
        "vignette",
        "block_ptr",
        "step_ptr",
        "pipe_ptr",
        "view_ptr",
        "session_ptr",
        "run_ptr",
        "run_root_ptr",
        "client_ptr",
        "machine_ptr",
        "server_ptr",
        "user_ptr",
        "identity_ptr",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    KIND_FIELD_NUMBER: _ClassVar[int]
    LEVEL_FIELD_NUMBER: _ClassVar[int]
    CHANGE_PTR_FIELD_NUMBER: _ClassVar[int]
    UNDO_OF_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    OLD_NODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    OLD_NODE_SECRET_PACKED_FIELD_NUMBER: _ClassVar[int]
    NEW_NODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    NEW_NODE_SECRET_PACKED_FIELD_NUMBER: _ClassVar[int]
    NEW_REVISION_FIELD_NUMBER: _ClassVar[int]
    CATEGORY_FIELD_NUMBER: _ClassVar[int]
    VIGNETTE_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    STEP_PTR_FIELD_NUMBER: _ClassVar[int]
    PIPE_PTR_FIELD_NUMBER: _ClassVar[int]
    VIEW_PTR_FIELD_NUMBER: _ClassVar[int]
    SESSION_PTR_FIELD_NUMBER: _ClassVar[int]
    RUN_PTR_FIELD_NUMBER: _ClassVar[int]
    RUN_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CLIENT_PTR_FIELD_NUMBER: _ClassVar[int]
    MACHINE_PTR_FIELD_NUMBER: _ClassVar[int]
    SERVER_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_PTR_FIELD_NUMBER: _ClassVar[int]
    IDENTITY_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    kind: LogKind
    level: LogLevel
    change_ptr: NodeReferenceData
    undo_of_ptr: NodeReferenceData
    type: AccessType
    node_ptr: NodeReferenceData
    properties: _containers.RepeatedScalarFieldContainer[int]
    old_node_packed: _struct_pb2.Struct
    old_node_secret_packed: _struct_pb2.Struct
    new_node_packed: _struct_pb2.Struct
    new_node_secret_packed: _struct_pb2.Struct
    new_revision: int
    category: ChangeCategory
    vignette: ChangeVignetteData
    block_ptr: NodeReferenceData
    step_ptr: NodeReferenceData
    pipe_ptr: NodeReferenceData
    view_ptr: NodeReferenceData
    session_ptr: NodeReferenceData
    run_ptr: NodeReferenceData
    run_root_ptr: NodeReferenceData
    client_ptr: NodeReferenceData
    machine_ptr: NodeReferenceData
    server_ptr: NodeReferenceData
    user_ptr: NodeReferenceData
    identity_ptr: NodeReferenceData
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        kind: _Optional[_Union[LogKind, str]] = ...,
        level: _Optional[_Union[LogLevel, str]] = ...,
        change_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        undo_of_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        type: _Optional[_Union[AccessType, str]] = ...,
        node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        properties: _Optional[_Iterable[int]] = ...,
        old_node_packed: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
        old_node_secret_packed: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
        new_node_packed: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
        new_node_secret_packed: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
        new_revision: _Optional[int] = ...,
        category: _Optional[_Union[ChangeCategory, str]] = ...,
        vignette: _Optional[_Union[ChangeVignetteData, _Mapping]] = ...,
        block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        step_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        pipe_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        view_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        session_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        run_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        run_root_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        client_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        machine_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        server_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        user_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        identity_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
    ) -> None: ...

class MachineData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "bench_ptr",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "name",
        "text",
        "region",
        "status",
        "current_status",
        "version",
        "current_version",
        "external_name",
        "external_id",
        "connection_uri",
        "client_ptr",
        "cpu",
        "current_cpu",
        "ram",
        "current_ram",
        "started_at",
        "terminated_at",
        "active_at",
        "restarted_at",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    CURRENT_STATUS_FIELD_NUMBER: _ClassVar[int]
    VERSION_FIELD_NUMBER: _ClassVar[int]
    CURRENT_VERSION_FIELD_NUMBER: _ClassVar[int]
    EXTERNAL_NAME_FIELD_NUMBER: _ClassVar[int]
    EXTERNAL_ID_FIELD_NUMBER: _ClassVar[int]
    CONNECTION_URI_FIELD_NUMBER: _ClassVar[int]
    CLIENT_PTR_FIELD_NUMBER: _ClassVar[int]
    CPU_FIELD_NUMBER: _ClassVar[int]
    CURRENT_CPU_FIELD_NUMBER: _ClassVar[int]
    RAM_FIELD_NUMBER: _ClassVar[int]
    CURRENT_RAM_FIELD_NUMBER: _ClassVar[int]
    STARTED_AT_FIELD_NUMBER: _ClassVar[int]
    TERMINATED_AT_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_AT_FIELD_NUMBER: _ClassVar[int]
    RESTARTED_AT_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    name: str
    text: TextData
    region: Region
    status: ResourceStatus
    current_status: ResourceStatus
    version: str
    current_version: str
    external_name: str
    external_id: str
    connection_uri: str
    client_ptr: NodeReferenceData
    cpu: float
    current_cpu: float
    ram: float
    current_ram: float
    started_at: _timestamp_pb2.Timestamp
    terminated_at: _timestamp_pb2.Timestamp
    active_at: _timestamp_pb2.Timestamp
    restarted_at: _timestamp_pb2.Timestamp
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        name: _Optional[str] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        region: _Optional[_Union[Region, str]] = ...,
        status: _Optional[_Union[ResourceStatus, str]] = ...,
        current_status: _Optional[_Union[ResourceStatus, str]] = ...,
        version: _Optional[str] = ...,
        current_version: _Optional[str] = ...,
        external_name: _Optional[str] = ...,
        external_id: _Optional[str] = ...,
        connection_uri: _Optional[str] = ...,
        client_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        cpu: _Optional[float] = ...,
        current_cpu: _Optional[float] = ...,
        ram: _Optional[float] = ...,
        current_ram: _Optional[float] = ...,
        started_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        terminated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        active_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        restarted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
    ) -> None: ...

class MembershipData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "bench_ptr",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "user_ptr",
        "is_owner",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    USER_PTR_FIELD_NUMBER: _ClassVar[int]
    IS_OWNER_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    user_ptr: NodeReferenceData
    is_owner: bool
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        user_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        is_owner: bool = ...,
    ) -> None: ...

class MessageData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "package_ptr",
        "bench_ptr",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "origin_ptr",
        "path",
        "reply_to_ptr",
        "title",
        "text",
        "value_packed",
        "is_pinned",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    ORIGIN_PTR_FIELD_NUMBER: _ClassVar[int]
    PATH_FIELD_NUMBER: _ClassVar[int]
    REPLY_TO_PTR_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    VALUE_PACKED_FIELD_NUMBER: _ClassVar[int]
    IS_PINNED_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    origin_ptr: NodeReferenceData
    path: PathData
    reply_to_ptr: NodeReferenceData
    title: str
    text: TextData
    value_packed: _struct_pb2.Struct
    is_pinned: bool
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        origin_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        path: _Optional[_Union[PathData, _Mapping]] = ...,
        reply_to_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        title: _Optional[str] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        value_packed: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
        is_pinned: bool = ...,
    ) -> None: ...

class BaseNodeData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "revision",
        "created_at",
        "updated_at",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    updated_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
    ) -> None: ...

class NotificationData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "package_ptr",
        "bench_ptr",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "kind",
        "type_ptr",
        "expires_at",
        "read_at",
        "title",
        "text",
        "value_packed",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    KIND_FIELD_NUMBER: _ClassVar[int]
    TYPE_PTR_FIELD_NUMBER: _ClassVar[int]
    EXPIRES_AT_FIELD_NUMBER: _ClassVar[int]
    READ_AT_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    VALUE_PACKED_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    kind: NotificationLevel
    type_ptr: NodeReferenceData
    expires_at: _timestamp_pb2.Timestamp
    read_at: _timestamp_pb2.Timestamp
    title: str
    text: TextData
    value_packed: _struct_pb2.Struct
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        kind: _Optional[_Union[NotificationLevel, str]] = ...,
        type_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        expires_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        read_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        title: _Optional[str] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        value_packed: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
    ) -> None: ...

class OrganizationData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "revision",
        "created_at",
        "updated_at",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "main_handle_ptr",
        "slug",
        "name",
        "text",
        "icon",
        "main_bench_ptr",
        "status",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    MAIN_HANDLE_PTR_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    MAIN_BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    updated_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    main_handle_ptr: NodeReferenceData
    slug: str
    name: str
    text: TextData
    icon: IconData
    main_bench_ptr: NodeReferenceData
    status: OrganizationStatus
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        main_handle_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        slug: _Optional[str] = ...,
        name: _Optional[str] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        icon: _Optional[_Union[IconData, _Mapping]] = ...,
        main_bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        status: _Optional[_Union[OrganizationStatus, str]] = ...,
    ) -> None: ...

class PackageData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "bench_ptr",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "text",
        "icon",
        "policies",
        "base_ptr",
        "is_snapshot",
        "is_overlay",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    POLICIES_FIELD_NUMBER: _ClassVar[int]
    BASE_PTR_FIELD_NUMBER: _ClassVar[int]
    IS_SNAPSHOT_FIELD_NUMBER: _ClassVar[int]
    IS_OVERLAY_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    text: TextData
    icon: IconData
    policies: _containers.RepeatedCompositeFieldContainer[PolicyData]
    base_ptr: NodeReferenceData
    is_snapshot: bool
    is_overlay: bool
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        icon: _Optional[_Union[IconData, _Mapping]] = ...,
        policies: _Optional[_Iterable[_Union[PolicyData, _Mapping]]] = ...,
        base_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        is_snapshot: bool = ...,
        is_overlay: bool = ...,
    ) -> None: ...

class PipeData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "ck",
        "parent_ptr",
        "package_ptr",
        "bench_ptr",
        "template_ptr",
        "templated_epoch",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "type",
        "name",
        "order_key",
        "source_ptr",
        "source_port",
        "target_ptr",
        "target_port",
        "filter_type",
        "line",
        "color",
        "is_hidden",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PORT_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    TARGET_PORT_FIELD_NUMBER: _ClassVar[int]
    FILTER_TYPE_FIELD_NUMBER: _ClassVar[int]
    LINE_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    IS_HIDDEN_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    templated_epoch: int
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    type: PipeType
    name: str
    order_key: str
    source_ptr: NodeReferenceData
    source_port: PortKeyData
    target_ptr: NodeReferenceData
    target_port: PortKeyData
    filter_type: PipeFilterType
    line: LineData
    color: ColorData
    is_hidden: bool
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        ck: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        templated_epoch: _Optional[int] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        type: _Optional[_Union[PipeType, str]] = ...,
        name: _Optional[str] = ...,
        order_key: _Optional[str] = ...,
        source_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        source_port: _Optional[_Union[PortKeyData, _Mapping]] = ...,
        target_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        target_port: _Optional[_Union[PortKeyData, _Mapping]] = ...,
        filter_type: _Optional[_Union[PipeFilterType, str]] = ...,
        line: _Optional[_Union[LineData, _Mapping]] = ...,
        color: _Optional[_Union[ColorData, _Mapping]] = ...,
        is_hidden: bool = ...,
    ) -> None: ...

class QueryData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "ck",
        "parent_ptr",
        "package_ptr",
        "bench_ptr",
        "template_ptr",
        "templated_epoch",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "name",
        "order_key",
        "read_type",
        "node_type",
        "base_ptr",
        "filter",
        "sort",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    READ_TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    BASE_PTR_FIELD_NUMBER: _ClassVar[int]
    FILTER_FIELD_NUMBER: _ClassVar[int]
    SORT_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    templated_epoch: int
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    name: str
    order_key: str
    read_type: ReadType
    node_type: NodeType
    base_ptr: NodeReferenceData
    filter: ExpressionData
    sort: _containers.RepeatedCompositeFieldContainer[ExpressionData]
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        ck: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        templated_epoch: _Optional[int] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        name: _Optional[str] = ...,
        order_key: _Optional[str] = ...,
        read_type: _Optional[_Union[ReadType, str]] = ...,
        node_type: _Optional[_Union[NodeType, str]] = ...,
        base_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        filter: _Optional[_Union[ExpressionData, _Mapping]] = ...,
        sort: _Optional[_Iterable[_Union[ExpressionData, _Mapping]]] = ...,
    ) -> None: ...

class RecordData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "package_ptr",
        "bench_ptr",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "value_packed",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    VALUE_PACKED_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    value_packed: _struct_pb2.Struct
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        value_packed: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
    ) -> None: ...

class RunData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "package_ptr",
        "bench_ptr",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "kind",
        "root_ptr",
        "code",
        "options",
        "status",
        "duration",
        "cached_duration",
        "attempts",
        "error",
        "scheduled_at",
        "scheduled_epoch",
        "started_at",
        "started_epoch",
        "killed_at",
        "halted_at",
        "halted_epoch",
        "terminated_at",
        "terminated_epoch",
        "inputs_packed",
        "outputs_packed",
        "value_packed",
        "logs",
        "spans",
        "events",
        "block_ptr",
        "step_ptr",
        "pipe_ptr",
        "view_ptr",
        "session_ptr",
        "run_ptr",
        "run_root_ptr",
        "client_ptr",
        "machine_ptr",
        "server_ptr",
        "user_ptr",
        "identity_ptr",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    KIND_FIELD_NUMBER: _ClassVar[int]
    ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CODE_FIELD_NUMBER: _ClassVar[int]
    OPTIONS_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    CACHED_DURATION_FIELD_NUMBER: _ClassVar[int]
    ATTEMPTS_FIELD_NUMBER: _ClassVar[int]
    ERROR_FIELD_NUMBER: _ClassVar[int]
    SCHEDULED_AT_FIELD_NUMBER: _ClassVar[int]
    SCHEDULED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    STARTED_AT_FIELD_NUMBER: _ClassVar[int]
    STARTED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    KILLED_AT_FIELD_NUMBER: _ClassVar[int]
    HALTED_AT_FIELD_NUMBER: _ClassVar[int]
    HALTED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    TERMINATED_AT_FIELD_NUMBER: _ClassVar[int]
    TERMINATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    INPUTS_PACKED_FIELD_NUMBER: _ClassVar[int]
    OUTPUTS_PACKED_FIELD_NUMBER: _ClassVar[int]
    VALUE_PACKED_FIELD_NUMBER: _ClassVar[int]
    LOGS_FIELD_NUMBER: _ClassVar[int]
    SPANS_FIELD_NUMBER: _ClassVar[int]
    EVENTS_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    STEP_PTR_FIELD_NUMBER: _ClassVar[int]
    PIPE_PTR_FIELD_NUMBER: _ClassVar[int]
    VIEW_PTR_FIELD_NUMBER: _ClassVar[int]
    SESSION_PTR_FIELD_NUMBER: _ClassVar[int]
    RUN_PTR_FIELD_NUMBER: _ClassVar[int]
    RUN_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CLIENT_PTR_FIELD_NUMBER: _ClassVar[int]
    MACHINE_PTR_FIELD_NUMBER: _ClassVar[int]
    SERVER_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_PTR_FIELD_NUMBER: _ClassVar[int]
    IDENTITY_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    kind: RunKind
    root_ptr: NodeReferenceData
    code: CodeData
    options: RunOptionsData
    status: RunStatus
    duration: float
    cached_duration: float
    attempts: _containers.RepeatedCompositeFieldContainer[RunAttemptData]
    error: RunErrorData
    scheduled_at: _timestamp_pb2.Timestamp
    scheduled_epoch: int
    started_at: _timestamp_pb2.Timestamp
    started_epoch: int
    killed_at: _timestamp_pb2.Timestamp
    halted_at: _timestamp_pb2.Timestamp
    halted_epoch: int
    terminated_at: _timestamp_pb2.Timestamp
    terminated_epoch: int
    inputs_packed: _struct_pb2.Struct
    outputs_packed: _struct_pb2.Struct
    value_packed: _struct_pb2.Struct
    logs: _containers.RepeatedCompositeFieldContainer[LogInfoData]
    spans: _containers.RepeatedCompositeFieldContainer[RunSpanData]
    events: _containers.RepeatedCompositeFieldContainer[RunEventData]
    block_ptr: NodeReferenceData
    step_ptr: NodeReferenceData
    pipe_ptr: NodeReferenceData
    view_ptr: NodeReferenceData
    session_ptr: NodeReferenceData
    run_ptr: NodeReferenceData
    run_root_ptr: NodeReferenceData
    client_ptr: NodeReferenceData
    machine_ptr: NodeReferenceData
    server_ptr: NodeReferenceData
    user_ptr: NodeReferenceData
    identity_ptr: NodeReferenceData
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        kind: _Optional[_Union[RunKind, str]] = ...,
        root_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        code: _Optional[_Union[CodeData, _Mapping]] = ...,
        options: _Optional[_Union[RunOptionsData, _Mapping]] = ...,
        status: _Optional[_Union[RunStatus, str]] = ...,
        duration: _Optional[float] = ...,
        cached_duration: _Optional[float] = ...,
        attempts: _Optional[_Iterable[_Union[RunAttemptData, _Mapping]]] = ...,
        error: _Optional[_Union[RunErrorData, _Mapping]] = ...,
        scheduled_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        scheduled_epoch: _Optional[int] = ...,
        started_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        started_epoch: _Optional[int] = ...,
        killed_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        halted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        halted_epoch: _Optional[int] = ...,
        terminated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        terminated_epoch: _Optional[int] = ...,
        inputs_packed: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
        outputs_packed: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
        value_packed: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
        logs: _Optional[_Iterable[_Union[LogInfoData, _Mapping]]] = ...,
        spans: _Optional[_Iterable[_Union[RunSpanData, _Mapping]]] = ...,
        events: _Optional[_Iterable[_Union[RunEventData, _Mapping]]] = ...,
        block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        step_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        pipe_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        view_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        session_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        run_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        run_root_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        client_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        machine_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        server_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        user_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        identity_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
    ) -> None: ...

class SecretData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "bench_ptr",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "title",
        "text",
        "region",
        "status",
        "current_status",
        "value_type",
        "value_packed",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    CURRENT_STATUS_FIELD_NUMBER: _ClassVar[int]
    VALUE_TYPE_FIELD_NUMBER: _ClassVar[int]
    VALUE_PACKED_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    title: str
    text: TextData
    region: Region
    status: ResourceStatus
    current_status: ResourceStatus
    value_type: TypeInfoData
    value_packed: _struct_pb2.Struct
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        title: _Optional[str] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        region: _Optional[_Union[Region, str]] = ...,
        status: _Optional[_Union[ResourceStatus, str]] = ...,
        current_status: _Optional[_Union[ResourceStatus, str]] = ...,
        value_type: _Optional[_Union[TypeInfoData, _Mapping]] = ...,
        value_packed: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
    ) -> None: ...

class ServerData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "bench_ptr",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "name",
        "text",
        "region",
        "status",
        "current_status",
        "version",
        "current_version",
        "min_cpu",
        "max_cpu",
        "min_ram",
        "max_ram",
        "active_at",
        "bumped_at",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    CURRENT_STATUS_FIELD_NUMBER: _ClassVar[int]
    VERSION_FIELD_NUMBER: _ClassVar[int]
    CURRENT_VERSION_FIELD_NUMBER: _ClassVar[int]
    MIN_CPU_FIELD_NUMBER: _ClassVar[int]
    MAX_CPU_FIELD_NUMBER: _ClassVar[int]
    MIN_RAM_FIELD_NUMBER: _ClassVar[int]
    MAX_RAM_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_AT_FIELD_NUMBER: _ClassVar[int]
    BUMPED_AT_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    name: str
    text: TextData
    region: Region
    status: ResourceStatus
    current_status: ResourceStatus
    version: str
    current_version: str
    min_cpu: float
    max_cpu: float
    min_ram: float
    max_ram: float
    active_at: _timestamp_pb2.Timestamp
    bumped_at: _timestamp_pb2.Timestamp
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        name: _Optional[str] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        region: _Optional[_Union[Region, str]] = ...,
        status: _Optional[_Union[ResourceStatus, str]] = ...,
        current_status: _Optional[_Union[ResourceStatus, str]] = ...,
        version: _Optional[str] = ...,
        current_version: _Optional[str] = ...,
        min_cpu: _Optional[float] = ...,
        max_cpu: _Optional[float] = ...,
        min_ram: _Optional[float] = ...,
        max_ram: _Optional[float] = ...,
        active_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        bumped_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
    ) -> None: ...

class SessionData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "package_ptr",
        "bench_ptr",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "status",
        "duration",
        "opened_at",
        "closed_at",
        "client_ptr",
        "server_ptr",
        "machine_ptr",
        "user_ptr",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    OPENED_AT_FIELD_NUMBER: _ClassVar[int]
    CLOSED_AT_FIELD_NUMBER: _ClassVar[int]
    CLIENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SERVER_PTR_FIELD_NUMBER: _ClassVar[int]
    MACHINE_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    status: SessionStatus
    duration: float
    opened_at: _timestamp_pb2.Timestamp
    closed_at: _timestamp_pb2.Timestamp
    client_ptr: NodeReferenceData
    server_ptr: NodeReferenceData
    machine_ptr: NodeReferenceData
    user_ptr: NodeReferenceData
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        status: _Optional[_Union[SessionStatus, str]] = ...,
        duration: _Optional[float] = ...,
        opened_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        closed_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        client_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        server_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        machine_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        user_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
    ) -> None: ...

class SignalData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "package_ptr",
        "bench_ptr",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "type_ptr",
        "value_packed",
        "block_ptr",
        "step_ptr",
        "pipe_ptr",
        "view_ptr",
        "session_ptr",
        "run_ptr",
        "run_root_ptr",
        "client_ptr",
        "machine_ptr",
        "server_ptr",
        "user_ptr",
        "identity_ptr",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    TYPE_PTR_FIELD_NUMBER: _ClassVar[int]
    VALUE_PACKED_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    STEP_PTR_FIELD_NUMBER: _ClassVar[int]
    PIPE_PTR_FIELD_NUMBER: _ClassVar[int]
    VIEW_PTR_FIELD_NUMBER: _ClassVar[int]
    SESSION_PTR_FIELD_NUMBER: _ClassVar[int]
    RUN_PTR_FIELD_NUMBER: _ClassVar[int]
    RUN_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CLIENT_PTR_FIELD_NUMBER: _ClassVar[int]
    MACHINE_PTR_FIELD_NUMBER: _ClassVar[int]
    SERVER_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_PTR_FIELD_NUMBER: _ClassVar[int]
    IDENTITY_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    type_ptr: NodeReferenceData
    value_packed: _struct_pb2.Struct
    block_ptr: NodeReferenceData
    step_ptr: NodeReferenceData
    pipe_ptr: NodeReferenceData
    view_ptr: NodeReferenceData
    session_ptr: NodeReferenceData
    run_ptr: NodeReferenceData
    run_root_ptr: NodeReferenceData
    client_ptr: NodeReferenceData
    machine_ptr: NodeReferenceData
    server_ptr: NodeReferenceData
    user_ptr: NodeReferenceData
    identity_ptr: NodeReferenceData
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        type_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        value_packed: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
        block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        step_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        pipe_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        view_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        session_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        run_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        run_root_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        client_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        machine_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        server_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        user_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        identity_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
    ) -> None: ...

class SkipData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "revision",
        "created_at",
        "updated_at",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "reference_ptr",
        "order_key",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    REFERENCE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    updated_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    reference_ptr: NodeReferenceData
    order_key: str
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        reference_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        order_key: _Optional[str] = ...,
    ) -> None: ...

class SpaceData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "ck",
        "parent_ptr",
        "package_ptr",
        "bench_ptr",
        "template_ptr",
        "templated_epoch",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "type",
        "name",
        "text",
        "order_key",
        "policies",
        "bar_position",
        "focus",
        "inspection_ptr",
        "base_ptr",
        "run_ptr",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    POLICIES_FIELD_NUMBER: _ClassVar[int]
    BAR_POSITION_FIELD_NUMBER: _ClassVar[int]
    FOCUS_FIELD_NUMBER: _ClassVar[int]
    INSPECTION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_PTR_FIELD_NUMBER: _ClassVar[int]
    RUN_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    templated_epoch: int
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    type: SpaceType
    name: str
    text: TextData
    order_key: str
    policies: _containers.RepeatedCompositeFieldContainer[PolicyData]
    bar_position: Anchor
    focus: SelectionData
    inspection_ptr: NodeReferenceData
    base_ptr: NodeReferenceData
    run_ptr: NodeReferenceData
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        ck: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        templated_epoch: _Optional[int] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        type: _Optional[_Union[SpaceType, str]] = ...,
        name: _Optional[str] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        order_key: _Optional[str] = ...,
        policies: _Optional[_Iterable[_Union[PolicyData, _Mapping]]] = ...,
        bar_position: _Optional[_Union[Anchor, str]] = ...,
        focus: _Optional[_Union[SelectionData, _Mapping]] = ...,
        inspection_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        base_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        run_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
    ) -> None: ...

class StepData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "ck",
        "parent_ptr",
        "package_ptr",
        "bench_ptr",
        "template_ptr",
        "templated_epoch",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "type",
        "name",
        "order_key",
        "text",
        "icon",
        "run_options",
        "value_type",
        "value_packed",
        "node_ptr",
        "code",
        "roles_ptr",
        "identity_ptr",
        "position",
        "size",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    RUN_OPTIONS_FIELD_NUMBER: _ClassVar[int]
    VALUE_TYPE_FIELD_NUMBER: _ClassVar[int]
    VALUE_PACKED_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    CODE_FIELD_NUMBER: _ClassVar[int]
    ROLES_PTR_FIELD_NUMBER: _ClassVar[int]
    IDENTITY_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    SIZE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    templated_epoch: int
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    type: StepType
    name: str
    order_key: str
    text: TextData
    icon: IconData
    run_options: RunOptionsData
    value_type: TypeInfoData
    value_packed: _struct_pb2.Struct
    node_ptr: NodeReferenceData
    code: CodeData
    roles_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    identity_ptr: NodeReferenceData
    position: Vector2Data
    size: BoxData
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        ck: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        templated_epoch: _Optional[int] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        type: _Optional[_Union[StepType, str]] = ...,
        name: _Optional[str] = ...,
        order_key: _Optional[str] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        icon: _Optional[_Union[IconData, _Mapping]] = ...,
        run_options: _Optional[_Union[RunOptionsData, _Mapping]] = ...,
        value_type: _Optional[_Union[TypeInfoData, _Mapping]] = ...,
        value_packed: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
        node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        code: _Optional[_Union[CodeData, _Mapping]] = ...,
        roles_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...,
        identity_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        position: _Optional[_Union[Vector2Data, _Mapping]] = ...,
        size: _Optional[_Union[BoxData, _Mapping]] = ...,
    ) -> None: ...

class StoreData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "bench_ptr",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "name",
        "text",
        "region",
        "status",
        "current_status",
        "version",
        "current_version",
        "external_name",
        "external_id",
        "connection_uri",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    CURRENT_STATUS_FIELD_NUMBER: _ClassVar[int]
    VERSION_FIELD_NUMBER: _ClassVar[int]
    CURRENT_VERSION_FIELD_NUMBER: _ClassVar[int]
    EXTERNAL_NAME_FIELD_NUMBER: _ClassVar[int]
    EXTERNAL_ID_FIELD_NUMBER: _ClassVar[int]
    CONNECTION_URI_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    name: str
    text: TextData
    region: Region
    status: ResourceStatus
    current_status: ResourceStatus
    version: str
    current_version: str
    external_name: str
    external_id: str
    connection_uri: str
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        name: _Optional[str] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        region: _Optional[_Union[Region, str]] = ...,
        status: _Optional[_Union[ResourceStatus, str]] = ...,
        current_status: _Optional[_Union[ResourceStatus, str]] = ...,
        version: _Optional[str] = ...,
        current_version: _Optional[str] = ...,
        external_name: _Optional[str] = ...,
        external_id: _Optional[str] = ...,
        connection_uri: _Optional[str] = ...,
    ) -> None: ...

class TriggerData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "ck",
        "parent_ptr",
        "package_ptr",
        "bench_ptr",
        "template_ptr",
        "templated_epoch",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "type",
        "name",
        "processed_epoch",
        "schedule",
        "signal_ptr",
        "condition",
        "is_paused",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    PROCESSED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    SCHEDULE_FIELD_NUMBER: _ClassVar[int]
    SIGNAL_PTR_FIELD_NUMBER: _ClassVar[int]
    CONDITION_FIELD_NUMBER: _ClassVar[int]
    IS_PAUSED_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    templated_epoch: int
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    type: TriggerType
    name: str
    processed_epoch: int
    schedule: ScheduleData
    signal_ptr: NodeReferenceData
    condition: ExpressionData
    is_paused: bool
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        ck: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        templated_epoch: _Optional[int] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        type: _Optional[_Union[TriggerType, str]] = ...,
        name: _Optional[str] = ...,
        processed_epoch: _Optional[int] = ...,
        schedule: _Optional[_Union[ScheduleData, _Mapping]] = ...,
        signal_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        condition: _Optional[_Union[ExpressionData, _Mapping]] = ...,
        is_paused: bool = ...,
    ) -> None: ...

class UserData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "revision",
        "created_at",
        "updated_at",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "main_handle_ptr",
        "slug",
        "name",
        "text",
        "email",
        "icon",
        "main_bench_ptr",
        "status",
        "password_salt",
        "password_hash",
        "last_logged_in_at",
        "is_staff",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    MAIN_HANDLE_PTR_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    EMAIL_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    MAIN_BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    PASSWORD_SALT_FIELD_NUMBER: _ClassVar[int]
    PASSWORD_HASH_FIELD_NUMBER: _ClassVar[int]
    LAST_LOGGED_IN_AT_FIELD_NUMBER: _ClassVar[int]
    IS_STAFF_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    updated_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    main_handle_ptr: NodeReferenceData
    slug: str
    name: str
    text: TextData
    email: str
    icon: IconData
    main_bench_ptr: NodeReferenceData
    status: UserStatus
    password_salt: bytes
    password_hash: bytes
    last_logged_in_at: _timestamp_pb2.Timestamp
    is_staff: bool
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        main_handle_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        slug: _Optional[str] = ...,
        name: _Optional[str] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        email: _Optional[str] = ...,
        icon: _Optional[_Union[IconData, _Mapping]] = ...,
        main_bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        status: _Optional[_Union[UserStatus, str]] = ...,
        password_salt: _Optional[bytes] = ...,
        password_hash: _Optional[bytes] = ...,
        last_logged_in_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        is_staff: bool = ...,
    ) -> None: ...

class VaultData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "parent_ptr",
        "bench_ptr",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "name",
        "text",
        "region",
        "status",
        "current_status",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    CURRENT_STATUS_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    name: str
    text: TextData
    region: Region
    status: ResourceStatus
    current_status: ResourceStatus
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        name: _Optional[str] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        region: _Optional[_Union[Region, str]] = ...,
        status: _Optional[_Union[ResourceStatus, str]] = ...,
        current_status: _Optional[_Union[ResourceStatus, str]] = ...,
    ) -> None: ...

class ViewData(_message.Message):
    __slots__ = (
        "metatype",
        "id",
        "ck",
        "parent_ptr",
        "package_ptr",
        "bench_ptr",
        "template_ptr",
        "templated_epoch",
        "revision",
        "created_at",
        "created_epoch",
        "updated_at",
        "updated_epoch",
        "deleted_at",
        "archived_at",
        "created_by_ptr",
        "updated_by_ptr",
        "set_properties",
        "type",
        "name",
        "title",
        "text",
        "order_key",
        "icon",
        "value_type",
        "value_packed",
        "node_ptr_node",
        "node_ptr_file",
        "node_ptr_secret",
        "variant",
        "font",
        "position",
        "size",
        "margin",
        "padding",
        "orientation",
        "alignment",
        "transform",
        "selection",
        "focus",
        "expansion",
        "is_hidden",
        "is_disabled",
        "is_input",
        "is_inline",
        "is_loading",
    )
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_EPOCH_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SET_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    VALUE_TYPE_FIELD_NUMBER: _ClassVar[int]
    VALUE_PACKED_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_NODE_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FILE_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_SECRET_FIELD_NUMBER: _ClassVar[int]
    VARIANT_FIELD_NUMBER: _ClassVar[int]
    FONT_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    SIZE_FIELD_NUMBER: _ClassVar[int]
    MARGIN_FIELD_NUMBER: _ClassVar[int]
    PADDING_FIELD_NUMBER: _ClassVar[int]
    ORIENTATION_FIELD_NUMBER: _ClassVar[int]
    ALIGNMENT_FIELD_NUMBER: _ClassVar[int]
    TRANSFORM_FIELD_NUMBER: _ClassVar[int]
    SELECTION_FIELD_NUMBER: _ClassVar[int]
    FOCUS_FIELD_NUMBER: _ClassVar[int]
    EXPANSION_FIELD_NUMBER: _ClassVar[int]
    IS_HIDDEN_FIELD_NUMBER: _ClassVar[int]
    IS_DISABLED_FIELD_NUMBER: _ClassVar[int]
    IS_INPUT_FIELD_NUMBER: _ClassVar[int]
    IS_INLINE_FIELD_NUMBER: _ClassVar[int]
    IS_LOADING_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    templated_epoch: int
    revision: int
    created_at: _timestamp_pb2.Timestamp
    created_epoch: int
    updated_at: _timestamp_pb2.Timestamp
    updated_epoch: int
    deleted_at: _timestamp_pb2.Timestamp
    archived_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_by_ptr: NodeReferenceData
    set_properties: _containers.RepeatedScalarFieldContainer[int]
    type: ViewType
    name: str
    title: str
    text: TextData
    order_key: str
    icon: IconData
    value_type: TypeInfoData
    value_packed: _struct_pb2.Struct
    node_ptr_node: NodeReferenceData
    node_ptr_file: FileReferenceData
    node_ptr_secret: SecretReferenceData
    variant: Variant
    font: FontData
    position: OffsetData
    size: BoxData
    margin: OffsetData
    padding: OffsetData
    orientation: Orientation
    alignment: Alignment
    transform: TransformData
    selection: SelectionData
    focus: SelectionData
    expansion: SelectionData
    is_hidden: bool
    is_disabled: bool
    is_input: bool
    is_inline: bool
    is_loading: bool
    def __init__(
        self,
        metatype: _Optional[_Union[ObjectType, str]] = ...,
        id: _Optional[str] = ...,
        ck: _Optional[str] = ...,
        parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        templated_epoch: _Optional[int] = ...,
        revision: _Optional[int] = ...,
        created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_epoch: _Optional[int] = ...,
        updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        updated_epoch: _Optional[int] = ...,
        deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...,
        created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        set_properties: _Optional[_Iterable[int]] = ...,
        type: _Optional[_Union[ViewType, str]] = ...,
        name: _Optional[str] = ...,
        title: _Optional[str] = ...,
        text: _Optional[_Union[TextData, _Mapping]] = ...,
        order_key: _Optional[str] = ...,
        icon: _Optional[_Union[IconData, _Mapping]] = ...,
        value_type: _Optional[_Union[TypeInfoData, _Mapping]] = ...,
        value_packed: _Optional[_Union[_struct_pb2.Struct, _Mapping]] = ...,
        node_ptr_node: _Optional[_Union[NodeReferenceData, _Mapping]] = ...,
        node_ptr_file: _Optional[_Union[FileReferenceData, _Mapping]] = ...,
        node_ptr_secret: _Optional[_Union[SecretReferenceData, _Mapping]] = ...,
        variant: _Optional[_Union[Variant, str]] = ...,
        font: _Optional[_Union[FontData, _Mapping]] = ...,
        position: _Optional[_Union[OffsetData, _Mapping]] = ...,
        size: _Optional[_Union[BoxData, _Mapping]] = ...,
        margin: _Optional[_Union[OffsetData, _Mapping]] = ...,
        padding: _Optional[_Union[OffsetData, _Mapping]] = ...,
        orientation: _Optional[_Union[Orientation, str]] = ...,
        alignment: _Optional[_Union[Alignment, str]] = ...,
        transform: _Optional[_Union[TransformData, _Mapping]] = ...,
        selection: _Optional[_Union[SelectionData, _Mapping]] = ...,
        focus: _Optional[_Union[SelectionData, _Mapping]] = ...,
        expansion: _Optional[_Union[SelectionData, _Mapping]] = ...,
        is_hidden: bool = ...,
        is_disabled: bool = ...,
        is_input: bool = ...,
        is_inline: bool = ...,
        is_loading: bool = ...,
    ) -> None: ...

class SomeNodeData(_message.Message):
    __slots__ = (
        "bench",
        "user",
        "organization",
        "handle",
        "client",
        "server",
        "store",
        "machine",
        "drive",
        "vault",
        "cache",
        "file",
        "secret",
        "membership",
        "invite",
        "branch",
        "package",
        "dependency",
        "space",
        "block",
        "trigger",
        "field",
        "query",
        "view",
        "step",
        "pipe",
        "badge",
        "message",
        "record",
        "session",
        "run",
        "signal",
        "log",
        "notification",
        "skip",
    )
    BENCH_FIELD_NUMBER: _ClassVar[int]
    USER_FIELD_NUMBER: _ClassVar[int]
    ORGANIZATION_FIELD_NUMBER: _ClassVar[int]
    HANDLE_FIELD_NUMBER: _ClassVar[int]
    CLIENT_FIELD_NUMBER: _ClassVar[int]
    SERVER_FIELD_NUMBER: _ClassVar[int]
    STORE_FIELD_NUMBER: _ClassVar[int]
    MACHINE_FIELD_NUMBER: _ClassVar[int]
    DRIVE_FIELD_NUMBER: _ClassVar[int]
    VAULT_FIELD_NUMBER: _ClassVar[int]
    CACHE_FIELD_NUMBER: _ClassVar[int]
    FILE_FIELD_NUMBER: _ClassVar[int]
    SECRET_FIELD_NUMBER: _ClassVar[int]
    MEMBERSHIP_FIELD_NUMBER: _ClassVar[int]
    INVITE_FIELD_NUMBER: _ClassVar[int]
    BRANCH_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_FIELD_NUMBER: _ClassVar[int]
    DEPENDENCY_FIELD_NUMBER: _ClassVar[int]
    SPACE_FIELD_NUMBER: _ClassVar[int]
    BLOCK_FIELD_NUMBER: _ClassVar[int]
    TRIGGER_FIELD_NUMBER: _ClassVar[int]
    FIELD_FIELD_NUMBER: _ClassVar[int]
    QUERY_FIELD_NUMBER: _ClassVar[int]
    VIEW_FIELD_NUMBER: _ClassVar[int]
    STEP_FIELD_NUMBER: _ClassVar[int]
    PIPE_FIELD_NUMBER: _ClassVar[int]
    BADGE_FIELD_NUMBER: _ClassVar[int]
    MESSAGE_FIELD_NUMBER: _ClassVar[int]
    RECORD_FIELD_NUMBER: _ClassVar[int]
    SESSION_FIELD_NUMBER: _ClassVar[int]
    RUN_FIELD_NUMBER: _ClassVar[int]
    SIGNAL_FIELD_NUMBER: _ClassVar[int]
    LOG_FIELD_NUMBER: _ClassVar[int]
    NOTIFICATION_FIELD_NUMBER: _ClassVar[int]
    SKIP_FIELD_NUMBER: _ClassVar[int]
    bench: BenchData
    user: UserData
    organization: OrganizationData
    handle: HandleData
    client: ClientData
    server: ServerData
    store: StoreData
    machine: MachineData
    drive: DriveData
    vault: VaultData
    cache: CacheData
    file: FileData
    secret: SecretData
    membership: MembershipData
    invite: InviteData
    branch: BranchData
    package: PackageData
    dependency: DependencyData
    space: SpaceData
    block: BlockData
    trigger: TriggerData
    field: FieldData
    query: QueryData
    view: ViewData
    step: StepData
    pipe: PipeData
    badge: BadgeData
    message: MessageData
    record: RecordData
    session: SessionData
    run: RunData
    signal: SignalData
    log: LogData
    notification: NotificationData
    skip: SkipData
    def __init__(
        self,
        bench: _Optional[_Union[BenchData, _Mapping]] = ...,
        user: _Optional[_Union[UserData, _Mapping]] = ...,
        organization: _Optional[_Union[OrganizationData, _Mapping]] = ...,
        handle: _Optional[_Union[HandleData, _Mapping]] = ...,
        client: _Optional[_Union[ClientData, _Mapping]] = ...,
        server: _Optional[_Union[ServerData, _Mapping]] = ...,
        store: _Optional[_Union[StoreData, _Mapping]] = ...,
        machine: _Optional[_Union[MachineData, _Mapping]] = ...,
        drive: _Optional[_Union[DriveData, _Mapping]] = ...,
        vault: _Optional[_Union[VaultData, _Mapping]] = ...,
        cache: _Optional[_Union[CacheData, _Mapping]] = ...,
        file: _Optional[_Union[FileData, _Mapping]] = ...,
        secret: _Optional[_Union[SecretData, _Mapping]] = ...,
        membership: _Optional[_Union[MembershipData, _Mapping]] = ...,
        invite: _Optional[_Union[InviteData, _Mapping]] = ...,
        branch: _Optional[_Union[BranchData, _Mapping]] = ...,
        package: _Optional[_Union[PackageData, _Mapping]] = ...,
        dependency: _Optional[_Union[DependencyData, _Mapping]] = ...,
        space: _Optional[_Union[SpaceData, _Mapping]] = ...,
        block: _Optional[_Union[BlockData, _Mapping]] = ...,
        trigger: _Optional[_Union[TriggerData, _Mapping]] = ...,
        field: _Optional[_Union[FieldData, _Mapping]] = ...,
        query: _Optional[_Union[QueryData, _Mapping]] = ...,
        view: _Optional[_Union[ViewData, _Mapping]] = ...,
        step: _Optional[_Union[StepData, _Mapping]] = ...,
        pipe: _Optional[_Union[PipeData, _Mapping]] = ...,
        badge: _Optional[_Union[BadgeData, _Mapping]] = ...,
        message: _Optional[_Union[MessageData, _Mapping]] = ...,
        record: _Optional[_Union[RecordData, _Mapping]] = ...,
        session: _Optional[_Union[SessionData, _Mapping]] = ...,
        run: _Optional[_Union[RunData, _Mapping]] = ...,
        signal: _Optional[_Union[SignalData, _Mapping]] = ...,
        log: _Optional[_Union[LogData, _Mapping]] = ...,
        notification: _Optional[_Union[NotificationData, _Mapping]] = ...,
        skip: _Optional[_Union[SkipData, _Mapping]] = ...,
    ) -> None: ...
