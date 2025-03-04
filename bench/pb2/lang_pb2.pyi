
# type: ignore
# ruff: noqa

from typing import TYPE_CHECKING, Union, AsyncIterator, Mapping



from google.protobuf import timestamp_pb2 as _timestamp_pb2
from google.protobuf import duration_pb2 as _duration_pb2
from google.protobuf import struct_pb2 as _struct_pb2
from google.type import datetime_pb2 as _datetime_pb2
from google.type import date_pb2 as _date_pb2
from google.type import timeofday_pb2 as _timeofday_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Iterable as _Iterable, Mapping as _Mapping, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class EnumType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ENUM_TYPE_UNSPECIFIED: _ClassVar[EnumType]
    ENUM_TYPE_ENUM_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_NODE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_STRUCT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_OBJECT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_BENCH_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_NODE_MODE: _ClassVar[EnumType]
    ENUM_TYPE_NODE_AREA: _ClassVar[EnumType]
    ENUM_TYPE_PROPERTY_REFERENCE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_EDIT_OPERATION_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_CHANGE_CATEGORY: _ClassVar[EnumType]
    ENUM_TYPE_PACKAGE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_CLOUD: _ClassVar[EnumType]
    ENUM_TYPE_REGION: _ClassVar[EnumType]
    ENUM_TYPE_REGION_ZONE: _ClassVar[EnumType]
    ENUM_TYPE_REGION_AREA: _ClassVar[EnumType]
    ENUM_TYPE_REGION_CONTINENT: _ClassVar[EnumType]
    ENUM_TYPE_BENCH_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_ACCESS_MODE: _ClassVar[EnumType]
    ENUM_TYPE_ACCESS_KIND: _ClassVar[EnumType]
    ENUM_TYPE_POLICY_EFFECT: _ClassVar[EnumType]
    ENUM_TYPE_MEMBERSHIP_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_INVITE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_QUERY_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_EDIT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_USE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_ACCESS_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_RESOURCE_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_RESOURCE_OCCUPANCY: _ClassVar[EnumType]
    ENUM_TYPE_SCALER_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_SCALER_STRATEGY: _ClassVar[EnumType]
    ENUM_TYPE_MACHINE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_BROWSER_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_STORE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_CLIENT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_FILE_RETENTION_MODE: _ClassVar[EnumType]
    ENUM_TYPE_FILE_KIND: _ClassVar[EnumType]
    ENUM_TYPE_FILE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_FILE_FORMAT: _ClassVar[EnumType]
    ENUM_TYPE_ICON_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_STREAM_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_PRIMITIVE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_FIELD_ZONE: _ClassVar[EnumType]
    ENUM_TYPE_TYPE_KIND: _ClassVar[EnumType]
    ENUM_TYPE_TYPE_FORMAT: _ClassVar[EnumType]
    ENUM_TYPE_BLOCK_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_DAY: _ClassVar[EnumType]
    ENUM_TYPE_MONTH: _ClassVar[EnumType]
    ENUM_TYPE_TIME_INTERVAL: _ClassVar[EnumType]
    ENUM_TYPE_TEXT_LINE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_TEXT_SPAN_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_EXPRESSION_KIND: _ClassVar[EnumType]
    ENUM_TYPE_EXPRESSION_OP: _ClassVar[EnumType]
    ENUM_TYPE_LITERAL_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_FUNCTIONAL_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_CONDITIONAL_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_AGGREGATION_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_SORT_MODE: _ClassVar[EnumType]
    ENUM_TYPE_SORT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_COMPUTED_VALUE_KIND: _ClassVar[EnumType]
    ENUM_TYPE_COMPUTED_VALUE_MODE: _ClassVar[EnumType]
    ENUM_TYPE_PATH_ELEMENT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_PATH_RUN_SELECTOR: _ClassVar[EnumType]
    ENUM_TYPE_SELECTION_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_RUN_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_RUN_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_RUN_SPAN_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_PLAN_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_PLAN_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_PLAN_TERMINATION_MODE: _ClassVar[EnumType]
    ENUM_TYPE_PLAN_FAILURE_MODE: _ClassVar[EnumType]
    ENUM_TYPE_TASK_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_TASK_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_SESSION_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_CACHE_MODE: _ClassVar[EnumType]
    ENUM_TYPE_LOG_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_SEVERITY: _ClassVar[EnumType]
    ENUM_TYPE_TRIGGER_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_TRIGGER_EFFECT: _ClassVar[EnumType]
    ENUM_TYPE_TRIGGER_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_SCHEDULE_FREQUENCY: _ClassVar[EnumType]
    ENUM_TYPE_ERROR_KIND: _ClassVar[EnumType]
    ENUM_TYPE_ERROR_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_BREAKPOINT_SITE: _ClassVar[EnumType]
    ENUM_TYPE_BREAKPOINT_ACTION: _ClassVar[EnumType]
    ENUM_TYPE_BREAKPOINT_TARGET: _ClassVar[EnumType]
    ENUM_TYPE_INTERRUPTION_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_INTERRUPTION_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_INTERRUPTION_RESPONSE: _ClassVar[EnumType]
    ENUM_TYPE_MODEL_DEVELOPER: _ClassVar[EnumType]
    ENUM_TYPE_MODEL_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_MODEL_FAMILY: _ClassVar[EnumType]
    ENUM_TYPE_MODEL_PROVIDER: _ClassVar[EnumType]
    ENUM_TYPE_CODE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_ACTION_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_ACTION_CATEGORY: _ClassVar[EnumType]
    ENUM_TYPE_PORT_SIDE: _ClassVar[EnumType]
    ENUM_TYPE_LINK_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_LINK_TRIGGER: _ClassVar[EnumType]
    ENUM_TYPE_SPACE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_VIEW_TYPE: _ClassVar[EnumType]
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
    ENUM_TYPE_SIDEBAR_ASPECT: _ClassVar[EnumType]
    ENUM_TYPE_CONTEXT_ASPECT: _ClassVar[EnumType]
    ENUM_TYPE_BUTTON_VARIANT: _ClassVar[EnumType]
    ENUM_TYPE_PICKER_VARIANT: _ClassVar[EnumType]
    ENUM_TYPE_USER_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_ORGANIZATION_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_CHANNEL_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_THREAD_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_THREAD_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_MESSAGE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_MESSAGE_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_MESSAGE_PLATFORM: _ClassVar[EnumType]
    ENUM_TYPE_NOTIFICATION_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_NOTIFICATION_STATUS: _ClassVar[EnumType]

class NodeType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    NODE_TYPE_UNSPECIFIED: _ClassVar[NodeType]
    NODE_TYPE_BENCH: _ClassVar[NodeType]
    NODE_TYPE_HANDLE: _ClassVar[NodeType]
    NODE_TYPE_USER: _ClassVar[NodeType]
    NODE_TYPE_ORGANIZATION: _ClassVar[NodeType]
    NODE_TYPE_TEAM: _ClassVar[NodeType]
    NODE_TYPE_CLIENT: _ClassVar[NodeType]
    NODE_TYPE_SCALER: _ClassVar[NodeType]
    NODE_TYPE_STORE: _ClassVar[NodeType]
    NODE_TYPE_MACHINE: _ClassVar[NodeType]
    NODE_TYPE_BROWSER: _ClassVar[NodeType]
    NODE_TYPE_FILE: _ClassVar[NodeType]
    NODE_TYPE_STREAM: _ClassVar[NodeType]
    NODE_TYPE_SECRET: _ClassVar[NodeType]
    NODE_TYPE_PACKAGE: _ClassVar[NodeType]
    NODE_TYPE_DEPENDENCY: _ClassVar[NodeType]
    NODE_TYPE_PAGE: _ClassVar[NodeType]
    NODE_TYPE_BLOCK: _ClassVar[NodeType]
    NODE_TYPE_CHOICE: _ClassVar[NodeType]
    NODE_TYPE_CLASS: _ClassVar[NodeType]
    NODE_TYPE_FIELD: _ClassVar[NodeType]
    NODE_TYPE_OPTION: _ClassVar[NodeType]
    NODE_TYPE_TAG: _ClassVar[NodeType]
    NODE_TYPE_FLOW: _ClassVar[NodeType]
    NODE_TYPE_ACTION: _ClassVar[NodeType]
    NODE_TYPE_LINK: _ClassVar[NodeType]
    NODE_TYPE_TRIGGER: _ClassVar[NodeType]
    NODE_TYPE_IMPLEMENTATION: _ClassVar[NodeType]
    NODE_TYPE_VIEW: _ClassVar[NodeType]
    NODE_TYPE_DATABASE: _ClassVar[NodeType]
    NODE_TYPE_CHANNEL: _ClassVar[NodeType]
    NODE_TYPE_ROLE: _ClassVar[NodeType]
    NODE_TYPE_SPACE: _ClassVar[NodeType]
    NODE_TYPE_THREAD: _ClassVar[NodeType]
    NODE_TYPE_MESSAGE: _ClassVar[NodeType]
    NODE_TYPE_NOTIFICATION: _ClassVar[NodeType]
    NODE_TYPE_RECORD: _ClassVar[NodeType]
    NODE_TYPE_MEMBERSHIP: _ClassVar[NodeType]
    NODE_TYPE_INVITE: _ClassVar[NodeType]
    NODE_TYPE_SESSION: _ClassVar[NodeType]
    NODE_TYPE_RUN: _ClassVar[NodeType]
    NODE_TYPE_RUN_SPAN: _ClassVar[NodeType]
    NODE_TYPE_INTERRUPTION: _ClassVar[NodeType]
    NODE_TYPE_LOG: _ClassVar[NodeType]
    NODE_TYPE_PLAN: _ClassVar[NodeType]
    NODE_TYPE_TASK: _ClassVar[NodeType]
    NODE_TYPE_SKIP: _ClassVar[NodeType]
    NODE_TYPE_EMPTY: _ClassVar[NodeType]

class StructType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    STRUCT_TYPE_UNSPECIFIED: _ClassVar[StructType]
    STRUCT_TYPE_CONTEXT: _ClassVar[StructType]
    STRUCT_TYPE_EDIT_CONTEXT: _ClassVar[StructType]
    STRUCT_TYPE_EDIT: _ClassVar[StructType]
    STRUCT_TYPE_EDIT_OPERATION: _ClassVar[StructType]
    STRUCT_TYPE_CHANGE: _ClassVar[StructType]
    STRUCT_TYPE_CHANGE_VIGNETTE: _ClassVar[StructType]
    STRUCT_TYPE_GRAPH_SCOPE: _ClassVar[StructType]
    STRUCT_TYPE_CLIENT_ORIGIN: _ClassVar[StructType]
    STRUCT_TYPE_NODE_REFERENCE: _ClassVar[StructType]
    STRUCT_TYPE_PROPERTY_REFERENCE: _ClassVar[StructType]
    STRUCT_TYPE_POLICY: _ClassVar[StructType]
    STRUCT_TYPE_POLICY_RULE: _ClassVar[StructType]
    STRUCT_TYPE_SUBJECT: _ClassVar[StructType]
    STRUCT_TYPE_ACCESS_ZONE: _ClassVar[StructType]
    STRUCT_TYPE_ACCESS_MATRIX: _ClassVar[StructType]
    STRUCT_TYPE_ACCESS: _ClassVar[StructType]
    STRUCT_TYPE_MACHINE_IMAGE: _ClassVar[StructType]
    STRUCT_TYPE_TYPE: _ClassVar[StructType]
    STRUCT_TYPE_TYPE_CONSTRAINT: _ClassVar[StructType]
    STRUCT_TYPE_FILE_INFO: _ClassVar[StructType]
    STRUCT_TYPE_ICON: _ClassVar[StructType]
    STRUCT_TYPE_SCHEDULE: _ClassVar[StructType]
    STRUCT_TYPE_TEXT: _ClassVar[StructType]
    STRUCT_TYPE_TEXT_LINE: _ClassVar[StructType]
    STRUCT_TYPE_TEXT_SPAN: _ClassVar[StructType]
    STRUCT_TYPE_TEXT_TABLE: _ClassVar[StructType]
    STRUCT_TYPE_TEXT_CELL: _ClassVar[StructType]
    STRUCT_TYPE_CODE: _ClassVar[StructType]
    STRUCT_TYPE_CODE_LINE: _ClassVar[StructType]
    STRUCT_TYPE_PATH: _ClassVar[StructType]
    STRUCT_TYPE_PATH_ELEMENT: _ClassVar[StructType]
    STRUCT_TYPE_EXPRESSION: _ClassVar[StructType]
    STRUCT_TYPE_AGGREGATION_RESULT: _ClassVar[StructType]
    STRUCT_TYPE_SELECTION: _ClassVar[StructType]
    STRUCT_TYPE_SELECT_OPTIONS: _ClassVar[StructType]
    STRUCT_TYPE_VALUE: _ClassVar[StructType]
    STRUCT_TYPE_COMPUTED_VALUE: _ClassVar[StructType]
    STRUCT_TYPE_ERROR: _ClassVar[StructType]
    STRUCT_TYPE_RUN_OPTIONS: _ClassVar[StructType]
    STRUCT_TYPE_RUN_TRACE: _ClassVar[StructType]
    STRUCT_TYPE_RUN_FRAME: _ClassVar[StructType]
    STRUCT_TYPE_BREAKPOINT: _ClassVar[StructType]
    STRUCT_TYPE_TEXT_OPTIONS: _ClassVar[StructType]
    STRUCT_TYPE_AUDIO_OPTIONS: _ClassVar[StructType]
    STRUCT_TYPE_IMAGE_OPTIONS: _ClassVar[StructType]
    STRUCT_TYPE_VIDEO_OPTIONS: _ClassVar[StructType]
    STRUCT_TYPE_COLOR: _ClassVar[StructType]
    STRUCT_TYPE_FONT: _ClassVar[StructType]
    STRUCT_TYPE_RECTANGLE: _ClassVar[StructType]
    STRUCT_TYPE_OFFSET: _ClassVar[StructType]
    STRUCT_TYPE_TRANSFORM: _ClassVar[StructType]
    STRUCT_TYPE_VECTOR2: _ClassVar[StructType]
    STRUCT_TYPE_VECTOR3: _ClassVar[StructType]
    STRUCT_TYPE_VECTOR4: _ClassVar[StructType]
    STRUCT_TYPE_LINE: _ClassVar[StructType]
    STRUCT_TYPE_RECTANGLE_CONSTRAINT: _ClassVar[StructType]
    STRUCT_TYPE_DOM_NODE: _ClassVar[StructType]

class ObjectType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    OBJECT_TYPE_UNSPECIFIED: _ClassVar[ObjectType]
    OBJECT_TYPE_BENCH: _ClassVar[ObjectType]
    OBJECT_TYPE_HANDLE: _ClassVar[ObjectType]
    OBJECT_TYPE_USER: _ClassVar[ObjectType]
    OBJECT_TYPE_ORGANIZATION: _ClassVar[ObjectType]
    OBJECT_TYPE_TEAM: _ClassVar[ObjectType]
    OBJECT_TYPE_CLIENT: _ClassVar[ObjectType]
    OBJECT_TYPE_SCALER: _ClassVar[ObjectType]
    OBJECT_TYPE_STORE: _ClassVar[ObjectType]
    OBJECT_TYPE_MACHINE: _ClassVar[ObjectType]
    OBJECT_TYPE_BROWSER: _ClassVar[ObjectType]
    OBJECT_TYPE_FILE: _ClassVar[ObjectType]
    OBJECT_TYPE_STREAM: _ClassVar[ObjectType]
    OBJECT_TYPE_SECRET: _ClassVar[ObjectType]
    OBJECT_TYPE_PACKAGE: _ClassVar[ObjectType]
    OBJECT_TYPE_DEPENDENCY: _ClassVar[ObjectType]
    OBJECT_TYPE_PAGE: _ClassVar[ObjectType]
    OBJECT_TYPE_BLOCK: _ClassVar[ObjectType]
    OBJECT_TYPE_CHOICE: _ClassVar[ObjectType]
    OBJECT_TYPE_CLASS: _ClassVar[ObjectType]
    OBJECT_TYPE_FIELD: _ClassVar[ObjectType]
    OBJECT_TYPE_OPTION: _ClassVar[ObjectType]
    OBJECT_TYPE_TAG: _ClassVar[ObjectType]
    OBJECT_TYPE_FLOW: _ClassVar[ObjectType]
    OBJECT_TYPE_ACTION: _ClassVar[ObjectType]
    OBJECT_TYPE_LINK: _ClassVar[ObjectType]
    OBJECT_TYPE_TRIGGER: _ClassVar[ObjectType]
    OBJECT_TYPE_IMPLEMENTATION: _ClassVar[ObjectType]
    OBJECT_TYPE_VIEW: _ClassVar[ObjectType]
    OBJECT_TYPE_DATABASE: _ClassVar[ObjectType]
    OBJECT_TYPE_CHANNEL: _ClassVar[ObjectType]
    OBJECT_TYPE_ROLE: _ClassVar[ObjectType]
    OBJECT_TYPE_SPACE: _ClassVar[ObjectType]
    OBJECT_TYPE_THREAD: _ClassVar[ObjectType]
    OBJECT_TYPE_MESSAGE: _ClassVar[ObjectType]
    OBJECT_TYPE_NOTIFICATION: _ClassVar[ObjectType]
    OBJECT_TYPE_RECORD: _ClassVar[ObjectType]
    OBJECT_TYPE_MEMBERSHIP: _ClassVar[ObjectType]
    OBJECT_TYPE_INVITE: _ClassVar[ObjectType]
    OBJECT_TYPE_SESSION: _ClassVar[ObjectType]
    OBJECT_TYPE_RUN: _ClassVar[ObjectType]
    OBJECT_TYPE_RUN_SPAN: _ClassVar[ObjectType]
    OBJECT_TYPE_INTERRUPTION: _ClassVar[ObjectType]
    OBJECT_TYPE_LOG: _ClassVar[ObjectType]
    OBJECT_TYPE_PLAN: _ClassVar[ObjectType]
    OBJECT_TYPE_TASK: _ClassVar[ObjectType]
    OBJECT_TYPE_SKIP: _ClassVar[ObjectType]
    OBJECT_TYPE_EMPTY: _ClassVar[ObjectType]
    OBJECT_TYPE_CONTEXT: _ClassVar[ObjectType]
    OBJECT_TYPE_EDIT_CONTEXT: _ClassVar[ObjectType]
    OBJECT_TYPE_EDIT: _ClassVar[ObjectType]
    OBJECT_TYPE_EDIT_OPERATION: _ClassVar[ObjectType]
    OBJECT_TYPE_CHANGE: _ClassVar[ObjectType]
    OBJECT_TYPE_CHANGE_VIGNETTE: _ClassVar[ObjectType]
    OBJECT_TYPE_GRAPH_SCOPE: _ClassVar[ObjectType]
    OBJECT_TYPE_CLIENT_ORIGIN: _ClassVar[ObjectType]
    OBJECT_TYPE_NODE_REFERENCE: _ClassVar[ObjectType]
    OBJECT_TYPE_PROPERTY_REFERENCE: _ClassVar[ObjectType]
    OBJECT_TYPE_POLICY: _ClassVar[ObjectType]
    OBJECT_TYPE_POLICY_RULE: _ClassVar[ObjectType]
    OBJECT_TYPE_SUBJECT: _ClassVar[ObjectType]
    OBJECT_TYPE_ACCESS_ZONE: _ClassVar[ObjectType]
    OBJECT_TYPE_ACCESS_MATRIX: _ClassVar[ObjectType]
    OBJECT_TYPE_ACCESS: _ClassVar[ObjectType]
    OBJECT_TYPE_MACHINE_IMAGE: _ClassVar[ObjectType]
    OBJECT_TYPE_TYPE: _ClassVar[ObjectType]
    OBJECT_TYPE_TYPE_CONSTRAINT: _ClassVar[ObjectType]
    OBJECT_TYPE_FILE_INFO: _ClassVar[ObjectType]
    OBJECT_TYPE_ICON: _ClassVar[ObjectType]
    OBJECT_TYPE_SCHEDULE: _ClassVar[ObjectType]
    OBJECT_TYPE_TEXT: _ClassVar[ObjectType]
    OBJECT_TYPE_TEXT_LINE: _ClassVar[ObjectType]
    OBJECT_TYPE_TEXT_SPAN: _ClassVar[ObjectType]
    OBJECT_TYPE_TEXT_TABLE: _ClassVar[ObjectType]
    OBJECT_TYPE_TEXT_CELL: _ClassVar[ObjectType]
    OBJECT_TYPE_CODE: _ClassVar[ObjectType]
    OBJECT_TYPE_CODE_LINE: _ClassVar[ObjectType]
    OBJECT_TYPE_PATH: _ClassVar[ObjectType]
    OBJECT_TYPE_PATH_ELEMENT: _ClassVar[ObjectType]
    OBJECT_TYPE_EXPRESSION: _ClassVar[ObjectType]
    OBJECT_TYPE_AGGREGATION_RESULT: _ClassVar[ObjectType]
    OBJECT_TYPE_SELECTION: _ClassVar[ObjectType]
    OBJECT_TYPE_SELECT_OPTIONS: _ClassVar[ObjectType]
    OBJECT_TYPE_VALUE: _ClassVar[ObjectType]
    OBJECT_TYPE_COMPUTED_VALUE: _ClassVar[ObjectType]
    OBJECT_TYPE_ERROR: _ClassVar[ObjectType]
    OBJECT_TYPE_RUN_OPTIONS: _ClassVar[ObjectType]
    OBJECT_TYPE_RUN_TRACE: _ClassVar[ObjectType]
    OBJECT_TYPE_RUN_FRAME: _ClassVar[ObjectType]
    OBJECT_TYPE_BREAKPOINT: _ClassVar[ObjectType]
    OBJECT_TYPE_TEXT_OPTIONS: _ClassVar[ObjectType]
    OBJECT_TYPE_AUDIO_OPTIONS: _ClassVar[ObjectType]
    OBJECT_TYPE_IMAGE_OPTIONS: _ClassVar[ObjectType]
    OBJECT_TYPE_VIDEO_OPTIONS: _ClassVar[ObjectType]
    OBJECT_TYPE_COLOR: _ClassVar[ObjectType]
    OBJECT_TYPE_FONT: _ClassVar[ObjectType]
    OBJECT_TYPE_RECTANGLE: _ClassVar[ObjectType]
    OBJECT_TYPE_OFFSET: _ClassVar[ObjectType]
    OBJECT_TYPE_TRANSFORM: _ClassVar[ObjectType]
    OBJECT_TYPE_VECTOR2: _ClassVar[ObjectType]
    OBJECT_TYPE_VECTOR3: _ClassVar[ObjectType]
    OBJECT_TYPE_VECTOR4: _ClassVar[ObjectType]
    OBJECT_TYPE_LINE: _ClassVar[ObjectType]
    OBJECT_TYPE_RECTANGLE_CONSTRAINT: _ClassVar[ObjectType]
    OBJECT_TYPE_DOM_NODE: _ClassVar[ObjectType]

class BenchType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    BENCH_TYPE_UNSPECIFIED: _ClassVar[BenchType]
    BENCH_TYPE_BENCH: _ClassVar[BenchType]
    BENCH_TYPE_HANDLE: _ClassVar[BenchType]
    BENCH_TYPE_USER: _ClassVar[BenchType]
    BENCH_TYPE_ORGANIZATION: _ClassVar[BenchType]
    BENCH_TYPE_TEAM: _ClassVar[BenchType]
    BENCH_TYPE_CLIENT: _ClassVar[BenchType]
    BENCH_TYPE_SCALER: _ClassVar[BenchType]
    BENCH_TYPE_STORE: _ClassVar[BenchType]
    BENCH_TYPE_MACHINE: _ClassVar[BenchType]
    BENCH_TYPE_BROWSER: _ClassVar[BenchType]
    BENCH_TYPE_FILE: _ClassVar[BenchType]
    BENCH_TYPE_STREAM: _ClassVar[BenchType]
    BENCH_TYPE_SECRET: _ClassVar[BenchType]
    BENCH_TYPE_PACKAGE: _ClassVar[BenchType]
    BENCH_TYPE_DEPENDENCY: _ClassVar[BenchType]
    BENCH_TYPE_PAGE: _ClassVar[BenchType]
    BENCH_TYPE_BLOCK: _ClassVar[BenchType]
    BENCH_TYPE_CHOICE: _ClassVar[BenchType]
    BENCH_TYPE_CLASS: _ClassVar[BenchType]
    BENCH_TYPE_FIELD: _ClassVar[BenchType]
    BENCH_TYPE_OPTION: _ClassVar[BenchType]
    BENCH_TYPE_TAG: _ClassVar[BenchType]
    BENCH_TYPE_FLOW: _ClassVar[BenchType]
    BENCH_TYPE_ACTION: _ClassVar[BenchType]
    BENCH_TYPE_LINK: _ClassVar[BenchType]
    BENCH_TYPE_TRIGGER: _ClassVar[BenchType]
    BENCH_TYPE_IMPLEMENTATION: _ClassVar[BenchType]
    BENCH_TYPE_VIEW: _ClassVar[BenchType]
    BENCH_TYPE_DATABASE: _ClassVar[BenchType]
    BENCH_TYPE_CHANNEL: _ClassVar[BenchType]
    BENCH_TYPE_ROLE: _ClassVar[BenchType]
    BENCH_TYPE_SPACE: _ClassVar[BenchType]
    BENCH_TYPE_THREAD: _ClassVar[BenchType]
    BENCH_TYPE_MESSAGE: _ClassVar[BenchType]
    BENCH_TYPE_NOTIFICATION: _ClassVar[BenchType]
    BENCH_TYPE_RECORD: _ClassVar[BenchType]
    BENCH_TYPE_MEMBERSHIP: _ClassVar[BenchType]
    BENCH_TYPE_INVITE: _ClassVar[BenchType]
    BENCH_TYPE_SESSION: _ClassVar[BenchType]
    BENCH_TYPE_RUN: _ClassVar[BenchType]
    BENCH_TYPE_RUN_SPAN: _ClassVar[BenchType]
    BENCH_TYPE_INTERRUPTION: _ClassVar[BenchType]
    BENCH_TYPE_LOG: _ClassVar[BenchType]
    BENCH_TYPE_PLAN: _ClassVar[BenchType]
    BENCH_TYPE_TASK: _ClassVar[BenchType]
    BENCH_TYPE_SKIP: _ClassVar[BenchType]
    BENCH_TYPE_EMPTY: _ClassVar[BenchType]
    BENCH_TYPE_CONTEXT: _ClassVar[BenchType]
    BENCH_TYPE_EDIT_CONTEXT: _ClassVar[BenchType]
    BENCH_TYPE_EDIT: _ClassVar[BenchType]
    BENCH_TYPE_EDIT_OPERATION: _ClassVar[BenchType]
    BENCH_TYPE_CHANGE: _ClassVar[BenchType]
    BENCH_TYPE_CHANGE_VIGNETTE: _ClassVar[BenchType]
    BENCH_TYPE_GRAPH_SCOPE: _ClassVar[BenchType]
    BENCH_TYPE_CLIENT_ORIGIN: _ClassVar[BenchType]
    BENCH_TYPE_NODE_REFERENCE: _ClassVar[BenchType]
    BENCH_TYPE_PROPERTY_REFERENCE: _ClassVar[BenchType]
    BENCH_TYPE_POLICY: _ClassVar[BenchType]
    BENCH_TYPE_POLICY_RULE: _ClassVar[BenchType]
    BENCH_TYPE_SUBJECT: _ClassVar[BenchType]
    BENCH_TYPE_ACCESS_ZONE: _ClassVar[BenchType]
    BENCH_TYPE_ACCESS_MATRIX: _ClassVar[BenchType]
    BENCH_TYPE_ACCESS: _ClassVar[BenchType]
    BENCH_TYPE_MACHINE_IMAGE: _ClassVar[BenchType]
    BENCH_TYPE_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_TYPE_CONSTRAINT: _ClassVar[BenchType]
    BENCH_TYPE_FILE_INFO: _ClassVar[BenchType]
    BENCH_TYPE_ICON: _ClassVar[BenchType]
    BENCH_TYPE_SCHEDULE: _ClassVar[BenchType]
    BENCH_TYPE_TEXT: _ClassVar[BenchType]
    BENCH_TYPE_TEXT_LINE: _ClassVar[BenchType]
    BENCH_TYPE_TEXT_SPAN: _ClassVar[BenchType]
    BENCH_TYPE_TEXT_TABLE: _ClassVar[BenchType]
    BENCH_TYPE_TEXT_CELL: _ClassVar[BenchType]
    BENCH_TYPE_CODE: _ClassVar[BenchType]
    BENCH_TYPE_CODE_LINE: _ClassVar[BenchType]
    BENCH_TYPE_PATH: _ClassVar[BenchType]
    BENCH_TYPE_PATH_ELEMENT: _ClassVar[BenchType]
    BENCH_TYPE_EXPRESSION: _ClassVar[BenchType]
    BENCH_TYPE_AGGREGATION_RESULT: _ClassVar[BenchType]
    BENCH_TYPE_SELECTION: _ClassVar[BenchType]
    BENCH_TYPE_SELECT_OPTIONS: _ClassVar[BenchType]
    BENCH_TYPE_VALUE: _ClassVar[BenchType]
    BENCH_TYPE_COMPUTED_VALUE: _ClassVar[BenchType]
    BENCH_TYPE_ERROR: _ClassVar[BenchType]
    BENCH_TYPE_RUN_OPTIONS: _ClassVar[BenchType]
    BENCH_TYPE_RUN_TRACE: _ClassVar[BenchType]
    BENCH_TYPE_RUN_FRAME: _ClassVar[BenchType]
    BENCH_TYPE_BREAKPOINT: _ClassVar[BenchType]
    BENCH_TYPE_TEXT_OPTIONS: _ClassVar[BenchType]
    BENCH_TYPE_AUDIO_OPTIONS: _ClassVar[BenchType]
    BENCH_TYPE_IMAGE_OPTIONS: _ClassVar[BenchType]
    BENCH_TYPE_VIDEO_OPTIONS: _ClassVar[BenchType]
    BENCH_TYPE_COLOR: _ClassVar[BenchType]
    BENCH_TYPE_FONT: _ClassVar[BenchType]
    BENCH_TYPE_RECTANGLE: _ClassVar[BenchType]
    BENCH_TYPE_OFFSET: _ClassVar[BenchType]
    BENCH_TYPE_TRANSFORM: _ClassVar[BenchType]
    BENCH_TYPE_VECTOR2: _ClassVar[BenchType]
    BENCH_TYPE_VECTOR3: _ClassVar[BenchType]
    BENCH_TYPE_VECTOR4: _ClassVar[BenchType]
    BENCH_TYPE_LINE: _ClassVar[BenchType]
    BENCH_TYPE_RECTANGLE_CONSTRAINT: _ClassVar[BenchType]
    BENCH_TYPE_DOM_NODE: _ClassVar[BenchType]
    BENCH_TYPE_ENUM_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_NODE_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_STRUCT_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_OBJECT_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_BENCH_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_NODE_MODE: _ClassVar[BenchType]
    BENCH_TYPE_NODE_AREA: _ClassVar[BenchType]
    BENCH_TYPE_PROPERTY_REFERENCE_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_EDIT_OPERATION_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_CHANGE_CATEGORY: _ClassVar[BenchType]
    BENCH_TYPE_PACKAGE_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_CLOUD: _ClassVar[BenchType]
    BENCH_TYPE_REGION: _ClassVar[BenchType]
    BENCH_TYPE_REGION_ZONE: _ClassVar[BenchType]
    BENCH_TYPE_REGION_AREA: _ClassVar[BenchType]
    BENCH_TYPE_REGION_CONTINENT: _ClassVar[BenchType]
    BENCH_TYPE_BENCH_STATUS: _ClassVar[BenchType]
    BENCH_TYPE_ACCESS_MODE: _ClassVar[BenchType]
    BENCH_TYPE_ACCESS_KIND: _ClassVar[BenchType]
    BENCH_TYPE_POLICY_EFFECT: _ClassVar[BenchType]
    BENCH_TYPE_MEMBERSHIP_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_INVITE_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_QUERY_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_EDIT_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_USE_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_ACCESS_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_RESOURCE_STATUS: _ClassVar[BenchType]
    BENCH_TYPE_RESOURCE_OCCUPANCY: _ClassVar[BenchType]
    BENCH_TYPE_SCALER_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_SCALER_STRATEGY: _ClassVar[BenchType]
    BENCH_TYPE_MACHINE_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_BROWSER_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_STORE_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_CLIENT_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_FILE_RETENTION_MODE: _ClassVar[BenchType]
    BENCH_TYPE_FILE_KIND: _ClassVar[BenchType]
    BENCH_TYPE_FILE_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_FILE_FORMAT: _ClassVar[BenchType]
    BENCH_TYPE_ICON_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_STREAM_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_PRIMITIVE_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_FIELD_ZONE: _ClassVar[BenchType]
    BENCH_TYPE_TYPE_KIND: _ClassVar[BenchType]
    BENCH_TYPE_TYPE_FORMAT: _ClassVar[BenchType]
    BENCH_TYPE_BLOCK_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_DAY: _ClassVar[BenchType]
    BENCH_TYPE_MONTH: _ClassVar[BenchType]
    BENCH_TYPE_TIME_INTERVAL: _ClassVar[BenchType]
    BENCH_TYPE_TEXT_LINE_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_TEXT_SPAN_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_EXPRESSION_KIND: _ClassVar[BenchType]
    BENCH_TYPE_EXPRESSION_OP: _ClassVar[BenchType]
    BENCH_TYPE_LITERAL_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_FUNCTIONAL_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_CONDITIONAL_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_AGGREGATION_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_SORT_MODE: _ClassVar[BenchType]
    BENCH_TYPE_SORT_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_COMPUTED_VALUE_KIND: _ClassVar[BenchType]
    BENCH_TYPE_COMPUTED_VALUE_MODE: _ClassVar[BenchType]
    BENCH_TYPE_PATH_ELEMENT_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_PATH_RUN_SELECTOR: _ClassVar[BenchType]
    BENCH_TYPE_SELECTION_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_RUN_STATUS: _ClassVar[BenchType]
    BENCH_TYPE_RUN_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_RUN_SPAN_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_PLAN_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_PLAN_STATUS: _ClassVar[BenchType]
    BENCH_TYPE_PLAN_TERMINATION_MODE: _ClassVar[BenchType]
    BENCH_TYPE_PLAN_FAILURE_MODE: _ClassVar[BenchType]
    BENCH_TYPE_TASK_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_TASK_STATUS: _ClassVar[BenchType]
    BENCH_TYPE_SESSION_STATUS: _ClassVar[BenchType]
    BENCH_TYPE_CACHE_MODE: _ClassVar[BenchType]
    BENCH_TYPE_LOG_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_SEVERITY: _ClassVar[BenchType]
    BENCH_TYPE_TRIGGER_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_TRIGGER_EFFECT: _ClassVar[BenchType]
    BENCH_TYPE_TRIGGER_STATUS: _ClassVar[BenchType]
    BENCH_TYPE_SCHEDULE_FREQUENCY: _ClassVar[BenchType]
    BENCH_TYPE_ERROR_KIND: _ClassVar[BenchType]
    BENCH_TYPE_ERROR_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_BREAKPOINT_SITE: _ClassVar[BenchType]
    BENCH_TYPE_BREAKPOINT_ACTION: _ClassVar[BenchType]
    BENCH_TYPE_BREAKPOINT_TARGET: _ClassVar[BenchType]
    BENCH_TYPE_INTERRUPTION_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_INTERRUPTION_STATUS: _ClassVar[BenchType]
    BENCH_TYPE_INTERRUPTION_RESPONSE: _ClassVar[BenchType]
    BENCH_TYPE_MODEL_DEVELOPER: _ClassVar[BenchType]
    BENCH_TYPE_MODEL_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_MODEL_FAMILY: _ClassVar[BenchType]
    BENCH_TYPE_MODEL_PROVIDER: _ClassVar[BenchType]
    BENCH_TYPE_CODE_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_ACTION_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_ACTION_CATEGORY: _ClassVar[BenchType]
    BENCH_TYPE_PORT_SIDE: _ClassVar[BenchType]
    BENCH_TYPE_LINK_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_LINK_TRIGGER: _ClassVar[BenchType]
    BENCH_TYPE_SPACE_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_VIEW_TYPE: _ClassVar[BenchType]
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
    BENCH_TYPE_SIDEBAR_ASPECT: _ClassVar[BenchType]
    BENCH_TYPE_CONTEXT_ASPECT: _ClassVar[BenchType]
    BENCH_TYPE_BUTTON_VARIANT: _ClassVar[BenchType]
    BENCH_TYPE_PICKER_VARIANT: _ClassVar[BenchType]
    BENCH_TYPE_USER_STATUS: _ClassVar[BenchType]
    BENCH_TYPE_ORGANIZATION_STATUS: _ClassVar[BenchType]
    BENCH_TYPE_CHANNEL_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_THREAD_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_THREAD_STATUS: _ClassVar[BenchType]
    BENCH_TYPE_MESSAGE_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_MESSAGE_STATUS: _ClassVar[BenchType]
    BENCH_TYPE_MESSAGE_PLATFORM: _ClassVar[BenchType]
    BENCH_TYPE_NOTIFICATION_TYPE: _ClassVar[BenchType]
    BENCH_TYPE_NOTIFICATION_STATUS: _ClassVar[BenchType]

class NodeMode(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    NODE_MODE_UNSPECIFIED: _ClassVar[NodeMode]
    NODE_MODE_BUILTIN: _ClassVar[NodeMode]
    NODE_MODE_PRODUCTION: _ClassVar[NodeMode]
    NODE_MODE_DEVELOPMENT: _ClassVar[NodeMode]
    NODE_MODE_TEST: _ClassVar[NodeMode]
    NODE_MODE_PREVIEW: _ClassVar[NodeMode]
    NODE_MODE_ARCHIVE: _ClassVar[NodeMode]

class NodeArea(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    NODE_AREA_UNSPECIFIED: _ClassVar[NodeArea]
    NODE_AREA_GLOBAL: _ClassVar[NodeArea]
    NODE_AREA_REGIONAL: _ClassVar[NodeArea]
    NODE_AREA_LOCAL: _ClassVar[NodeArea]

class PropertyReferenceType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PROPERTY_REFERENCE_TYPE_UNSPECIFIED: _ClassVar[PropertyReferenceType]
    PROPERTY_REFERENCE_TYPE_ID: _ClassVar[PropertyReferenceType]
    PROPERTY_REFERENCE_TYPE_CK: _ClassVar[PropertyReferenceType]
    PROPERTY_REFERENCE_TYPE_BENCH_ID: _ClassVar[PropertyReferenceType]
    PROPERTY_REFERENCE_TYPE_BASE_CK: _ClassVar[PropertyReferenceType]
    PROPERTY_REFERENCE_TYPE_BASE_BENCH_ID: _ClassVar[PropertyReferenceType]
    PROPERTY_REFERENCE_TYPE_NODE_TYPE: _ClassVar[PropertyReferenceType]

class EditOperationType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    EDIT_OPERATION_TYPE_UNSPECIFIED: _ClassVar[EditOperationType]
    EDIT_OPERATION_TYPE_SET: _ClassVar[EditOperationType]
    EDIT_OPERATION_TYPE_CLEAR: _ClassVar[EditOperationType]

class ChangeCategory(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CHANGE_CATEGORY_UNSPECIFIED: _ClassVar[ChangeCategory]
    CHANGE_CATEGORY_SPACE: _ClassVar[ChangeCategory]
    CHANGE_CATEGORY_RUNTIME: _ClassVar[ChangeCategory]

class PackageType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PACKAGE_TYPE_UNSPECIFIED: _ClassVar[PackageType]
    PACKAGE_TYPE_MAIN: _ClassVar[PackageType]
    PACKAGE_TYPE_SIDE: _ClassVar[PackageType]
    PACKAGE_TYPE_SNAPSHOT: _ClassVar[PackageType]

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

class RegionZone(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    REGION_ZONE_UNSPECIFIED: _ClassVar[RegionZone]

class RegionArea(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    REGION_AREA_UNSPECIFIED: _ClassVar[RegionArea]
    REGION_AREA_EUROPE_CENTRAL: _ClassVar[RegionArea]
    REGION_AREA_NORTH_AMERICA_EAST: _ClassVar[RegionArea]
    REGION_AREA_NORTH_AMERICA_WEST: _ClassVar[RegionArea]
    REGION_AREA_SOUTH_AMERICA_EAST: _ClassVar[RegionArea]
    REGION_AREA_MIDDLE_EAST_CENTRAL: _ClassVar[RegionArea]
    REGION_AREA_MIDDLE_EAST_WEST: _ClassVar[RegionArea]
    REGION_AREA_AFRICA_SOUTH: _ClassVar[RegionArea]
    REGION_AREA_ASIA_WEST: _ClassVar[RegionArea]
    REGION_AREA_ASIA_SOUTH: _ClassVar[RegionArea]
    REGION_AREA_ASIA_EAST: _ClassVar[RegionArea]
    REGION_AREA_AUSTRALIA_SOUTH: _ClassVar[RegionArea]

class RegionContinent(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    REGION_CONTINENT_UNSPECIFIED: _ClassVar[RegionContinent]
    REGION_CONTINENT_EUROPE: _ClassVar[RegionContinent]
    REGION_CONTINENT_NORTH_AMERICA: _ClassVar[RegionContinent]
    REGION_CONTINENT_SOUTH_AMERICA: _ClassVar[RegionContinent]
    REGION_CONTINENT_MIDDLE_EAST: _ClassVar[RegionContinent]
    REGION_CONTINENT_AFRICA: _ClassVar[RegionContinent]
    REGION_CONTINENT_ASIA: _ClassVar[RegionContinent]
    REGION_CONTINENT_AUSTRALIA: _ClassVar[RegionContinent]
    REGION_CONTINENT_PRIVATE: _ClassVar[RegionContinent]

class BenchStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    BENCH_STATUS_UNSPECIFIED: _ClassVar[BenchStatus]
    BENCH_STATUS_RESERVED: _ClassVar[BenchStatus]
    BENCH_STATUS_ACTIVATED: _ClassVar[BenchStatus]

class AccessMode(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ACCESS_MODE_UNSPECIFIED: _ClassVar[AccessMode]
    ACCESS_MODE_ADAPTIVE: _ClassVar[AccessMode]
    ACCESS_MODE_ATOMIC: _ClassVar[AccessMode]

class AccessKind(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ACCESS_KIND_UNSPECIFIED: _ClassVar[AccessKind]
    ACCESS_KIND_READ: _ClassVar[AccessKind]
    ACCESS_KIND_EDIT: _ClassVar[AccessKind]
    ACCESS_KIND_USE: _ClassVar[AccessKind]

class PolicyEffect(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    POLICY_EFFECT_UNSPECIFIED: _ClassVar[PolicyEffect]
    POLICY_EFFECT_ALLOW: _ClassVar[PolicyEffect]
    POLICY_EFFECT_DENY: _ClassVar[PolicyEffect]

class MembershipType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MEMBERSHIP_TYPE_UNSPECIFIED: _ClassVar[MembershipType]
    MEMBERSHIP_TYPE_BENCH: _ClassVar[MembershipType]
    MEMBERSHIP_TYPE_ORGANIZATION: _ClassVar[MembershipType]
    MEMBERSHIP_TYPE_TEAM: _ClassVar[MembershipType]
    MEMBERSHIP_TYPE_CHANNEL: _ClassVar[MembershipType]
    MEMBERSHIP_TYPE_THREAD: _ClassVar[MembershipType]

class InviteType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    INVITE_TYPE_UNSPECIFIED: _ClassVar[InviteType]
    INVITE_TYPE_BENCH: _ClassVar[InviteType]
    INVITE_TYPE_ORGANIZATION: _ClassVar[InviteType]
    INVITE_TYPE_TEAM: _ClassVar[InviteType]
    INVITE_TYPE_CHANNEL: _ClassVar[InviteType]
    INVITE_TYPE_THREAD: _ClassVar[InviteType]

class QueryType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    QUERY_TYPE_UNSPECIFIED: _ClassVar[QueryType]
    QUERY_TYPE_GET: _ClassVar[QueryType]
    QUERY_TYPE_SEARCH: _ClassVar[QueryType]
    QUERY_TYPE_AGGREGATE: _ClassVar[QueryType]

class EditType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    EDIT_TYPE_UNSPECIFIED: _ClassVar[EditType]
    EDIT_TYPE_CREATE: _ClassVar[EditType]
    EDIT_TYPE_UPSERT: _ClassVar[EditType]
    EDIT_TYPE_UPDATE: _ClassVar[EditType]
    EDIT_TYPE_MOVE: _ClassVar[EditType]
    EDIT_TYPE_DELETE: _ClassVar[EditType]
    EDIT_TYPE_RESTORE: _ClassVar[EditType]
    EDIT_TYPE_ERASE: _ClassVar[EditType]

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

class ResourceStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RESOURCE_STATUS_UNSPECIFIED: _ClassVar[ResourceStatus]
    RESOURCE_STATUS_DECLARED: _ClassVar[ResourceStatus]
    RESOURCE_STATUS_UP: _ClassVar[ResourceStatus]
    RESOURCE_STATUS_SLEEPING: _ClassVar[ResourceStatus]
    RESOURCE_STATUS_DOWN: _ClassVar[ResourceStatus]
    RESOURCE_STATUS_DEGRADED: _ClassVar[ResourceStatus]
    RESOURCE_STATUS_DECOMMISSIONED: _ClassVar[ResourceStatus]

class ResourceOccupancy(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RESOURCE_OCCUPANCY_UNSPECIFIED: _ClassVar[ResourceOccupancy]
    RESOURCE_OCCUPANCY_AVAILABLE: _ClassVar[ResourceOccupancy]
    RESOURCE_OCCUPANCY_RESERVED: _ClassVar[ResourceOccupancy]
    RESOURCE_OCCUPANCY_OCCUPIED: _ClassVar[ResourceOccupancy]
    RESOURCE_OCCUPANCY_DIRTY: _ClassVar[ResourceOccupancy]

class ScalerType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SCALER_TYPE_UNSPECIFIED: _ClassVar[ScalerType]
    SCALER_TYPE_MACHINE: _ClassVar[ScalerType]
    SCALER_TYPE_BROWSER: _ClassVar[ScalerType]

class ScalerStrategy(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SCALER_STRATEGY_UNSPECIFIED: _ClassVar[ScalerStrategy]
    SCALER_STRATEGY_MANUAL: _ClassVar[ScalerStrategy]
    SCALER_STRATEGY_AUTO: _ClassVar[ScalerStrategy]

class MachineType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MACHINE_TYPE_UNSPECIFIED: _ClassVar[MachineType]
    MACHINE_TYPE_RUNTIME: _ClassVar[MachineType]

class BrowserType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    BROWSER_TYPE_UNSPECIFIED: _ClassVar[BrowserType]
    BROWSER_TYPE_CHROMIUM: _ClassVar[BrowserType]

class StoreType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    STORE_TYPE_UNSPECIFIED: _ClassVar[StoreType]
    STORE_TYPE_POSTGRES: _ClassVar[StoreType]

class ClientType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CLIENT_TYPE_UNSPECIFIED: _ClassVar[ClientType]
    CLIENT_TYPE_WEB: _ClassVar[ClientType]
    CLIENT_TYPE_BROWSER_PLUGIN: _ClassVar[ClientType]
    CLIENT_TYPE_DESKTOP: _ClassVar[ClientType]
    CLIENT_TYPE_MOBILE: _ClassVar[ClientType]
    CLIENT_TYPE_MACHINE: _ClassVar[ClientType]

class FileRetentionMode(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FILE_RETENTION_MODE_UNSPECIFIED: _ClassVar[FileRetentionMode]
    FILE_RETENTION_MODE_AUTOMATIC: _ClassVar[FileRetentionMode]
    FILE_RETENTION_MODE_MANUAL: _ClassVar[FileRetentionMode]
    FILE_RETENTION_MODE_TIMED: _ClassVar[FileRetentionMode]

class FileKind(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FILE_KIND_UNSPECIFIED: _ClassVar[FileKind]
    FILE_KIND_DRIVE: _ClassVar[FileKind]
    FILE_KIND_DRIVE_INLINE: _ClassVar[FileKind]
    FILE_KIND_INLINE: _ClassVar[FileKind]
    FILE_KIND_EXTERNAL: _ClassVar[FileKind]

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

class IconType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ICON_TYPE_UNSPECIFIED: _ClassVar[IconType]
    ICON_TYPE_EMOJI: _ClassVar[IconType]
    ICON_TYPE_FONT_AWESOME: _ClassVar[IconType]
    ICON_TYPE_VS_CODE: _ClassVar[IconType]

class StreamType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    STREAM_TYPE_UNSPECIFIED: _ClassVar[StreamType]
    STREAM_TYPE_TEXT: _ClassVar[StreamType]
    STREAM_TYPE_CODE: _ClassVar[StreamType]
    STREAM_TYPE_IMAGE: _ClassVar[StreamType]
    STREAM_TYPE_AUDIO: _ClassVar[StreamType]
    STREAM_TYPE_VIDEO: _ClassVar[StreamType]

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
    PRIMITIVE_TYPE_DURATION: _ClassVar[PrimitiveType]

class FieldType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FIELD_TYPE_UNSPECIFIED: _ClassVar[FieldType]
    FIELD_TYPE_VARIABLE: _ClassVar[FieldType]
    FIELD_TYPE_MEMBER: _ClassVar[FieldType]
    FIELD_TYPE_INPUT: _ClassVar[FieldType]
    FIELD_TYPE_OUTPUT: _ClassVar[FieldType]

class TypeKind(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TYPE_KIND_UNSPECIFIED: _ClassVar[TypeKind]
    TYPE_KIND_PRIMITIVE: _ClassVar[TypeKind]
    TYPE_KIND_STRUCT: _ClassVar[TypeKind]
    TYPE_KIND_NODE: _ClassVar[TypeKind]
    TYPE_KIND_ENUM: _ClassVar[TypeKind]
    TYPE_KIND_BASED_NODE: _ClassVar[TypeKind]
    TYPE_KIND_CUSTOM_OBJECT: _ClassVar[TypeKind]
    TYPE_KIND_PARTIAL_OBJECT: _ClassVar[TypeKind]

class TypeFormat(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TYPE_FORMAT_UNSPECIFIED: _ClassVar[TypeFormat]
    TYPE_FORMAT_URL: _ClassVar[TypeFormat]
    TYPE_FORMAT_EMAIL: _ClassVar[TypeFormat]
    TYPE_FORMAT_EMOJI: _ClassVar[TypeFormat]
    TYPE_FORMAT_PHONE_NUMBER: _ClassVar[TypeFormat]
    TYPE_FORMAT_SLUG: _ClassVar[TypeFormat]

class BlockType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    BLOCK_TYPE_UNSPECIFIED: _ClassVar[BlockType]
    BLOCK_TYPE_FILE: _ClassVar[BlockType]
    BLOCK_TYPE_PAGE: _ClassVar[BlockType]
    BLOCK_TYPE_CHOICE: _ClassVar[BlockType]
    BLOCK_TYPE_CLASS: _ClassVar[BlockType]
    BLOCK_TYPE_TAG: _ClassVar[BlockType]
    BLOCK_TYPE_FLOW: _ClassVar[BlockType]
    BLOCK_TYPE_IMPLEMENTATION: _ClassVar[BlockType]
    BLOCK_TYPE_VIEW: _ClassVar[BlockType]
    BLOCK_TYPE_DATABASE: _ClassVar[BlockType]
    BLOCK_TYPE_CHANNEL: _ClassVar[BlockType]
    BLOCK_TYPE_ROLE: _ClassVar[BlockType]
    BLOCK_TYPE_PLAN: _ClassVar[BlockType]
    BLOCK_TYPE_TASK: _ClassVar[BlockType]
    BLOCK_TYPE_PARAGRAPH: _ClassVar[BlockType]
    BLOCK_TYPE_HEADING_1: _ClassVar[BlockType]
    BLOCK_TYPE_HEADING_2: _ClassVar[BlockType]
    BLOCK_TYPE_HEADING_3: _ClassVar[BlockType]
    BLOCK_TYPE_HEADING_4: _ClassVar[BlockType]
    BLOCK_TYPE_CALLOUT: _ClassVar[BlockType]
    BLOCK_TYPE_QUOTE: _ClassVar[BlockType]
    BLOCK_TYPE_LIST_UNORDERED: _ClassVar[BlockType]
    BLOCK_TYPE_LIST_ORDERED: _ClassVar[BlockType]
    BLOCK_TYPE_DIVIDER: _ClassVar[BlockType]
    BLOCK_TYPE_TABLE: _ClassVar[BlockType]
    BLOCK_TYPE_TABLE_ROW: _ClassVar[BlockType]
    BLOCK_TYPE_CODE: _ClassVar[BlockType]

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

class TextLineType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TEXT_LINE_TYPE_UNSPECIFIED: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_PARAGRAPH: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_HEADING_1: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_HEADING_2: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_HEADING_3: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_HEADING_4: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_CALLOUT: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_QUOTE: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_LIST_UNORDERED: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_LIST_ORDERED: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_DIVIDER: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_TABLE: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_TABLE_ROW: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_CODE: _ClassVar[TextLineType]

class TextSpanType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TEXT_SPAN_TYPE_UNSPECIFIED: _ClassVar[TextSpanType]
    TEXT_SPAN_TYPE_TEXT: _ClassVar[TextSpanType]
    TEXT_SPAN_TYPE_HARD_BREAK: _ClassVar[TextSpanType]
    TEXT_SPAN_TYPE_MENTION: _ClassVar[TextSpanType]
    TEXT_SPAN_TYPE_LINK: _ClassVar[TextSpanType]
    TEXT_SPAN_TYPE_CITATION: _ClassVar[TextSpanType]
    TEXT_SPAN_TYPE_EQUATION: _ClassVar[TextSpanType]

class ExpressionKind(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    EXPRESSION_KIND_UNSPECIFIED: _ClassVar[ExpressionKind]
    EXPRESSION_KIND_LITERAL: _ClassVar[ExpressionKind]
    EXPRESSION_KIND_FUNCTIONAL: _ClassVar[ExpressionKind]
    EXPRESSION_KIND_CONDITIONAL: _ClassVar[ExpressionKind]
    EXPRESSION_KIND_SORT: _ClassVar[ExpressionKind]
    EXPRESSION_KIND_AGGREGATION: _ClassVar[ExpressionKind]

class ExpressionType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    EXPRESSION_TYPE_UNSPECIFIED: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_VALUE: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_NONE: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_TRUE: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_FALSE: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_ADD: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_SUBTRACT: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_MULTIPLY: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_DIVIDE: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_MODULO: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_POWER: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_NOT: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_AND: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_OR: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_EQUALS: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_NOT_EQUALS: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_GREATER_THAN: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_GREATER_THAN_OR_EQUALS: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_LESS_THAN: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_LESS_THAN_OR_EQUALS: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_MATCHES: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_STARTS_WITH: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_ENDS_WITH: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_MATCHES_REGEX: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_CONTAINS: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_NOT_CONTAINS: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_IN: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_NOT_IN: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_EXISTS: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_NOT_EXISTS: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_NEAR: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_EXISTENCE: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_COUNT: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_SUM: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_MIN: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_MAX: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_AVERAGE: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_MEDIAN: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_HISTOGRAM: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_ASCENDING: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_DESCENDING: _ClassVar[ExpressionType]

class LiteralType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    LITERAL_TYPE_UNSPECIFIED: _ClassVar[LiteralType]
    LITERAL_TYPE_VALUE: _ClassVar[LiteralType]
    LITERAL_TYPE_NONE: _ClassVar[LiteralType]
    LITERAL_TYPE_TRUE: _ClassVar[LiteralType]
    LITERAL_TYPE_FALSE: _ClassVar[LiteralType]

class FunctionalType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FUNCTIONAL_TYPE_UNSPECIFIED: _ClassVar[FunctionalType]
    FUNCTIONAL_TYPE_ADD: _ClassVar[FunctionalType]
    FUNCTIONAL_TYPE_SUBTRACT: _ClassVar[FunctionalType]
    FUNCTIONAL_TYPE_MULTIPLY: _ClassVar[FunctionalType]
    FUNCTIONAL_TYPE_DIVIDE: _ClassVar[FunctionalType]
    FUNCTIONAL_TYPE_MODULO: _ClassVar[FunctionalType]
    FUNCTIONAL_TYPE_POWER: _ClassVar[FunctionalType]

class ConditionalType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CONDITIONAL_TYPE_UNSPECIFIED: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_NOT: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_AND: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_OR: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_EQUALS: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_NOT_EQUALS: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_GREATER_THAN: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_GREATER_THAN_OR_EQUALS: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_LESS_THAN: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_LESS_THAN_OR_EQUALS: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_MATCHES: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_STARTS_WITH: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_ENDS_WITH: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_MATCHES_REGEX: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_CONTAINS: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_NOT_CONTAINS: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_IN: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_NOT_IN: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_EXISTS: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_NOT_EXISTS: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_NEAR: _ClassVar[ConditionalType]

class AggregationType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    AGGREGATION_TYPE_UNSPECIFIED: _ClassVar[AggregationType]
    AGGREGATION_TYPE_EXISTENCE: _ClassVar[AggregationType]
    AGGREGATION_TYPE_COUNT: _ClassVar[AggregationType]
    AGGREGATION_TYPE_SUM: _ClassVar[AggregationType]
    AGGREGATION_TYPE_MIN: _ClassVar[AggregationType]
    AGGREGATION_TYPE_MAX: _ClassVar[AggregationType]
    AGGREGATION_TYPE_AVERAGE: _ClassVar[AggregationType]
    AGGREGATION_TYPE_MEDIAN: _ClassVar[AggregationType]
    AGGREGATION_TYPE_HISTOGRAM: _ClassVar[AggregationType]

class SortMode(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SORT_MODE_UNSPECIFIED: _ClassVar[SortMode]
    SORT_MODE_MAX: _ClassVar[SortMode]
    SORT_MODE_MIN: _ClassVar[SortMode]
    SORT_MODE_AVERAGE: _ClassVar[SortMode]
    SORT_MODE_SUM: _ClassVar[SortMode]
    SORT_MODE_MEDIAN: _ClassVar[SortMode]

class SortType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SORT_TYPE_UNSPECIFIED: _ClassVar[SortType]
    SORT_TYPE_ASCENDING: _ClassVar[SortType]
    SORT_TYPE_DESCENDING: _ClassVar[SortType]

class ComputedValueKind(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    COMPUTED_VALUE_KIND_UNSPECIFIED: _ClassVar[ComputedValueKind]
    COMPUTED_VALUE_KIND_PATH: _ClassVar[ComputedValueKind]
    COMPUTED_VALUE_KIND_EXPRESSION: _ClassVar[ComputedValueKind]
    COMPUTED_VALUE_KIND_CODE: _ClassVar[ComputedValueKind]

class ComputedValueMode(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    COMPUTED_VALUE_MODE_UNSPECIFIED: _ClassVar[ComputedValueMode]
    COMPUTED_VALUE_MODE_ALWAYS: _ClassVar[ComputedValueMode]
    COMPUTED_VALUE_MODE_IF_SOURCE_SET: _ClassVar[ComputedValueMode]
    COMPUTED_VALUE_MODE_IF_TARGET_UNSET: _ClassVar[ComputedValueMode]

class PathElementType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PATH_ELEMENT_TYPE_UNSPECIFIED: _ClassVar[PathElementType]
    PATH_ELEMENT_TYPE_ROOT: _ClassVar[PathElementType]
    PATH_ELEMENT_TYPE_BENCH: _ClassVar[PathElementType]
    PATH_ELEMENT_TYPE_PACKAGE: _ClassVar[PathElementType]
    PATH_ELEMENT_TYPE_NODE: _ClassVar[PathElementType]
    PATH_ELEMENT_TYPE_CURRENT: _ClassVar[PathElementType]
    PATH_ELEMENT_TYPE_CONTAINER: _ClassVar[PathElementType]
    PATH_ELEMENT_TYPE_CLOSEST: _ClassVar[PathElementType]
    PATH_ELEMENT_TYPE_PARENT: _ClassVar[PathElementType]
    PATH_ELEMENT_TYPE_CHILD: _ClassVar[PathElementType]
    PATH_ELEMENT_TYPE_ATTRIBUTE: _ClassVar[PathElementType]
    PATH_ELEMENT_TYPE_CONTEXT: _ClassVar[PathElementType]
    PATH_ELEMENT_TYPE_RUN: _ClassVar[PathElementType]

class PathRunSelector(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PATH_RUN_SELECTOR_UNSPECIFIED: _ClassVar[PathRunSelector]
    PATH_RUN_SELECTOR_LATEST: _ClassVar[PathRunSelector]

class SelectionType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SELECTION_TYPE_UNSPECIFIED: _ClassVar[SelectionType]
    SELECTION_TYPE_LIST: _ClassVar[SelectionType]
    SELECTION_TYPE_RANGE: _ClassVar[SelectionType]
    SELECTION_TYPE_SCOPE: _ClassVar[SelectionType]
    SELECTION_TYPE_TAG: _ClassVar[SelectionType]
    SELECTION_TYPE_COMBINATION: _ClassVar[SelectionType]

class RunStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RUN_STATUS_UNSPECIFIED: _ClassVar[RunStatus]
    RUN_STATUS_CREATED: _ClassVar[RunStatus]
    RUN_STATUS_SCHEDULED: _ClassVar[RunStatus]
    RUN_STATUS_QUEUED: _ClassVar[RunStatus]
    RUN_STATUS_RUNNING: _ClassVar[RunStatus]
    RUN_STATUS_PAUSED: _ClassVar[RunStatus]
    RUN_STATUS_YIELDED: _ClassVar[RunStatus]
    RUN_STATUS_WAITING: _ClassVar[RunStatus]
    RUN_STATUS_CANCELLED: _ClassVar[RunStatus]
    RUN_STATUS_ABORTED: _ClassVar[RunStatus]
    RUN_STATUS_FAILED: _ClassVar[RunStatus]
    RUN_STATUS_COMPLETED: _ClassVar[RunStatus]

class RunType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RUN_TYPE_UNSPECIFIED: _ClassVar[RunType]
    RUN_TYPE_CODE: _ClassVar[RunType]
    RUN_TYPE_ACTION: _ClassVar[RunType]
    RUN_TYPE_FLOW: _ClassVar[RunType]
    RUN_TYPE_LINK: _ClassVar[RunType]

class RunSpanType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RUN_SPAN_TYPE_UNSPECIFIED: _ClassVar[RunSpanType]
    RUN_SPAN_TYPE_ATTEMPT: _ClassVar[RunSpanType]
    RUN_SPAN_TYPE_WAIT: _ClassVar[RunSpanType]
    RUN_SPAN_TYPE_ACQUIRE: _ClassVar[RunSpanType]
    RUN_SPAN_TYPE_DELEGATE: _ClassVar[RunSpanType]
    RUN_SPAN_TYPE_FLOW_PLAN: _ClassVar[RunSpanType]
    RUN_SPAN_TYPE_MODEL_PREPARE: _ClassVar[RunSpanType]
    RUN_SPAN_TYPE_MODEL_GENERATE: _ClassVar[RunSpanType]
    RUN_SPAN_TYPE_MODEL_PARSE: _ClassVar[RunSpanType]
    RUN_SPAN_TYPE_FILE_UPLOAD: _ClassVar[RunSpanType]
    RUN_SPAN_TYPE_FILE_PREPARE_UPLOAD: _ClassVar[RunSpanType]
    RUN_SPAN_TYPE_FILE_DOWNLOAD: _ClassVar[RunSpanType]
    RUN_SPAN_TYPE_FILE_PREPARE_DOWNLOAD: _ClassVar[RunSpanType]

class PlanType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PLAN_TYPE_UNSPECIFIED: _ClassVar[PlanType]
    PLAN_TYPE_SERIAL: _ClassVar[PlanType]
    PLAN_TYPE_PARALLEL: _ClassVar[PlanType]

class PlanStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PLAN_STATUS_UNSPECIFIED: _ClassVar[PlanStatus]
    PLAN_STATUS_CREATED: _ClassVar[PlanStatus]
    PLAN_STATUS_RUNNING: _ClassVar[PlanStatus]
    PLAN_STATUS_CANCELLED: _ClassVar[PlanStatus]
    PLAN_STATUS_ABORTED: _ClassVar[PlanStatus]
    PLAN_STATUS_FAILED: _ClassVar[PlanStatus]
    PLAN_STATUS_COMPLETED: _ClassVar[PlanStatus]

class PlanTerminationMode(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PLAN_TERMINATION_MODE_UNSPECIFIED: _ClassVar[PlanTerminationMode]
    PLAN_TERMINATION_MODE_PASS: _ClassVar[PlanTerminationMode]
    PLAN_TERMINATION_MODE_RETURN: _ClassVar[PlanTerminationMode]

class PlanFailureMode(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PLAN_FAILURE_MODE_UNSPECIFIED: _ClassVar[PlanFailureMode]
    PLAN_FAILURE_MODE_FAIL: _ClassVar[PlanFailureMode]
    PLAN_FAILURE_MODE_COMPLETE: _ClassVar[PlanFailureMode]
    PLAN_FAILURE_MODE_CONTINUE: _ClassVar[PlanFailureMode]

class TaskType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TASK_TYPE_UNSPECIFIED: _ClassVar[TaskType]
    TASK_TYPE_GENERIC: _ClassVar[TaskType]
    TASK_TYPE_RUN: _ClassVar[TaskType]

class TaskStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TASK_STATUS_UNSPECIFIED: _ClassVar[TaskStatus]
    TASK_STATUS_CREATED: _ClassVar[TaskStatus]
    TASK_STATUS_ASSIGNED: _ClassVar[TaskStatus]
    TASK_STATUS_RUNNING: _ClassVar[TaskStatus]
    TASK_STATUS_WAITING: _ClassVar[TaskStatus]
    TASK_STATUS_REVIEWING: _ClassVar[TaskStatus]
    TASK_STATUS_CANCELLED: _ClassVar[TaskStatus]
    TASK_STATUS_ABORTED: _ClassVar[TaskStatus]
    TASK_STATUS_FAILED: _ClassVar[TaskStatus]
    TASK_STATUS_COMPLETED: _ClassVar[TaskStatus]

class SessionStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SESSION_STATUS_UNSPECIFIED: _ClassVar[SessionStatus]
    SESSION_STATUS_PENDING: _ClassVar[SessionStatus]
    SESSION_STATUS_OPEN: _ClassVar[SessionStatus]
    SESSION_STATUS_CLOSED: _ClassVar[SessionStatus]

class CacheMode(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CACHE_MODE_UNSPECIFIED: _ClassVar[CacheMode]
    CACHE_MODE_NEVER: _ClassVar[CacheMode]
    CACHE_MODE_ALWAYS: _ClassVar[CacheMode]

class LogType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    LOG_TYPE_UNSPECIFIED: _ClassVar[LogType]
    LOG_TYPE_PRINT: _ClassVar[LogType]
    LOG_TYPE_CHANGE: _ClassVar[LogType]
    LOG_TYPE_EDIT: _ClassVar[LogType]

class Severity(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SEVERITY_UNSPECIFIED: _ClassVar[Severity]
    SEVERITY_TRACE: _ClassVar[Severity]
    SEVERITY_DEBUG: _ClassVar[Severity]
    SEVERITY_INFO: _ClassVar[Severity]
    SEVERITY_WARNING: _ClassVar[Severity]
    SEVERITY_ERROR: _ClassVar[Severity]
    SEVERITY_PANIC: _ClassVar[Severity]

class TriggerType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TRIGGER_TYPE_UNSPECIFIED: _ClassVar[TriggerType]
    TRIGGER_TYPE_SCHEDULE: _ClassVar[TriggerType]
    TRIGGER_TYPE_MESSAGE: _ClassVar[TriggerType]

class TriggerEffect(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TRIGGER_EFFECT_UNSPECIFIED: _ClassVar[TriggerEffect]
    TRIGGER_EFFECT_START_RUN: _ClassVar[TriggerEffect]
    TRIGGER_EFFECT_ENSURE_RUN: _ClassVar[TriggerEffect]
    TRIGGER_EFFECT_REPLACE_RUN: _ClassVar[TriggerEffect]

class TriggerStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TRIGGER_STATUS_UNSPECIFIED: _ClassVar[TriggerStatus]
    TRIGGER_STATUS_INACTIVE: _ClassVar[TriggerStatus]
    TRIGGER_STATUS_OPEN: _ClassVar[TriggerStatus]
    TRIGGER_STATUS_CLOSED: _ClassVar[TriggerStatus]

class ScheduleFrequency(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SCHEDULE_FREQUENCY_UNSPECIFIED: _ClassVar[ScheduleFrequency]
    SCHEDULE_FREQUENCY_YEAR: _ClassVar[ScheduleFrequency]
    SCHEDULE_FREQUENCY_MONTH: _ClassVar[ScheduleFrequency]
    SCHEDULE_FREQUENCY_WEEK: _ClassVar[ScheduleFrequency]
    SCHEDULE_FREQUENCY_DAY: _ClassVar[ScheduleFrequency]
    SCHEDULE_FREQUENCY_HOUR: _ClassVar[ScheduleFrequency]
    SCHEDULE_FREQUENCY_MINUTE: _ClassVar[ScheduleFrequency]

class ErrorKind(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ERROR_KIND_UNSPECIFIED: _ClassVar[ErrorKind]
    ERROR_KIND_INTERNAL: _ClassVar[ErrorKind]
    ERROR_KIND_RUNTIME: _ClassVar[ErrorKind]

class ErrorType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ERROR_TYPE_UNSPECIFIED: _ClassVar[ErrorType]
    ERROR_TYPE_ABORTED: _ClassVar[ErrorType]
    ERROR_TYPE_RUNTIME_UNAVAILABLE: _ClassVar[ErrorType]
    ERROR_TYPE_RUN_IMPOSSIBLE: _ClassVar[ErrorType]
    ERROR_TYPE_NOT_SUPPORTED: _ClassVar[ErrorType]
    ERROR_TYPE_INVALID_VALUE: _ClassVar[ErrorType]
    ERROR_TYPE_INVALID_COMPUTED: _ClassVar[ErrorType]
    ERROR_TYPE_CODE_INVALID: _ClassVar[ErrorType]
    ERROR_TYPE_TEXT_INVALID: _ClassVar[ErrorType]
    ERROR_TYPE_INCAPABLE: _ClassVar[ErrorType]
    ERROR_TYPE_REFUSED: _ClassVar[ErrorType]
    ERROR_TYPE_NON_RETRYABLE: _ClassVar[ErrorType]
    ERROR_TYPE_MODEL_FAILED: _ClassVar[ErrorType]
    ERROR_TYPE_INVALID_CONTINUATION: _ClassVar[ErrorType]
    ERROR_TYPE_INVALID_CALL: _ClassVar[ErrorType]
    ERROR_TYPE_INVALID_PLAN: _ClassVar[ErrorType]
    ERROR_TYPE_INTERRUPTION_CANCELLED: _ClassVar[ErrorType]
    ERROR_TYPE_RETRYABLE: _ClassVar[ErrorType]

class BreakpointSite(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    BREAKPOINT_SITE_UNSPECIFIED: _ClassVar[BreakpointSite]
    BREAKPOINT_SITE_RUN_BEFORE: _ClassVar[BreakpointSite]
    BREAKPOINT_SITE_RUN_AFTER_FAILED: _ClassVar[BreakpointSite]
    BREAKPOINT_SITE_RUN_AFTER_COMPLETED: _ClassVar[BreakpointSite]
    BREAKPOINT_SITE_RUN_AFTER: _ClassVar[BreakpointSite]

class BreakpointAction(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    BREAKPOINT_ACTION_UNSPECIFIED: _ClassVar[BreakpointAction]
    BREAKPOINT_ACTION_YIELD: _ClassVar[BreakpointAction]

class BreakpointScope(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    BREAKPOINT_SCOPE_UNSPECIFIED: _ClassVar[BreakpointScope]
    BREAKPOINT_SCOPE_SELF: _ClassVar[BreakpointScope]
    BREAKPOINT_SCOPE_CHILD: _ClassVar[BreakpointScope]
    BREAKPOINT_SCOPE_ACTION: _ClassVar[BreakpointScope]
    BREAKPOINT_SCOPE_LINK: _ClassVar[BreakpointScope]

class InterruptionType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    INTERRUPTION_TYPE_UNSPECIFIED: _ClassVar[InterruptionType]
    INTERRUPTION_TYPE_PAUSE: _ClassVar[InterruptionType]
    INTERRUPTION_TYPE_YIELD: _ClassVar[InterruptionType]
    INTERRUPTION_TYPE_WAIT: _ClassVar[InterruptionType]

class InterruptionStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    INTERRUPTION_STATUS_UNSPECIFIED: _ClassVar[InterruptionStatus]
    INTERRUPTION_STATUS_OPEN: _ClassVar[InterruptionStatus]
    INTERRUPTION_STATUS_CANCELLED: _ClassVar[InterruptionStatus]
    INTERRUPTION_STATUS_COMPLETED: _ClassVar[InterruptionStatus]

class InterruptionResponse(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    INTERRUPTION_RESPONSE_UNSPECIFIED: _ClassVar[InterruptionResponse]
    INTERRUPTION_RESPONSE_ACCEPT: _ClassVar[InterruptionResponse]
    INTERRUPTION_RESPONSE_REJECT: _ClassVar[InterruptionResponse]

class ModelDeveloper(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MODEL_DEVELOPER_UNSPECIFIED: _ClassVar[ModelDeveloper]
    MODEL_DEVELOPER_META: _ClassVar[ModelDeveloper]
    MODEL_DEVELOPER_OPENAI: _ClassVar[ModelDeveloper]
    MODEL_DEVELOPER_ANTHROPIC: _ClassVar[ModelDeveloper]
    MODEL_DEVELOPER_GOOGLE: _ClassVar[ModelDeveloper]
    MODEL_DEVELOPER_AMAZON: _ClassVar[ModelDeveloper]
    MODEL_DEVELOPER_MICROSOFT: _ClassVar[ModelDeveloper]
    MODEL_DEVELOPER_DEEPSEEK: _ClassVar[ModelDeveloper]

class ModelType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MODEL_TYPE_UNSPECIFIED: _ClassVar[ModelType]
    MODEL_TYPE_META_LLAMA_3_1_80B: _ClassVar[ModelType]
    MODEL_TYPE_META_LLAMA_3_1_400B: _ClassVar[ModelType]
    MODEL_TYPE_OPENAI_GPT4_0: _ClassVar[ModelType]
    MODEL_TYPE_OPENAI_GPT4_O_MINI: _ClassVar[ModelType]
    MODEL_TYPE_OPENAI_GPT4_5: _ClassVar[ModelType]
    MODEL_TYPE_OPENAI_O1: _ClassVar[ModelType]
    MODEL_TYPE_OPENAI_O1_MINI: _ClassVar[ModelType]
    MODEL_TYPE_OPENAI_O3_MINI: _ClassVar[ModelType]
    MODEL_TYPE_ANTHROPIC_CLAUDE_3_5_SONNET: _ClassVar[ModelType]
    MODEL_TYPE_ANTHROPIC_CLAUDE_3_7_SONNET: _ClassVar[ModelType]
    MODEL_TYPE_GOOGLE_GEMINI_2_0_FLASH: _ClassVar[ModelType]
    MODEL_TYPE_GOOGLE_GEMINI_2_0_FLASH_THINKING: _ClassVar[ModelType]
    MODEL_TYPE_DEEPSEEK_R_1_0: _ClassVar[ModelType]

class ModelFamily(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MODEL_FAMILY_UNSPECIFIED: _ClassVar[ModelFamily]
    MODEL_FAMILY_META_LLAMA: _ClassVar[ModelFamily]
    MODEL_FAMILY_OPENAI_GPT: _ClassVar[ModelFamily]
    MODEL_FAMILY_OPENAI_O: _ClassVar[ModelFamily]
    MODEL_FAMILY_ANTHROPIC_CLAUDE: _ClassVar[ModelFamily]
    MODEL_FAMILY_GOOGLE_GEMINI: _ClassVar[ModelFamily]
    MODEL_FAMILY_DEEPSEEK_R: _ClassVar[ModelFamily]

class ModelProvider(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MODEL_PROVIDER_UNSPECIFIED: _ClassVar[ModelProvider]
    MODEL_PROVIDER_META: _ClassVar[ModelProvider]
    MODEL_PROVIDER_OPENAI: _ClassVar[ModelProvider]
    MODEL_PROVIDER_ANTHROPIC: _ClassVar[ModelProvider]
    MODEL_PROVIDER_GOOGLE: _ClassVar[ModelProvider]
    MODEL_PROVIDER_AMAZON: _ClassVar[ModelProvider]
    MODEL_PROVIDER_MICROSOFT: _ClassVar[ModelProvider]
    MODEL_PROVIDER_DEEPSEEK: _ClassVar[ModelProvider]

class CodeType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CODE_TYPE_UNSPECIFIED: _ClassVar[CodeType]
    CODE_TYPE_SNIPPET: _ClassVar[CodeType]
    CODE_TYPE_SCRIPT: _ClassVar[CodeType]
    CODE_TYPE_FUNCTION: _ClassVar[CodeType]

class ActionType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ACTION_TYPE_UNSPECIFIED: _ClassVar[ActionType]
    ACTION_TYPE_START: _ClassVar[ActionType]
    ACTION_TYPE_COMPLETE: _ClassVar[ActionType]
    ACTION_TYPE_FAIL: _ClassVar[ActionType]
    ACTION_TYPE_TOOL: _ClassVar[ActionType]
    ACTION_TYPE_CODE: _ClassVar[ActionType]
    ACTION_TYPE_DO: _ClassVar[ActionType]
    ACTION_TYPE_THINK: _ClassVar[ActionType]
    ACTION_TYPE_ROUTE: _ClassVar[ActionType]
    ACTION_TYPE_GENERATE: _ClassVar[ActionType]
    ACTION_TYPE_TRANSFORM: _ClassVar[ActionType]
    ACTION_TYPE_EXTRACT: _ClassVar[ActionType]
    ACTION_TYPE_CLASSIFY: _ClassVar[ActionType]
    ACTION_TYPE_SUMMARIZE: _ClassVar[ActionType]
    ACTION_TYPE_COMPARE: _ClassVar[ActionType]
    ACTION_TYPE_TRANSLATE: _ClassVar[ActionType]
    ACTION_TYPE_EDIT: _ClassVar[ActionType]
    ACTION_TYPE_CREATE: _ClassVar[ActionType]
    ACTION_TYPE_DUPLICATE: _ClassVar[ActionType]
    ACTION_TYPE_UPDATE: _ClassVar[ActionType]
    ACTION_TYPE_DELETE: _ClassVar[ActionType]
    ACTION_TYPE_WAIT: _ClassVar[ActionType]
    ACTION_TYPE_SEND: _ClassVar[ActionType]
    ACTION_TYPE_RECEIVE: _ClassVar[ActionType]
    ACTION_TYPE_YIELD: _ClassVar[ActionType]
    ACTION_TYPE_LOOK: _ClassVar[ActionType]
    ACTION_TYPE_CLICK: _ClassVar[ActionType]
    ACTION_TYPE_PRESS: _ClassVar[ActionType]
    ACTION_TYPE_TYPE: _ClassVar[ActionType]
    ACTION_TYPE_SCROLL: _ClassVar[ActionType]
    ACTION_TYPE_SELECT: _ClassVar[ActionType]
    ACTION_TYPE_DRAG: _ClassVar[ActionType]
    ACTION_TYPE_GO_BACKWARD: _ClassVar[ActionType]
    ACTION_TYPE_GO_FORWARD: _ClassVar[ActionType]
    ACTION_TYPE_GO_TO_URL: _ClassVar[ActionType]
    ACTION_TYPE_GO_TO_TAB: _ClassVar[ActionType]
    ACTION_TYPE_OPEN_TAB: _ClassVar[ActionType]
    ACTION_TYPE_CLOSE_TAB: _ClassVar[ActionType]
    ACTION_TYPE_HTTP: _ClassVar[ActionType]
    ACTION_TYPE_REST: _ClassVar[ActionType]
    ACTION_TYPE_GRAPHQL: _ClassVar[ActionType]
    ACTION_TYPE_SQL: _ClassVar[ActionType]
    ACTION_TYPE_WEB: _ClassVar[ActionType]

class ActionCategory(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ACTION_CATEGORY_UNSPECIFIED: _ClassVar[ActionCategory]
    ACTION_CATEGORY_READ: _ClassVar[ActionCategory]
    ACTION_CATEGORY_SEARCH: _ClassVar[ActionCategory]
    ACTION_CATEGORY_BROWSE: _ClassVar[ActionCategory]
    ACTION_CATEGORY_TYPE: _ClassVar[ActionCategory]
    ACTION_CATEGORY_SPEAK: _ClassVar[ActionCategory]
    ACTION_CATEGORY_THINK: _ClassVar[ActionCategory]
    ACTION_CATEGORY_WAIT: _ClassVar[ActionCategory]
    ACTION_CATEGORY_WORK: _ClassVar[ActionCategory]
    ACTION_CATEGORY_INTERACT: _ClassVar[ActionCategory]

class PortSide(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PORT_SIDE_UNSPECIFIED: _ClassVar[PortSide]
    PORT_SIDE_INCOMING: _ClassVar[PortSide]
    PORT_SIDE_OUTGOING: _ClassVar[PortSide]

class LinkType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    LINK_TYPE_UNSPECIFIED: _ClassVar[LinkType]
    LINK_TYPE_DECIDE: _ClassVar[LinkType]
    LINK_TYPE_REQUIRE: _ClassVar[LinkType]

class LinkTrigger(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    LINK_TRIGGER_UNSPECIFIED: _ClassVar[LinkTrigger]
    LINK_TRIGGER_ON_COMPLETED: _ClassVar[LinkTrigger]
    LINK_TRIGGER_ON_FAILED: _ClassVar[LinkTrigger]
    LINK_TRIGGER_ON_TERMINATED: _ClassVar[LinkTrigger]

class SpaceType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SPACE_TYPE_UNSPECIFIED: _ClassVar[SpaceType]
    SPACE_TYPE_BROWSER: _ClassVar[SpaceType]
    SPACE_TYPE_DESKTOP: _ClassVar[SpaceType]
    SPACE_TYPE_MOBILE: _ClassVar[SpaceType]

class ViewType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    VIEW_TYPE_UNSPECIFIED: _ClassVar[ViewType]
    VIEW_TYPE_MACHINE: _ClassVar[ViewType]
    VIEW_TYPE_BROWSER: _ClassVar[ViewType]
    VIEW_TYPE_PAGE: _ClassVar[ViewType]
    VIEW_TYPE_BLOCK: _ClassVar[ViewType]
    VIEW_TYPE_CHOICE: _ClassVar[ViewType]
    VIEW_TYPE_CLASS: _ClassVar[ViewType]
    VIEW_TYPE_FIELD: _ClassVar[ViewType]
    VIEW_TYPE_FLOW: _ClassVar[ViewType]
    VIEW_TYPE_ACTION: _ClassVar[ViewType]
    VIEW_TYPE_LINK: _ClassVar[ViewType]
    VIEW_TYPE_TRIGGER: _ClassVar[ViewType]
    VIEW_TYPE_IMPLEMENTATION: _ClassVar[ViewType]
    VIEW_TYPE_VIEW: _ClassVar[ViewType]
    VIEW_TYPE_DATABASE: _ClassVar[ViewType]
    VIEW_TYPE_CHANNEL: _ClassVar[ViewType]
    VIEW_TYPE_THREAD: _ClassVar[ViewType]
    VIEW_TYPE_RUN: _ClassVar[ViewType]
    VIEW_TYPE_OBJECT: _ClassVar[ViewType]
    VIEW_TYPE_TYPE: _ClassVar[ViewType]
    VIEW_TYPE_FIELD_LIST: _ClassVar[ViewType]
    VIEW_TYPE_PATH: _ClassVar[ViewType]
    VIEW_TYPE_COMPUTED_VALUE: _ClassVar[ViewType]
    VIEW_TYPE_SELECTION: _ClassVar[ViewType]
    VIEW_TYPE_USER_WIZARD: _ClassVar[ViewType]
    VIEW_TYPE_BENCH_WIZARD: _ClassVar[ViewType]
    VIEW_TYPE_EMPTY: _ClassVar[ViewType]
    VIEW_TYPE_CREATE: _ClassVar[ViewType]
    VIEW_TYPE_CHAT: _ClassVar[ViewType]
    VIEW_TYPE_SIDEBAR: _ClassVar[ViewType]
    VIEW_TYPE_CONTEXT: _ClassVar[ViewType]
    VIEW_TYPE_ACTIVITY: _ClassVar[ViewType]
    VIEW_TYPE_CATALOG: _ClassVar[ViewType]
    VIEW_TYPE_WINDOW: _ClassVar[ViewType]
    VIEW_TYPE_TAB: _ClassVar[ViewType]
    VIEW_TYPE_HISTORY: _ClassVar[ViewType]
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
    VIEW_TYPE_TREE: _ClassVar[ViewType]
    VIEW_TYPE_FEED: _ClassVar[ViewType]
    VIEW_TYPE_GALLERY: _ClassVar[ViewType]
    VIEW_TYPE_BOARD: _ClassVar[ViewType]
    VIEW_TYPE_BREADCRUMB: _ClassVar[ViewType]
    VIEW_TYPE_PROGRESS: _ClassVar[ViewType]
    VIEW_TYPE_AVATAR: _ClassVar[ViewType]
    VIEW_TYPE_BADGE: _ClassVar[ViewType]
    VIEW_TYPE_SHAPE: _ClassVar[ViewType]
    VIEW_TYPE_CHART: _ClassVar[ViewType]
    VIEW_TYPE_BUTTON: _ClassVar[ViewType]
    VIEW_TYPE_MULTI_BUTTON: _ClassVar[ViewType]
    VIEW_TYPE_NUMBER: _ClassVar[ViewType]
    VIEW_TYPE_SLIDER: _ClassVar[ViewType]
    VIEW_TYPE_STRING: _ClassVar[ViewType]
    VIEW_TYPE_TEXT: _ClassVar[ViewType]
    VIEW_TYPE_CODE: _ClassVar[ViewType]
    VIEW_TYPE_JSON: _ClassVar[ViewType]
    VIEW_TYPE_TOGGLE: _ClassVar[ViewType]
    VIEW_TYPE_PICKER: _ClassVar[ViewType]
    VIEW_TYPE_COLOR: _ClassVar[ViewType]
    VIEW_TYPE_ICON: _ClassVar[ViewType]
    VIEW_TYPE_DATETIME: _ClassVar[ViewType]
    VIEW_TYPE_DURATION: _ClassVar[ViewType]
    VIEW_TYPE_FILE: _ClassVar[ViewType]
    VIEW_TYPE_IMAGE: _ClassVar[ViewType]
    VIEW_TYPE_AUDIO: _ClassVar[ViewType]
    VIEW_TYPE_VIDEO: _ClassVar[ViewType]
    VIEW_TYPE_DOCUMENT: _ClassVar[ViewType]

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

class Orientation(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ORIENTATION_UNSPECIFIED: _ClassVar[Orientation]
    ORIENTATION_HORIZONTAL: _ClassVar[Orientation]
    ORIENTATION_HORIZONTAL_REVERSED: _ClassVar[Orientation]
    ORIENTATION_VERTICAL: _ClassVar[Orientation]
    ORIENTATION_VERTICAL_REVERSED: _ClassVar[Orientation]

class Alignment(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ALIGNMENT_UNSPECIFIED: _ClassVar[Alignment]
    ALIGNMENT_START: _ClassVar[Alignment]
    ALIGNMENT_MIDDLE: _ClassVar[Alignment]
    ALIGNMENT_END: _ClassVar[Alignment]
    ALIGNMENT_SPACE_BETWEEN: _ClassVar[Alignment]

class UserWizardViewStage(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    USER_WIZARD_VIEW_STAGE_UNSPECIFIED: _ClassVar[UserWizardViewStage]
    USER_WIZARD_VIEW_STAGE_SIGN_UP: _ClassVar[UserWizardViewStage]
    USER_WIZARD_VIEW_STAGE_LOG_IN: _ClassVar[UserWizardViewStage]

class TreeViewPreset(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TREE_VIEW_PRESET_UNSPECIFIED: _ClassVar[TreeViewPreset]
    TREE_VIEW_PRESET_PACKAGE: _ClassVar[TreeViewPreset]
    TREE_VIEW_PRESET_OUTLINE: _ClassVar[TreeViewPreset]

class SidebarAspect(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SIDEBAR_ASPECT_UNSPECIFIED: _ClassVar[SidebarAspect]
    SIDEBAR_ASPECT_BENCH: _ClassVar[SidebarAspect]
    SIDEBAR_ASPECT_ACTIVITY: _ClassVar[SidebarAspect]
    SIDEBAR_ASPECT_CATALOG: _ClassVar[SidebarAspect]
    SIDEBAR_ASPECT_LIBRARY: _ClassVar[SidebarAspect]

class ContextAspect(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CONTEXT_ASPECT_UNSPECIFIED: _ClassVar[ContextAspect]
    CONTEXT_ASPECT_DETAIL: _ClassVar[ContextAspect]
    CONTEXT_ASPECT_RUN: _ClassVar[ContextAspect]
    CONTEXT_ASPECT_CHAT: _ClassVar[ContextAspect]
    CONTEXT_ASPECT_LOG: _ClassVar[ContextAspect]

class ButtonVariant(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    BUTTON_VARIANT_UNSPECIFIED: _ClassVar[ButtonVariant]
    BUTTON_VARIANT_PRIMARY: _ClassVar[ButtonVariant]
    BUTTON_VARIANT_SECONDARY: _ClassVar[ButtonVariant]
    BUTTON_VARIANT_LINK: _ClassVar[ButtonVariant]

class PickerVariant(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PICKER_VARIANT_UNSPECIFIED: _ClassVar[PickerVariant]
    PICKER_VARIANT_MULTI_TOGGLE: _ClassVar[PickerVariant]
    PICKER_VARIANT_DROPDOWN: _ClassVar[PickerVariant]
    PICKER_VARIANT_DROPDOWN_LARGE: _ClassVar[PickerVariant]

class UserStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    USER_STATUS_UNSPECIFIED: _ClassVar[UserStatus]
    USER_STATUS_INVITED: _ClassVar[UserStatus]
    USER_STATUS_RESERVED: _ClassVar[UserStatus]
    USER_STATUS_WAITLISTED: _ClassVar[UserStatus]
    USER_STATUS_REGISTERED: _ClassVar[UserStatus]
    USER_STATUS_ACTIVATED: _ClassVar[UserStatus]

class OrganizationStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ORGANIZATION_STATUS_UNSPECIFIED: _ClassVar[OrganizationStatus]
    ORGANIZATION_STATUS_REGISTERED: _ClassVar[OrganizationStatus]
    ORGANIZATION_STATUS_ACTIVATED: _ClassVar[OrganizationStatus]

class ChannelType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CHANNEL_TYPE_UNSPECIFIED: _ClassVar[ChannelType]
    CHANNEL_TYPE_TEXT: _ClassVar[ChannelType]

class ThreadType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    THREAD_TYPE_UNSPECIFIED: _ClassVar[ThreadType]
    THREAD_TYPE_SOURCE: _ClassVar[ThreadType]
    THREAD_TYPE_RUN: _ClassVar[ThreadType]

class ThreadStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    THREAD_STATUS_UNSPECIFIED: _ClassVar[ThreadStatus]
    THREAD_STATUS_OPEN: _ClassVar[ThreadStatus]
    THREAD_STATUS_CLOSED: _ClassVar[ThreadStatus]

class MessageType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MESSAGE_TYPE_UNSPECIFIED: _ClassVar[MessageType]
    MESSAGE_TYPE_REGULAR: _ClassVar[MessageType]
    MESSAGE_TYPE_REPLY: _ClassVar[MessageType]
    MESSAGE_TYPE_FORWARDED: _ClassVar[MessageType]
    MESSAGE_TYPE_THREAD: _ClassVar[MessageType]
    MESSAGE_TYPE_RUN: _ClassVar[MessageType]
    MESSAGE_TYPE_INTERRUPTION: _ClassVar[MessageType]
    MESSAGE_TYPE_POLL: _ClassVar[MessageType]
    MESSAGE_TYPE_EDIT: _ClassVar[MessageType]

class MessageStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MESSAGE_STATUS_UNSPECIFIED: _ClassVar[MessageStatus]
    MESSAGE_STATUS_DRAFT: _ClassVar[MessageStatus]
    MESSAGE_STATUS_SENDING: _ClassVar[MessageStatus]
    MESSAGE_STATUS_SENT: _ClassVar[MessageStatus]
    MESSAGE_STATUS_FAILED: _ClassVar[MessageStatus]
    MESSAGE_STATUS_RECEIVED: _ClassVar[MessageStatus]
    MESSAGE_STATUS_READ: _ClassVar[MessageStatus]

class MessagePlatform(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MESSAGE_PLATFORM_UNSPECIFIED: _ClassVar[MessagePlatform]
    MESSAGE_PLATFORM_BENCH: _ClassVar[MessagePlatform]
    MESSAGE_PLATFORM_WEBHOOK: _ClassVar[MessagePlatform]
    MESSAGE_PLATFORM_EMAIL: _ClassVar[MessagePlatform]
    MESSAGE_PLATFORM_SMS: _ClassVar[MessagePlatform]

class NotificationType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    NOTIFICATION_TYPE_UNSPECIFIED: _ClassVar[NotificationType]
    NOTIFICATION_TYPE_MESSAGE: _ClassVar[NotificationType]

class NotificationStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    NOTIFICATION_STATUS_UNSPECIFIED: _ClassVar[NotificationStatus]
    NOTIFICATION_STATUS_SENDING: _ClassVar[NotificationStatus]
    NOTIFICATION_STATUS_SENT: _ClassVar[NotificationStatus]
    NOTIFICATION_STATUS_FAILED: _ClassVar[NotificationStatus]
    NOTIFICATION_STATUS_RECEIVED: _ClassVar[NotificationStatus]
    NOTIFICATION_STATUS_READ: _ClassVar[NotificationStatus]

class BuiltinEnum(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    BUILTIN_ENUM_UNSPECIFIED: _ClassVar[BuiltinEnum]

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
ENUM_TYPE_UNSPECIFIED: EnumType
ENUM_TYPE_ENUM_TYPE: EnumType
ENUM_TYPE_NODE_TYPE: EnumType
ENUM_TYPE_STRUCT_TYPE: EnumType
ENUM_TYPE_OBJECT_TYPE: EnumType
ENUM_TYPE_BENCH_TYPE: EnumType
ENUM_TYPE_NODE_MODE: EnumType
ENUM_TYPE_NODE_AREA: EnumType
ENUM_TYPE_PROPERTY_REFERENCE_TYPE: EnumType
ENUM_TYPE_EDIT_OPERATION_TYPE: EnumType
ENUM_TYPE_CHANGE_CATEGORY: EnumType
ENUM_TYPE_PACKAGE_TYPE: EnumType
ENUM_TYPE_CLOUD: EnumType
ENUM_TYPE_REGION: EnumType
ENUM_TYPE_REGION_ZONE: EnumType
ENUM_TYPE_REGION_AREA: EnumType
ENUM_TYPE_REGION_CONTINENT: EnumType
ENUM_TYPE_BENCH_STATUS: EnumType
ENUM_TYPE_ACCESS_MODE: EnumType
ENUM_TYPE_ACCESS_KIND: EnumType
ENUM_TYPE_POLICY_EFFECT: EnumType
ENUM_TYPE_MEMBERSHIP_TYPE: EnumType
ENUM_TYPE_INVITE_TYPE: EnumType
ENUM_TYPE_QUERY_TYPE: EnumType
ENUM_TYPE_EDIT_TYPE: EnumType
ENUM_TYPE_USE_TYPE: EnumType
ENUM_TYPE_ACCESS_TYPE: EnumType
ENUM_TYPE_RESOURCE_STATUS: EnumType
ENUM_TYPE_RESOURCE_OCCUPANCY: EnumType
ENUM_TYPE_SCALER_TYPE: EnumType
ENUM_TYPE_SCALER_STRATEGY: EnumType
ENUM_TYPE_MACHINE_TYPE: EnumType
ENUM_TYPE_BROWSER_TYPE: EnumType
ENUM_TYPE_STORE_TYPE: EnumType
ENUM_TYPE_CLIENT_TYPE: EnumType
ENUM_TYPE_FILE_RETENTION_MODE: EnumType
ENUM_TYPE_FILE_KIND: EnumType
ENUM_TYPE_FILE_TYPE: EnumType
ENUM_TYPE_FILE_FORMAT: EnumType
ENUM_TYPE_ICON_TYPE: EnumType
ENUM_TYPE_STREAM_TYPE: EnumType
ENUM_TYPE_PRIMITIVE_TYPE: EnumType
ENUM_TYPE_FIELD_ZONE: EnumType
ENUM_TYPE_TYPE_KIND: EnumType
ENUM_TYPE_TYPE_FORMAT: EnumType
ENUM_TYPE_BLOCK_TYPE: EnumType
ENUM_TYPE_DAY: EnumType
ENUM_TYPE_MONTH: EnumType
ENUM_TYPE_TIME_INTERVAL: EnumType
ENUM_TYPE_TEXT_LINE_TYPE: EnumType
ENUM_TYPE_TEXT_SPAN_TYPE: EnumType
ENUM_TYPE_EXPRESSION_KIND: EnumType
ENUM_TYPE_EXPRESSION_OP: EnumType
ENUM_TYPE_LITERAL_TYPE: EnumType
ENUM_TYPE_FUNCTIONAL_TYPE: EnumType
ENUM_TYPE_CONDITIONAL_TYPE: EnumType
ENUM_TYPE_AGGREGATION_TYPE: EnumType
ENUM_TYPE_SORT_MODE: EnumType
ENUM_TYPE_SORT_TYPE: EnumType
ENUM_TYPE_COMPUTED_VALUE_KIND: EnumType
ENUM_TYPE_COMPUTED_VALUE_MODE: EnumType
ENUM_TYPE_PATH_ELEMENT_TYPE: EnumType
ENUM_TYPE_PATH_RUN_SELECTOR: EnumType
ENUM_TYPE_SELECTION_TYPE: EnumType
ENUM_TYPE_RUN_STATUS: EnumType
ENUM_TYPE_RUN_TYPE: EnumType
ENUM_TYPE_RUN_SPAN_TYPE: EnumType
ENUM_TYPE_PLAN_TYPE: EnumType
ENUM_TYPE_PLAN_STATUS: EnumType
ENUM_TYPE_PLAN_TERMINATION_MODE: EnumType
ENUM_TYPE_PLAN_FAILURE_MODE: EnumType
ENUM_TYPE_TASK_TYPE: EnumType
ENUM_TYPE_TASK_STATUS: EnumType
ENUM_TYPE_SESSION_STATUS: EnumType
ENUM_TYPE_CACHE_MODE: EnumType
ENUM_TYPE_LOG_TYPE: EnumType
ENUM_TYPE_SEVERITY: EnumType
ENUM_TYPE_TRIGGER_TYPE: EnumType
ENUM_TYPE_TRIGGER_EFFECT: EnumType
ENUM_TYPE_TRIGGER_STATUS: EnumType
ENUM_TYPE_SCHEDULE_FREQUENCY: EnumType
ENUM_TYPE_ERROR_KIND: EnumType
ENUM_TYPE_ERROR_TYPE: EnumType
ENUM_TYPE_BREAKPOINT_SITE: EnumType
ENUM_TYPE_BREAKPOINT_ACTION: EnumType
ENUM_TYPE_BREAKPOINT_TARGET: EnumType
ENUM_TYPE_INTERRUPTION_TYPE: EnumType
ENUM_TYPE_INTERRUPTION_STATUS: EnumType
ENUM_TYPE_INTERRUPTION_RESPONSE: EnumType
ENUM_TYPE_MODEL_DEVELOPER: EnumType
ENUM_TYPE_MODEL_TYPE: EnumType
ENUM_TYPE_MODEL_FAMILY: EnumType
ENUM_TYPE_MODEL_PROVIDER: EnumType
ENUM_TYPE_CODE_TYPE: EnumType
ENUM_TYPE_ACTION_TYPE: EnumType
ENUM_TYPE_ACTION_CATEGORY: EnumType
ENUM_TYPE_PORT_SIDE: EnumType
ENUM_TYPE_LINK_TYPE: EnumType
ENUM_TYPE_LINK_TRIGGER: EnumType
ENUM_TYPE_SPACE_TYPE: EnumType
ENUM_TYPE_VIEW_TYPE: EnumType
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
ENUM_TYPE_SIDEBAR_ASPECT: EnumType
ENUM_TYPE_CONTEXT_ASPECT: EnumType
ENUM_TYPE_BUTTON_VARIANT: EnumType
ENUM_TYPE_PICKER_VARIANT: EnumType
ENUM_TYPE_USER_STATUS: EnumType
ENUM_TYPE_ORGANIZATION_STATUS: EnumType
ENUM_TYPE_CHANNEL_TYPE: EnumType
ENUM_TYPE_THREAD_TYPE: EnumType
ENUM_TYPE_THREAD_STATUS: EnumType
ENUM_TYPE_MESSAGE_TYPE: EnumType
ENUM_TYPE_MESSAGE_STATUS: EnumType
ENUM_TYPE_MESSAGE_PLATFORM: EnumType
ENUM_TYPE_NOTIFICATION_TYPE: EnumType
ENUM_TYPE_NOTIFICATION_STATUS: EnumType
NODE_TYPE_UNSPECIFIED: NodeType
NODE_TYPE_BENCH: NodeType
NODE_TYPE_HANDLE: NodeType
NODE_TYPE_USER: NodeType
NODE_TYPE_ORGANIZATION: NodeType
NODE_TYPE_TEAM: NodeType
NODE_TYPE_CLIENT: NodeType
NODE_TYPE_SCALER: NodeType
NODE_TYPE_STORE: NodeType
NODE_TYPE_MACHINE: NodeType
NODE_TYPE_BROWSER: NodeType
NODE_TYPE_FILE: NodeType
NODE_TYPE_STREAM: NodeType
NODE_TYPE_SECRET: NodeType
NODE_TYPE_PACKAGE: NodeType
NODE_TYPE_DEPENDENCY: NodeType
NODE_TYPE_PAGE: NodeType
NODE_TYPE_BLOCK: NodeType
NODE_TYPE_CHOICE: NodeType
NODE_TYPE_CLASS: NodeType
NODE_TYPE_FIELD: NodeType
NODE_TYPE_OPTION: NodeType
NODE_TYPE_TAG: NodeType
NODE_TYPE_FLOW: NodeType
NODE_TYPE_ACTION: NodeType
NODE_TYPE_LINK: NodeType
NODE_TYPE_TRIGGER: NodeType
NODE_TYPE_IMPLEMENTATION: NodeType
NODE_TYPE_VIEW: NodeType
NODE_TYPE_DATABASE: NodeType
NODE_TYPE_CHANNEL: NodeType
NODE_TYPE_ROLE: NodeType
NODE_TYPE_SPACE: NodeType
NODE_TYPE_THREAD: NodeType
NODE_TYPE_MESSAGE: NodeType
NODE_TYPE_NOTIFICATION: NodeType
NODE_TYPE_RECORD: NodeType
NODE_TYPE_MEMBERSHIP: NodeType
NODE_TYPE_INVITE: NodeType
NODE_TYPE_SESSION: NodeType
NODE_TYPE_RUN: NodeType
NODE_TYPE_RUN_SPAN: NodeType
NODE_TYPE_INTERRUPTION: NodeType
NODE_TYPE_LOG: NodeType
NODE_TYPE_PLAN: NodeType
NODE_TYPE_TASK: NodeType
NODE_TYPE_SKIP: NodeType
NODE_TYPE_EMPTY: NodeType
STRUCT_TYPE_UNSPECIFIED: StructType
STRUCT_TYPE_CONTEXT: StructType
STRUCT_TYPE_EDIT_CONTEXT: StructType
STRUCT_TYPE_EDIT: StructType
STRUCT_TYPE_EDIT_OPERATION: StructType
STRUCT_TYPE_CHANGE: StructType
STRUCT_TYPE_CHANGE_VIGNETTE: StructType
STRUCT_TYPE_GRAPH_SCOPE: StructType
STRUCT_TYPE_CLIENT_ORIGIN: StructType
STRUCT_TYPE_NODE_REFERENCE: StructType
STRUCT_TYPE_PROPERTY_REFERENCE: StructType
STRUCT_TYPE_POLICY: StructType
STRUCT_TYPE_POLICY_RULE: StructType
STRUCT_TYPE_SUBJECT: StructType
STRUCT_TYPE_ACCESS_ZONE: StructType
STRUCT_TYPE_ACCESS_MATRIX: StructType
STRUCT_TYPE_ACCESS: StructType
STRUCT_TYPE_MACHINE_IMAGE: StructType
STRUCT_TYPE_TYPE: StructType
STRUCT_TYPE_TYPE_CONSTRAINT: StructType
STRUCT_TYPE_FILE_INFO: StructType
STRUCT_TYPE_ICON: StructType
STRUCT_TYPE_SCHEDULE: StructType
STRUCT_TYPE_TEXT: StructType
STRUCT_TYPE_TEXT_LINE: StructType
STRUCT_TYPE_TEXT_SPAN: StructType
STRUCT_TYPE_TEXT_TABLE: StructType
STRUCT_TYPE_TEXT_CELL: StructType
STRUCT_TYPE_CODE: StructType
STRUCT_TYPE_CODE_LINE: StructType
STRUCT_TYPE_PATH: StructType
STRUCT_TYPE_PATH_ELEMENT: StructType
STRUCT_TYPE_EXPRESSION: StructType
STRUCT_TYPE_AGGREGATION_RESULT: StructType
STRUCT_TYPE_SELECTION: StructType
STRUCT_TYPE_SELECT_OPTIONS: StructType
STRUCT_TYPE_VALUE: StructType
STRUCT_TYPE_COMPUTED_VALUE: StructType
STRUCT_TYPE_ERROR: StructType
STRUCT_TYPE_RUN_OPTIONS: StructType
STRUCT_TYPE_RUN_TRACE: StructType
STRUCT_TYPE_RUN_FRAME: StructType
STRUCT_TYPE_BREAKPOINT: StructType
STRUCT_TYPE_TEXT_OPTIONS: StructType
STRUCT_TYPE_AUDIO_OPTIONS: StructType
STRUCT_TYPE_IMAGE_OPTIONS: StructType
STRUCT_TYPE_VIDEO_OPTIONS: StructType
STRUCT_TYPE_COLOR: StructType
STRUCT_TYPE_FONT: StructType
STRUCT_TYPE_RECTANGLE: StructType
STRUCT_TYPE_OFFSET: StructType
STRUCT_TYPE_TRANSFORM: StructType
STRUCT_TYPE_VECTOR2: StructType
STRUCT_TYPE_VECTOR3: StructType
STRUCT_TYPE_VECTOR4: StructType
STRUCT_TYPE_LINE: StructType
STRUCT_TYPE_RECTANGLE_CONSTRAINT: StructType
STRUCT_TYPE_DOM_NODE: StructType
OBJECT_TYPE_UNSPECIFIED: ObjectType
OBJECT_TYPE_BENCH: ObjectType
OBJECT_TYPE_HANDLE: ObjectType
OBJECT_TYPE_USER: ObjectType
OBJECT_TYPE_ORGANIZATION: ObjectType
OBJECT_TYPE_TEAM: ObjectType
OBJECT_TYPE_CLIENT: ObjectType
OBJECT_TYPE_SCALER: ObjectType
OBJECT_TYPE_STORE: ObjectType
OBJECT_TYPE_MACHINE: ObjectType
OBJECT_TYPE_BROWSER: ObjectType
OBJECT_TYPE_FILE: ObjectType
OBJECT_TYPE_STREAM: ObjectType
OBJECT_TYPE_SECRET: ObjectType
OBJECT_TYPE_PACKAGE: ObjectType
OBJECT_TYPE_DEPENDENCY: ObjectType
OBJECT_TYPE_PAGE: ObjectType
OBJECT_TYPE_BLOCK: ObjectType
OBJECT_TYPE_CHOICE: ObjectType
OBJECT_TYPE_CLASS: ObjectType
OBJECT_TYPE_FIELD: ObjectType
OBJECT_TYPE_OPTION: ObjectType
OBJECT_TYPE_TAG: ObjectType
OBJECT_TYPE_FLOW: ObjectType
OBJECT_TYPE_ACTION: ObjectType
OBJECT_TYPE_LINK: ObjectType
OBJECT_TYPE_TRIGGER: ObjectType
OBJECT_TYPE_IMPLEMENTATION: ObjectType
OBJECT_TYPE_VIEW: ObjectType
OBJECT_TYPE_DATABASE: ObjectType
OBJECT_TYPE_CHANNEL: ObjectType
OBJECT_TYPE_ROLE: ObjectType
OBJECT_TYPE_SPACE: ObjectType
OBJECT_TYPE_THREAD: ObjectType
OBJECT_TYPE_MESSAGE: ObjectType
OBJECT_TYPE_NOTIFICATION: ObjectType
OBJECT_TYPE_RECORD: ObjectType
OBJECT_TYPE_MEMBERSHIP: ObjectType
OBJECT_TYPE_INVITE: ObjectType
OBJECT_TYPE_SESSION: ObjectType
OBJECT_TYPE_RUN: ObjectType
OBJECT_TYPE_RUN_SPAN: ObjectType
OBJECT_TYPE_INTERRUPTION: ObjectType
OBJECT_TYPE_LOG: ObjectType
OBJECT_TYPE_PLAN: ObjectType
OBJECT_TYPE_TASK: ObjectType
OBJECT_TYPE_SKIP: ObjectType
OBJECT_TYPE_EMPTY: ObjectType
OBJECT_TYPE_CONTEXT: ObjectType
OBJECT_TYPE_EDIT_CONTEXT: ObjectType
OBJECT_TYPE_EDIT: ObjectType
OBJECT_TYPE_EDIT_OPERATION: ObjectType
OBJECT_TYPE_CHANGE: ObjectType
OBJECT_TYPE_CHANGE_VIGNETTE: ObjectType
OBJECT_TYPE_GRAPH_SCOPE: ObjectType
OBJECT_TYPE_CLIENT_ORIGIN: ObjectType
OBJECT_TYPE_NODE_REFERENCE: ObjectType
OBJECT_TYPE_PROPERTY_REFERENCE: ObjectType
OBJECT_TYPE_POLICY: ObjectType
OBJECT_TYPE_POLICY_RULE: ObjectType
OBJECT_TYPE_SUBJECT: ObjectType
OBJECT_TYPE_ACCESS_ZONE: ObjectType
OBJECT_TYPE_ACCESS_MATRIX: ObjectType
OBJECT_TYPE_ACCESS: ObjectType
OBJECT_TYPE_MACHINE_IMAGE: ObjectType
OBJECT_TYPE_TYPE: ObjectType
OBJECT_TYPE_TYPE_CONSTRAINT: ObjectType
OBJECT_TYPE_FILE_INFO: ObjectType
OBJECT_TYPE_ICON: ObjectType
OBJECT_TYPE_SCHEDULE: ObjectType
OBJECT_TYPE_TEXT: ObjectType
OBJECT_TYPE_TEXT_LINE: ObjectType
OBJECT_TYPE_TEXT_SPAN: ObjectType
OBJECT_TYPE_TEXT_TABLE: ObjectType
OBJECT_TYPE_TEXT_CELL: ObjectType
OBJECT_TYPE_CODE: ObjectType
OBJECT_TYPE_CODE_LINE: ObjectType
OBJECT_TYPE_PATH: ObjectType
OBJECT_TYPE_PATH_ELEMENT: ObjectType
OBJECT_TYPE_EXPRESSION: ObjectType
OBJECT_TYPE_AGGREGATION_RESULT: ObjectType
OBJECT_TYPE_SELECTION: ObjectType
OBJECT_TYPE_SELECT_OPTIONS: ObjectType
OBJECT_TYPE_VALUE: ObjectType
OBJECT_TYPE_COMPUTED_VALUE: ObjectType
OBJECT_TYPE_ERROR: ObjectType
OBJECT_TYPE_RUN_OPTIONS: ObjectType
OBJECT_TYPE_RUN_TRACE: ObjectType
OBJECT_TYPE_RUN_FRAME: ObjectType
OBJECT_TYPE_BREAKPOINT: ObjectType
OBJECT_TYPE_TEXT_OPTIONS: ObjectType
OBJECT_TYPE_AUDIO_OPTIONS: ObjectType
OBJECT_TYPE_IMAGE_OPTIONS: ObjectType
OBJECT_TYPE_VIDEO_OPTIONS: ObjectType
OBJECT_TYPE_COLOR: ObjectType
OBJECT_TYPE_FONT: ObjectType
OBJECT_TYPE_RECTANGLE: ObjectType
OBJECT_TYPE_OFFSET: ObjectType
OBJECT_TYPE_TRANSFORM: ObjectType
OBJECT_TYPE_VECTOR2: ObjectType
OBJECT_TYPE_VECTOR3: ObjectType
OBJECT_TYPE_VECTOR4: ObjectType
OBJECT_TYPE_LINE: ObjectType
OBJECT_TYPE_RECTANGLE_CONSTRAINT: ObjectType
OBJECT_TYPE_DOM_NODE: ObjectType
BENCH_TYPE_UNSPECIFIED: BenchType
BENCH_TYPE_BENCH: BenchType
BENCH_TYPE_HANDLE: BenchType
BENCH_TYPE_USER: BenchType
BENCH_TYPE_ORGANIZATION: BenchType
BENCH_TYPE_TEAM: BenchType
BENCH_TYPE_CLIENT: BenchType
BENCH_TYPE_SCALER: BenchType
BENCH_TYPE_STORE: BenchType
BENCH_TYPE_MACHINE: BenchType
BENCH_TYPE_BROWSER: BenchType
BENCH_TYPE_FILE: BenchType
BENCH_TYPE_STREAM: BenchType
BENCH_TYPE_SECRET: BenchType
BENCH_TYPE_PACKAGE: BenchType
BENCH_TYPE_DEPENDENCY: BenchType
BENCH_TYPE_PAGE: BenchType
BENCH_TYPE_BLOCK: BenchType
BENCH_TYPE_CHOICE: BenchType
BENCH_TYPE_CLASS: BenchType
BENCH_TYPE_FIELD: BenchType
BENCH_TYPE_OPTION: BenchType
BENCH_TYPE_TAG: BenchType
BENCH_TYPE_FLOW: BenchType
BENCH_TYPE_ACTION: BenchType
BENCH_TYPE_LINK: BenchType
BENCH_TYPE_TRIGGER: BenchType
BENCH_TYPE_IMPLEMENTATION: BenchType
BENCH_TYPE_VIEW: BenchType
BENCH_TYPE_DATABASE: BenchType
BENCH_TYPE_CHANNEL: BenchType
BENCH_TYPE_ROLE: BenchType
BENCH_TYPE_SPACE: BenchType
BENCH_TYPE_THREAD: BenchType
BENCH_TYPE_MESSAGE: BenchType
BENCH_TYPE_NOTIFICATION: BenchType
BENCH_TYPE_RECORD: BenchType
BENCH_TYPE_MEMBERSHIP: BenchType
BENCH_TYPE_INVITE: BenchType
BENCH_TYPE_SESSION: BenchType
BENCH_TYPE_RUN: BenchType
BENCH_TYPE_RUN_SPAN: BenchType
BENCH_TYPE_INTERRUPTION: BenchType
BENCH_TYPE_LOG: BenchType
BENCH_TYPE_PLAN: BenchType
BENCH_TYPE_TASK: BenchType
BENCH_TYPE_SKIP: BenchType
BENCH_TYPE_EMPTY: BenchType
BENCH_TYPE_CONTEXT: BenchType
BENCH_TYPE_EDIT_CONTEXT: BenchType
BENCH_TYPE_EDIT: BenchType
BENCH_TYPE_EDIT_OPERATION: BenchType
BENCH_TYPE_CHANGE: BenchType
BENCH_TYPE_CHANGE_VIGNETTE: BenchType
BENCH_TYPE_GRAPH_SCOPE: BenchType
BENCH_TYPE_CLIENT_ORIGIN: BenchType
BENCH_TYPE_NODE_REFERENCE: BenchType
BENCH_TYPE_PROPERTY_REFERENCE: BenchType
BENCH_TYPE_POLICY: BenchType
BENCH_TYPE_POLICY_RULE: BenchType
BENCH_TYPE_SUBJECT: BenchType
BENCH_TYPE_ACCESS_ZONE: BenchType
BENCH_TYPE_ACCESS_MATRIX: BenchType
BENCH_TYPE_ACCESS: BenchType
BENCH_TYPE_MACHINE_IMAGE: BenchType
BENCH_TYPE_TYPE: BenchType
BENCH_TYPE_TYPE_CONSTRAINT: BenchType
BENCH_TYPE_FILE_INFO: BenchType
BENCH_TYPE_ICON: BenchType
BENCH_TYPE_SCHEDULE: BenchType
BENCH_TYPE_TEXT: BenchType
BENCH_TYPE_TEXT_LINE: BenchType
BENCH_TYPE_TEXT_SPAN: BenchType
BENCH_TYPE_TEXT_TABLE: BenchType
BENCH_TYPE_TEXT_CELL: BenchType
BENCH_TYPE_CODE: BenchType
BENCH_TYPE_CODE_LINE: BenchType
BENCH_TYPE_PATH: BenchType
BENCH_TYPE_PATH_ELEMENT: BenchType
BENCH_TYPE_EXPRESSION: BenchType
BENCH_TYPE_AGGREGATION_RESULT: BenchType
BENCH_TYPE_SELECTION: BenchType
BENCH_TYPE_SELECT_OPTIONS: BenchType
BENCH_TYPE_VALUE: BenchType
BENCH_TYPE_COMPUTED_VALUE: BenchType
BENCH_TYPE_ERROR: BenchType
BENCH_TYPE_RUN_OPTIONS: BenchType
BENCH_TYPE_RUN_TRACE: BenchType
BENCH_TYPE_RUN_FRAME: BenchType
BENCH_TYPE_BREAKPOINT: BenchType
BENCH_TYPE_TEXT_OPTIONS: BenchType
BENCH_TYPE_AUDIO_OPTIONS: BenchType
BENCH_TYPE_IMAGE_OPTIONS: BenchType
BENCH_TYPE_VIDEO_OPTIONS: BenchType
BENCH_TYPE_COLOR: BenchType
BENCH_TYPE_FONT: BenchType
BENCH_TYPE_RECTANGLE: BenchType
BENCH_TYPE_OFFSET: BenchType
BENCH_TYPE_TRANSFORM: BenchType
BENCH_TYPE_VECTOR2: BenchType
BENCH_TYPE_VECTOR3: BenchType
BENCH_TYPE_VECTOR4: BenchType
BENCH_TYPE_LINE: BenchType
BENCH_TYPE_RECTANGLE_CONSTRAINT: BenchType
BENCH_TYPE_DOM_NODE: BenchType
BENCH_TYPE_ENUM_TYPE: BenchType
BENCH_TYPE_NODE_TYPE: BenchType
BENCH_TYPE_STRUCT_TYPE: BenchType
BENCH_TYPE_OBJECT_TYPE: BenchType
BENCH_TYPE_BENCH_TYPE: BenchType
BENCH_TYPE_NODE_MODE: BenchType
BENCH_TYPE_NODE_AREA: BenchType
BENCH_TYPE_PROPERTY_REFERENCE_TYPE: BenchType
BENCH_TYPE_EDIT_OPERATION_TYPE: BenchType
BENCH_TYPE_CHANGE_CATEGORY: BenchType
BENCH_TYPE_PACKAGE_TYPE: BenchType
BENCH_TYPE_CLOUD: BenchType
BENCH_TYPE_REGION: BenchType
BENCH_TYPE_REGION_ZONE: BenchType
BENCH_TYPE_REGION_AREA: BenchType
BENCH_TYPE_REGION_CONTINENT: BenchType
BENCH_TYPE_BENCH_STATUS: BenchType
BENCH_TYPE_ACCESS_MODE: BenchType
BENCH_TYPE_ACCESS_KIND: BenchType
BENCH_TYPE_POLICY_EFFECT: BenchType
BENCH_TYPE_MEMBERSHIP_TYPE: BenchType
BENCH_TYPE_INVITE_TYPE: BenchType
BENCH_TYPE_QUERY_TYPE: BenchType
BENCH_TYPE_EDIT_TYPE: BenchType
BENCH_TYPE_USE_TYPE: BenchType
BENCH_TYPE_ACCESS_TYPE: BenchType
BENCH_TYPE_RESOURCE_STATUS: BenchType
BENCH_TYPE_RESOURCE_OCCUPANCY: BenchType
BENCH_TYPE_SCALER_TYPE: BenchType
BENCH_TYPE_SCALER_STRATEGY: BenchType
BENCH_TYPE_MACHINE_TYPE: BenchType
BENCH_TYPE_BROWSER_TYPE: BenchType
BENCH_TYPE_STORE_TYPE: BenchType
BENCH_TYPE_CLIENT_TYPE: BenchType
BENCH_TYPE_FILE_RETENTION_MODE: BenchType
BENCH_TYPE_FILE_KIND: BenchType
BENCH_TYPE_FILE_TYPE: BenchType
BENCH_TYPE_FILE_FORMAT: BenchType
BENCH_TYPE_ICON_TYPE: BenchType
BENCH_TYPE_STREAM_TYPE: BenchType
BENCH_TYPE_PRIMITIVE_TYPE: BenchType
BENCH_TYPE_FIELD_ZONE: BenchType
BENCH_TYPE_TYPE_KIND: BenchType
BENCH_TYPE_TYPE_FORMAT: BenchType
BENCH_TYPE_BLOCK_TYPE: BenchType
BENCH_TYPE_DAY: BenchType
BENCH_TYPE_MONTH: BenchType
BENCH_TYPE_TIME_INTERVAL: BenchType
BENCH_TYPE_TEXT_LINE_TYPE: BenchType
BENCH_TYPE_TEXT_SPAN_TYPE: BenchType
BENCH_TYPE_EXPRESSION_KIND: BenchType
BENCH_TYPE_EXPRESSION_OP: BenchType
BENCH_TYPE_LITERAL_TYPE: BenchType
BENCH_TYPE_FUNCTIONAL_TYPE: BenchType
BENCH_TYPE_CONDITIONAL_TYPE: BenchType
BENCH_TYPE_AGGREGATION_TYPE: BenchType
BENCH_TYPE_SORT_MODE: BenchType
BENCH_TYPE_SORT_TYPE: BenchType
BENCH_TYPE_COMPUTED_VALUE_KIND: BenchType
BENCH_TYPE_COMPUTED_VALUE_MODE: BenchType
BENCH_TYPE_PATH_ELEMENT_TYPE: BenchType
BENCH_TYPE_PATH_RUN_SELECTOR: BenchType
BENCH_TYPE_SELECTION_TYPE: BenchType
BENCH_TYPE_RUN_STATUS: BenchType
BENCH_TYPE_RUN_TYPE: BenchType
BENCH_TYPE_RUN_SPAN_TYPE: BenchType
BENCH_TYPE_PLAN_TYPE: BenchType
BENCH_TYPE_PLAN_STATUS: BenchType
BENCH_TYPE_PLAN_TERMINATION_MODE: BenchType
BENCH_TYPE_PLAN_FAILURE_MODE: BenchType
BENCH_TYPE_TASK_TYPE: BenchType
BENCH_TYPE_TASK_STATUS: BenchType
BENCH_TYPE_SESSION_STATUS: BenchType
BENCH_TYPE_CACHE_MODE: BenchType
BENCH_TYPE_LOG_TYPE: BenchType
BENCH_TYPE_SEVERITY: BenchType
BENCH_TYPE_TRIGGER_TYPE: BenchType
BENCH_TYPE_TRIGGER_EFFECT: BenchType
BENCH_TYPE_TRIGGER_STATUS: BenchType
BENCH_TYPE_SCHEDULE_FREQUENCY: BenchType
BENCH_TYPE_ERROR_KIND: BenchType
BENCH_TYPE_ERROR_TYPE: BenchType
BENCH_TYPE_BREAKPOINT_SITE: BenchType
BENCH_TYPE_BREAKPOINT_ACTION: BenchType
BENCH_TYPE_BREAKPOINT_TARGET: BenchType
BENCH_TYPE_INTERRUPTION_TYPE: BenchType
BENCH_TYPE_INTERRUPTION_STATUS: BenchType
BENCH_TYPE_INTERRUPTION_RESPONSE: BenchType
BENCH_TYPE_MODEL_DEVELOPER: BenchType
BENCH_TYPE_MODEL_TYPE: BenchType
BENCH_TYPE_MODEL_FAMILY: BenchType
BENCH_TYPE_MODEL_PROVIDER: BenchType
BENCH_TYPE_CODE_TYPE: BenchType
BENCH_TYPE_ACTION_TYPE: BenchType
BENCH_TYPE_ACTION_CATEGORY: BenchType
BENCH_TYPE_PORT_SIDE: BenchType
BENCH_TYPE_LINK_TYPE: BenchType
BENCH_TYPE_LINK_TRIGGER: BenchType
BENCH_TYPE_SPACE_TYPE: BenchType
BENCH_TYPE_VIEW_TYPE: BenchType
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
BENCH_TYPE_SIDEBAR_ASPECT: BenchType
BENCH_TYPE_CONTEXT_ASPECT: BenchType
BENCH_TYPE_BUTTON_VARIANT: BenchType
BENCH_TYPE_PICKER_VARIANT: BenchType
BENCH_TYPE_USER_STATUS: BenchType
BENCH_TYPE_ORGANIZATION_STATUS: BenchType
BENCH_TYPE_CHANNEL_TYPE: BenchType
BENCH_TYPE_THREAD_TYPE: BenchType
BENCH_TYPE_THREAD_STATUS: BenchType
BENCH_TYPE_MESSAGE_TYPE: BenchType
BENCH_TYPE_MESSAGE_STATUS: BenchType
BENCH_TYPE_MESSAGE_PLATFORM: BenchType
BENCH_TYPE_NOTIFICATION_TYPE: BenchType
BENCH_TYPE_NOTIFICATION_STATUS: BenchType
NODE_MODE_UNSPECIFIED: NodeMode
NODE_MODE_BUILTIN: NodeMode
NODE_MODE_PRODUCTION: NodeMode
NODE_MODE_DEVELOPMENT: NodeMode
NODE_MODE_TEST: NodeMode
NODE_MODE_PREVIEW: NodeMode
NODE_MODE_ARCHIVE: NodeMode
NODE_AREA_UNSPECIFIED: NodeArea
NODE_AREA_GLOBAL: NodeArea
NODE_AREA_REGIONAL: NodeArea
NODE_AREA_LOCAL: NodeArea
PROPERTY_REFERENCE_TYPE_UNSPECIFIED: PropertyReferenceType
PROPERTY_REFERENCE_TYPE_ID: PropertyReferenceType
PROPERTY_REFERENCE_TYPE_CK: PropertyReferenceType
PROPERTY_REFERENCE_TYPE_BENCH_ID: PropertyReferenceType
PROPERTY_REFERENCE_TYPE_BASE_CK: PropertyReferenceType
PROPERTY_REFERENCE_TYPE_BASE_BENCH_ID: PropertyReferenceType
PROPERTY_REFERENCE_TYPE_NODE_TYPE: PropertyReferenceType
EDIT_OPERATION_TYPE_UNSPECIFIED: EditOperationType
EDIT_OPERATION_TYPE_SET: EditOperationType
EDIT_OPERATION_TYPE_CLEAR: EditOperationType
CHANGE_CATEGORY_UNSPECIFIED: ChangeCategory
CHANGE_CATEGORY_SPACE: ChangeCategory
CHANGE_CATEGORY_RUNTIME: ChangeCategory
PACKAGE_TYPE_UNSPECIFIED: PackageType
PACKAGE_TYPE_MAIN: PackageType
PACKAGE_TYPE_SIDE: PackageType
PACKAGE_TYPE_SNAPSHOT: PackageType
CLOUD_UNSPECIFIED: Cloud
CLOUD_AWS: Cloud
CLOUD_AZURE: Cloud
CLOUD_GCP: Cloud
CLOUD_OCI: Cloud
CLOUD_ALIBABA: Cloud
CLOUD_HETZNER: Cloud
CLOUD_NEON: Cloud
CLOUD_PRIVATE: Cloud
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
REGION_ZONE_UNSPECIFIED: RegionZone
REGION_AREA_UNSPECIFIED: RegionArea
REGION_AREA_EUROPE_CENTRAL: RegionArea
REGION_AREA_NORTH_AMERICA_EAST: RegionArea
REGION_AREA_NORTH_AMERICA_WEST: RegionArea
REGION_AREA_SOUTH_AMERICA_EAST: RegionArea
REGION_AREA_MIDDLE_EAST_CENTRAL: RegionArea
REGION_AREA_MIDDLE_EAST_WEST: RegionArea
REGION_AREA_AFRICA_SOUTH: RegionArea
REGION_AREA_ASIA_WEST: RegionArea
REGION_AREA_ASIA_SOUTH: RegionArea
REGION_AREA_ASIA_EAST: RegionArea
REGION_AREA_AUSTRALIA_SOUTH: RegionArea
REGION_CONTINENT_UNSPECIFIED: RegionContinent
REGION_CONTINENT_EUROPE: RegionContinent
REGION_CONTINENT_NORTH_AMERICA: RegionContinent
REGION_CONTINENT_SOUTH_AMERICA: RegionContinent
REGION_CONTINENT_MIDDLE_EAST: RegionContinent
REGION_CONTINENT_AFRICA: RegionContinent
REGION_CONTINENT_ASIA: RegionContinent
REGION_CONTINENT_AUSTRALIA: RegionContinent
REGION_CONTINENT_PRIVATE: RegionContinent
BENCH_STATUS_UNSPECIFIED: BenchStatus
BENCH_STATUS_RESERVED: BenchStatus
BENCH_STATUS_ACTIVATED: BenchStatus
ACCESS_MODE_UNSPECIFIED: AccessMode
ACCESS_MODE_ADAPTIVE: AccessMode
ACCESS_MODE_ATOMIC: AccessMode
ACCESS_KIND_UNSPECIFIED: AccessKind
ACCESS_KIND_READ: AccessKind
ACCESS_KIND_EDIT: AccessKind
ACCESS_KIND_USE: AccessKind
POLICY_EFFECT_UNSPECIFIED: PolicyEffect
POLICY_EFFECT_ALLOW: PolicyEffect
POLICY_EFFECT_DENY: PolicyEffect
MEMBERSHIP_TYPE_UNSPECIFIED: MembershipType
MEMBERSHIP_TYPE_BENCH: MembershipType
MEMBERSHIP_TYPE_ORGANIZATION: MembershipType
MEMBERSHIP_TYPE_TEAM: MembershipType
MEMBERSHIP_TYPE_CHANNEL: MembershipType
MEMBERSHIP_TYPE_THREAD: MembershipType
INVITE_TYPE_UNSPECIFIED: InviteType
INVITE_TYPE_BENCH: InviteType
INVITE_TYPE_ORGANIZATION: InviteType
INVITE_TYPE_TEAM: InviteType
INVITE_TYPE_CHANNEL: InviteType
INVITE_TYPE_THREAD: InviteType
QUERY_TYPE_UNSPECIFIED: QueryType
QUERY_TYPE_GET: QueryType
QUERY_TYPE_SEARCH: QueryType
QUERY_TYPE_AGGREGATE: QueryType
EDIT_TYPE_UNSPECIFIED: EditType
EDIT_TYPE_CREATE: EditType
EDIT_TYPE_UPSERT: EditType
EDIT_TYPE_UPDATE: EditType
EDIT_TYPE_MOVE: EditType
EDIT_TYPE_DELETE: EditType
EDIT_TYPE_RESTORE: EditType
EDIT_TYPE_ERASE: EditType
USE_TYPE_UNSPECIFIED: UseType
USE_TYPE_START: UseType
USE_TYPE_PAUSE: UseType
USE_TYPE_RESUME: UseType
USE_TYPE_STOP: UseType
USE_TYPE_KILL: UseType
USE_TYPE_SEND: UseType
USE_TYPE_RECEIVE: UseType
ACCESS_TYPE_UNSPECIFIED: AccessType
ACCESS_TYPE_GET: AccessType
ACCESS_TYPE_SEARCH: AccessType
ACCESS_TYPE_AGGREGATE: AccessType
ACCESS_TYPE_CREATE: AccessType
ACCESS_TYPE_UPSERT: AccessType
ACCESS_TYPE_UPDATE: AccessType
ACCESS_TYPE_MOVE: AccessType
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
RESOURCE_STATUS_UNSPECIFIED: ResourceStatus
RESOURCE_STATUS_DECLARED: ResourceStatus
RESOURCE_STATUS_UP: ResourceStatus
RESOURCE_STATUS_SLEEPING: ResourceStatus
RESOURCE_STATUS_DOWN: ResourceStatus
RESOURCE_STATUS_DEGRADED: ResourceStatus
RESOURCE_STATUS_DECOMMISSIONED: ResourceStatus
RESOURCE_OCCUPANCY_UNSPECIFIED: ResourceOccupancy
RESOURCE_OCCUPANCY_AVAILABLE: ResourceOccupancy
RESOURCE_OCCUPANCY_RESERVED: ResourceOccupancy
RESOURCE_OCCUPANCY_OCCUPIED: ResourceOccupancy
RESOURCE_OCCUPANCY_DIRTY: ResourceOccupancy
SCALER_TYPE_UNSPECIFIED: ScalerType
SCALER_TYPE_MACHINE: ScalerType
SCALER_TYPE_BROWSER: ScalerType
SCALER_STRATEGY_UNSPECIFIED: ScalerStrategy
SCALER_STRATEGY_MANUAL: ScalerStrategy
SCALER_STRATEGY_AUTO: ScalerStrategy
MACHINE_TYPE_UNSPECIFIED: MachineType
MACHINE_TYPE_RUNTIME: MachineType
BROWSER_TYPE_UNSPECIFIED: BrowserType
BROWSER_TYPE_CHROMIUM: BrowserType
STORE_TYPE_UNSPECIFIED: StoreType
STORE_TYPE_POSTGRES: StoreType
CLIENT_TYPE_UNSPECIFIED: ClientType
CLIENT_TYPE_WEB: ClientType
CLIENT_TYPE_BROWSER_PLUGIN: ClientType
CLIENT_TYPE_DESKTOP: ClientType
CLIENT_TYPE_MOBILE: ClientType
CLIENT_TYPE_MACHINE: ClientType
FILE_RETENTION_MODE_UNSPECIFIED: FileRetentionMode
FILE_RETENTION_MODE_AUTOMATIC: FileRetentionMode
FILE_RETENTION_MODE_MANUAL: FileRetentionMode
FILE_RETENTION_MODE_TIMED: FileRetentionMode
FILE_KIND_UNSPECIFIED: FileKind
FILE_KIND_DRIVE: FileKind
FILE_KIND_DRIVE_INLINE: FileKind
FILE_KIND_INLINE: FileKind
FILE_KIND_EXTERNAL: FileKind
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
ICON_TYPE_UNSPECIFIED: IconType
ICON_TYPE_EMOJI: IconType
ICON_TYPE_FONT_AWESOME: IconType
ICON_TYPE_VS_CODE: IconType
STREAM_TYPE_UNSPECIFIED: StreamType
STREAM_TYPE_TEXT: StreamType
STREAM_TYPE_CODE: StreamType
STREAM_TYPE_IMAGE: StreamType
STREAM_TYPE_AUDIO: StreamType
STREAM_TYPE_VIDEO: StreamType
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
PRIMITIVE_TYPE_DURATION: PrimitiveType
FIELD_TYPE_UNSPECIFIED: FieldType
FIELD_TYPE_VARIABLE: FieldType
FIELD_TYPE_MEMBER: FieldType
FIELD_TYPE_INPUT: FieldType
FIELD_TYPE_OUTPUT: FieldType
TYPE_KIND_UNSPECIFIED: TypeKind
TYPE_KIND_PRIMITIVE: TypeKind
TYPE_KIND_STRUCT: TypeKind
TYPE_KIND_NODE: TypeKind
TYPE_KIND_ENUM: TypeKind
TYPE_KIND_BASED_NODE: TypeKind
TYPE_KIND_CUSTOM_OBJECT: TypeKind
TYPE_KIND_PARTIAL_OBJECT: TypeKind
TYPE_FORMAT_UNSPECIFIED: TypeFormat
TYPE_FORMAT_URL: TypeFormat
TYPE_FORMAT_EMAIL: TypeFormat
TYPE_FORMAT_EMOJI: TypeFormat
TYPE_FORMAT_PHONE_NUMBER: TypeFormat
TYPE_FORMAT_SLUG: TypeFormat
BLOCK_TYPE_UNSPECIFIED: BlockType
BLOCK_TYPE_FILE: BlockType
BLOCK_TYPE_PAGE: BlockType
BLOCK_TYPE_CHOICE: BlockType
BLOCK_TYPE_CLASS: BlockType
BLOCK_TYPE_TAG: BlockType
BLOCK_TYPE_FLOW: BlockType
BLOCK_TYPE_IMPLEMENTATION: BlockType
BLOCK_TYPE_VIEW: BlockType
BLOCK_TYPE_DATABASE: BlockType
BLOCK_TYPE_CHANNEL: BlockType
BLOCK_TYPE_ROLE: BlockType
BLOCK_TYPE_PLAN: BlockType
BLOCK_TYPE_TASK: BlockType
BLOCK_TYPE_PARAGRAPH: BlockType
BLOCK_TYPE_HEADING_1: BlockType
BLOCK_TYPE_HEADING_2: BlockType
BLOCK_TYPE_HEADING_3: BlockType
BLOCK_TYPE_HEADING_4: BlockType
BLOCK_TYPE_CALLOUT: BlockType
BLOCK_TYPE_QUOTE: BlockType
BLOCK_TYPE_LIST_UNORDERED: BlockType
BLOCK_TYPE_LIST_ORDERED: BlockType
BLOCK_TYPE_DIVIDER: BlockType
BLOCK_TYPE_TABLE: BlockType
BLOCK_TYPE_TABLE_ROW: BlockType
BLOCK_TYPE_CODE: BlockType
DAY_UNSPECIFIED: Day
DAY_MONDAY: Day
DAY_TUESDAY: Day
DAY_WEDNESDAY: Day
DAY_THURSDAY: Day
DAY_FRIDAY: Day
DAY_SATURDAY: Day
DAY_SUNDAY: Day
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
TIME_INTERVAL_UNSPECIFIED: TimeInterval
TIME_INTERVAL_SECOND: TimeInterval
TIME_INTERVAL_MINUTE: TimeInterval
TIME_INTERVAL_HOUR: TimeInterval
TIME_INTERVAL_DAY: TimeInterval
TIME_INTERVAL_WEEK: TimeInterval
TIME_INTERVAL_MONTH: TimeInterval
TIME_INTERVAL_YEAR: TimeInterval
TEXT_LINE_TYPE_UNSPECIFIED: TextLineType
TEXT_LINE_TYPE_PARAGRAPH: TextLineType
TEXT_LINE_TYPE_HEADING_1: TextLineType
TEXT_LINE_TYPE_HEADING_2: TextLineType
TEXT_LINE_TYPE_HEADING_3: TextLineType
TEXT_LINE_TYPE_HEADING_4: TextLineType
TEXT_LINE_TYPE_CALLOUT: TextLineType
TEXT_LINE_TYPE_QUOTE: TextLineType
TEXT_LINE_TYPE_LIST_UNORDERED: TextLineType
TEXT_LINE_TYPE_LIST_ORDERED: TextLineType
TEXT_LINE_TYPE_DIVIDER: TextLineType
TEXT_LINE_TYPE_TABLE: TextLineType
TEXT_LINE_TYPE_TABLE_ROW: TextLineType
TEXT_LINE_TYPE_CODE: TextLineType
TEXT_SPAN_TYPE_UNSPECIFIED: TextSpanType
TEXT_SPAN_TYPE_TEXT: TextSpanType
TEXT_SPAN_TYPE_HARD_BREAK: TextSpanType
TEXT_SPAN_TYPE_MENTION: TextSpanType
TEXT_SPAN_TYPE_LINK: TextSpanType
TEXT_SPAN_TYPE_CITATION: TextSpanType
TEXT_SPAN_TYPE_EQUATION: TextSpanType
EXPRESSION_KIND_UNSPECIFIED: ExpressionKind
EXPRESSION_KIND_LITERAL: ExpressionKind
EXPRESSION_KIND_FUNCTIONAL: ExpressionKind
EXPRESSION_KIND_CONDITIONAL: ExpressionKind
EXPRESSION_KIND_SORT: ExpressionKind
EXPRESSION_KIND_AGGREGATION: ExpressionKind
EXPRESSION_TYPE_UNSPECIFIED: ExpressionType
EXPRESSION_TYPE_VALUE: ExpressionType
EXPRESSION_TYPE_NONE: ExpressionType
EXPRESSION_TYPE_TRUE: ExpressionType
EXPRESSION_TYPE_FALSE: ExpressionType
EXPRESSION_TYPE_ADD: ExpressionType
EXPRESSION_TYPE_SUBTRACT: ExpressionType
EXPRESSION_TYPE_MULTIPLY: ExpressionType
EXPRESSION_TYPE_DIVIDE: ExpressionType
EXPRESSION_TYPE_MODULO: ExpressionType
EXPRESSION_TYPE_POWER: ExpressionType
EXPRESSION_TYPE_NOT: ExpressionType
EXPRESSION_TYPE_AND: ExpressionType
EXPRESSION_TYPE_OR: ExpressionType
EXPRESSION_TYPE_EQUALS: ExpressionType
EXPRESSION_TYPE_NOT_EQUALS: ExpressionType
EXPRESSION_TYPE_GREATER_THAN: ExpressionType
EXPRESSION_TYPE_GREATER_THAN_OR_EQUALS: ExpressionType
EXPRESSION_TYPE_LESS_THAN: ExpressionType
EXPRESSION_TYPE_LESS_THAN_OR_EQUALS: ExpressionType
EXPRESSION_TYPE_MATCHES: ExpressionType
EXPRESSION_TYPE_STARTS_WITH: ExpressionType
EXPRESSION_TYPE_ENDS_WITH: ExpressionType
EXPRESSION_TYPE_MATCHES_REGEX: ExpressionType
EXPRESSION_TYPE_CONTAINS: ExpressionType
EXPRESSION_TYPE_NOT_CONTAINS: ExpressionType
EXPRESSION_TYPE_IN: ExpressionType
EXPRESSION_TYPE_NOT_IN: ExpressionType
EXPRESSION_TYPE_EXISTS: ExpressionType
EXPRESSION_TYPE_NOT_EXISTS: ExpressionType
EXPRESSION_TYPE_NEAR: ExpressionType
EXPRESSION_TYPE_EXISTENCE: ExpressionType
EXPRESSION_TYPE_COUNT: ExpressionType
EXPRESSION_TYPE_SUM: ExpressionType
EXPRESSION_TYPE_MIN: ExpressionType
EXPRESSION_TYPE_MAX: ExpressionType
EXPRESSION_TYPE_AVERAGE: ExpressionType
EXPRESSION_TYPE_MEDIAN: ExpressionType
EXPRESSION_TYPE_HISTOGRAM: ExpressionType
EXPRESSION_TYPE_ASCENDING: ExpressionType
EXPRESSION_TYPE_DESCENDING: ExpressionType
LITERAL_TYPE_UNSPECIFIED: LiteralType
LITERAL_TYPE_VALUE: LiteralType
LITERAL_TYPE_NONE: LiteralType
LITERAL_TYPE_TRUE: LiteralType
LITERAL_TYPE_FALSE: LiteralType
FUNCTIONAL_TYPE_UNSPECIFIED: FunctionalType
FUNCTIONAL_TYPE_ADD: FunctionalType
FUNCTIONAL_TYPE_SUBTRACT: FunctionalType
FUNCTIONAL_TYPE_MULTIPLY: FunctionalType
FUNCTIONAL_TYPE_DIVIDE: FunctionalType
FUNCTIONAL_TYPE_MODULO: FunctionalType
FUNCTIONAL_TYPE_POWER: FunctionalType
CONDITIONAL_TYPE_UNSPECIFIED: ConditionalType
CONDITIONAL_TYPE_NOT: ConditionalType
CONDITIONAL_TYPE_AND: ConditionalType
CONDITIONAL_TYPE_OR: ConditionalType
CONDITIONAL_TYPE_EQUALS: ConditionalType
CONDITIONAL_TYPE_NOT_EQUALS: ConditionalType
CONDITIONAL_TYPE_GREATER_THAN: ConditionalType
CONDITIONAL_TYPE_GREATER_THAN_OR_EQUALS: ConditionalType
CONDITIONAL_TYPE_LESS_THAN: ConditionalType
CONDITIONAL_TYPE_LESS_THAN_OR_EQUALS: ConditionalType
CONDITIONAL_TYPE_MATCHES: ConditionalType
CONDITIONAL_TYPE_STARTS_WITH: ConditionalType
CONDITIONAL_TYPE_ENDS_WITH: ConditionalType
CONDITIONAL_TYPE_MATCHES_REGEX: ConditionalType
CONDITIONAL_TYPE_CONTAINS: ConditionalType
CONDITIONAL_TYPE_NOT_CONTAINS: ConditionalType
CONDITIONAL_TYPE_IN: ConditionalType
CONDITIONAL_TYPE_NOT_IN: ConditionalType
CONDITIONAL_TYPE_EXISTS: ConditionalType
CONDITIONAL_TYPE_NOT_EXISTS: ConditionalType
CONDITIONAL_TYPE_NEAR: ConditionalType
AGGREGATION_TYPE_UNSPECIFIED: AggregationType
AGGREGATION_TYPE_EXISTENCE: AggregationType
AGGREGATION_TYPE_COUNT: AggregationType
AGGREGATION_TYPE_SUM: AggregationType
AGGREGATION_TYPE_MIN: AggregationType
AGGREGATION_TYPE_MAX: AggregationType
AGGREGATION_TYPE_AVERAGE: AggregationType
AGGREGATION_TYPE_MEDIAN: AggregationType
AGGREGATION_TYPE_HISTOGRAM: AggregationType
SORT_MODE_UNSPECIFIED: SortMode
SORT_MODE_MAX: SortMode
SORT_MODE_MIN: SortMode
SORT_MODE_AVERAGE: SortMode
SORT_MODE_SUM: SortMode
SORT_MODE_MEDIAN: SortMode
SORT_TYPE_UNSPECIFIED: SortType
SORT_TYPE_ASCENDING: SortType
SORT_TYPE_DESCENDING: SortType
COMPUTED_VALUE_KIND_UNSPECIFIED: ComputedValueKind
COMPUTED_VALUE_KIND_PATH: ComputedValueKind
COMPUTED_VALUE_KIND_EXPRESSION: ComputedValueKind
COMPUTED_VALUE_KIND_CODE: ComputedValueKind
COMPUTED_VALUE_MODE_UNSPECIFIED: ComputedValueMode
COMPUTED_VALUE_MODE_ALWAYS: ComputedValueMode
COMPUTED_VALUE_MODE_IF_SOURCE_SET: ComputedValueMode
COMPUTED_VALUE_MODE_IF_TARGET_UNSET: ComputedValueMode
PATH_ELEMENT_TYPE_UNSPECIFIED: PathElementType
PATH_ELEMENT_TYPE_ROOT: PathElementType
PATH_ELEMENT_TYPE_BENCH: PathElementType
PATH_ELEMENT_TYPE_PACKAGE: PathElementType
PATH_ELEMENT_TYPE_NODE: PathElementType
PATH_ELEMENT_TYPE_CURRENT: PathElementType
PATH_ELEMENT_TYPE_CONTAINER: PathElementType
PATH_ELEMENT_TYPE_CLOSEST: PathElementType
PATH_ELEMENT_TYPE_PARENT: PathElementType
PATH_ELEMENT_TYPE_CHILD: PathElementType
PATH_ELEMENT_TYPE_ATTRIBUTE: PathElementType
PATH_ELEMENT_TYPE_CONTEXT: PathElementType
PATH_ELEMENT_TYPE_RUN: PathElementType
PATH_RUN_SELECTOR_UNSPECIFIED: PathRunSelector
PATH_RUN_SELECTOR_LATEST: PathRunSelector
SELECTION_TYPE_UNSPECIFIED: SelectionType
SELECTION_TYPE_LIST: SelectionType
SELECTION_TYPE_RANGE: SelectionType
SELECTION_TYPE_SCOPE: SelectionType
SELECTION_TYPE_TAG: SelectionType
SELECTION_TYPE_COMBINATION: SelectionType
RUN_STATUS_UNSPECIFIED: RunStatus
RUN_STATUS_CREATED: RunStatus
RUN_STATUS_SCHEDULED: RunStatus
RUN_STATUS_QUEUED: RunStatus
RUN_STATUS_RUNNING: RunStatus
RUN_STATUS_PAUSED: RunStatus
RUN_STATUS_YIELDED: RunStatus
RUN_STATUS_WAITING: RunStatus
RUN_STATUS_CANCELLED: RunStatus
RUN_STATUS_ABORTED: RunStatus
RUN_STATUS_FAILED: RunStatus
RUN_STATUS_COMPLETED: RunStatus
RUN_TYPE_UNSPECIFIED: RunType
RUN_TYPE_CODE: RunType
RUN_TYPE_ACTION: RunType
RUN_TYPE_FLOW: RunType
RUN_TYPE_LINK: RunType
RUN_SPAN_TYPE_UNSPECIFIED: RunSpanType
RUN_SPAN_TYPE_ATTEMPT: RunSpanType
RUN_SPAN_TYPE_WAIT: RunSpanType
RUN_SPAN_TYPE_ACQUIRE: RunSpanType
RUN_SPAN_TYPE_DELEGATE: RunSpanType
RUN_SPAN_TYPE_FLOW_PLAN: RunSpanType
RUN_SPAN_TYPE_MODEL_PREPARE: RunSpanType
RUN_SPAN_TYPE_MODEL_GENERATE: RunSpanType
RUN_SPAN_TYPE_MODEL_PARSE: RunSpanType
RUN_SPAN_TYPE_FILE_UPLOAD: RunSpanType
RUN_SPAN_TYPE_FILE_PREPARE_UPLOAD: RunSpanType
RUN_SPAN_TYPE_FILE_DOWNLOAD: RunSpanType
RUN_SPAN_TYPE_FILE_PREPARE_DOWNLOAD: RunSpanType
PLAN_TYPE_UNSPECIFIED: PlanType
PLAN_TYPE_SERIAL: PlanType
PLAN_TYPE_PARALLEL: PlanType
PLAN_STATUS_UNSPECIFIED: PlanStatus
PLAN_STATUS_CREATED: PlanStatus
PLAN_STATUS_RUNNING: PlanStatus
PLAN_STATUS_CANCELLED: PlanStatus
PLAN_STATUS_ABORTED: PlanStatus
PLAN_STATUS_FAILED: PlanStatus
PLAN_STATUS_COMPLETED: PlanStatus
PLAN_TERMINATION_MODE_UNSPECIFIED: PlanTerminationMode
PLAN_TERMINATION_MODE_PASS: PlanTerminationMode
PLAN_TERMINATION_MODE_RETURN: PlanTerminationMode
PLAN_FAILURE_MODE_UNSPECIFIED: PlanFailureMode
PLAN_FAILURE_MODE_FAIL: PlanFailureMode
PLAN_FAILURE_MODE_COMPLETE: PlanFailureMode
PLAN_FAILURE_MODE_CONTINUE: PlanFailureMode
TASK_TYPE_UNSPECIFIED: TaskType
TASK_TYPE_GENERIC: TaskType
TASK_TYPE_RUN: TaskType
TASK_STATUS_UNSPECIFIED: TaskStatus
TASK_STATUS_CREATED: TaskStatus
TASK_STATUS_ASSIGNED: TaskStatus
TASK_STATUS_RUNNING: TaskStatus
TASK_STATUS_WAITING: TaskStatus
TASK_STATUS_REVIEWING: TaskStatus
TASK_STATUS_CANCELLED: TaskStatus
TASK_STATUS_ABORTED: TaskStatus
TASK_STATUS_FAILED: TaskStatus
TASK_STATUS_COMPLETED: TaskStatus
SESSION_STATUS_UNSPECIFIED: SessionStatus
SESSION_STATUS_PENDING: SessionStatus
SESSION_STATUS_OPEN: SessionStatus
SESSION_STATUS_CLOSED: SessionStatus
CACHE_MODE_UNSPECIFIED: CacheMode
CACHE_MODE_NEVER: CacheMode
CACHE_MODE_ALWAYS: CacheMode
LOG_TYPE_UNSPECIFIED: LogType
LOG_TYPE_PRINT: LogType
LOG_TYPE_CHANGE: LogType
LOG_TYPE_EDIT: LogType
SEVERITY_UNSPECIFIED: Severity
SEVERITY_TRACE: Severity
SEVERITY_DEBUG: Severity
SEVERITY_INFO: Severity
SEVERITY_WARNING: Severity
SEVERITY_ERROR: Severity
SEVERITY_PANIC: Severity
TRIGGER_TYPE_UNSPECIFIED: TriggerType
TRIGGER_TYPE_SCHEDULE: TriggerType
TRIGGER_TYPE_MESSAGE: TriggerType
TRIGGER_EFFECT_UNSPECIFIED: TriggerEffect
TRIGGER_EFFECT_START_RUN: TriggerEffect
TRIGGER_EFFECT_ENSURE_RUN: TriggerEffect
TRIGGER_EFFECT_REPLACE_RUN: TriggerEffect
TRIGGER_STATUS_UNSPECIFIED: TriggerStatus
TRIGGER_STATUS_INACTIVE: TriggerStatus
TRIGGER_STATUS_OPEN: TriggerStatus
TRIGGER_STATUS_CLOSED: TriggerStatus
SCHEDULE_FREQUENCY_UNSPECIFIED: ScheduleFrequency
SCHEDULE_FREQUENCY_YEAR: ScheduleFrequency
SCHEDULE_FREQUENCY_MONTH: ScheduleFrequency
SCHEDULE_FREQUENCY_WEEK: ScheduleFrequency
SCHEDULE_FREQUENCY_DAY: ScheduleFrequency
SCHEDULE_FREQUENCY_HOUR: ScheduleFrequency
SCHEDULE_FREQUENCY_MINUTE: ScheduleFrequency
ERROR_KIND_UNSPECIFIED: ErrorKind
ERROR_KIND_INTERNAL: ErrorKind
ERROR_KIND_RUNTIME: ErrorKind
ERROR_TYPE_UNSPECIFIED: ErrorType
ERROR_TYPE_ABORTED: ErrorType
ERROR_TYPE_RUNTIME_UNAVAILABLE: ErrorType
ERROR_TYPE_RUN_IMPOSSIBLE: ErrorType
ERROR_TYPE_NOT_SUPPORTED: ErrorType
ERROR_TYPE_INVALID_VALUE: ErrorType
ERROR_TYPE_INVALID_COMPUTED: ErrorType
ERROR_TYPE_CODE_INVALID: ErrorType
ERROR_TYPE_TEXT_INVALID: ErrorType
ERROR_TYPE_INCAPABLE: ErrorType
ERROR_TYPE_REFUSED: ErrorType
ERROR_TYPE_NON_RETRYABLE: ErrorType
ERROR_TYPE_MODEL_FAILED: ErrorType
ERROR_TYPE_INVALID_CONTINUATION: ErrorType
ERROR_TYPE_INVALID_CALL: ErrorType
ERROR_TYPE_INVALID_PLAN: ErrorType
ERROR_TYPE_INTERRUPTION_CANCELLED: ErrorType
ERROR_TYPE_RETRYABLE: ErrorType
BREAKPOINT_SITE_UNSPECIFIED: BreakpointSite
BREAKPOINT_SITE_RUN_BEFORE: BreakpointSite
BREAKPOINT_SITE_RUN_AFTER_FAILED: BreakpointSite
BREAKPOINT_SITE_RUN_AFTER_COMPLETED: BreakpointSite
BREAKPOINT_SITE_RUN_AFTER: BreakpointSite
BREAKPOINT_ACTION_UNSPECIFIED: BreakpointAction
BREAKPOINT_ACTION_YIELD: BreakpointAction
BREAKPOINT_SCOPE_UNSPECIFIED: BreakpointScope
BREAKPOINT_SCOPE_SELF: BreakpointScope
BREAKPOINT_SCOPE_CHILD: BreakpointScope
BREAKPOINT_SCOPE_ACTION: BreakpointScope
BREAKPOINT_SCOPE_LINK: BreakpointScope
INTERRUPTION_TYPE_UNSPECIFIED: InterruptionType
INTERRUPTION_TYPE_PAUSE: InterruptionType
INTERRUPTION_TYPE_YIELD: InterruptionType
INTERRUPTION_TYPE_WAIT: InterruptionType
INTERRUPTION_STATUS_UNSPECIFIED: InterruptionStatus
INTERRUPTION_STATUS_OPEN: InterruptionStatus
INTERRUPTION_STATUS_CANCELLED: InterruptionStatus
INTERRUPTION_STATUS_COMPLETED: InterruptionStatus
INTERRUPTION_RESPONSE_UNSPECIFIED: InterruptionResponse
INTERRUPTION_RESPONSE_ACCEPT: InterruptionResponse
INTERRUPTION_RESPONSE_REJECT: InterruptionResponse
MODEL_DEVELOPER_UNSPECIFIED: ModelDeveloper
MODEL_DEVELOPER_META: ModelDeveloper
MODEL_DEVELOPER_OPENAI: ModelDeveloper
MODEL_DEVELOPER_ANTHROPIC: ModelDeveloper
MODEL_DEVELOPER_GOOGLE: ModelDeveloper
MODEL_DEVELOPER_AMAZON: ModelDeveloper
MODEL_DEVELOPER_MICROSOFT: ModelDeveloper
MODEL_DEVELOPER_DEEPSEEK: ModelDeveloper
MODEL_TYPE_UNSPECIFIED: ModelType
MODEL_TYPE_META_LLAMA_3_1_80B: ModelType
MODEL_TYPE_META_LLAMA_3_1_400B: ModelType
MODEL_TYPE_OPENAI_GPT4_0: ModelType
MODEL_TYPE_OPENAI_GPT4_O_MINI: ModelType
MODEL_TYPE_OPENAI_GPT4_5: ModelType
MODEL_TYPE_OPENAI_O1: ModelType
MODEL_TYPE_OPENAI_O1_MINI: ModelType
MODEL_TYPE_OPENAI_O3_MINI: ModelType
MODEL_TYPE_ANTHROPIC_CLAUDE_3_5_SONNET: ModelType
MODEL_TYPE_ANTHROPIC_CLAUDE_3_7_SONNET: ModelType
MODEL_TYPE_GOOGLE_GEMINI_2_0_FLASH: ModelType
MODEL_TYPE_GOOGLE_GEMINI_2_0_FLASH_THINKING: ModelType
MODEL_TYPE_DEEPSEEK_R_1_0: ModelType
MODEL_FAMILY_UNSPECIFIED: ModelFamily
MODEL_FAMILY_META_LLAMA: ModelFamily
MODEL_FAMILY_OPENAI_GPT: ModelFamily
MODEL_FAMILY_OPENAI_O: ModelFamily
MODEL_FAMILY_ANTHROPIC_CLAUDE: ModelFamily
MODEL_FAMILY_GOOGLE_GEMINI: ModelFamily
MODEL_FAMILY_DEEPSEEK_R: ModelFamily
MODEL_PROVIDER_UNSPECIFIED: ModelProvider
MODEL_PROVIDER_META: ModelProvider
MODEL_PROVIDER_OPENAI: ModelProvider
MODEL_PROVIDER_ANTHROPIC: ModelProvider
MODEL_PROVIDER_GOOGLE: ModelProvider
MODEL_PROVIDER_AMAZON: ModelProvider
MODEL_PROVIDER_MICROSOFT: ModelProvider
MODEL_PROVIDER_DEEPSEEK: ModelProvider
CODE_TYPE_UNSPECIFIED: CodeType
CODE_TYPE_SNIPPET: CodeType
CODE_TYPE_SCRIPT: CodeType
CODE_TYPE_FUNCTION: CodeType
ACTION_TYPE_UNSPECIFIED: ActionType
ACTION_TYPE_START: ActionType
ACTION_TYPE_COMPLETE: ActionType
ACTION_TYPE_FAIL: ActionType
ACTION_TYPE_TOOL: ActionType
ACTION_TYPE_CODE: ActionType
ACTION_TYPE_DO: ActionType
ACTION_TYPE_THINK: ActionType
ACTION_TYPE_ROUTE: ActionType
ACTION_TYPE_GENERATE: ActionType
ACTION_TYPE_TRANSFORM: ActionType
ACTION_TYPE_EXTRACT: ActionType
ACTION_TYPE_CLASSIFY: ActionType
ACTION_TYPE_SUMMARIZE: ActionType
ACTION_TYPE_COMPARE: ActionType
ACTION_TYPE_TRANSLATE: ActionType
ACTION_TYPE_EDIT: ActionType
ACTION_TYPE_CREATE: ActionType
ACTION_TYPE_DUPLICATE: ActionType
ACTION_TYPE_UPDATE: ActionType
ACTION_TYPE_DELETE: ActionType
ACTION_TYPE_WAIT: ActionType
ACTION_TYPE_SEND: ActionType
ACTION_TYPE_RECEIVE: ActionType
ACTION_TYPE_YIELD: ActionType
ACTION_TYPE_LOOK: ActionType
ACTION_TYPE_CLICK: ActionType
ACTION_TYPE_PRESS: ActionType
ACTION_TYPE_TYPE: ActionType
ACTION_TYPE_SCROLL: ActionType
ACTION_TYPE_SELECT: ActionType
ACTION_TYPE_DRAG: ActionType
ACTION_TYPE_GO_BACKWARD: ActionType
ACTION_TYPE_GO_FORWARD: ActionType
ACTION_TYPE_GO_TO_URL: ActionType
ACTION_TYPE_GO_TO_TAB: ActionType
ACTION_TYPE_OPEN_TAB: ActionType
ACTION_TYPE_CLOSE_TAB: ActionType
ACTION_TYPE_HTTP: ActionType
ACTION_TYPE_REST: ActionType
ACTION_TYPE_GRAPHQL: ActionType
ACTION_TYPE_SQL: ActionType
ACTION_TYPE_WEB: ActionType
ACTION_CATEGORY_UNSPECIFIED: ActionCategory
ACTION_CATEGORY_READ: ActionCategory
ACTION_CATEGORY_SEARCH: ActionCategory
ACTION_CATEGORY_BROWSE: ActionCategory
ACTION_CATEGORY_TYPE: ActionCategory
ACTION_CATEGORY_SPEAK: ActionCategory
ACTION_CATEGORY_THINK: ActionCategory
ACTION_CATEGORY_WAIT: ActionCategory
ACTION_CATEGORY_WORK: ActionCategory
ACTION_CATEGORY_INTERACT: ActionCategory
PORT_SIDE_UNSPECIFIED: PortSide
PORT_SIDE_INCOMING: PortSide
PORT_SIDE_OUTGOING: PortSide
LINK_TYPE_UNSPECIFIED: LinkType
LINK_TYPE_DECIDE: LinkType
LINK_TYPE_REQUIRE: LinkType
LINK_TRIGGER_UNSPECIFIED: LinkTrigger
LINK_TRIGGER_ON_COMPLETED: LinkTrigger
LINK_TRIGGER_ON_FAILED: LinkTrigger
LINK_TRIGGER_ON_TERMINATED: LinkTrigger
SPACE_TYPE_UNSPECIFIED: SpaceType
SPACE_TYPE_BROWSER: SpaceType
SPACE_TYPE_DESKTOP: SpaceType
SPACE_TYPE_MOBILE: SpaceType
VIEW_TYPE_UNSPECIFIED: ViewType
VIEW_TYPE_MACHINE: ViewType
VIEW_TYPE_BROWSER: ViewType
VIEW_TYPE_PAGE: ViewType
VIEW_TYPE_BLOCK: ViewType
VIEW_TYPE_CHOICE: ViewType
VIEW_TYPE_CLASS: ViewType
VIEW_TYPE_FIELD: ViewType
VIEW_TYPE_FLOW: ViewType
VIEW_TYPE_ACTION: ViewType
VIEW_TYPE_LINK: ViewType
VIEW_TYPE_TRIGGER: ViewType
VIEW_TYPE_IMPLEMENTATION: ViewType
VIEW_TYPE_VIEW: ViewType
VIEW_TYPE_DATABASE: ViewType
VIEW_TYPE_CHANNEL: ViewType
VIEW_TYPE_THREAD: ViewType
VIEW_TYPE_RUN: ViewType
VIEW_TYPE_OBJECT: ViewType
VIEW_TYPE_TYPE: ViewType
VIEW_TYPE_FIELD_LIST: ViewType
VIEW_TYPE_PATH: ViewType
VIEW_TYPE_COMPUTED_VALUE: ViewType
VIEW_TYPE_SELECTION: ViewType
VIEW_TYPE_USER_WIZARD: ViewType
VIEW_TYPE_BENCH_WIZARD: ViewType
VIEW_TYPE_EMPTY: ViewType
VIEW_TYPE_CREATE: ViewType
VIEW_TYPE_CHAT: ViewType
VIEW_TYPE_SIDEBAR: ViewType
VIEW_TYPE_CONTEXT: ViewType
VIEW_TYPE_ACTIVITY: ViewType
VIEW_TYPE_CATALOG: ViewType
VIEW_TYPE_WINDOW: ViewType
VIEW_TYPE_TAB: ViewType
VIEW_TYPE_HISTORY: ViewType
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
VIEW_TYPE_TREE: ViewType
VIEW_TYPE_FEED: ViewType
VIEW_TYPE_GALLERY: ViewType
VIEW_TYPE_BOARD: ViewType
VIEW_TYPE_BREADCRUMB: ViewType
VIEW_TYPE_PROGRESS: ViewType
VIEW_TYPE_AVATAR: ViewType
VIEW_TYPE_BADGE: ViewType
VIEW_TYPE_SHAPE: ViewType
VIEW_TYPE_CHART: ViewType
VIEW_TYPE_BUTTON: ViewType
VIEW_TYPE_MULTI_BUTTON: ViewType
VIEW_TYPE_NUMBER: ViewType
VIEW_TYPE_SLIDER: ViewType
VIEW_TYPE_STRING: ViewType
VIEW_TYPE_TEXT: ViewType
VIEW_TYPE_CODE: ViewType
VIEW_TYPE_JSON: ViewType
VIEW_TYPE_TOGGLE: ViewType
VIEW_TYPE_PICKER: ViewType
VIEW_TYPE_COLOR: ViewType
VIEW_TYPE_ICON: ViewType
VIEW_TYPE_DATETIME: ViewType
VIEW_TYPE_DURATION: ViewType
VIEW_TYPE_FILE: ViewType
VIEW_TYPE_IMAGE: ViewType
VIEW_TYPE_AUDIO: ViewType
VIEW_TYPE_VIDEO: ViewType
VIEW_TYPE_DOCUMENT: ViewType
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
ORIENTATION_UNSPECIFIED: Orientation
ORIENTATION_HORIZONTAL: Orientation
ORIENTATION_HORIZONTAL_REVERSED: Orientation
ORIENTATION_VERTICAL: Orientation
ORIENTATION_VERTICAL_REVERSED: Orientation
ALIGNMENT_UNSPECIFIED: Alignment
ALIGNMENT_START: Alignment
ALIGNMENT_MIDDLE: Alignment
ALIGNMENT_END: Alignment
ALIGNMENT_SPACE_BETWEEN: Alignment
USER_WIZARD_VIEW_STAGE_UNSPECIFIED: UserWizardViewStage
USER_WIZARD_VIEW_STAGE_SIGN_UP: UserWizardViewStage
USER_WIZARD_VIEW_STAGE_LOG_IN: UserWizardViewStage
TREE_VIEW_PRESET_UNSPECIFIED: TreeViewPreset
TREE_VIEW_PRESET_PACKAGE: TreeViewPreset
TREE_VIEW_PRESET_OUTLINE: TreeViewPreset
SIDEBAR_ASPECT_UNSPECIFIED: SidebarAspect
SIDEBAR_ASPECT_BENCH: SidebarAspect
SIDEBAR_ASPECT_ACTIVITY: SidebarAspect
SIDEBAR_ASPECT_CATALOG: SidebarAspect
SIDEBAR_ASPECT_LIBRARY: SidebarAspect
CONTEXT_ASPECT_UNSPECIFIED: ContextAspect
CONTEXT_ASPECT_DETAIL: ContextAspect
CONTEXT_ASPECT_RUN: ContextAspect
CONTEXT_ASPECT_CHAT: ContextAspect
CONTEXT_ASPECT_LOG: ContextAspect
BUTTON_VARIANT_UNSPECIFIED: ButtonVariant
BUTTON_VARIANT_PRIMARY: ButtonVariant
BUTTON_VARIANT_SECONDARY: ButtonVariant
BUTTON_VARIANT_LINK: ButtonVariant
PICKER_VARIANT_UNSPECIFIED: PickerVariant
PICKER_VARIANT_MULTI_TOGGLE: PickerVariant
PICKER_VARIANT_DROPDOWN: PickerVariant
PICKER_VARIANT_DROPDOWN_LARGE: PickerVariant
USER_STATUS_UNSPECIFIED: UserStatus
USER_STATUS_INVITED: UserStatus
USER_STATUS_RESERVED: UserStatus
USER_STATUS_WAITLISTED: UserStatus
USER_STATUS_REGISTERED: UserStatus
USER_STATUS_ACTIVATED: UserStatus
ORGANIZATION_STATUS_UNSPECIFIED: OrganizationStatus
ORGANIZATION_STATUS_REGISTERED: OrganizationStatus
ORGANIZATION_STATUS_ACTIVATED: OrganizationStatus
CHANNEL_TYPE_UNSPECIFIED: ChannelType
CHANNEL_TYPE_TEXT: ChannelType
THREAD_TYPE_UNSPECIFIED: ThreadType
THREAD_TYPE_SOURCE: ThreadType
THREAD_TYPE_RUN: ThreadType
THREAD_STATUS_UNSPECIFIED: ThreadStatus
THREAD_STATUS_OPEN: ThreadStatus
THREAD_STATUS_CLOSED: ThreadStatus
MESSAGE_TYPE_UNSPECIFIED: MessageType
MESSAGE_TYPE_REGULAR: MessageType
MESSAGE_TYPE_REPLY: MessageType
MESSAGE_TYPE_FORWARDED: MessageType
MESSAGE_TYPE_THREAD: MessageType
MESSAGE_TYPE_RUN: MessageType
MESSAGE_TYPE_INTERRUPTION: MessageType
MESSAGE_TYPE_POLL: MessageType
MESSAGE_TYPE_EDIT: MessageType
MESSAGE_STATUS_UNSPECIFIED: MessageStatus
MESSAGE_STATUS_DRAFT: MessageStatus
MESSAGE_STATUS_SENDING: MessageStatus
MESSAGE_STATUS_SENT: MessageStatus
MESSAGE_STATUS_FAILED: MessageStatus
MESSAGE_STATUS_RECEIVED: MessageStatus
MESSAGE_STATUS_READ: MessageStatus
MESSAGE_PLATFORM_UNSPECIFIED: MessagePlatform
MESSAGE_PLATFORM_BENCH: MessagePlatform
MESSAGE_PLATFORM_WEBHOOK: MessagePlatform
MESSAGE_PLATFORM_EMAIL: MessagePlatform
MESSAGE_PLATFORM_SMS: MessagePlatform
NOTIFICATION_TYPE_UNSPECIFIED: NotificationType
NOTIFICATION_TYPE_MESSAGE: NotificationType
NOTIFICATION_STATUS_UNSPECIFIED: NotificationStatus
NOTIFICATION_STATUS_SENDING: NotificationStatus
NOTIFICATION_STATUS_SENT: NotificationStatus
NOTIFICATION_STATUS_FAILED: NotificationStatus
NOTIFICATION_STATUS_RECEIVED: NotificationStatus
NOTIFICATION_STATUS_READ: NotificationStatus
BUILTIN_ENUM_UNSPECIFIED: BuiltinEnum
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

class GraphScopeData(_message.Message):
    __slots__ = ("metatype", "bench_id", "package_id")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    BENCH_ID_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_ID_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    bench_id: str
    package_id: str
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., bench_id: _Optional[str] = ..., package_id: _Optional[str] = ...) -> None: ...

class PropertyReferenceData(_message.Message):
    __slots__ = ("metatype", "object_type", "id", "node_subtype", "references_node_type", "references_meta")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    OBJECT_TYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    NODE_SUBTYPE_FIELD_NUMBER: _ClassVar[int]
    REFERENCES_NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    REFERENCES_META_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    object_type: ObjectType
    id: int
    node_subtype: int
    references_node_type: NodeType
    references_meta: PropertyReferenceType
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., object_type: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[int] = ..., node_subtype: _Optional[int] = ..., references_node_type: _Optional[_Union[NodeType, str]] = ..., references_meta: _Optional[_Union[PropertyReferenceType, str]] = ...) -> None: ...

class NodeReferenceData(_message.Message):
    __slots__ = ("metatype", "node_type", "id", "ck", "bench_id", "base_ck", "base_bench_id")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    BENCH_ID_FIELD_NUMBER: _ClassVar[int]
    BASE_CK_FIELD_NUMBER: _ClassVar[int]
    BASE_BENCH_ID_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    node_type: NodeType
    id: str
    ck: str
    bench_id: str
    base_ck: str
    base_bench_id: str
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., node_type: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., ck: _Optional[str] = ..., bench_id: _Optional[str] = ..., base_ck: _Optional[str] = ..., base_bench_id: _Optional[str] = ...) -> None: ...

class CodeLineData(_message.Message):
    __slots__ = ("metatype", "content")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    CONTENT_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    content: str
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., content: _Optional[str] = ...) -> None: ...

class CodeData(_message.Message):
    __slots__ = ("metatype", "language", "lines")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    LANGUAGE_FIELD_NUMBER: _ClassVar[int]
    LINES_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    language: str
    lines: _containers.RepeatedCompositeFieldContainer[CodeLineData]
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., language: _Optional[str] = ..., lines: _Optional[_Iterable[_Union[CodeLineData, _Mapping]]] = ...) -> None: ...

class TextSpanData(_message.Message):
    __slots__ = ("metatype", "type", "content", "node_ptr", "url", "color", "background_color", "is_bold", "is_italic", "is_strikethrough", "is_underline", "is_code")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    CONTENT_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    URL_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    BACKGROUND_COLOR_FIELD_NUMBER: _ClassVar[int]
    IS_BOLD_FIELD_NUMBER: _ClassVar[int]
    IS_ITALIC_FIELD_NUMBER: _ClassVar[int]
    IS_STRIKETHROUGH_FIELD_NUMBER: _ClassVar[int]
    IS_UNDERLINE_FIELD_NUMBER: _ClassVar[int]
    IS_CODE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    type: TextSpanType
    content: str
    node_ptr: NodeReferenceData
    url: str
    color: ColorType
    background_color: ColorType
    is_bold: bool
    is_italic: bool
    is_strikethrough: bool
    is_underline: bool
    is_code: bool
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., type: _Optional[_Union[TextSpanType, str]] = ..., content: _Optional[str] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., url: _Optional[str] = ..., color: _Optional[_Union[ColorType, str]] = ..., background_color: _Optional[_Union[ColorType, str]] = ..., is_bold: bool = ..., is_italic: bool = ..., is_strikethrough: bool = ..., is_underline: bool = ..., is_code: bool = ...) -> None: ...

class TextTableData(_message.Message):
    __slots__ = ("metatype", "lines", "has_row_header", "has_column_header")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    LINES_FIELD_NUMBER: _ClassVar[int]
    HAS_ROW_HEADER_FIELD_NUMBER: _ClassVar[int]
    HAS_COLUMN_HEADER_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    lines: _containers.RepeatedCompositeFieldContainer[TextLineData]
    has_row_header: bool
    has_column_header: bool
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., lines: _Optional[_Iterable[_Union[TextLineData, _Mapping]]] = ..., has_row_header: bool = ..., has_column_header: bool = ...) -> None: ...

class TextCellData(_message.Message):
    __slots__ = ("metatype", "spans", "colspan", "rowspan", "alignment", "color", "background_color", "is_bold", "is_italic", "is_strikethrough", "is_underline", "is_code")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    SPANS_FIELD_NUMBER: _ClassVar[int]
    COLSPAN_FIELD_NUMBER: _ClassVar[int]
    ROWSPAN_FIELD_NUMBER: _ClassVar[int]
    ALIGNMENT_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    BACKGROUND_COLOR_FIELD_NUMBER: _ClassVar[int]
    IS_BOLD_FIELD_NUMBER: _ClassVar[int]
    IS_ITALIC_FIELD_NUMBER: _ClassVar[int]
    IS_STRIKETHROUGH_FIELD_NUMBER: _ClassVar[int]
    IS_UNDERLINE_FIELD_NUMBER: _ClassVar[int]
    IS_CODE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    spans: _containers.RepeatedCompositeFieldContainer[TextSpanData]
    colspan: int
    rowspan: int
    alignment: Alignment
    color: ColorType
    background_color: ColorType
    is_bold: bool
    is_italic: bool
    is_strikethrough: bool
    is_underline: bool
    is_code: bool
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., spans: _Optional[_Iterable[_Union[TextSpanData, _Mapping]]] = ..., colspan: _Optional[int] = ..., rowspan: _Optional[int] = ..., alignment: _Optional[_Union[Alignment, str]] = ..., color: _Optional[_Union[ColorType, str]] = ..., background_color: _Optional[_Union[ColorType, str]] = ..., is_bold: bool = ..., is_italic: bool = ..., is_strikethrough: bool = ..., is_underline: bool = ..., is_code: bool = ...) -> None: ...

class TextLineData(_message.Message):
    __slots__ = ("metatype", "type", "spans", "content", "table", "cells", "color", "background_color", "is_bold", "is_italic", "is_strikethrough", "is_underline", "is_code")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    SPANS_FIELD_NUMBER: _ClassVar[int]
    CONTENT_FIELD_NUMBER: _ClassVar[int]
    TABLE_FIELD_NUMBER: _ClassVar[int]
    CELLS_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    BACKGROUND_COLOR_FIELD_NUMBER: _ClassVar[int]
    IS_BOLD_FIELD_NUMBER: _ClassVar[int]
    IS_ITALIC_FIELD_NUMBER: _ClassVar[int]
    IS_STRIKETHROUGH_FIELD_NUMBER: _ClassVar[int]
    IS_UNDERLINE_FIELD_NUMBER: _ClassVar[int]
    IS_CODE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    type: TextLineType
    spans: _containers.RepeatedCompositeFieldContainer[TextSpanData]
    content: str
    table: TextTableData
    cells: _containers.RepeatedCompositeFieldContainer[TextCellData]
    color: ColorType
    background_color: ColorType
    is_bold: bool
    is_italic: bool
    is_strikethrough: bool
    is_underline: bool
    is_code: bool
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., type: _Optional[_Union[TextLineType, str]] = ..., spans: _Optional[_Iterable[_Union[TextSpanData, _Mapping]]] = ..., content: _Optional[str] = ..., table: _Optional[_Union[TextTableData, _Mapping]] = ..., cells: _Optional[_Iterable[_Union[TextCellData, _Mapping]]] = ..., color: _Optional[_Union[ColorType, str]] = ..., background_color: _Optional[_Union[ColorType, str]] = ..., is_bold: bool = ..., is_italic: bool = ..., is_strikethrough: bool = ..., is_underline: bool = ..., is_code: bool = ...) -> None: ...

class TextData(_message.Message):
    __slots__ = ("metatype", "lines")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    LINES_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    lines: _containers.RepeatedCompositeFieldContainer[TextLineData]
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., lines: _Optional[_Iterable[_Union[TextLineData, _Mapping]]] = ...) -> None: ...

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
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., name: _Optional[str] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., rules: _Optional[_Iterable[_Union[PolicyRuleData, _Mapping]]] = ..., scopes_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...) -> None: ...

class PolicyRuleData(_message.Message):
    __slots__ = ("metatype", "name", "text", "subject_is_delegated", "subject_is_authenticated", "subject_is_staff", "subject_is_member", "subject_is_owner", "effect", "verbs", "verb_kinds", "object_node_types", "object_properties_ptr", "object_properties_is_system", "object_properties_is_sensitive", "object_properties_is_kernel")
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
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., name: _Optional[str] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., subject_is_delegated: bool = ..., subject_is_authenticated: bool = ..., subject_is_staff: bool = ..., subject_is_member: bool = ..., subject_is_owner: bool = ..., effect: _Optional[_Union[PolicyEffect, str]] = ..., verbs: _Optional[_Iterable[_Union[AccessType, str]]] = ..., verb_kinds: _Optional[_Iterable[_Union[AccessKind, str]]] = ..., object_node_types: _Optional[_Iterable[_Union[NodeType, str]]] = ..., object_properties_ptr: _Optional[_Iterable[_Union[PropertyReferenceData, _Mapping]]] = ..., object_properties_is_system: bool = ..., object_properties_is_sensitive: bool = ..., object_properties_is_kernel: bool = ...) -> None: ...

class SubjectData(_message.Message):
    __slots__ = ("metatype", "id", "is_authenticated", "is_staff", "is_system", "client_ptr", "user_ptr", "machine_ptr", "owned_ptr", "memberships_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    IS_AUTHENTICATED_FIELD_NUMBER: _ClassVar[int]
    IS_STAFF_FIELD_NUMBER: _ClassVar[int]
    IS_SYSTEM_FIELD_NUMBER: _ClassVar[int]
    CLIENT_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_PTR_FIELD_NUMBER: _ClassVar[int]
    MACHINE_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_PTR_FIELD_NUMBER: _ClassVar[int]
    MEMBERSHIPS_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: int
    is_authenticated: bool
    is_staff: bool
    is_system: bool
    client_ptr: NodeReferenceData
    user_ptr: NodeReferenceData
    machine_ptr: NodeReferenceData
    owned_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    memberships_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[int] = ..., is_authenticated: bool = ..., is_staff: bool = ..., is_system: bool = ..., client_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., user_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., machine_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ..., memberships_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...) -> None: ...

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
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[int] = ..., parent_id: _Optional[int] = ..., scope_id: _Optional[str] = ..., identity_id: _Optional[int] = ..., rules: _Optional[_Iterable[_Union[PolicyRuleData, _Mapping]]] = ...) -> None: ...

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
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., subject: _Optional[_Union[SubjectData, _Mapping]] = ..., identities: _Optional[_Iterable[_Union[SubjectData, _Mapping]]] = ..., scoped_zones: _Optional[_Iterable[_Union[AccessZoneData, _Mapping]]] = ..., base_zones: _Optional[_Iterable[_Union[AccessZoneData, _Mapping]]] = ...) -> None: ...

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
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., mode: _Optional[_Union[AccessMode, str]] = ..., decision: _Optional[_Union[PolicyEffect, str]] = ..., verb: _Optional[_Union[AccessType, str]] = ..., node_type: _Optional[_Union[NodeType, str]] = ..., allowed_properties_ptr: _Optional[_Iterable[_Union[PropertyReferenceData, _Mapping]]] = ...) -> None: ...

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
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., type: _Optional[_Union[ColorType, str]] = ..., shade: _Optional[_Union[ColorShade, str]] = ..., hex: _Optional[str] = ...) -> None: ...

class PathElementData(_message.Message):
    __slots__ = ("metatype", "type", "name", "node_ptr", "property_ptr", "run")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    PROPERTY_PTR_FIELD_NUMBER: _ClassVar[int]
    RUN_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    type: PathElementType
    name: str
    node_ptr: NodeReferenceData
    property_ptr: PropertyReferenceData
    run: PathRunSelector
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., type: _Optional[_Union[PathElementType, str]] = ..., name: _Optional[str] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., property_ptr: _Optional[_Union[PropertyReferenceData, _Mapping]] = ..., run: _Optional[_Union[PathRunSelector, str]] = ...) -> None: ...

class PathData(_message.Message):
    __slots__ = ("metatype", "elements")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ELEMENTS_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    elements: _containers.RepeatedCompositeFieldContainer[PathElementData]
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., elements: _Optional[_Iterable[_Union[PathElementData, _Mapping]]] = ...) -> None: ...

class SelectionData(_message.Message):
    __slots__ = ("metatype", "type", "selections", "node_types", "nodes_ptr", "scopes_ptr", "tags_ptr", "from_node_ptr", "to_node_ptr", "fields_ptr", "properties_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    SELECTIONS_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPES_FIELD_NUMBER: _ClassVar[int]
    NODES_PTR_FIELD_NUMBER: _ClassVar[int]
    SCOPES_PTR_FIELD_NUMBER: _ClassVar[int]
    TAGS_PTR_FIELD_NUMBER: _ClassVar[int]
    FROM_NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    TO_NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    FIELDS_PTR_FIELD_NUMBER: _ClassVar[int]
    PROPERTIES_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    type: SelectionType
    selections: _containers.RepeatedCompositeFieldContainer[SelectionData]
    node_types: _containers.RepeatedScalarFieldContainer[NodeType]
    nodes_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    scopes_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    tags_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    from_node_ptr: NodeReferenceData
    to_node_ptr: NodeReferenceData
    fields_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    properties_ptr: _containers.RepeatedCompositeFieldContainer[PropertyReferenceData]
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., type: _Optional[_Union[SelectionType, str]] = ..., selections: _Optional[_Iterable[_Union[SelectionData, _Mapping]]] = ..., node_types: _Optional[_Iterable[_Union[NodeType, str]]] = ..., nodes_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ..., scopes_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ..., tags_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ..., from_node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., to_node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., fields_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ..., properties_ptr: _Optional[_Iterable[_Union[PropertyReferenceData, _Mapping]]] = ...) -> None: ...

class ExpressionData(_message.Message):
    __slots__ = ("metatype", "type", "property_ptr", "field_ptr", "block_ptr", "clauses", "value_packed", "sort_mode", "tolerance")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    PROPERTY_PTR_FIELD_NUMBER: _ClassVar[int]
    FIELD_PTR_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    CLAUSES_FIELD_NUMBER: _ClassVar[int]
    VALUE_PACKED_FIELD_NUMBER: _ClassVar[int]
    SORT_MODE_FIELD_NUMBER: _ClassVar[int]
    TOLERANCE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    type: ExpressionType
    property_ptr: PropertyReferenceData
    field_ptr: NodeReferenceData
    block_ptr: NodeReferenceData
    clauses: _containers.RepeatedCompositeFieldContainer[ExpressionData]
    value_packed: _struct_pb2.Value
    sort_mode: SortMode
    tolerance: float
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., type: _Optional[_Union[ExpressionType, str]] = ..., property_ptr: _Optional[_Union[PropertyReferenceData, _Mapping]] = ..., field_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., clauses: _Optional[_Iterable[_Union[ExpressionData, _Mapping]]] = ..., value_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., sort_mode: _Optional[_Union[SortMode, str]] = ..., tolerance: _Optional[float] = ...) -> None: ...

class AggregationResultData(_message.Message):
    __slots__ = ("metatype", "op", "exists", "count", "scalar")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    OP_FIELD_NUMBER: _ClassVar[int]
    EXISTS_FIELD_NUMBER: _ClassVar[int]
    COUNT_FIELD_NUMBER: _ClassVar[int]
    SCALAR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    op: AggregationType
    exists: bool
    count: int
    scalar: float
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., op: _Optional[_Union[AggregationType, str]] = ..., exists: bool = ..., count: _Optional[int] = ..., scalar: _Optional[float] = ...) -> None: ...

class ValueData(_message.Message):
    __slots__ = ("metatype", "name", "text", "value_type", "value_packed")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    VALUE_TYPE_FIELD_NUMBER: _ClassVar[int]
    VALUE_PACKED_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    name: str
    text: TextData
    value_type: TypeData
    value_packed: _struct_pb2.Value
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., name: _Optional[str] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., value_type: _Optional[_Union[TypeData, _Mapping]] = ..., value_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ...) -> None: ...

class ComputedValueData(_message.Message):
    __slots__ = ("metatype", "kind", "name", "mode", "target_path", "source_path", "source_expression", "source_code", "is_active")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    KIND_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    TARGET_PATH_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PATH_FIELD_NUMBER: _ClassVar[int]
    SOURCE_EXPRESSION_FIELD_NUMBER: _ClassVar[int]
    SOURCE_CODE_FIELD_NUMBER: _ClassVar[int]
    IS_ACTIVE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    kind: ComputedValueKind
    name: str
    mode: ComputedValueMode
    target_path: PathData
    source_path: PathData
    source_expression: ExpressionData
    source_code: CodeData
    is_active: bool
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., kind: _Optional[_Union[ComputedValueKind, str]] = ..., name: _Optional[str] = ..., mode: _Optional[_Union[ComputedValueMode, str]] = ..., target_path: _Optional[_Union[PathData, _Mapping]] = ..., source_path: _Optional[_Union[PathData, _Mapping]] = ..., source_expression: _Optional[_Union[ExpressionData, _Mapping]] = ..., source_code: _Optional[_Union[CodeData, _Mapping]] = ..., is_active: bool = ...) -> None: ...

class IconData(_message.Message):
    __slots__ = ("metatype", "type", "emoji", "fa_name", "vsc_name", "color")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    EMOJI_FIELD_NUMBER: _ClassVar[int]
    FA_NAME_FIELD_NUMBER: _ClassVar[int]
    VSC_NAME_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    type: IconType
    emoji: str
    fa_name: str
    vsc_name: str
    color: ColorData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., type: _Optional[_Union[IconType, str]] = ..., emoji: _Optional[str] = ..., fa_name: _Optional[str] = ..., vsc_name: _Optional[str] = ..., color: _Optional[_Union[ColorData, _Mapping]] = ...) -> None: ...

class SelectOptionsData(_message.Message):
    __slots__ = ("metatype", "select_all_properties", "include_properties_ptr", "exclude_properties_ptr", "select_properties_ptr", "select_fields_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    SELECT_ALL_PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    INCLUDE_PROPERTIES_PTR_FIELD_NUMBER: _ClassVar[int]
    EXCLUDE_PROPERTIES_PTR_FIELD_NUMBER: _ClassVar[int]
    SELECT_PROPERTIES_PTR_FIELD_NUMBER: _ClassVar[int]
    SELECT_FIELDS_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    select_all_properties: bool
    include_properties_ptr: _containers.RepeatedCompositeFieldContainer[PropertyReferenceData]
    exclude_properties_ptr: _containers.RepeatedCompositeFieldContainer[PropertyReferenceData]
    select_properties_ptr: _containers.RepeatedCompositeFieldContainer[PropertyReferenceData]
    select_fields_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., select_all_properties: bool = ..., include_properties_ptr: _Optional[_Iterable[_Union[PropertyReferenceData, _Mapping]]] = ..., exclude_properties_ptr: _Optional[_Iterable[_Union[PropertyReferenceData, _Mapping]]] = ..., select_properties_ptr: _Optional[_Iterable[_Union[PropertyReferenceData, _Mapping]]] = ..., select_fields_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...) -> None: ...

class TypeConstraintData(_message.Message):
    __slots__ = ("metatype", "min_value", "max_value", "step_value", "min_length", "max_length", "regex", "starts_with", "ends_with", "node_types", "node_scope_ptr", "node_max_depth", "node_subtypes")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    MIN_VALUE_FIELD_NUMBER: _ClassVar[int]
    MAX_VALUE_FIELD_NUMBER: _ClassVar[int]
    STEP_VALUE_FIELD_NUMBER: _ClassVar[int]
    MIN_LENGTH_FIELD_NUMBER: _ClassVar[int]
    MAX_LENGTH_FIELD_NUMBER: _ClassVar[int]
    REGEX_FIELD_NUMBER: _ClassVar[int]
    STARTS_WITH_FIELD_NUMBER: _ClassVar[int]
    ENDS_WITH_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPES_FIELD_NUMBER: _ClassVar[int]
    NODE_SCOPE_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_MAX_DEPTH_FIELD_NUMBER: _ClassVar[int]
    NODE_SUBTYPES_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    min_value: float
    max_value: float
    step_value: float
    min_length: int
    max_length: int
    regex: str
    starts_with: str
    ends_with: str
    node_types: _containers.RepeatedScalarFieldContainer[NodeType]
    node_scope_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    node_max_depth: int
    node_subtypes: _containers.RepeatedScalarFieldContainer[int]
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., min_value: _Optional[float] = ..., max_value: _Optional[float] = ..., step_value: _Optional[float] = ..., min_length: _Optional[int] = ..., max_length: _Optional[int] = ..., regex: _Optional[str] = ..., starts_with: _Optional[str] = ..., ends_with: _Optional[str] = ..., node_types: _Optional[_Iterable[_Union[NodeType, str]]] = ..., node_scope_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ..., node_max_depth: _Optional[int] = ..., node_subtypes: _Optional[_Iterable[int]] = ...) -> None: ...

class TypeData(_message.Message):
    __slots__ = ("metatype", "kind", "primitive_type", "bench_type", "base_type_ptr", "base_field_types", "property_field_types", "oneof_ptr", "default_packed", "format", "condition", "constraint", "is_required", "is_list", "is_secret")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    KIND_FIELD_NUMBER: _ClassVar[int]
    PRIMITIVE_TYPE_FIELD_NUMBER: _ClassVar[int]
    BENCH_TYPE_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_FIELD_TYPES_FIELD_NUMBER: _ClassVar[int]
    PROPERTY_FIELD_TYPES_FIELD_NUMBER: _ClassVar[int]
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
    base_field_types: _containers.RepeatedScalarFieldContainer[FieldType]
    property_field_types: _containers.RepeatedScalarFieldContainer[FieldType]
    oneof_ptr: NodeReferenceData
    default_packed: _struct_pb2.Value
    format: TypeFormat
    condition: ExpressionData
    constraint: TypeConstraintData
    is_required: bool
    is_list: bool
    is_secret: bool
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., kind: _Optional[_Union[TypeKind, str]] = ..., primitive_type: _Optional[_Union[PrimitiveType, str]] = ..., bench_type: _Optional[_Union[BenchType, str]] = ..., base_type_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., base_field_types: _Optional[_Iterable[_Union[FieldType, str]]] = ..., property_field_types: _Optional[_Iterable[_Union[FieldType, str]]] = ..., oneof_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., default_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., format: _Optional[_Union[TypeFormat, str]] = ..., condition: _Optional[_Union[ExpressionData, _Mapping]] = ..., constraint: _Optional[_Union[TypeConstraintData, _Mapping]] = ..., is_required: bool = ..., is_list: bool = ..., is_secret: bool = ...) -> None: ...

class DomNodeData(_message.Message):
    __slots__ = ("metatype", "id", "tag", "text", "attributes", "is_focused")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    TAG_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    ATTRIBUTES_FIELD_NUMBER: _ClassVar[int]
    IS_FOCUSED_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    tag: str
    text: str
    attributes: _struct_pb2.Value
    is_focused: bool
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., tag: _Optional[str] = ..., text: _Optional[str] = ..., attributes: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., is_focused: bool = ...) -> None: ...

class FileInfoData(_message.Message):
    __slots__ = ("metatype", "type", "kind", "name", "mime_type", "format", "size", "sha256", "external_url", "inline_content", "width", "height", "aspect_ratio", "codec", "duration")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    KIND_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    MIME_TYPE_FIELD_NUMBER: _ClassVar[int]
    FORMAT_FIELD_NUMBER: _ClassVar[int]
    SIZE_FIELD_NUMBER: _ClassVar[int]
    SHA256_FIELD_NUMBER: _ClassVar[int]
    EXTERNAL_URL_FIELD_NUMBER: _ClassVar[int]
    INLINE_CONTENT_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    ASPECT_RATIO_FIELD_NUMBER: _ClassVar[int]
    CODEC_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    type: FileType
    kind: FileKind
    name: str
    mime_type: str
    format: FileFormat
    size: int
    sha256: str
    external_url: str
    inline_content: bytes
    width: int
    height: int
    aspect_ratio: float
    codec: str
    duration: _duration_pb2.Duration
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., type: _Optional[_Union[FileType, str]] = ..., kind: _Optional[_Union[FileKind, str]] = ..., name: _Optional[str] = ..., mime_type: _Optional[str] = ..., format: _Optional[_Union[FileFormat, str]] = ..., size: _Optional[int] = ..., sha256: _Optional[str] = ..., external_url: _Optional[str] = ..., inline_content: _Optional[bytes] = ..., width: _Optional[int] = ..., height: _Optional[int] = ..., aspect_ratio: _Optional[float] = ..., codec: _Optional[str] = ..., duration: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ...) -> None: ...

class MachineImageData(_message.Message):
    __slots__ = ("metatype",)
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ...) -> None: ...

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
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., type: _Optional[_Union[ClientType, str]] = ..., id: _Optional[str] = ..., nonce: _Optional[str] = ...) -> None: ...

class RunTraceData(_message.Message):
    __slots__ = ("metatype", "frames")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    FRAMES_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    frames: _containers.RepeatedCompositeFieldContainer[RunFrameData]
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., frames: _Optional[_Iterable[_Union[RunFrameData, _Mapping]]] = ...) -> None: ...

class RunFrameData(_message.Message):
    __slots__ = ("metatype",)
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ...) -> None: ...

class ErrorData(_message.Message):
    __slots__ = ("metatype", "kind", "type", "title", "text", "nodes_ptr", "trace")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    KIND_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    NODES_PTR_FIELD_NUMBER: _ClassVar[int]
    TRACE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    kind: ErrorKind
    type: ErrorType
    title: str
    text: TextData
    nodes_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    trace: RunTraceData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., kind: _Optional[_Union[ErrorKind, str]] = ..., type: _Optional[_Union[ErrorType, str]] = ..., title: _Optional[str] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., nodes_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ..., trace: _Optional[_Union[RunTraceData, _Mapping]] = ...) -> None: ...

class BreakpointData(_message.Message):
    __slots__ = ("metatype", "site", "scope", "action")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    SITE_FIELD_NUMBER: _ClassVar[int]
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    ACTION_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    site: BreakpointSite
    scope: BreakpointScope
    action: BreakpointAction
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., site: _Optional[_Union[BreakpointSite, str]] = ..., scope: _Optional[_Union[BreakpointScope, str]] = ..., action: _Optional[_Union[BreakpointAction, str]] = ...) -> None: ...

class TextOptionsData(_message.Message):
    __slots__ = ("metatype", "temperature")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TEMPERATURE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    temperature: float
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., temperature: _Optional[float] = ...) -> None: ...

class AudioOptionsData(_message.Message):
    __slots__ = ("metatype",)
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ...) -> None: ...

class ImageOptionsData(_message.Message):
    __slots__ = ("metatype",)
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ...) -> None: ...

class VideoOptionsData(_message.Message):
    __slots__ = ("metatype",)
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ...) -> None: ...

class RunOptionsData(_message.Message):
    __slots__ = ("metatype", "max_attempts", "max_concurrency", "max_runs", "timeout", "retry_interval", "backoff", "max_retry_interval", "retry_on", "breakpoints", "cache_mode", "cache_retention", "model_developer", "model_family", "model_type", "text_options", "audio_options", "image_options", "video_options")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    MAX_ATTEMPTS_FIELD_NUMBER: _ClassVar[int]
    MAX_CONCURRENCY_FIELD_NUMBER: _ClassVar[int]
    MAX_RUNS_FIELD_NUMBER: _ClassVar[int]
    TIMEOUT_FIELD_NUMBER: _ClassVar[int]
    RETRY_INTERVAL_FIELD_NUMBER: _ClassVar[int]
    BACKOFF_FIELD_NUMBER: _ClassVar[int]
    MAX_RETRY_INTERVAL_FIELD_NUMBER: _ClassVar[int]
    RETRY_ON_FIELD_NUMBER: _ClassVar[int]
    BREAKPOINTS_FIELD_NUMBER: _ClassVar[int]
    CACHE_MODE_FIELD_NUMBER: _ClassVar[int]
    CACHE_RETENTION_FIELD_NUMBER: _ClassVar[int]
    MODEL_DEVELOPER_FIELD_NUMBER: _ClassVar[int]
    MODEL_FAMILY_FIELD_NUMBER: _ClassVar[int]
    MODEL_TYPE_FIELD_NUMBER: _ClassVar[int]
    TEXT_OPTIONS_FIELD_NUMBER: _ClassVar[int]
    AUDIO_OPTIONS_FIELD_NUMBER: _ClassVar[int]
    IMAGE_OPTIONS_FIELD_NUMBER: _ClassVar[int]
    VIDEO_OPTIONS_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    max_attempts: int
    max_concurrency: int
    max_runs: int
    timeout: _duration_pb2.Duration
    retry_interval: _duration_pb2.Duration
    backoff: float
    max_retry_interval: _duration_pb2.Duration
    retry_on: _containers.RepeatedScalarFieldContainer[ErrorType]
    breakpoints: _containers.RepeatedCompositeFieldContainer[BreakpointData]
    cache_mode: CacheMode
    cache_retention: _duration_pb2.Duration
    model_developer: ModelDeveloper
    model_family: ModelFamily
    model_type: ModelType
    text_options: TextOptionsData
    audio_options: AudioOptionsData
    image_options: ImageOptionsData
    video_options: VideoOptionsData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., max_attempts: _Optional[int] = ..., max_concurrency: _Optional[int] = ..., max_runs: _Optional[int] = ..., timeout: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., retry_interval: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., backoff: _Optional[float] = ..., max_retry_interval: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., retry_on: _Optional[_Iterable[_Union[ErrorType, str]]] = ..., breakpoints: _Optional[_Iterable[_Union[BreakpointData, _Mapping]]] = ..., cache_mode: _Optional[_Union[CacheMode, str]] = ..., cache_retention: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., model_developer: _Optional[_Union[ModelDeveloper, str]] = ..., model_family: _Optional[_Union[ModelFamily, str]] = ..., model_type: _Optional[_Union[ModelType, str]] = ..., text_options: _Optional[_Union[TextOptionsData, _Mapping]] = ..., audio_options: _Optional[_Union[AudioOptionsData, _Mapping]] = ..., image_options: _Optional[_Union[ImageOptionsData, _Mapping]] = ..., video_options: _Optional[_Union[VideoOptionsData, _Mapping]] = ...) -> None: ...

class ChangeVignetteData(_message.Message):
    __slots__ = ("metatype", "name", "title", "subtype", "icon")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    SUBTYPE_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    name: str
    title: str
    subtype: int
    icon: IconData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., name: _Optional[str] = ..., title: _Optional[str] = ..., subtype: _Optional[int] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ...) -> None: ...

class EditOperationData(_message.Message):
    __slots__ = ("metatype", "type", "path", "new_value_packed", "old_value_packed")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    PATH_FIELD_NUMBER: _ClassVar[int]
    NEW_VALUE_PACKED_FIELD_NUMBER: _ClassVar[int]
    OLD_VALUE_PACKED_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    type: EditOperationType
    path: _containers.RepeatedScalarFieldContainer[str]
    new_value_packed: _struct_pb2.Value
    old_value_packed: _struct_pb2.Value
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., type: _Optional[_Union[EditOperationType, str]] = ..., path: _Optional[_Iterable[str]] = ..., new_value_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., old_value_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ...) -> None: ...

class EditData(_message.Message):
    __slots__ = ("metatype", "id", "type", "node_ptr", "vignette", "edited_at", "old_edited_at", "node_data", "operations", "scope", "change_key", "category", "subject_ptr", "origin", "context", "epoch", "undo_of_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    VIGNETTE_FIELD_NUMBER: _ClassVar[int]
    EDITED_AT_FIELD_NUMBER: _ClassVar[int]
    OLD_EDITED_AT_FIELD_NUMBER: _ClassVar[int]
    NODE_DATA_FIELD_NUMBER: _ClassVar[int]
    OPERATIONS_FIELD_NUMBER: _ClassVar[int]
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    CHANGE_KEY_FIELD_NUMBER: _ClassVar[int]
    CATEGORY_FIELD_NUMBER: _ClassVar[int]
    SUBJECT_PTR_FIELD_NUMBER: _ClassVar[int]
    ORIGIN_FIELD_NUMBER: _ClassVar[int]
    CONTEXT_FIELD_NUMBER: _ClassVar[int]
    EPOCH_FIELD_NUMBER: _ClassVar[int]
    UNDO_OF_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    type: EditType
    node_ptr: NodeReferenceData
    vignette: ChangeVignetteData
    edited_at: _timestamp_pb2.Timestamp
    old_edited_at: _timestamp_pb2.Timestamp
    node_data: SomeNodeData
    operations: _containers.RepeatedCompositeFieldContainer[EditOperationData]
    scope: GraphScopeData
    change_key: str
    category: ChangeCategory
    subject_ptr: NodeReferenceData
    origin: ClientOriginData
    context: EditContextData
    epoch: int
    undo_of_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., type: _Optional[_Union[EditType, str]] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., vignette: _Optional[_Union[ChangeVignetteData, _Mapping]] = ..., edited_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., old_edited_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., node_data: _Optional[_Union[SomeNodeData, _Mapping]] = ..., operations: _Optional[_Iterable[_Union[EditOperationData, _Mapping]]] = ..., scope: _Optional[_Union[GraphScopeData, _Mapping]] = ..., change_key: _Optional[str] = ..., category: _Optional[_Union[ChangeCategory, str]] = ..., subject_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., origin: _Optional[_Union[ClientOriginData, _Mapping]] = ..., context: _Optional[_Union[EditContextData, _Mapping]] = ..., epoch: _Optional[int] = ..., undo_of_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class ChangeData(_message.Message):
    __slots__ = ("metatype", "key", "code", "edits")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    CODE_FIELD_NUMBER: _ClassVar[int]
    EDITS_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    key: str
    code: CodeData
    edits: _containers.RepeatedCompositeFieldContainer[EditData]
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., key: _Optional[str] = ..., code: _Optional[_Union[CodeData, _Mapping]] = ..., edits: _Optional[_Iterable[_Union[EditData, _Mapping]]] = ...) -> None: ...

class EditContextData(_message.Message):
    __slots__ = ("metatype", "page_ptr", "flow_ptr", "action_ptr", "session_ptr", "run_ptr", "run_root_ptr", "identity_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    PAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    FLOW_PTR_FIELD_NUMBER: _ClassVar[int]
    ACTION_PTR_FIELD_NUMBER: _ClassVar[int]
    SESSION_PTR_FIELD_NUMBER: _ClassVar[int]
    RUN_PTR_FIELD_NUMBER: _ClassVar[int]
    RUN_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    IDENTITY_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    page_ptr: NodeReferenceData
    flow_ptr: NodeReferenceData
    action_ptr: NodeReferenceData
    session_ptr: NodeReferenceData
    run_ptr: NodeReferenceData
    run_root_ptr: NodeReferenceData
    identity_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., page_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., flow_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., action_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., session_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., run_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., run_root_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., identity_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class ContextData(_message.Message):
    __slots__ = ("metatype", "session_ptr", "client_ptr", "machine_ptr", "user_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    SESSION_PTR_FIELD_NUMBER: _ClassVar[int]
    CLIENT_PTR_FIELD_NUMBER: _ClassVar[int]
    MACHINE_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    session_ptr: NodeReferenceData
    client_ptr: NodeReferenceData
    machine_ptr: NodeReferenceData
    user_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., session_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., client_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., machine_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., user_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

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
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., type: _Optional[_Union[FontType, str]] = ..., weight: _Optional[_Union[FontWeight, str]] = ..., size: _Optional[_Union[FontSize, str]] = ...) -> None: ...

class TransformData(_message.Message):
    __slots__ = ("metatype", "translate_x", "translate_y", "scale_x", "scale_y", "skew_x", "skew_y", "rotate_x")
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
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., translate_x: _Optional[float] = ..., translate_y: _Optional[float] = ..., scale_x: _Optional[float] = ..., scale_y: _Optional[float] = ..., skew_x: _Optional[float] = ..., skew_y: _Optional[float] = ..., rotate_x: _Optional[float] = ...) -> None: ...

class RectangleData(_message.Message):
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
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., width: _Optional[int] = ..., height: _Optional[int] = ..., width_relative: _Optional[float] = ..., height_relative: _Optional[float] = ...) -> None: ...

class OffsetData(_message.Message):
    __slots__ = ("metatype", "top", "right", "bottom", "left", "top_relative", "right_relative", "bottom_relative", "left_relative")
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
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., top: _Optional[int] = ..., right: _Optional[int] = ..., bottom: _Optional[int] = ..., left: _Optional[int] = ..., top_relative: _Optional[float] = ..., right_relative: _Optional[float] = ..., bottom_relative: _Optional[float] = ..., left_relative: _Optional[float] = ...) -> None: ...

class Vector2Data(_message.Message):
    __slots__ = ("metatype", "x", "y")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    x: float
    y: float
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., x: _Optional[float] = ..., y: _Optional[float] = ...) -> None: ...

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
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., x: _Optional[float] = ..., y: _Optional[float] = ..., z: _Optional[float] = ...) -> None: ...

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
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., x: _Optional[float] = ..., y: _Optional[float] = ..., z: _Optional[float] = ..., w: _Optional[float] = ...) -> None: ...

class LineData(_message.Message):
    __slots__ = ("metatype", "points")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    POINTS_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    points: _containers.RepeatedCompositeFieldContainer[Vector2Data]
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., points: _Optional[_Iterable[_Union[Vector2Data, _Mapping]]] = ...) -> None: ...

class RectangleConstraintData(_message.Message):
    __slots__ = ("metatype", "min_width", "max_width", "min_height", "max_height")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    min_width: int
    max_width: int
    min_height: int
    max_height: int
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., min_width: _Optional[int] = ..., max_width: _Optional[int] = ..., min_height: _Optional[int] = ..., max_height: _Optional[int] = ...) -> None: ...

class ScheduleData(_message.Message):
    __slots__ = ("metatype", "frequency", "interval", "start", "end", "count", "week_start", "by_set_pos", "by_month", "by_month_day", "by_year_day", "by_easter", "by_week_no", "by_week_day", "by_hour", "by_minute", "by_second")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    FREQUENCY_FIELD_NUMBER: _ClassVar[int]
    INTERVAL_FIELD_NUMBER: _ClassVar[int]
    START_FIELD_NUMBER: _ClassVar[int]
    END_FIELD_NUMBER: _ClassVar[int]
    COUNT_FIELD_NUMBER: _ClassVar[int]
    WEEK_START_FIELD_NUMBER: _ClassVar[int]
    BY_SET_POS_FIELD_NUMBER: _ClassVar[int]
    BY_MONTH_FIELD_NUMBER: _ClassVar[int]
    BY_MONTH_DAY_FIELD_NUMBER: _ClassVar[int]
    BY_YEAR_DAY_FIELD_NUMBER: _ClassVar[int]
    BY_EASTER_FIELD_NUMBER: _ClassVar[int]
    BY_WEEK_NO_FIELD_NUMBER: _ClassVar[int]
    BY_WEEK_DAY_FIELD_NUMBER: _ClassVar[int]
    BY_HOUR_FIELD_NUMBER: _ClassVar[int]
    BY_MINUTE_FIELD_NUMBER: _ClassVar[int]
    BY_SECOND_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    frequency: ScheduleFrequency
    interval: int
    start: _timestamp_pb2.Timestamp
    end: _timestamp_pb2.Timestamp
    count: int
    week_start: Day
    by_set_pos: _containers.RepeatedScalarFieldContainer[int]
    by_month: _containers.RepeatedScalarFieldContainer[Month]
    by_month_day: _containers.RepeatedScalarFieldContainer[int]
    by_year_day: _containers.RepeatedScalarFieldContainer[int]
    by_easter: _containers.RepeatedScalarFieldContainer[int]
    by_week_no: _containers.RepeatedScalarFieldContainer[int]
    by_week_day: _containers.RepeatedScalarFieldContainer[Day]
    by_hour: _containers.RepeatedScalarFieldContainer[int]
    by_minute: _containers.RepeatedScalarFieldContainer[int]
    by_second: _containers.RepeatedScalarFieldContainer[int]
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., frequency: _Optional[_Union[ScheduleFrequency, str]] = ..., interval: _Optional[int] = ..., start: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., end: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., count: _Optional[int] = ..., week_start: _Optional[_Union[Day, str]] = ..., by_set_pos: _Optional[_Iterable[int]] = ..., by_month: _Optional[_Iterable[_Union[Month, str]]] = ..., by_month_day: _Optional[_Iterable[int]] = ..., by_year_day: _Optional[_Iterable[int]] = ..., by_easter: _Optional[_Iterable[int]] = ..., by_week_no: _Optional[_Iterable[int]] = ..., by_week_day: _Optional[_Iterable[_Union[Day, str]]] = ..., by_hour: _Optional[_Iterable[int]] = ..., by_minute: _Optional[_Iterable[int]] = ..., by_second: _Optional[_Iterable[int]] = ...) -> None: ...

class BaseNodeData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "subnode_packed")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    subnode_packed: _struct_pb2.Value
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ...) -> None: ...

class SkipData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "subnode_packed", "reference_ptr", "order_key")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    REFERENCE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    subnode_packed: _struct_pb2.Value
    reference_ptr: NodeReferenceData
    order_key: str
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., reference_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ...) -> None: ...

class EmptyData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "subnode_packed")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    subnode_packed: _struct_pb2.Value
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ...) -> None: ...

class InviteData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "subnode_packed", "type", "to_ptr", "user_ptr", "user_email", "is_owner")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    TO_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_EMAIL_FIELD_NUMBER: _ClassVar[int]
    IS_OWNER_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    subnode_packed: _struct_pb2.Value
    type: InviteType
    to_ptr: NodeReferenceData
    user_ptr: NodeReferenceData
    user_email: str
    is_owner: bool
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[InviteType, str]] = ..., to_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., user_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., user_email: _Optional[str] = ..., is_owner: bool = ...) -> None: ...

class MembershipData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "subnode_packed", "type", "to_ptr", "member_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    TO_PTR_FIELD_NUMBER: _ClassVar[int]
    MEMBER_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    subnode_packed: _struct_pb2.Value
    type: MembershipType
    to_ptr: NodeReferenceData
    member_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[MembershipType, str]] = ..., to_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., member_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class BrowserData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "mode", "subnode_packed", "type", "name", "status", "text", "region", "occupancy", "owned_by_ptr", "scaler_ptr", "tags_ptr", "activated_at", "deactivated_at", "reset_at", "suspended_at", "decommissioned_at", "active_at", "version", "target_version", "external_name", "external_id", "connection_uri", "debugger_uri", "view_uri", "client_ptr", "size", "is_headless", "is_insecure")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    OCCUPANCY_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SCALER_PTR_FIELD_NUMBER: _ClassVar[int]
    TAGS_PTR_FIELD_NUMBER: _ClassVar[int]
    ACTIVATED_AT_FIELD_NUMBER: _ClassVar[int]
    DEACTIVATED_AT_FIELD_NUMBER: _ClassVar[int]
    RESET_AT_FIELD_NUMBER: _ClassVar[int]
    SUSPENDED_AT_FIELD_NUMBER: _ClassVar[int]
    DECOMMISSIONED_AT_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_AT_FIELD_NUMBER: _ClassVar[int]
    VERSION_FIELD_NUMBER: _ClassVar[int]
    TARGET_VERSION_FIELD_NUMBER: _ClassVar[int]
    EXTERNAL_NAME_FIELD_NUMBER: _ClassVar[int]
    EXTERNAL_ID_FIELD_NUMBER: _ClassVar[int]
    CONNECTION_URI_FIELD_NUMBER: _ClassVar[int]
    DEBUGGER_URI_FIELD_NUMBER: _ClassVar[int]
    VIEW_URI_FIELD_NUMBER: _ClassVar[int]
    CLIENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SIZE_FIELD_NUMBER: _ClassVar[int]
    IS_HEADLESS_FIELD_NUMBER: _ClassVar[int]
    IS_INSECURE_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    type: BrowserType
    name: str
    status: ResourceStatus
    text: TextData
    region: Region
    occupancy: ResourceOccupancy
    owned_by_ptr: NodeReferenceData
    scaler_ptr: NodeReferenceData
    tags_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    activated_at: _timestamp_pb2.Timestamp
    deactivated_at: _timestamp_pb2.Timestamp
    reset_at: _timestamp_pb2.Timestamp
    suspended_at: _timestamp_pb2.Timestamp
    decommissioned_at: _timestamp_pb2.Timestamp
    active_at: _timestamp_pb2.Timestamp
    version: str
    target_version: str
    external_name: str
    external_id: str
    connection_uri: str
    debugger_uri: str
    view_uri: str
    client_ptr: NodeReferenceData
    size: Vector2Data
    is_headless: bool
    is_insecure: bool
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[BrowserType, str]] = ..., name: _Optional[str] = ..., status: _Optional[_Union[ResourceStatus, str]] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., region: _Optional[_Union[Region, str]] = ..., occupancy: _Optional[_Union[ResourceOccupancy, str]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., scaler_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., tags_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ..., activated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deactivated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., reset_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., suspended_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., decommissioned_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., active_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., version: _Optional[str] = ..., target_version: _Optional[str] = ..., external_name: _Optional[str] = ..., external_id: _Optional[str] = ..., connection_uri: _Optional[str] = ..., debugger_uri: _Optional[str] = ..., view_uri: _Optional[str] = ..., client_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., size: _Optional[_Union[Vector2Data, _Mapping]] = ..., is_headless: bool = ..., is_insecure: bool = ...) -> None: ...

class FileData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "mode", "subnode_packed", "type", "kind", "name", "status", "text", "region", "occupancy", "owned_by_ptr", "scaler_ptr", "tags_ptr", "activated_at", "deactivated_at", "reset_at", "suspended_at", "decommissioned_at", "active_at", "mime_type", "format", "size", "sha256", "external_url", "inline_content", "width", "height", "aspect_ratio", "codec", "duration", "retention", "expires_at")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    KIND_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    OCCUPANCY_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SCALER_PTR_FIELD_NUMBER: _ClassVar[int]
    TAGS_PTR_FIELD_NUMBER: _ClassVar[int]
    ACTIVATED_AT_FIELD_NUMBER: _ClassVar[int]
    DEACTIVATED_AT_FIELD_NUMBER: _ClassVar[int]
    RESET_AT_FIELD_NUMBER: _ClassVar[int]
    SUSPENDED_AT_FIELD_NUMBER: _ClassVar[int]
    DECOMMISSIONED_AT_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_AT_FIELD_NUMBER: _ClassVar[int]
    MIME_TYPE_FIELD_NUMBER: _ClassVar[int]
    FORMAT_FIELD_NUMBER: _ClassVar[int]
    SIZE_FIELD_NUMBER: _ClassVar[int]
    SHA256_FIELD_NUMBER: _ClassVar[int]
    EXTERNAL_URL_FIELD_NUMBER: _ClassVar[int]
    INLINE_CONTENT_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    ASPECT_RATIO_FIELD_NUMBER: _ClassVar[int]
    CODEC_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    RETENTION_FIELD_NUMBER: _ClassVar[int]
    EXPIRES_AT_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    type: FileType
    kind: FileKind
    name: str
    status: ResourceStatus
    text: TextData
    region: Region
    occupancy: ResourceOccupancy
    owned_by_ptr: NodeReferenceData
    scaler_ptr: NodeReferenceData
    tags_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    activated_at: _timestamp_pb2.Timestamp
    deactivated_at: _timestamp_pb2.Timestamp
    reset_at: _timestamp_pb2.Timestamp
    suspended_at: _timestamp_pb2.Timestamp
    decommissioned_at: _timestamp_pb2.Timestamp
    active_at: _timestamp_pb2.Timestamp
    mime_type: str
    format: FileFormat
    size: int
    sha256: str
    external_url: str
    inline_content: bytes
    width: int
    height: int
    aspect_ratio: float
    codec: str
    duration: _duration_pb2.Duration
    retention: FileRetentionMode
    expires_at: _timestamp_pb2.Timestamp
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[FileType, str]] = ..., kind: _Optional[_Union[FileKind, str]] = ..., name: _Optional[str] = ..., status: _Optional[_Union[ResourceStatus, str]] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., region: _Optional[_Union[Region, str]] = ..., occupancy: _Optional[_Union[ResourceOccupancy, str]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., scaler_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., tags_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ..., activated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deactivated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., reset_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., suspended_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., decommissioned_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., active_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mime_type: _Optional[str] = ..., format: _Optional[_Union[FileFormat, str]] = ..., size: _Optional[int] = ..., sha256: _Optional[str] = ..., external_url: _Optional[str] = ..., inline_content: _Optional[bytes] = ..., width: _Optional[int] = ..., height: _Optional[int] = ..., aspect_ratio: _Optional[float] = ..., codec: _Optional[str] = ..., duration: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., retention: _Optional[_Union[FileRetentionMode, str]] = ..., expires_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...) -> None: ...

class MachineData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "mode", "subnode_packed", "type", "name", "status", "text", "region", "occupancy", "owned_by_ptr", "scaler_ptr", "tags_ptr", "activated_at", "deactivated_at", "reset_at", "suspended_at", "decommissioned_at", "active_at", "version", "target_version", "external_name", "external_id", "connection_uri", "client_ptr", "cpu", "ram")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    OCCUPANCY_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SCALER_PTR_FIELD_NUMBER: _ClassVar[int]
    TAGS_PTR_FIELD_NUMBER: _ClassVar[int]
    ACTIVATED_AT_FIELD_NUMBER: _ClassVar[int]
    DEACTIVATED_AT_FIELD_NUMBER: _ClassVar[int]
    RESET_AT_FIELD_NUMBER: _ClassVar[int]
    SUSPENDED_AT_FIELD_NUMBER: _ClassVar[int]
    DECOMMISSIONED_AT_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_AT_FIELD_NUMBER: _ClassVar[int]
    VERSION_FIELD_NUMBER: _ClassVar[int]
    TARGET_VERSION_FIELD_NUMBER: _ClassVar[int]
    EXTERNAL_NAME_FIELD_NUMBER: _ClassVar[int]
    EXTERNAL_ID_FIELD_NUMBER: _ClassVar[int]
    CONNECTION_URI_FIELD_NUMBER: _ClassVar[int]
    CLIENT_PTR_FIELD_NUMBER: _ClassVar[int]
    CPU_FIELD_NUMBER: _ClassVar[int]
    RAM_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    type: MachineType
    name: str
    status: ResourceStatus
    text: TextData
    region: Region
    occupancy: ResourceOccupancy
    owned_by_ptr: NodeReferenceData
    scaler_ptr: NodeReferenceData
    tags_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    activated_at: _timestamp_pb2.Timestamp
    deactivated_at: _timestamp_pb2.Timestamp
    reset_at: _timestamp_pb2.Timestamp
    suspended_at: _timestamp_pb2.Timestamp
    decommissioned_at: _timestamp_pb2.Timestamp
    active_at: _timestamp_pb2.Timestamp
    version: str
    target_version: str
    external_name: str
    external_id: str
    connection_uri: str
    client_ptr: NodeReferenceData
    cpu: float
    ram: float
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[MachineType, str]] = ..., name: _Optional[str] = ..., status: _Optional[_Union[ResourceStatus, str]] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., region: _Optional[_Union[Region, str]] = ..., occupancy: _Optional[_Union[ResourceOccupancy, str]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., scaler_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., tags_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ..., activated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deactivated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., reset_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., suspended_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., decommissioned_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., active_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., version: _Optional[str] = ..., target_version: _Optional[str] = ..., external_name: _Optional[str] = ..., external_id: _Optional[str] = ..., connection_uri: _Optional[str] = ..., client_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., cpu: _Optional[float] = ..., ram: _Optional[float] = ...) -> None: ...

class ScalerData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "mode", "subnode_packed", "type", "name", "status", "text", "region", "activated_at", "deactivated_at", "reset_at", "suspended_at", "decommissioned_at", "active_at", "strategy", "target_count", "min_count", "max_count", "min_ready_count", "is_active", "is_main")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    ACTIVATED_AT_FIELD_NUMBER: _ClassVar[int]
    DEACTIVATED_AT_FIELD_NUMBER: _ClassVar[int]
    RESET_AT_FIELD_NUMBER: _ClassVar[int]
    SUSPENDED_AT_FIELD_NUMBER: _ClassVar[int]
    DECOMMISSIONED_AT_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_AT_FIELD_NUMBER: _ClassVar[int]
    STRATEGY_FIELD_NUMBER: _ClassVar[int]
    TARGET_COUNT_FIELD_NUMBER: _ClassVar[int]
    MIN_COUNT_FIELD_NUMBER: _ClassVar[int]
    MAX_COUNT_FIELD_NUMBER: _ClassVar[int]
    MIN_READY_COUNT_FIELD_NUMBER: _ClassVar[int]
    IS_ACTIVE_FIELD_NUMBER: _ClassVar[int]
    IS_MAIN_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    type: ScalerType
    name: str
    status: ResourceStatus
    text: TextData
    region: Region
    activated_at: _timestamp_pb2.Timestamp
    deactivated_at: _timestamp_pb2.Timestamp
    reset_at: _timestamp_pb2.Timestamp
    suspended_at: _timestamp_pb2.Timestamp
    decommissioned_at: _timestamp_pb2.Timestamp
    active_at: _timestamp_pb2.Timestamp
    strategy: ScalerStrategy
    target_count: int
    min_count: int
    max_count: int
    min_ready_count: int
    is_active: bool
    is_main: bool
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[ScalerType, str]] = ..., name: _Optional[str] = ..., status: _Optional[_Union[ResourceStatus, str]] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., region: _Optional[_Union[Region, str]] = ..., activated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deactivated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., reset_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., suspended_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., decommissioned_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., active_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., strategy: _Optional[_Union[ScalerStrategy, str]] = ..., target_count: _Optional[int] = ..., min_count: _Optional[int] = ..., max_count: _Optional[int] = ..., min_ready_count: _Optional[int] = ..., is_active: bool = ..., is_main: bool = ...) -> None: ...

class MachineScalerData(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class BrowserScalerData(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class SecretData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "mode", "subnode_packed", "name", "status", "text", "region", "occupancy", "owned_by_ptr", "scaler_ptr", "tags_ptr", "activated_at", "deactivated_at", "reset_at", "suspended_at", "decommissioned_at", "active_at", "value_type", "value_packed")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    OCCUPANCY_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SCALER_PTR_FIELD_NUMBER: _ClassVar[int]
    TAGS_PTR_FIELD_NUMBER: _ClassVar[int]
    ACTIVATED_AT_FIELD_NUMBER: _ClassVar[int]
    DEACTIVATED_AT_FIELD_NUMBER: _ClassVar[int]
    RESET_AT_FIELD_NUMBER: _ClassVar[int]
    SUSPENDED_AT_FIELD_NUMBER: _ClassVar[int]
    DECOMMISSIONED_AT_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_AT_FIELD_NUMBER: _ClassVar[int]
    VALUE_TYPE_FIELD_NUMBER: _ClassVar[int]
    VALUE_PACKED_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    name: str
    status: ResourceStatus
    text: TextData
    region: Region
    occupancy: ResourceOccupancy
    owned_by_ptr: NodeReferenceData
    scaler_ptr: NodeReferenceData
    tags_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    activated_at: _timestamp_pb2.Timestamp
    deactivated_at: _timestamp_pb2.Timestamp
    reset_at: _timestamp_pb2.Timestamp
    suspended_at: _timestamp_pb2.Timestamp
    decommissioned_at: _timestamp_pb2.Timestamp
    active_at: _timestamp_pb2.Timestamp
    value_type: TypeData
    value_packed: _struct_pb2.Value
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., name: _Optional[str] = ..., status: _Optional[_Union[ResourceStatus, str]] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., region: _Optional[_Union[Region, str]] = ..., occupancy: _Optional[_Union[ResourceOccupancy, str]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., scaler_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., tags_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ..., activated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deactivated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., reset_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., suspended_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., decommissioned_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., active_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., value_type: _Optional[_Union[TypeData, _Mapping]] = ..., value_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ...) -> None: ...

class StoreData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "mode", "subnode_packed", "type", "name", "status", "text", "region", "activated_at", "deactivated_at", "reset_at", "suspended_at", "decommissioned_at", "active_at", "version", "target_version", "external_name", "external_id", "connection_uri")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    ACTIVATED_AT_FIELD_NUMBER: _ClassVar[int]
    DEACTIVATED_AT_FIELD_NUMBER: _ClassVar[int]
    RESET_AT_FIELD_NUMBER: _ClassVar[int]
    SUSPENDED_AT_FIELD_NUMBER: _ClassVar[int]
    DECOMMISSIONED_AT_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_AT_FIELD_NUMBER: _ClassVar[int]
    VERSION_FIELD_NUMBER: _ClassVar[int]
    TARGET_VERSION_FIELD_NUMBER: _ClassVar[int]
    EXTERNAL_NAME_FIELD_NUMBER: _ClassVar[int]
    EXTERNAL_ID_FIELD_NUMBER: _ClassVar[int]
    CONNECTION_URI_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    type: StoreType
    name: str
    status: ResourceStatus
    text: TextData
    region: Region
    activated_at: _timestamp_pb2.Timestamp
    deactivated_at: _timestamp_pb2.Timestamp
    reset_at: _timestamp_pb2.Timestamp
    suspended_at: _timestamp_pb2.Timestamp
    decommissioned_at: _timestamp_pb2.Timestamp
    active_at: _timestamp_pb2.Timestamp
    version: str
    target_version: str
    external_name: str
    external_id: str
    connection_uri: str
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[StoreType, str]] = ..., name: _Optional[str] = ..., status: _Optional[_Union[ResourceStatus, str]] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., region: _Optional[_Union[Region, str]] = ..., activated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deactivated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., reset_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., suspended_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., decommissioned_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., active_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., version: _Optional[str] = ..., target_version: _Optional[str] = ..., external_name: _Optional[str] = ..., external_id: _Optional[str] = ..., connection_uri: _Optional[str] = ...) -> None: ...

class StreamData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "mode", "subnode_packed", "type", "name", "status", "text", "region", "occupancy", "owned_by_ptr", "scaler_ptr", "tags_ptr", "activated_at", "deactivated_at", "reset_at", "suspended_at", "decommissioned_at", "active_at")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    OCCUPANCY_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SCALER_PTR_FIELD_NUMBER: _ClassVar[int]
    TAGS_PTR_FIELD_NUMBER: _ClassVar[int]
    ACTIVATED_AT_FIELD_NUMBER: _ClassVar[int]
    DEACTIVATED_AT_FIELD_NUMBER: _ClassVar[int]
    RESET_AT_FIELD_NUMBER: _ClassVar[int]
    SUSPENDED_AT_FIELD_NUMBER: _ClassVar[int]
    DECOMMISSIONED_AT_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_AT_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    type: StreamType
    name: str
    status: ResourceStatus
    text: TextData
    region: Region
    occupancy: ResourceOccupancy
    owned_by_ptr: NodeReferenceData
    scaler_ptr: NodeReferenceData
    tags_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    activated_at: _timestamp_pb2.Timestamp
    deactivated_at: _timestamp_pb2.Timestamp
    reset_at: _timestamp_pb2.Timestamp
    suspended_at: _timestamp_pb2.Timestamp
    decommissioned_at: _timestamp_pb2.Timestamp
    active_at: _timestamp_pb2.Timestamp
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[StreamType, str]] = ..., name: _Optional[str] = ..., status: _Optional[_Union[ResourceStatus, str]] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., region: _Optional[_Union[Region, str]] = ..., occupancy: _Optional[_Union[ResourceOccupancy, str]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., scaler_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., tags_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ..., activated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deactivated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., reset_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., suspended_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., decommissioned_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., active_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...) -> None: ...

class BenchData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "subnode_packed", "main_handle_ptr", "slug", "name", "text", "icon", "owner_ptr", "region", "encryption_key", "status", "main_store_ptr", "main_package_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    MAIN_HANDLE_PTR_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    OWNER_PTR_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    ENCRYPTION_KEY_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    MAIN_STORE_PTR_FIELD_NUMBER: _ClassVar[int]
    MAIN_PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    subnode_packed: _struct_pb2.Value
    main_handle_ptr: NodeReferenceData
    slug: str
    name: str
    text: TextData
    icon: IconData
    owner_ptr: NodeReferenceData
    region: Region
    encryption_key: str
    status: BenchStatus
    main_store_ptr: NodeReferenceData
    main_package_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., main_handle_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., slug: _Optional[str] = ..., name: _Optional[str] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., owner_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., region: _Optional[_Union[Region, str]] = ..., encryption_key: _Optional[str] = ..., status: _Optional[_Union[BenchStatus, str]] = ..., main_store_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., main_package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class ClientData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "user_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "subnode_packed", "type", "name", "device_type", "device_name", "operating_system", "browser_name", "browser_version", "place_id", "access_token", "seen_at", "logged_in_at", "space_ptr", "machine_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
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
    user_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    subnode_packed: _struct_pb2.Value
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
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., user_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[ClientType, str]] = ..., name: _Optional[str] = ..., device_type: _Optional[str] = ..., device_name: _Optional[str] = ..., operating_system: _Optional[str] = ..., browser_name: _Optional[str] = ..., browser_version: _Optional[str] = ..., place_id: _Optional[str] = ..., access_token: _Optional[str] = ..., seen_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., logged_in_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., machine_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class HandleData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "subnode_packed", "slug")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    subnode_packed: _struct_pb2.Value
    slug: str
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., slug: _Optional[str] = ...) -> None: ...

class OrganizationData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "subnode_packed", "slug", "name", "text", "icon", "region", "status", "main_bench_ptr", "main_handle_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    MAIN_BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    MAIN_HANDLE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    subnode_packed: _struct_pb2.Value
    slug: str
    name: str
    text: TextData
    icon: IconData
    region: Region
    status: OrganizationStatus
    main_bench_ptr: NodeReferenceData
    main_handle_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., slug: _Optional[str] = ..., name: _Optional[str] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., region: _Optional[_Union[Region, str]] = ..., status: _Optional[_Union[OrganizationStatus, str]] = ..., main_bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., main_handle_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class TeamData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "team_ptr", "organization_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "subnode_packed", "slug", "name", "text", "icon", "main_bench_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    TEAM_PTR_FIELD_NUMBER: _ClassVar[int]
    ORGANIZATION_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    MAIN_BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    team_ptr: NodeReferenceData
    organization_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    subnode_packed: _struct_pb2.Value
    slug: str
    name: str
    text: TextData
    icon: IconData
    main_bench_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., team_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., organization_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., slug: _Optional[str] = ..., name: _Optional[str] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., main_bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class UserData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "subnode_packed", "slug", "name", "icon", "text", "region", "status", "main_bench_ptr", "main_handle_ptr", "email", "password_salt", "password_hash", "last_logged_in_at", "is_staff")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    MAIN_BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    MAIN_HANDLE_PTR_FIELD_NUMBER: _ClassVar[int]
    EMAIL_FIELD_NUMBER: _ClassVar[int]
    PASSWORD_SALT_FIELD_NUMBER: _ClassVar[int]
    PASSWORD_HASH_FIELD_NUMBER: _ClassVar[int]
    LAST_LOGGED_IN_AT_FIELD_NUMBER: _ClassVar[int]
    IS_STAFF_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    subnode_packed: _struct_pb2.Value
    slug: str
    name: str
    icon: IconData
    text: TextData
    region: Region
    status: UserStatus
    main_bench_ptr: NodeReferenceData
    main_handle_ptr: NodeReferenceData
    email: str
    password_salt: bytes
    password_hash: bytes
    last_logged_in_at: _timestamp_pb2.Timestamp
    is_staff: bool
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., slug: _Optional[str] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., region: _Optional[_Union[Region, str]] = ..., status: _Optional[_Union[UserStatus, str]] = ..., main_bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., main_handle_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., email: _Optional[str] = ..., password_salt: _Optional[bytes] = ..., password_hash: _Optional[bytes] = ..., last_logged_in_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., is_staff: bool = ...) -> None: ...

class InterruptionData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "mode", "subnode_packed", "type", "root_ptr", "page_ptr", "flow_ptr", "action_ptr", "link_ptr", "attempt", "breakpoint_site", "status", "duration", "closed_at", "title", "text", "inputs_packed", "outputs_packed", "response", "message_ptr", "cancel_trigger_ptr", "complete_trigger_ptr", "session_ptr", "client_ptr", "machine_ptr", "user_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    FLOW_PTR_FIELD_NUMBER: _ClassVar[int]
    ACTION_PTR_FIELD_NUMBER: _ClassVar[int]
    LINK_PTR_FIELD_NUMBER: _ClassVar[int]
    ATTEMPT_FIELD_NUMBER: _ClassVar[int]
    BREAKPOINT_SITE_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    CLOSED_AT_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    INPUTS_PACKED_FIELD_NUMBER: _ClassVar[int]
    OUTPUTS_PACKED_FIELD_NUMBER: _ClassVar[int]
    RESPONSE_FIELD_NUMBER: _ClassVar[int]
    MESSAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CANCEL_TRIGGER_PTR_FIELD_NUMBER: _ClassVar[int]
    COMPLETE_TRIGGER_PTR_FIELD_NUMBER: _ClassVar[int]
    SESSION_PTR_FIELD_NUMBER: _ClassVar[int]
    CLIENT_PTR_FIELD_NUMBER: _ClassVar[int]
    MACHINE_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    type: InterruptionType
    root_ptr: NodeReferenceData
    page_ptr: NodeReferenceData
    flow_ptr: NodeReferenceData
    action_ptr: NodeReferenceData
    link_ptr: NodeReferenceData
    attempt: int
    breakpoint_site: BreakpointSite
    status: InterruptionStatus
    duration: _duration_pb2.Duration
    closed_at: _timestamp_pb2.Timestamp
    title: str
    text: TextData
    inputs_packed: _struct_pb2.Value
    outputs_packed: _struct_pb2.Value
    response: InterruptionResponse
    message_ptr: NodeReferenceData
    cancel_trigger_ptr: NodeReferenceData
    complete_trigger_ptr: NodeReferenceData
    session_ptr: NodeReferenceData
    client_ptr: NodeReferenceData
    machine_ptr: NodeReferenceData
    user_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[InterruptionType, str]] = ..., root_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., page_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., flow_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., action_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., link_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., attempt: _Optional[int] = ..., breakpoint_site: _Optional[_Union[BreakpointSite, str]] = ..., status: _Optional[_Union[InterruptionStatus, str]] = ..., duration: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., closed_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., title: _Optional[str] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., inputs_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., outputs_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., response: _Optional[_Union[InterruptionResponse, str]] = ..., message_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., cancel_trigger_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., complete_trigger_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., session_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., client_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., machine_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., user_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class LogData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "mode", "subnode_packed", "type", "severity", "title", "text", "session_ptr", "client_ptr", "machine_ptr", "user_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    SEVERITY_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    SESSION_PTR_FIELD_NUMBER: _ClassVar[int]
    CLIENT_PTR_FIELD_NUMBER: _ClassVar[int]
    MACHINE_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    type: LogType
    severity: Severity
    title: str
    text: TextData
    session_ptr: NodeReferenceData
    client_ptr: NodeReferenceData
    machine_ptr: NodeReferenceData
    user_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[LogType, str]] = ..., severity: _Optional[_Union[Severity, str]] = ..., title: _Optional[str] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., session_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., client_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., machine_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., user_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class PlanData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "mode", "subnode_packed", "type", "name", "termination_mode", "failure_mode", "implemented_by_ptr", "owned_by_ptr", "status", "duration", "started_at", "terminated_at", "error", "session_ptr", "client_ptr", "machine_ptr", "user_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TERMINATION_MODE_FIELD_NUMBER: _ClassVar[int]
    FAILURE_MODE_FIELD_NUMBER: _ClassVar[int]
    IMPLEMENTED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    STARTED_AT_FIELD_NUMBER: _ClassVar[int]
    TERMINATED_AT_FIELD_NUMBER: _ClassVar[int]
    ERROR_FIELD_NUMBER: _ClassVar[int]
    SESSION_PTR_FIELD_NUMBER: _ClassVar[int]
    CLIENT_PTR_FIELD_NUMBER: _ClassVar[int]
    MACHINE_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    type: PlanType
    name: str
    termination_mode: PlanTerminationMode
    failure_mode: PlanFailureMode
    implemented_by_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    status: PlanStatus
    duration: _duration_pb2.Duration
    started_at: _timestamp_pb2.Timestamp
    terminated_at: _timestamp_pb2.Timestamp
    error: ErrorData
    session_ptr: NodeReferenceData
    client_ptr: NodeReferenceData
    machine_ptr: NodeReferenceData
    user_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[PlanType, str]] = ..., name: _Optional[str] = ..., termination_mode: _Optional[_Union[PlanTerminationMode, str]] = ..., failure_mode: _Optional[_Union[PlanFailureMode, str]] = ..., implemented_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., status: _Optional[_Union[PlanStatus, str]] = ..., duration: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., started_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., terminated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., error: _Optional[_Union[ErrorData, _Mapping]] = ..., session_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., client_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., machine_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., user_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class RunData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "mode", "subnode_packed", "type", "root_ptr", "incoming_ptr", "options", "status", "attempt", "duration", "scheduled_at", "started_at", "stopped_at", "interrupted_at", "paused_at", "resumed_at", "terminated_at", "error", "interruption_ptr", "thread_ptr", "page_ptr", "flow_ptr", "action_ptr", "link_ptr", "plan_ptr", "task_ptr", "trigger_ptr", "trigger_key", "trigger_run_ptr", "variables_packed", "inputs_packed", "outputs_packed", "name", "text", "code", "session_ptr", "client_ptr", "machine_ptr", "user_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    INCOMING_PTR_FIELD_NUMBER: _ClassVar[int]
    OPTIONS_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    ATTEMPT_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    SCHEDULED_AT_FIELD_NUMBER: _ClassVar[int]
    STARTED_AT_FIELD_NUMBER: _ClassVar[int]
    STOPPED_AT_FIELD_NUMBER: _ClassVar[int]
    INTERRUPTED_AT_FIELD_NUMBER: _ClassVar[int]
    PAUSED_AT_FIELD_NUMBER: _ClassVar[int]
    RESUMED_AT_FIELD_NUMBER: _ClassVar[int]
    TERMINATED_AT_FIELD_NUMBER: _ClassVar[int]
    ERROR_FIELD_NUMBER: _ClassVar[int]
    INTERRUPTION_PTR_FIELD_NUMBER: _ClassVar[int]
    THREAD_PTR_FIELD_NUMBER: _ClassVar[int]
    PAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    FLOW_PTR_FIELD_NUMBER: _ClassVar[int]
    ACTION_PTR_FIELD_NUMBER: _ClassVar[int]
    LINK_PTR_FIELD_NUMBER: _ClassVar[int]
    PLAN_PTR_FIELD_NUMBER: _ClassVar[int]
    TASK_PTR_FIELD_NUMBER: _ClassVar[int]
    TRIGGER_PTR_FIELD_NUMBER: _ClassVar[int]
    TRIGGER_KEY_FIELD_NUMBER: _ClassVar[int]
    TRIGGER_RUN_PTR_FIELD_NUMBER: _ClassVar[int]
    VARIABLES_PACKED_FIELD_NUMBER: _ClassVar[int]
    INPUTS_PACKED_FIELD_NUMBER: _ClassVar[int]
    OUTPUTS_PACKED_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    CODE_FIELD_NUMBER: _ClassVar[int]
    SESSION_PTR_FIELD_NUMBER: _ClassVar[int]
    CLIENT_PTR_FIELD_NUMBER: _ClassVar[int]
    MACHINE_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    type: RunType
    root_ptr: NodeReferenceData
    incoming_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    options: RunOptionsData
    status: RunStatus
    attempt: int
    duration: _duration_pb2.Duration
    scheduled_at: _timestamp_pb2.Timestamp
    started_at: _timestamp_pb2.Timestamp
    stopped_at: _timestamp_pb2.Timestamp
    interrupted_at: _timestamp_pb2.Timestamp
    paused_at: _timestamp_pb2.Timestamp
    resumed_at: _timestamp_pb2.Timestamp
    terminated_at: _timestamp_pb2.Timestamp
    error: ErrorData
    interruption_ptr: NodeReferenceData
    thread_ptr: NodeReferenceData
    page_ptr: NodeReferenceData
    flow_ptr: NodeReferenceData
    action_ptr: NodeReferenceData
    link_ptr: NodeReferenceData
    plan_ptr: NodeReferenceData
    task_ptr: NodeReferenceData
    trigger_ptr: NodeReferenceData
    trigger_key: str
    trigger_run_ptr: NodeReferenceData
    variables_packed: _struct_pb2.Value
    inputs_packed: _struct_pb2.Value
    outputs_packed: _struct_pb2.Value
    name: str
    text: TextData
    code: CodeData
    session_ptr: NodeReferenceData
    client_ptr: NodeReferenceData
    machine_ptr: NodeReferenceData
    user_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[RunType, str]] = ..., root_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., incoming_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ..., options: _Optional[_Union[RunOptionsData, _Mapping]] = ..., status: _Optional[_Union[RunStatus, str]] = ..., attempt: _Optional[int] = ..., duration: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., scheduled_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., started_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., stopped_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., interrupted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., paused_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., resumed_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., terminated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., error: _Optional[_Union[ErrorData, _Mapping]] = ..., interruption_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., thread_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., page_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., flow_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., action_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., link_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., plan_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., task_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., trigger_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., trigger_key: _Optional[str] = ..., trigger_run_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., variables_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., inputs_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., outputs_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., name: _Optional[str] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., code: _Optional[_Union[CodeData, _Mapping]] = ..., session_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., client_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., machine_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., user_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class SessionData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "mode", "subnode_packed", "status", "duration", "opened_at", "closed_at", "session_ptr", "client_ptr", "machine_ptr", "user_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    OPENED_AT_FIELD_NUMBER: _ClassVar[int]
    CLOSED_AT_FIELD_NUMBER: _ClassVar[int]
    SESSION_PTR_FIELD_NUMBER: _ClassVar[int]
    CLIENT_PTR_FIELD_NUMBER: _ClassVar[int]
    MACHINE_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    status: SessionStatus
    duration: _duration_pb2.Duration
    opened_at: _timestamp_pb2.Timestamp
    closed_at: _timestamp_pb2.Timestamp
    session_ptr: NodeReferenceData
    client_ptr: NodeReferenceData
    machine_ptr: NodeReferenceData
    user_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., status: _Optional[_Union[SessionStatus, str]] = ..., duration: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., opened_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., closed_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., session_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., client_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., machine_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., user_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class RunSpanData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "mode", "subnode_packed", "type", "root_ptr", "severity", "status", "duration", "started_at", "terminated_at", "interrupted_at", "interruption_ptr", "error", "title", "text", "code", "nodes_ptr", "session_ptr", "client_ptr", "machine_ptr", "user_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    SEVERITY_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    STARTED_AT_FIELD_NUMBER: _ClassVar[int]
    TERMINATED_AT_FIELD_NUMBER: _ClassVar[int]
    INTERRUPTED_AT_FIELD_NUMBER: _ClassVar[int]
    INTERRUPTION_PTR_FIELD_NUMBER: _ClassVar[int]
    ERROR_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    CODE_FIELD_NUMBER: _ClassVar[int]
    NODES_PTR_FIELD_NUMBER: _ClassVar[int]
    SESSION_PTR_FIELD_NUMBER: _ClassVar[int]
    CLIENT_PTR_FIELD_NUMBER: _ClassVar[int]
    MACHINE_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    type: RunSpanType
    root_ptr: NodeReferenceData
    severity: Severity
    status: RunStatus
    duration: _duration_pb2.Duration
    started_at: _timestamp_pb2.Timestamp
    terminated_at: _timestamp_pb2.Timestamp
    interrupted_at: _timestamp_pb2.Timestamp
    interruption_ptr: NodeReferenceData
    error: ErrorData
    title: str
    text: TextData
    code: CodeData
    nodes_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    session_ptr: NodeReferenceData
    client_ptr: NodeReferenceData
    machine_ptr: NodeReferenceData
    user_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[RunSpanType, str]] = ..., root_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., severity: _Optional[_Union[Severity, str]] = ..., status: _Optional[_Union[RunStatus, str]] = ..., duration: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., started_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., terminated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., interrupted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., interruption_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., error: _Optional[_Union[ErrorData, _Mapping]] = ..., title: _Optional[str] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., code: _Optional[_Union[CodeData, _Mapping]] = ..., nodes_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ..., session_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., client_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., machine_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., user_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class TaskData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "mode", "subnode_packed", "type", "name", "order_key", "icon", "text", "definition_ptr", "thread_ptr", "tags_ptr", "owned_by_ptr", "implemented_by_ptr", "status", "duration", "due_at", "started_at", "terminated_at", "error", "node_ptr", "clazz_ptr", "value_packed", "is_manual", "session_ptr", "client_ptr", "machine_ptr", "user_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    THREAD_PTR_FIELD_NUMBER: _ClassVar[int]
    TAGS_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    IMPLEMENTED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    DUE_AT_FIELD_NUMBER: _ClassVar[int]
    STARTED_AT_FIELD_NUMBER: _ClassVar[int]
    TERMINATED_AT_FIELD_NUMBER: _ClassVar[int]
    ERROR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    CLAZZ_PTR_FIELD_NUMBER: _ClassVar[int]
    VALUE_PACKED_FIELD_NUMBER: _ClassVar[int]
    IS_MANUAL_FIELD_NUMBER: _ClassVar[int]
    SESSION_PTR_FIELD_NUMBER: _ClassVar[int]
    CLIENT_PTR_FIELD_NUMBER: _ClassVar[int]
    MACHINE_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    type: TaskType
    name: str
    order_key: str
    icon: IconData
    text: TextData
    definition_ptr: NodeReferenceData
    thread_ptr: NodeReferenceData
    tags_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    owned_by_ptr: NodeReferenceData
    implemented_by_ptr: NodeReferenceData
    status: TaskStatus
    duration: _duration_pb2.Duration
    due_at: _timestamp_pb2.Timestamp
    started_at: _timestamp_pb2.Timestamp
    terminated_at: _timestamp_pb2.Timestamp
    error: ErrorData
    node_ptr: NodeReferenceData
    clazz_ptr: NodeReferenceData
    value_packed: _struct_pb2.Value
    is_manual: bool
    session_ptr: NodeReferenceData
    client_ptr: NodeReferenceData
    machine_ptr: NodeReferenceData
    user_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[TaskType, str]] = ..., name: _Optional[str] = ..., order_key: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., thread_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., tags_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., implemented_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., status: _Optional[_Union[TaskStatus, str]] = ..., duration: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., due_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., started_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., terminated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., error: _Optional[_Union[ErrorData, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., clazz_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., value_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., is_manual: bool = ..., session_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., client_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., machine_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., user_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class ActionData(_message.Message):
    __slots__ = ("metatype", "id", "ck", "parent_ptr", "bench_ptr", "package_ptr", "template_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_at", "mode", "computed_values", "subnode_packed", "type", "category", "name", "order_key", "icon", "text", "run_options", "selection", "code", "tool_ptr", "variables_packed", "inputs_packed", "position")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    COMPUTED_VALUES_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    CATEGORY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    RUN_OPTIONS_FIELD_NUMBER: _ClassVar[int]
    SELECTION_FIELD_NUMBER: _ClassVar[int]
    CODE_FIELD_NUMBER: _ClassVar[int]
    TOOL_PTR_FIELD_NUMBER: _ClassVar[int]
    VARIABLES_PACKED_FIELD_NUMBER: _ClassVar[int]
    INPUTS_PACKED_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    computed_values: _containers.RepeatedCompositeFieldContainer[ComputedValueData]
    subnode_packed: _struct_pb2.Value
    type: ActionType
    category: ActionCategory
    name: str
    order_key: str
    icon: IconData
    text: TextData
    run_options: RunOptionsData
    selection: SelectionData
    code: CodeData
    tool_ptr: NodeReferenceData
    variables_packed: _struct_pb2.Value
    inputs_packed: _struct_pb2.Value
    position: Vector2Data
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., ck: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., computed_values: _Optional[_Iterable[_Union[ComputedValueData, _Mapping]]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[ActionType, str]] = ..., category: _Optional[_Union[ActionCategory, str]] = ..., name: _Optional[str] = ..., order_key: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., run_options: _Optional[_Union[RunOptionsData, _Mapping]] = ..., selection: _Optional[_Union[SelectionData, _Mapping]] = ..., code: _Optional[_Union[CodeData, _Mapping]] = ..., tool_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., variables_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., inputs_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., position: _Optional[_Union[Vector2Data, _Mapping]] = ...) -> None: ...

class StartActionData(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class CompleteActionData(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class FailActionData(_message.Message):
    __slots__ = ("error_title", "error_text")
    ERROR_TITLE_FIELD_NUMBER: _ClassVar[int]
    ERROR_TEXT_FIELD_NUMBER: _ClassVar[int]
    error_title: str
    error_text: TextData
    def __init__(self, error_title: _Optional[str] = ..., error_text: _Optional[_Union[TextData, _Mapping]] = ...) -> None: ...

class CodeActionData(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class ToolActionData(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class GenerateActionData(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class TransformActionData(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class RouteActionData(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class EditActionData(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class CreateActionData(_message.Message):
    __slots__ = ("node_partial_packed", "node_ptr")
    NODE_PARTIAL_PACKED_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    node_partial_packed: _struct_pb2.Value
    node_ptr: NodeReferenceData
    def __init__(self, node_partial_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class DuplicateActionData(_message.Message):
    __slots__ = ("node_ptr", "node_partial_packed", "is_shallow", "duplicated_node_ptr")
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PARTIAL_PACKED_FIELD_NUMBER: _ClassVar[int]
    IS_SHALLOW_FIELD_NUMBER: _ClassVar[int]
    DUPLICATED_NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    node_ptr: NodeReferenceData
    node_partial_packed: _struct_pb2.Value
    is_shallow: bool
    duplicated_node_ptr: NodeReferenceData
    def __init__(self, node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., node_partial_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., is_shallow: bool = ..., duplicated_node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class UpdateActionData(_message.Message):
    __slots__ = ("node_ptr", "node_partial_packed")
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PARTIAL_PACKED_FIELD_NUMBER: _ClassVar[int]
    node_ptr: NodeReferenceData
    node_partial_packed: _struct_pb2.Value
    def __init__(self, node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., node_partial_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ...) -> None: ...

class DeleteActionData(_message.Message):
    __slots__ = ("node_ptr",)
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    node_ptr: NodeReferenceData
    def __init__(self, node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class WaitActionData(_message.Message):
    __slots__ = ("delay", "node_ptr")
    DELAY_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    delay: _duration_pb2.Duration
    node_ptr: NodeReferenceData
    def __init__(self, delay: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class SendActionData(_message.Message):
    __slots__ = ("message_in_packed", "message_ptr")
    MESSAGE_IN_PACKED_FIELD_NUMBER: _ClassVar[int]
    MESSAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    message_in_packed: _struct_pb2.Value
    message_ptr: NodeReferenceData
    def __init__(self, message_in_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., message_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class ReceiveActionData(_message.Message):
    __slots__ = ("message_ptr",)
    MESSAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    message_ptr: NodeReferenceData
    def __init__(self, message_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class YieldActionData(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class LookActionData(_message.Message):
    __slots__ = ("application_ptr", "element_id", "element_position", "exclude_image", "screenshot_ptr", "dom_nodes")
    APPLICATION_PTR_FIELD_NUMBER: _ClassVar[int]
    ELEMENT_ID_FIELD_NUMBER: _ClassVar[int]
    ELEMENT_POSITION_FIELD_NUMBER: _ClassVar[int]
    EXCLUDE_IMAGE_FIELD_NUMBER: _ClassVar[int]
    SCREENSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    DOM_NODES_FIELD_NUMBER: _ClassVar[int]
    application_ptr: NodeReferenceData
    element_id: str
    element_position: Vector2Data
    exclude_image: bool
    screenshot_ptr: NodeReferenceData
    dom_nodes: _containers.RepeatedCompositeFieldContainer[DomNodeData]
    def __init__(self, application_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., element_id: _Optional[str] = ..., element_position: _Optional[_Union[Vector2Data, _Mapping]] = ..., exclude_image: bool = ..., screenshot_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., dom_nodes: _Optional[_Iterable[_Union[DomNodeData, _Mapping]]] = ...) -> None: ...

class ClickActionData(_message.Message):
    __slots__ = ("application_ptr", "element_id", "element_position", "button")
    APPLICATION_PTR_FIELD_NUMBER: _ClassVar[int]
    ELEMENT_ID_FIELD_NUMBER: _ClassVar[int]
    ELEMENT_POSITION_FIELD_NUMBER: _ClassVar[int]
    BUTTON_FIELD_NUMBER: _ClassVar[int]
    application_ptr: NodeReferenceData
    element_id: str
    element_position: Vector2Data
    button: str
    def __init__(self, application_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., element_id: _Optional[str] = ..., element_position: _Optional[_Union[Vector2Data, _Mapping]] = ..., button: _Optional[str] = ...) -> None: ...

class PressActionData(_message.Message):
    __slots__ = ("application_ptr", "element_id", "element_position", "combination", "delay")
    APPLICATION_PTR_FIELD_NUMBER: _ClassVar[int]
    ELEMENT_ID_FIELD_NUMBER: _ClassVar[int]
    ELEMENT_POSITION_FIELD_NUMBER: _ClassVar[int]
    COMBINATION_FIELD_NUMBER: _ClassVar[int]
    DELAY_FIELD_NUMBER: _ClassVar[int]
    application_ptr: NodeReferenceData
    element_id: str
    element_position: Vector2Data
    combination: str
    delay: _duration_pb2.Duration
    def __init__(self, application_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., element_id: _Optional[str] = ..., element_position: _Optional[_Union[Vector2Data, _Mapping]] = ..., combination: _Optional[str] = ..., delay: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ...) -> None: ...

class TypeActionData(_message.Message):
    __slots__ = ("application_ptr", "element_id", "element_position", "string", "delay")
    APPLICATION_PTR_FIELD_NUMBER: _ClassVar[int]
    ELEMENT_ID_FIELD_NUMBER: _ClassVar[int]
    ELEMENT_POSITION_FIELD_NUMBER: _ClassVar[int]
    STRING_FIELD_NUMBER: _ClassVar[int]
    DELAY_FIELD_NUMBER: _ClassVar[int]
    application_ptr: NodeReferenceData
    element_id: str
    element_position: Vector2Data
    string: str
    delay: _duration_pb2.Duration
    def __init__(self, application_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., element_id: _Optional[str] = ..., element_position: _Optional[_Union[Vector2Data, _Mapping]] = ..., string: _Optional[str] = ..., delay: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ...) -> None: ...

class ScrollActionData(_message.Message):
    __slots__ = ("application_ptr", "element_id", "element_position", "amount")
    APPLICATION_PTR_FIELD_NUMBER: _ClassVar[int]
    ELEMENT_ID_FIELD_NUMBER: _ClassVar[int]
    ELEMENT_POSITION_FIELD_NUMBER: _ClassVar[int]
    AMOUNT_FIELD_NUMBER: _ClassVar[int]
    application_ptr: NodeReferenceData
    element_id: str
    element_position: Vector2Data
    amount: Vector2Data
    def __init__(self, application_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., element_id: _Optional[str] = ..., element_position: _Optional[_Union[Vector2Data, _Mapping]] = ..., amount: _Optional[_Union[Vector2Data, _Mapping]] = ...) -> None: ...

class SelectActionData(_message.Message):
    __slots__ = ("application_ptr", "element_id", "element_position")
    APPLICATION_PTR_FIELD_NUMBER: _ClassVar[int]
    ELEMENT_ID_FIELD_NUMBER: _ClassVar[int]
    ELEMENT_POSITION_FIELD_NUMBER: _ClassVar[int]
    application_ptr: NodeReferenceData
    element_id: str
    element_position: Vector2Data
    def __init__(self, application_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., element_id: _Optional[str] = ..., element_position: _Optional[_Union[Vector2Data, _Mapping]] = ...) -> None: ...

class GoBackwardActionData(_message.Message):
    __slots__ = ("application_ptr", "element_id", "element_position")
    APPLICATION_PTR_FIELD_NUMBER: _ClassVar[int]
    ELEMENT_ID_FIELD_NUMBER: _ClassVar[int]
    ELEMENT_POSITION_FIELD_NUMBER: _ClassVar[int]
    application_ptr: NodeReferenceData
    element_id: str
    element_position: Vector2Data
    def __init__(self, application_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., element_id: _Optional[str] = ..., element_position: _Optional[_Union[Vector2Data, _Mapping]] = ...) -> None: ...

class GoForwardActionData(_message.Message):
    __slots__ = ("application_ptr", "element_id", "element_position")
    APPLICATION_PTR_FIELD_NUMBER: _ClassVar[int]
    ELEMENT_ID_FIELD_NUMBER: _ClassVar[int]
    ELEMENT_POSITION_FIELD_NUMBER: _ClassVar[int]
    application_ptr: NodeReferenceData
    element_id: str
    element_position: Vector2Data
    def __init__(self, application_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., element_id: _Optional[str] = ..., element_position: _Optional[_Union[Vector2Data, _Mapping]] = ...) -> None: ...

class GoToUrlActionData(_message.Message):
    __slots__ = ("application_ptr", "element_id", "element_position", "url")
    APPLICATION_PTR_FIELD_NUMBER: _ClassVar[int]
    ELEMENT_ID_FIELD_NUMBER: _ClassVar[int]
    ELEMENT_POSITION_FIELD_NUMBER: _ClassVar[int]
    URL_FIELD_NUMBER: _ClassVar[int]
    application_ptr: NodeReferenceData
    element_id: str
    element_position: Vector2Data
    url: str
    def __init__(self, application_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., element_id: _Optional[str] = ..., element_position: _Optional[_Union[Vector2Data, _Mapping]] = ..., url: _Optional[str] = ...) -> None: ...

class GoToTabActionData(_message.Message):
    __slots__ = ("application_ptr", "element_id", "element_position", "tab_index")
    APPLICATION_PTR_FIELD_NUMBER: _ClassVar[int]
    ELEMENT_ID_FIELD_NUMBER: _ClassVar[int]
    ELEMENT_POSITION_FIELD_NUMBER: _ClassVar[int]
    TAB_INDEX_FIELD_NUMBER: _ClassVar[int]
    application_ptr: NodeReferenceData
    element_id: str
    element_position: Vector2Data
    tab_index: int
    def __init__(self, application_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., element_id: _Optional[str] = ..., element_position: _Optional[_Union[Vector2Data, _Mapping]] = ..., tab_index: _Optional[int] = ...) -> None: ...

class BlockData(_message.Message):
    __slots__ = ("metatype", "id", "ck", "parent_ptr", "bench_ptr", "package_ptr", "template_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_at", "mode", "subnode_packed", "type", "order_key", "line", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    LINE_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    type: BlockType
    order_key: str
    line: TextLineData
    node_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., ck: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[BlockType, str]] = ..., order_key: _Optional[str] = ..., line: _Optional[_Union[TextLineData, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class ChannelData(_message.Message):
    __slots__ = ("metatype", "id", "ck", "parent_ptr", "bench_ptr", "package_ptr", "template_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_at", "mode", "subnode_packed", "type", "name", "order_key", "icon", "text", "definition_ptr", "thread_ptr", "tags_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    THREAD_PTR_FIELD_NUMBER: _ClassVar[int]
    TAGS_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    type: ChannelType
    name: str
    order_key: str
    icon: IconData
    text: TextData
    definition_ptr: NodeReferenceData
    thread_ptr: NodeReferenceData
    tags_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., ck: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[ChannelType, str]] = ..., name: _Optional[str] = ..., order_key: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., thread_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., tags_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...) -> None: ...

class ChoiceData(_message.Message):
    __slots__ = ("metatype", "id", "ck", "parent_ptr", "bench_ptr", "package_ptr", "template_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_at", "mode", "subnode_packed", "name", "order_key", "icon", "text", "definition_ptr", "thread_ptr", "tags_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    THREAD_PTR_FIELD_NUMBER: _ClassVar[int]
    TAGS_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    name: str
    order_key: str
    icon: IconData
    text: TextData
    definition_ptr: NodeReferenceData
    thread_ptr: NodeReferenceData
    tags_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., ck: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., name: _Optional[str] = ..., order_key: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., thread_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., tags_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...) -> None: ...

class ClassData(_message.Message):
    __slots__ = ("metatype", "id", "ck", "parent_ptr", "bench_ptr", "package_ptr", "template_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_at", "mode", "subnode_packed", "name", "order_key", "icon", "text", "definition_ptr", "thread_ptr", "tags_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    THREAD_PTR_FIELD_NUMBER: _ClassVar[int]
    TAGS_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    name: str
    order_key: str
    icon: IconData
    text: TextData
    definition_ptr: NodeReferenceData
    thread_ptr: NodeReferenceData
    tags_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., ck: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., name: _Optional[str] = ..., order_key: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., thread_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., tags_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...) -> None: ...

class DatabaseData(_message.Message):
    __slots__ = ("metatype", "id", "ck", "parent_ptr", "bench_ptr", "package_ptr", "template_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_at", "mode", "subnode_packed", "name", "order_key", "icon", "text", "definition_ptr", "thread_ptr", "tags_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    THREAD_PTR_FIELD_NUMBER: _ClassVar[int]
    TAGS_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    name: str
    order_key: str
    icon: IconData
    text: TextData
    definition_ptr: NodeReferenceData
    thread_ptr: NodeReferenceData
    tags_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., ck: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., name: _Optional[str] = ..., order_key: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., thread_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., tags_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...) -> None: ...

class DependencyData(_message.Message):
    __slots__ = ("metatype", "id", "ck", "parent_ptr", "bench_ptr", "package_ptr", "template_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_at", "mode", "subnode_packed", "depends_on_bench_ptr", "depends_on_packages_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    DEPENDS_ON_BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    DEPENDS_ON_PACKAGES_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    depends_on_bench_ptr: NodeReferenceData
    depends_on_packages_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., ck: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., depends_on_bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., depends_on_packages_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...) -> None: ...

class FieldData(_message.Message):
    __slots__ = ("metatype", "id", "ck", "parent_ptr", "bench_ptr", "package_ptr", "template_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_at", "mode", "subnode_packed", "type", "name", "order_key", "text", "icon", "kind", "primitive_type", "bench_type", "base_type_ptr", "base_field_types", "property_field_types", "oneof_ptr", "default_packed", "format", "condition", "constraint", "is_required", "is_list", "is_secret")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    KIND_FIELD_NUMBER: _ClassVar[int]
    PRIMITIVE_TYPE_FIELD_NUMBER: _ClassVar[int]
    BENCH_TYPE_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_FIELD_TYPES_FIELD_NUMBER: _ClassVar[int]
    PROPERTY_FIELD_TYPES_FIELD_NUMBER: _ClassVar[int]
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
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    type: FieldType
    name: str
    order_key: str
    text: TextData
    icon: IconData
    kind: TypeKind
    primitive_type: PrimitiveType
    bench_type: BenchType
    base_type_ptr: NodeReferenceData
    base_field_types: _containers.RepeatedScalarFieldContainer[FieldType]
    property_field_types: _containers.RepeatedScalarFieldContainer[FieldType]
    oneof_ptr: NodeReferenceData
    default_packed: _struct_pb2.Value
    format: TypeFormat
    condition: ExpressionData
    constraint: TypeConstraintData
    is_required: bool
    is_list: bool
    is_secret: bool
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., ck: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[FieldType, str]] = ..., name: _Optional[str] = ..., order_key: _Optional[str] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., kind: _Optional[_Union[TypeKind, str]] = ..., primitive_type: _Optional[_Union[PrimitiveType, str]] = ..., bench_type: _Optional[_Union[BenchType, str]] = ..., base_type_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., base_field_types: _Optional[_Iterable[_Union[FieldType, str]]] = ..., property_field_types: _Optional[_Iterable[_Union[FieldType, str]]] = ..., oneof_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., default_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., format: _Optional[_Union[TypeFormat, str]] = ..., condition: _Optional[_Union[ExpressionData, _Mapping]] = ..., constraint: _Optional[_Union[TypeConstraintData, _Mapping]] = ..., is_required: bool = ..., is_list: bool = ..., is_secret: bool = ...) -> None: ...

class FlowData(_message.Message):
    __slots__ = ("metatype", "id", "ck", "parent_ptr", "bench_ptr", "package_ptr", "template_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_at", "mode", "computed_values", "subnode_packed", "name", "order_key", "icon", "text", "definition_ptr", "thread_ptr", "tags_ptr", "run_options", "selection", "roles_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    COMPUTED_VALUES_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    THREAD_PTR_FIELD_NUMBER: _ClassVar[int]
    TAGS_PTR_FIELD_NUMBER: _ClassVar[int]
    RUN_OPTIONS_FIELD_NUMBER: _ClassVar[int]
    SELECTION_FIELD_NUMBER: _ClassVar[int]
    ROLES_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    computed_values: _containers.RepeatedCompositeFieldContainer[ComputedValueData]
    subnode_packed: _struct_pb2.Value
    name: str
    order_key: str
    icon: IconData
    text: TextData
    definition_ptr: NodeReferenceData
    thread_ptr: NodeReferenceData
    tags_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    run_options: RunOptionsData
    selection: SelectionData
    roles_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., ck: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., computed_values: _Optional[_Iterable[_Union[ComputedValueData, _Mapping]]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., name: _Optional[str] = ..., order_key: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., thread_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., tags_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ..., run_options: _Optional[_Union[RunOptionsData, _Mapping]] = ..., selection: _Optional[_Union[SelectionData, _Mapping]] = ..., roles_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...) -> None: ...

class ImplementationData(_message.Message):
    __slots__ = ("metatype", "id", "ck", "parent_ptr", "bench_ptr", "package_ptr", "template_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_at", "mode", "subnode_packed", "name", "order_key", "text", "icon", "definition_ptr", "thread_ptr", "tags_ptr", "target_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    THREAD_PTR_FIELD_NUMBER: _ClassVar[int]
    TAGS_PTR_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    name: str
    order_key: str
    text: TextData
    icon: IconData
    definition_ptr: NodeReferenceData
    thread_ptr: NodeReferenceData
    tags_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    target_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., ck: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., name: _Optional[str] = ..., order_key: _Optional[str] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., thread_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., tags_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ..., target_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class LinkData(_message.Message):
    __slots__ = ("metatype", "id", "ck", "parent_ptr", "bench_ptr", "package_ptr", "template_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_at", "mode", "computed_values", "subnode_packed", "type", "name", "order_key", "text", "source_ptr", "target_ptr", "run_options", "trigger", "delay", "is_manual", "color")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    COMPUTED_VALUES_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    RUN_OPTIONS_FIELD_NUMBER: _ClassVar[int]
    TRIGGER_FIELD_NUMBER: _ClassVar[int]
    DELAY_FIELD_NUMBER: _ClassVar[int]
    IS_MANUAL_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    computed_values: _containers.RepeatedCompositeFieldContainer[ComputedValueData]
    subnode_packed: _struct_pb2.Value
    type: LinkType
    name: str
    order_key: str
    text: TextData
    source_ptr: NodeReferenceData
    target_ptr: NodeReferenceData
    run_options: RunOptionsData
    trigger: LinkTrigger
    delay: _duration_pb2.Duration
    is_manual: bool
    color: ColorData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., ck: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., computed_values: _Optional[_Iterable[_Union[ComputedValueData, _Mapping]]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[LinkType, str]] = ..., name: _Optional[str] = ..., order_key: _Optional[str] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., source_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., run_options: _Optional[_Union[RunOptionsData, _Mapping]] = ..., trigger: _Optional[_Union[LinkTrigger, str]] = ..., delay: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., is_manual: bool = ..., color: _Optional[_Union[ColorData, _Mapping]] = ...) -> None: ...

class OptionData(_message.Message):
    __slots__ = ("metatype", "id", "ck", "parent_ptr", "bench_ptr", "package_ptr", "template_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_at", "mode", "subnode_packed", "name", "order_key", "text", "icon")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    name: str
    order_key: str
    text: TextData
    icon: IconData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., ck: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., name: _Optional[str] = ..., order_key: _Optional[str] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ...) -> None: ...

class PackageData(_message.Message):
    __slots__ = ("metatype", "id", "ck", "parent_ptr", "bench_ptr", "package_ptr", "template_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_at", "mode", "subnode_packed", "type", "name", "slug", "text", "icon", "owned_by_ptr", "base_ptr", "main_channel_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_PTR_FIELD_NUMBER: _ClassVar[int]
    MAIN_CHANNEL_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    type: PackageType
    name: str
    slug: str
    text: TextData
    icon: IconData
    owned_by_ptr: NodeReferenceData
    base_ptr: NodeReferenceData
    main_channel_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., ck: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[PackageType, str]] = ..., name: _Optional[str] = ..., slug: _Optional[str] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., base_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., main_channel_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class PageData(_message.Message):
    __slots__ = ("metatype", "id", "ck", "parent_ptr", "bench_ptr", "package_ptr", "template_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_at", "mode", "subnode_packed", "name", "order_key", "icon", "text", "definition_ptr", "thread_ptr", "tags_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    THREAD_PTR_FIELD_NUMBER: _ClassVar[int]
    TAGS_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    name: str
    order_key: str
    icon: IconData
    text: TextData
    definition_ptr: NodeReferenceData
    thread_ptr: NodeReferenceData
    tags_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., ck: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., name: _Optional[str] = ..., order_key: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., thread_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., tags_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...) -> None: ...

class ViewData(_message.Message):
    __slots__ = ("metatype", "id", "ck", "parent_ptr", "bench_ptr", "package_ptr", "template_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_at", "mode", "subnode_packed", "type", "name", "order_key", "icon", "text", "definition_ptr", "thread_ptr", "tags_ptr", "subviews_packed", "title", "value_type", "node_ptr", "position", "size", "margin", "padding", "orientation", "alignment", "transform", "constraint", "selection", "focus", "is_hidden", "is_disabled", "is_input", "is_inline", "is_minimal", "is_loading")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    THREAD_PTR_FIELD_NUMBER: _ClassVar[int]
    TAGS_PTR_FIELD_NUMBER: _ClassVar[int]
    SUBVIEWS_PACKED_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    VALUE_TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    SIZE_FIELD_NUMBER: _ClassVar[int]
    MARGIN_FIELD_NUMBER: _ClassVar[int]
    PADDING_FIELD_NUMBER: _ClassVar[int]
    ORIENTATION_FIELD_NUMBER: _ClassVar[int]
    ALIGNMENT_FIELD_NUMBER: _ClassVar[int]
    TRANSFORM_FIELD_NUMBER: _ClassVar[int]
    CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    SELECTION_FIELD_NUMBER: _ClassVar[int]
    FOCUS_FIELD_NUMBER: _ClassVar[int]
    IS_HIDDEN_FIELD_NUMBER: _ClassVar[int]
    IS_DISABLED_FIELD_NUMBER: _ClassVar[int]
    IS_INPUT_FIELD_NUMBER: _ClassVar[int]
    IS_INLINE_FIELD_NUMBER: _ClassVar[int]
    IS_MINIMAL_FIELD_NUMBER: _ClassVar[int]
    IS_LOADING_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    type: ViewType
    name: str
    order_key: str
    icon: IconData
    text: TextData
    definition_ptr: NodeReferenceData
    thread_ptr: NodeReferenceData
    tags_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    subviews_packed: _struct_pb2.Value
    title: str
    value_type: TypeData
    node_ptr: NodeReferenceData
    position: OffsetData
    size: RectangleData
    margin: OffsetData
    padding: OffsetData
    orientation: Orientation
    alignment: Alignment
    transform: TransformData
    constraint: RectangleConstraintData
    selection: SelectionData
    focus: SelectionData
    is_hidden: bool
    is_disabled: bool
    is_input: bool
    is_inline: bool
    is_minimal: bool
    is_loading: bool
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., ck: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[ViewType, str]] = ..., name: _Optional[str] = ..., order_key: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., thread_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., tags_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ..., subviews_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., title: _Optional[str] = ..., value_type: _Optional[_Union[TypeData, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., position: _Optional[_Union[OffsetData, _Mapping]] = ..., size: _Optional[_Union[RectangleData, _Mapping]] = ..., margin: _Optional[_Union[OffsetData, _Mapping]] = ..., padding: _Optional[_Union[OffsetData, _Mapping]] = ..., orientation: _Optional[_Union[Orientation, str]] = ..., alignment: _Optional[_Union[Alignment, str]] = ..., transform: _Optional[_Union[TransformData, _Mapping]] = ..., constraint: _Optional[_Union[RectangleConstraintData, _Mapping]] = ..., selection: _Optional[_Union[SelectionData, _Mapping]] = ..., focus: _Optional[_Union[SelectionData, _Mapping]] = ..., is_hidden: bool = ..., is_disabled: bool = ..., is_input: bool = ..., is_inline: bool = ..., is_minimal: bool = ..., is_loading: bool = ...) -> None: ...

class RunViewData(_message.Message):
    __slots__ = ("variables_packed", "inputs_packed")
    VARIABLES_PACKED_FIELD_NUMBER: _ClassVar[int]
    INPUTS_PACKED_FIELD_NUMBER: _ClassVar[int]
    variables_packed: _struct_pb2.Value
    inputs_packed: _struct_pb2.Value
    def __init__(self, variables_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., inputs_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ...) -> None: ...

class ObjectViewData(_message.Message):
    __slots__ = ("expanded_sections", "collapsed_sections")
    EXPANDED_SECTIONS_FIELD_NUMBER: _ClassVar[int]
    COLLAPSED_SECTIONS_FIELD_NUMBER: _ClassVar[int]
    expanded_sections: _containers.RepeatedScalarFieldContainer[str]
    collapsed_sections: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, expanded_sections: _Optional[_Iterable[str]] = ..., collapsed_sections: _Optional[_Iterable[str]] = ...) -> None: ...

class UserWizardViewData(_message.Message):
    __slots__ = ("stage",)
    STAGE_FIELD_NUMBER: _ClassVar[int]
    stage: UserWizardViewStage
    def __init__(self, stage: _Optional[_Union[UserWizardViewStage, str]] = ...) -> None: ...

class ChatViewData(_message.Message):
    __slots__ = ("draft_text", "draft_nodes_ptr", "draft_reply_to_ptr", "scope_ptr")
    DRAFT_TEXT_FIELD_NUMBER: _ClassVar[int]
    DRAFT_NODES_PTR_FIELD_NUMBER: _ClassVar[int]
    DRAFT_REPLY_TO_PTR_FIELD_NUMBER: _ClassVar[int]
    SCOPE_PTR_FIELD_NUMBER: _ClassVar[int]
    draft_text: TextData
    draft_nodes_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    draft_reply_to_ptr: NodeReferenceData
    scope_ptr: NodeReferenceData
    def __init__(self, draft_text: _Optional[_Union[TextData, _Mapping]] = ..., draft_nodes_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ..., draft_reply_to_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., scope_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class SidebarViewData(_message.Message):
    __slots__ = ("aspect",)
    ASPECT_FIELD_NUMBER: _ClassVar[int]
    aspect: SidebarAspect
    def __init__(self, aspect: _Optional[_Union[SidebarAspect, str]] = ...) -> None: ...

class ContextViewData(_message.Message):
    __slots__ = ("aspect",)
    ASPECT_FIELD_NUMBER: _ClassVar[int]
    aspect: ContextAspect
    def __init__(self, aspect: _Optional[_Union[ContextAspect, str]] = ...) -> None: ...

class ListViewData(_message.Message):
    __slots__ = ("query_node_type", "filter", "sort")
    QUERY_NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    FILTER_FIELD_NUMBER: _ClassVar[int]
    SORT_FIELD_NUMBER: _ClassVar[int]
    query_node_type: NodeType
    filter: ExpressionData
    sort: _containers.RepeatedCompositeFieldContainer[ExpressionData]
    def __init__(self, query_node_type: _Optional[_Union[NodeType, str]] = ..., filter: _Optional[_Union[ExpressionData, _Mapping]] = ..., sort: _Optional[_Iterable[_Union[ExpressionData, _Mapping]]] = ...) -> None: ...

class TreeViewData(_message.Message):
    __slots__ = ("preset", "expanded_nodes_ptr", "collapsed_nodes_ptr")
    PRESET_FIELD_NUMBER: _ClassVar[int]
    EXPANDED_NODES_PTR_FIELD_NUMBER: _ClassVar[int]
    COLLAPSED_NODES_PTR_FIELD_NUMBER: _ClassVar[int]
    preset: TreeViewPreset
    expanded_nodes_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    collapsed_nodes_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    def __init__(self, preset: _Optional[_Union[TreeViewPreset, str]] = ..., expanded_nodes_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ..., collapsed_nodes_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...) -> None: ...

class FeedViewData(_message.Message):
    __slots__ = ("query_node_type", "filter")
    QUERY_NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    FILTER_FIELD_NUMBER: _ClassVar[int]
    query_node_type: NodeType
    filter: ExpressionData
    def __init__(self, query_node_type: _Optional[_Union[NodeType, str]] = ..., filter: _Optional[_Union[ExpressionData, _Mapping]] = ...) -> None: ...

class ButtonViewData(_message.Message):
    __slots__ = ("variant",)
    VARIANT_FIELD_NUMBER: _ClassVar[int]
    variant: ButtonVariant
    def __init__(self, variant: _Optional[_Union[ButtonVariant, str]] = ...) -> None: ...

class PickerViewData(_message.Message):
    __slots__ = ("variant",)
    VARIANT_FIELD_NUMBER: _ClassVar[int]
    variant: PickerVariant
    def __init__(self, variant: _Optional[_Union[PickerVariant, str]] = ...) -> None: ...

class DatetimeViewData(_message.Message):
    __slots__ = ("is_relative",)
    IS_RELATIVE_FIELD_NUMBER: _ClassVar[int]
    is_relative: bool
    def __init__(self, is_relative: bool = ...) -> None: ...

class IconViewData(_message.Message):
    __slots__ = ("include_color",)
    INCLUDE_COLOR_FIELD_NUMBER: _ClassVar[int]
    include_color: bool
    def __init__(self, include_color: bool = ...) -> None: ...

class RoleData(_message.Message):
    __slots__ = ("metatype", "id", "ck", "parent_ptr", "bench_ptr", "package_ptr", "template_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_at", "mode", "subnode_packed", "name", "order_key", "icon", "text", "definition_ptr", "thread_ptr", "tags_ptr", "color")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    THREAD_PTR_FIELD_NUMBER: _ClassVar[int]
    TAGS_PTR_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    name: str
    order_key: str
    icon: IconData
    text: TextData
    definition_ptr: NodeReferenceData
    thread_ptr: NodeReferenceData
    tags_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    color: ColorType
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., ck: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., name: _Optional[str] = ..., order_key: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., thread_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., tags_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ..., color: _Optional[_Union[ColorType, str]] = ...) -> None: ...

class SpaceData(_message.Message):
    __slots__ = ("metatype", "id", "ck", "parent_ptr", "bench_ptr", "package_ptr", "template_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_at", "mode", "subnode_packed", "type", "name", "text", "order_key", "owned_by_ptr", "focus", "selection", "inspection_ptr", "channel_ptr", "thread_ptr", "run_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    FOCUS_FIELD_NUMBER: _ClassVar[int]
    SELECTION_FIELD_NUMBER: _ClassVar[int]
    INSPECTION_PTR_FIELD_NUMBER: _ClassVar[int]
    CHANNEL_PTR_FIELD_NUMBER: _ClassVar[int]
    THREAD_PTR_FIELD_NUMBER: _ClassVar[int]
    RUN_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    type: SpaceType
    name: str
    text: TextData
    order_key: str
    owned_by_ptr: NodeReferenceData
    focus: SelectionData
    selection: SelectionData
    inspection_ptr: NodeReferenceData
    channel_ptr: NodeReferenceData
    thread_ptr: NodeReferenceData
    run_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., ck: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[SpaceType, str]] = ..., name: _Optional[str] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., order_key: _Optional[str] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., focus: _Optional[_Union[SelectionData, _Mapping]] = ..., selection: _Optional[_Union[SelectionData, _Mapping]] = ..., inspection_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., channel_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., thread_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., run_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class TagData(_message.Message):
    __slots__ = ("metatype", "id", "ck", "parent_ptr", "bench_ptr", "package_ptr", "template_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_at", "mode", "subnode_packed", "name", "order_key", "text", "icon", "definition_ptr", "thread_ptr", "tags_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    THREAD_PTR_FIELD_NUMBER: _ClassVar[int]
    TAGS_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    name: str
    order_key: str
    text: TextData
    icon: IconData
    definition_ptr: NodeReferenceData
    thread_ptr: NodeReferenceData
    tags_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., ck: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., name: _Optional[str] = ..., order_key: _Optional[str] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., thread_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., tags_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...) -> None: ...

class TriggerData(_message.Message):
    __slots__ = ("metatype", "id", "ck", "parent_ptr", "bench_ptr", "package_ptr", "template_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_at", "mode", "subnode_packed", "type", "name", "text", "effect", "scope_ptr", "run_root_ptr", "run_ptr", "interruption_ptr", "status", "processed_at", "processed_count", "processed_key", "closed_at")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    EFFECT_FIELD_NUMBER: _ClassVar[int]
    SCOPE_PTR_FIELD_NUMBER: _ClassVar[int]
    RUN_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    RUN_PTR_FIELD_NUMBER: _ClassVar[int]
    INTERRUPTION_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    PROCESSED_AT_FIELD_NUMBER: _ClassVar[int]
    PROCESSED_COUNT_FIELD_NUMBER: _ClassVar[int]
    PROCESSED_KEY_FIELD_NUMBER: _ClassVar[int]
    CLOSED_AT_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    ck: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    subnode_packed: _struct_pb2.Value
    type: TriggerType
    name: str
    text: TextData
    effect: TriggerEffect
    scope_ptr: NodeReferenceData
    run_root_ptr: NodeReferenceData
    run_ptr: NodeReferenceData
    interruption_ptr: NodeReferenceData
    status: TriggerStatus
    processed_at: _timestamp_pb2.Timestamp
    processed_count: int
    processed_key: str
    closed_at: _timestamp_pb2.Timestamp
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., ck: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[TriggerType, str]] = ..., name: _Optional[str] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., effect: _Optional[_Union[TriggerEffect, str]] = ..., scope_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., run_root_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., run_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., interruption_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., status: _Optional[_Union[TriggerStatus, str]] = ..., processed_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., processed_count: _Optional[int] = ..., processed_key: _Optional[str] = ..., closed_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...) -> None: ...

class ScheduleTriggerData(_message.Message):
    __slots__ = ("schedule",)
    SCHEDULE_FIELD_NUMBER: _ClassVar[int]
    schedule: ScheduleData
    def __init__(self, schedule: _Optional[_Union[ScheduleData, _Mapping]] = ...) -> None: ...

class MessageTriggerData(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class MessageData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "subnode_packed", "type", "platform", "channel_ptr", "thread_ptr", "scope_ptr", "run_root_ptr", "run_ptr", "status", "failed_at", "sent_at", "received_at", "read_at", "reply_to_ptr", "forwarded_from_ptr", "title", "text", "value_packed", "clazz_ptr", "nodes_ptr", "selection", "created_interruption_ptr", "created_run_ptr", "created_thread_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    PLATFORM_FIELD_NUMBER: _ClassVar[int]
    CHANNEL_PTR_FIELD_NUMBER: _ClassVar[int]
    THREAD_PTR_FIELD_NUMBER: _ClassVar[int]
    SCOPE_PTR_FIELD_NUMBER: _ClassVar[int]
    RUN_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    RUN_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    FAILED_AT_FIELD_NUMBER: _ClassVar[int]
    SENT_AT_FIELD_NUMBER: _ClassVar[int]
    RECEIVED_AT_FIELD_NUMBER: _ClassVar[int]
    READ_AT_FIELD_NUMBER: _ClassVar[int]
    REPLY_TO_PTR_FIELD_NUMBER: _ClassVar[int]
    FORWARDED_FROM_PTR_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    VALUE_PACKED_FIELD_NUMBER: _ClassVar[int]
    CLAZZ_PTR_FIELD_NUMBER: _ClassVar[int]
    NODES_PTR_FIELD_NUMBER: _ClassVar[int]
    SELECTION_FIELD_NUMBER: _ClassVar[int]
    CREATED_INTERRUPTION_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_RUN_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_THREAD_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    subnode_packed: _struct_pb2.Value
    type: MessageType
    platform: MessagePlatform
    channel_ptr: NodeReferenceData
    thread_ptr: NodeReferenceData
    scope_ptr: NodeReferenceData
    run_root_ptr: NodeReferenceData
    run_ptr: NodeReferenceData
    status: MessageStatus
    failed_at: _timestamp_pb2.Timestamp
    sent_at: _timestamp_pb2.Timestamp
    received_at: _timestamp_pb2.Timestamp
    read_at: _timestamp_pb2.Timestamp
    reply_to_ptr: NodeReferenceData
    forwarded_from_ptr: NodeReferenceData
    title: str
    text: TextData
    value_packed: _struct_pb2.Value
    clazz_ptr: NodeReferenceData
    nodes_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    selection: SelectionData
    created_interruption_ptr: NodeReferenceData
    created_run_ptr: NodeReferenceData
    created_thread_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[MessageType, str]] = ..., platform: _Optional[_Union[MessagePlatform, str]] = ..., channel_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., thread_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., scope_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., run_root_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., run_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., status: _Optional[_Union[MessageStatus, str]] = ..., failed_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., sent_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., received_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., read_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., reply_to_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., forwarded_from_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., title: _Optional[str] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., value_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., clazz_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., nodes_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ..., selection: _Optional[_Union[SelectionData, _Mapping]] = ..., created_interruption_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_run_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_thread_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class NotificationData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "subnode_packed", "type", "channel_ptr", "thread_ptr", "status", "failed_at", "sent_at", "received_at", "read_at", "title", "text", "nodes_ptr", "message_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    CHANNEL_PTR_FIELD_NUMBER: _ClassVar[int]
    THREAD_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    FAILED_AT_FIELD_NUMBER: _ClassVar[int]
    SENT_AT_FIELD_NUMBER: _ClassVar[int]
    RECEIVED_AT_FIELD_NUMBER: _ClassVar[int]
    READ_AT_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    NODES_PTR_FIELD_NUMBER: _ClassVar[int]
    MESSAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    subnode_packed: _struct_pb2.Value
    type: NotificationType
    channel_ptr: NodeReferenceData
    thread_ptr: NodeReferenceData
    status: NotificationStatus
    failed_at: _timestamp_pb2.Timestamp
    sent_at: _timestamp_pb2.Timestamp
    received_at: _timestamp_pb2.Timestamp
    read_at: _timestamp_pb2.Timestamp
    title: str
    text: TextData
    nodes_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    message_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[NotificationType, str]] = ..., channel_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., thread_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., status: _Optional[_Union[NotificationStatus, str]] = ..., failed_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., sent_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., received_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., read_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., title: _Optional[str] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., nodes_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ..., message_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class RecordData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "subnode_packed", "name", "order_key", "icon", "text", "database_ptr", "value_packed")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    DATABASE_PTR_FIELD_NUMBER: _ClassVar[int]
    VALUE_PACKED_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    subnode_packed: _struct_pb2.Value
    name: str
    order_key: str
    icon: IconData
    text: TextData
    database_ptr: NodeReferenceData
    value_packed: _struct_pb2.Value
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., name: _Optional[str] = ..., order_key: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., database_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., value_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ...) -> None: ...

class ThreadData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "subnode_packed", "type", "channel_ptr", "scope_ptr", "run_root_ptr", "run_ptr", "owned_by_ptr", "status", "closed_at", "created_from_ptr", "title", "text")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    SUBNODE_PACKED_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    CHANNEL_PTR_FIELD_NUMBER: _ClassVar[int]
    SCOPE_PTR_FIELD_NUMBER: _ClassVar[int]
    RUN_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    RUN_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    CLOSED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_FROM_PTR_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    metatype: ObjectType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    subnode_packed: _struct_pb2.Value
    type: ThreadType
    channel_ptr: NodeReferenceData
    scope_ptr: NodeReferenceData
    run_root_ptr: NodeReferenceData
    run_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    status: ThreadStatus
    closed_at: _timestamp_pb2.Timestamp
    created_from_ptr: NodeReferenceData
    title: str
    text: TextData
    def __init__(self, metatype: _Optional[_Union[ObjectType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., subnode_packed: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., type: _Optional[_Union[ThreadType, str]] = ..., channel_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., scope_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., run_root_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., run_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., status: _Optional[_Union[ThreadStatus, str]] = ..., closed_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_from_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., title: _Optional[str] = ..., text: _Optional[_Union[TextData, _Mapping]] = ...) -> None: ...

class SomeNodeData(_message.Message):
    __slots__ = ("bench", "handle", "user", "organization", "team", "client", "scaler", "store", "machine", "browser", "file", "stream", "secret", "package", "dependency", "page", "block", "choice", "field", "option", "tag", "flow", "action", "link", "trigger", "implementation", "view", "database", "channel", "role", "space", "thread", "message", "notification", "record", "membership", "invite", "session", "run", "run_span", "interruption", "log", "plan", "task", "skip", "empty")
    BENCH_FIELD_NUMBER: _ClassVar[int]
    HANDLE_FIELD_NUMBER: _ClassVar[int]
    USER_FIELD_NUMBER: _ClassVar[int]
    ORGANIZATION_FIELD_NUMBER: _ClassVar[int]
    TEAM_FIELD_NUMBER: _ClassVar[int]
    CLIENT_FIELD_NUMBER: _ClassVar[int]
    SCALER_FIELD_NUMBER: _ClassVar[int]
    STORE_FIELD_NUMBER: _ClassVar[int]
    MACHINE_FIELD_NUMBER: _ClassVar[int]
    BROWSER_FIELD_NUMBER: _ClassVar[int]
    FILE_FIELD_NUMBER: _ClassVar[int]
    STREAM_FIELD_NUMBER: _ClassVar[int]
    SECRET_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_FIELD_NUMBER: _ClassVar[int]
    DEPENDENCY_FIELD_NUMBER: _ClassVar[int]
    PAGE_FIELD_NUMBER: _ClassVar[int]
    BLOCK_FIELD_NUMBER: _ClassVar[int]
    CHOICE_FIELD_NUMBER: _ClassVar[int]
    CLASS_FIELD_NUMBER: _ClassVar[int]
    FIELD_FIELD_NUMBER: _ClassVar[int]
    OPTION_FIELD_NUMBER: _ClassVar[int]
    TAG_FIELD_NUMBER: _ClassVar[int]
    FLOW_FIELD_NUMBER: _ClassVar[int]
    ACTION_FIELD_NUMBER: _ClassVar[int]
    LINK_FIELD_NUMBER: _ClassVar[int]
    TRIGGER_FIELD_NUMBER: _ClassVar[int]
    IMPLEMENTATION_FIELD_NUMBER: _ClassVar[int]
    VIEW_FIELD_NUMBER: _ClassVar[int]
    DATABASE_FIELD_NUMBER: _ClassVar[int]
    CHANNEL_FIELD_NUMBER: _ClassVar[int]
    ROLE_FIELD_NUMBER: _ClassVar[int]
    SPACE_FIELD_NUMBER: _ClassVar[int]
    THREAD_FIELD_NUMBER: _ClassVar[int]
    MESSAGE_FIELD_NUMBER: _ClassVar[int]
    NOTIFICATION_FIELD_NUMBER: _ClassVar[int]
    RECORD_FIELD_NUMBER: _ClassVar[int]
    MEMBERSHIP_FIELD_NUMBER: _ClassVar[int]
    INVITE_FIELD_NUMBER: _ClassVar[int]
    SESSION_FIELD_NUMBER: _ClassVar[int]
    RUN_FIELD_NUMBER: _ClassVar[int]
    RUN_SPAN_FIELD_NUMBER: _ClassVar[int]
    INTERRUPTION_FIELD_NUMBER: _ClassVar[int]
    LOG_FIELD_NUMBER: _ClassVar[int]
    PLAN_FIELD_NUMBER: _ClassVar[int]
    TASK_FIELD_NUMBER: _ClassVar[int]
    SKIP_FIELD_NUMBER: _ClassVar[int]
    EMPTY_FIELD_NUMBER: _ClassVar[int]
    bench: BenchData
    handle: HandleData
    user: UserData
    organization: OrganizationData
    team: TeamData
    client: ClientData
    scaler: ScalerData
    store: StoreData
    machine: MachineData
    browser: BrowserData
    file: FileData
    stream: StreamData
    secret: SecretData
    package: PackageData
    dependency: DependencyData
    page: PageData
    block: BlockData
    choice: ChoiceData
    field: FieldData
    option: OptionData
    tag: TagData
    flow: FlowData
    action: ActionData
    link: LinkData
    trigger: TriggerData
    implementation: ImplementationData
    view: ViewData
    database: DatabaseData
    channel: ChannelData
    role: RoleData
    space: SpaceData
    thread: ThreadData
    message: MessageData
    notification: NotificationData
    record: RecordData
    membership: MembershipData
    invite: InviteData
    session: SessionData
    run: RunData
    run_span: RunSpanData
    interruption: InterruptionData
    log: LogData
    plan: PlanData
    task: TaskData
    skip: SkipData
    empty: EmptyData
    def __init__(self, bench: _Optional[_Union[BenchData, _Mapping]] = ..., handle: _Optional[_Union[HandleData, _Mapping]] = ..., user: _Optional[_Union[UserData, _Mapping]] = ..., organization: _Optional[_Union[OrganizationData, _Mapping]] = ..., team: _Optional[_Union[TeamData, _Mapping]] = ..., client: _Optional[_Union[ClientData, _Mapping]] = ..., scaler: _Optional[_Union[ScalerData, _Mapping]] = ..., store: _Optional[_Union[StoreData, _Mapping]] = ..., machine: _Optional[_Union[MachineData, _Mapping]] = ..., browser: _Optional[_Union[BrowserData, _Mapping]] = ..., file: _Optional[_Union[FileData, _Mapping]] = ..., stream: _Optional[_Union[StreamData, _Mapping]] = ..., secret: _Optional[_Union[SecretData, _Mapping]] = ..., package: _Optional[_Union[PackageData, _Mapping]] = ..., dependency: _Optional[_Union[DependencyData, _Mapping]] = ..., page: _Optional[_Union[PageData, _Mapping]] = ..., block: _Optional[_Union[BlockData, _Mapping]] = ..., choice: _Optional[_Union[ChoiceData, _Mapping]] = ..., field: _Optional[_Union[FieldData, _Mapping]] = ..., option: _Optional[_Union[OptionData, _Mapping]] = ..., tag: _Optional[_Union[TagData, _Mapping]] = ..., flow: _Optional[_Union[FlowData, _Mapping]] = ..., action: _Optional[_Union[ActionData, _Mapping]] = ..., link: _Optional[_Union[LinkData, _Mapping]] = ..., trigger: _Optional[_Union[TriggerData, _Mapping]] = ..., implementation: _Optional[_Union[ImplementationData, _Mapping]] = ..., view: _Optional[_Union[ViewData, _Mapping]] = ..., database: _Optional[_Union[DatabaseData, _Mapping]] = ..., channel: _Optional[_Union[ChannelData, _Mapping]] = ..., role: _Optional[_Union[RoleData, _Mapping]] = ..., space: _Optional[_Union[SpaceData, _Mapping]] = ..., thread: _Optional[_Union[ThreadData, _Mapping]] = ..., message: _Optional[_Union[MessageData, _Mapping]] = ..., notification: _Optional[_Union[NotificationData, _Mapping]] = ..., record: _Optional[_Union[RecordData, _Mapping]] = ..., membership: _Optional[_Union[MembershipData, _Mapping]] = ..., invite: _Optional[_Union[InviteData, _Mapping]] = ..., session: _Optional[_Union[SessionData, _Mapping]] = ..., run: _Optional[_Union[RunData, _Mapping]] = ..., run_span: _Optional[_Union[RunSpanData, _Mapping]] = ..., interruption: _Optional[_Union[InterruptionData, _Mapping]] = ..., log: _Optional[_Union[LogData, _Mapping]] = ..., plan: _Optional[_Union[PlanData, _Mapping]] = ..., task: _Optional[_Union[TaskData, _Mapping]] = ..., skip: _Optional[_Union[SkipData, _Mapping]] = ..., empty: _Optional[_Union[EmptyData, _Mapping]] = ..., **kwargs) -> None: ...
