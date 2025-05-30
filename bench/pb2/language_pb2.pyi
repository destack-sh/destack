
# type: ignore
# ruff: noqa

from typing import TYPE_CHECKING, Union, AsyncIterator, Mapping

if TYPE_CHECKING:
    from bench.language import Session, Session, IsSubject, Client



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

class ActionCardinality(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ACTION_CARDINALITY_UNSPECIFIED: _ClassVar[ActionCardinality]
    ACTION_CARDINALITY_UNARY: _ClassVar[ActionCardinality]

class AggregationType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    AGGREGATION_TYPE_UNSPECIFIED: _ClassVar[AggregationType]
    AGGREGATION_TYPE_EXISTS: _ClassVar[AggregationType]
    AGGREGATION_TYPE_COUNT: _ClassVar[AggregationType]
    AGGREGATION_TYPE_SUM: _ClassVar[AggregationType]
    AGGREGATION_TYPE_MIN: _ClassVar[AggregationType]
    AGGREGATION_TYPE_MAX: _ClassVar[AggregationType]
    AGGREGATION_TYPE_AVERAGE: _ClassVar[AggregationType]
    AGGREGATION_TYPE_MEDIAN: _ClassVar[AggregationType]
    AGGREGATION_TYPE_HISTOGRAM: _ClassVar[AggregationType]

class Align(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ALIGN_UNSPECIFIED: _ClassVar[Align]
    ALIGN_START: _ClassVar[Align]
    ALIGN_CENTER: _ClassVar[Align]
    ALIGN_END: _ClassVar[Align]

class Area(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    AREA_UNSPECIFIED: _ClassVar[Area]
    AREA_EUROPE_CENTRAL: _ClassVar[Area]
    AREA_NORTH_AMERICA_EAST: _ClassVar[Area]
    AREA_NORTH_AMERICA_WEST: _ClassVar[Area]
    AREA_SOUTH_AMERICA_EAST: _ClassVar[Area]
    AREA_MIDDLE_EAST_CENTRAL: _ClassVar[Area]
    AREA_MIDDLE_EAST_WEST: _ClassVar[Area]
    AREA_AFRICA_SOUTH: _ClassVar[Area]
    AREA_ASIA_WEST: _ClassVar[Area]
    AREA_ASIA_SOUTH: _ClassVar[Area]
    AREA_ASIA_EAST: _ClassVar[Area]
    AREA_AUSTRALIA_SOUTH: _ClassVar[Area]

class AttributeType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ATTRIBUTE_TYPE_UNSPECIFIED: _ClassVar[AttributeType]
    ATTRIBUTE_TYPE_PROPERTY: _ClassVar[AttributeType]
    ATTRIBUTE_TYPE_FIELD: _ClassVar[AttributeType]
    ATTRIBUTE_TYPE_QUERY: _ClassVar[AttributeType]

class BenchRoleType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    BENCH_ROLE_TYPE_UNSPECIFIED: _ClassVar[BenchRoleType]
    BENCH_ROLE_TYPE_ADMIN: _ClassVar[BenchRoleType]
    BENCH_ROLE_TYPE_MEMBER: _ClassVar[BenchRoleType]

class BenchStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    BENCH_STATUS_UNSPECIFIED: _ClassVar[BenchStatus]
    BENCH_STATUS_CREATING: _ClassVar[BenchStatus]
    BENCH_STATUS_ACTIVE: _ClassVar[BenchStatus]

class BlockType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    BLOCK_TYPE_UNSPECIFIED: _ClassVar[BlockType]
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
    BLOCK_TYPE_CODE: _ClassVar[BlockType]
    BLOCK_TYPE_NODE: _ClassVar[BlockType]

class BorderType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    BORDER_TYPE_UNSPECIFIED: _ClassVar[BorderType]
    BORDER_TYPE_NONE: _ClassVar[BorderType]
    BORDER_TYPE_STYLE: _ClassVar[BorderType]
    BORDER_TYPE_FIELD: _ClassVar[BorderType]
    BORDER_TYPE_SOLID: _ClassVar[BorderType]
    BORDER_TYPE_DASHED: _ClassVar[BorderType]
    BORDER_TYPE_DOTTED: _ClassVar[BorderType]
    BORDER_TYPE_DOUBLE: _ClassVar[BorderType]

class CascadeAction(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CASCADE_ACTION_UNSPECIFIED: _ClassVar[CascadeAction]
    CASCADE_ACTION_RESTRICT: _ClassVar[CascadeAction]
    CASCADE_ACTION_CASCADE: _ClassVar[CascadeAction]
    CASCADE_ACTION_SET_NULL: _ClassVar[CascadeAction]

class ChangeStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CHANGE_STATUS_UNSPECIFIED: _ClassVar[ChangeStatus]
    CHANGE_STATUS_COMPLETED: _ClassVar[ChangeStatus]
    CHANGE_STATUS_FAILED: _ClassVar[ChangeStatus]

class ClientType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CLIENT_TYPE_UNSPECIFIED: _ClassVar[ClientType]
    CLIENT_TYPE_WEB: _ClassVar[ClientType]
    CLIENT_TYPE_BROWSER_PLUGIN: _ClassVar[ClientType]
    CLIENT_TYPE_DESKTOP: _ClassVar[ClientType]
    CLIENT_TYPE_MOBILE: _ClassVar[ClientType]
    CLIENT_TYPE_MACHINE: _ClassVar[ClientType]

class Cloud(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CLOUD_UNSPECIFIED: _ClassVar[Cloud]
    CLOUD_AWS: _ClassVar[Cloud]
    CLOUD_AZURE: _ClassVar[Cloud]
    CLOUD_GCP: _ClassVar[Cloud]
    CLOUD_OCI: _ClassVar[Cloud]
    CLOUD_ALIBABA: _ClassVar[Cloud]
    CLOUD_HETZNER: _ClassVar[Cloud]
    CLOUD_PRIVATE: _ClassVar[Cloud]

class CodeType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CODE_TYPE_UNSPECIFIED: _ClassVar[CodeType]
    CODE_TYPE_SNIPPET: _ClassVar[CodeType]
    CODE_TYPE_SCRIPT: _ClassVar[CodeType]
    CODE_TYPE_FUNCTION: _ClassVar[CodeType]

class ColorHue(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    COLOR_HUE_UNSPECIFIED: _ClassVar[ColorHue]
    COLOR_HUE_GRAY: _ClassVar[ColorHue]
    COLOR_HUE_RED: _ClassVar[ColorHue]
    COLOR_HUE_ORANGE: _ClassVar[ColorHue]
    COLOR_HUE_AMBER: _ClassVar[ColorHue]
    COLOR_HUE_YELLOW: _ClassVar[ColorHue]
    COLOR_HUE_LIME: _ClassVar[ColorHue]
    COLOR_HUE_GREEN: _ClassVar[ColorHue]
    COLOR_HUE_EMERALD: _ClassVar[ColorHue]
    COLOR_HUE_TEAL: _ClassVar[ColorHue]
    COLOR_HUE_CYAN: _ClassVar[ColorHue]
    COLOR_HUE_SKY: _ClassVar[ColorHue]
    COLOR_HUE_BLUE: _ClassVar[ColorHue]
    COLOR_HUE_INDIGO: _ClassVar[ColorHue]
    COLOR_HUE_VIOLET: _ClassVar[ColorHue]
    COLOR_HUE_PURPLE: _ClassVar[ColorHue]
    COLOR_HUE_FUCHSIA: _ClassVar[ColorHue]
    COLOR_HUE_PINK: _ClassVar[ColorHue]
    COLOR_HUE_ROSE: _ClassVar[ColorHue]

class ColorShade(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    COLOR_SHADE_UNSPECIFIED: _ClassVar[ColorShade]
    COLOR_SHADE_S25: _ClassVar[ColorShade]
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
    COLOR_TYPE_BUILTIN: _ClassVar[ColorType]
    COLOR_TYPE_STYLE: _ClassVar[ColorType]
    COLOR_TYPE_FIELD: _ClassVar[ColorType]
    COLOR_TYPE_RGB: _ClassVar[ColorType]
    COLOR_TYPE_HSL: _ClassVar[ColorType]
    COLOR_TYPE_P3: _ClassVar[ColorType]

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
    CONDITIONAL_TYPE_CONTAINS: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_NOT_CONTAINS: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_IN: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_NOT_IN: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_EXISTS: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_NOT_EXISTS: _ClassVar[ConditionalType]

class Continent(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CONTINENT_UNSPECIFIED: _ClassVar[Continent]
    CONTINENT_EUROPE: _ClassVar[Continent]
    CONTINENT_NORTH_AMERICA: _ClassVar[Continent]
    CONTINENT_SOUTH_AMERICA: _ClassVar[Continent]
    CONTINENT_MIDDLE_EAST: _ClassVar[Continent]
    CONTINENT_AFRICA: _ClassVar[Continent]
    CONTINENT_ASIA: _ClassVar[Continent]
    CONTINENT_AUSTRALIA: _ClassVar[Continent]
    CONTINENT_PRIVATE: _ClassVar[Continent]

class CursorStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CURSOR_STATUS_UNSPECIFIED: _ClassVar[CursorStatus]
    CURSOR_STATUS_CREATED: _ClassVar[CursorStatus]
    CURSOR_STATUS_WORKING: _ClassVar[CursorStatus]
    CURSOR_STATUS_READING: _ClassVar[CursorStatus]
    CURSOR_STATUS_WRITING: _ClassVar[CursorStatus]
    CURSOR_STATUS_THINKING: _ClassVar[CursorStatus]
    CURSOR_STATUS_WAITING: _ClassVar[CursorStatus]
    CURSOR_STATUS_IDLE: _ClassVar[CursorStatus]
    CURSOR_STATUS_CANCELLED: _ClassVar[CursorStatus]
    CURSOR_STATUS_COMPLETED: _ClassVar[CursorStatus]

class CursorType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CURSOR_TYPE_UNSPECIFIED: _ClassVar[CursorType]
    CURSOR_TYPE_THREAD: _ClassVar[CursorType]
    CURSOR_TYPE_PAGE: _ClassVar[CursorType]
    CURSOR_TYPE_TABLE: _ClassVar[CursorType]
    CURSOR_TYPE_ACTION: _ClassVar[CursorType]
    CURSOR_TYPE_WEB: _ClassVar[CursorType]
    CURSOR_TYPE_CUSTOM: _ClassVar[CursorType]

class DatabaseType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    DATABASE_TYPE_UNSPECIFIED: _ClassVar[DatabaseType]
    DATABASE_TYPE_POSTGRES: _ClassVar[DatabaseType]

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

class DefaultFactory(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    DEFAULT_FACTORY_UNSPECIFIED: _ClassVar[DefaultFactory]
    DEFAULT_FACTORY_UUID: _ClassVar[DefaultFactory]
    DEFAULT_FACTORY_NOW: _ClassVar[DefaultFactory]
    DEFAULT_FACTORY_REGION: _ClassVar[DefaultFactory]

class DimensionType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    DIMENSION_TYPE_UNSPECIFIED: _ClassVar[DimensionType]
    DIMENSION_TYPE_FIXED: _ClassVar[DimensionType]
    DIMENSION_TYPE_FIT: _ClassVar[DimensionType]
    DIMENSION_TYPE_FILL: _ClassVar[DimensionType]

class Direction(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    DIRECTION_UNSPECIFIED: _ClassVar[Direction]
    DIRECTION_HORIZONTAL: _ClassVar[Direction]
    DIRECTION_VERTICAL: _ClassVar[Direction]

class Distribute(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    DISTRIBUTE_UNSPECIFIED: _ClassVar[Distribute]
    DISTRIBUTE_START: _ClassVar[Distribute]
    DISTRIBUTE_CENTER: _ClassVar[Distribute]
    DISTRIBUTE_END: _ClassVar[Distribute]
    DISTRIBUTE_SPACE_BETWEEN: _ClassVar[Distribute]
    DISTRIBUTE_SPACE_AROUND: _ClassVar[Distribute]
    DISTRIBUTE_SPACE_EVENLY: _ClassVar[Distribute]

class EdgeType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    EDGE_TYPE_UNSPECIFIED: _ClassVar[EdgeType]
    EDGE_TYPE_NODE_PARENT: _ClassVar[EdgeType]
    EDGE_TYPE_NODE_ANCESTOR: _ClassVar[EdgeType]
    EDGE_TYPE_NODE_REGULAR: _ClassVar[EdgeType]
    EDGE_TYPE_NODE_TEMPLATE: _ClassVar[EdgeType]

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

class EffectType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    EFFECT_TYPE_UNSPECIFIED: _ClassVar[EffectType]
    EFFECT_TYPE_NONE: _ClassVar[EffectType]
    EFFECT_TYPE_STYLE: _ClassVar[EffectType]
    EFFECT_TYPE_FIELD: _ClassVar[EffectType]
    EFFECT_TYPE_APPEAR: _ClassVar[EffectType]
    EFFECT_TYPE_ENTER: _ClassVar[EffectType]
    EFFECT_TYPE_EXIT: _ClassVar[EffectType]
    EFFECT_TYPE_HOVER: _ClassVar[EffectType]
    EFFECT_TYPE_PRESS: _ClassVar[EffectType]
    EFFECT_TYPE_DRAG: _ClassVar[EffectType]
    EFFECT_TYPE_FOCUS: _ClassVar[EffectType]
    EFFECT_TYPE_LOOP: _ClassVar[EffectType]

class EnumType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ENUM_TYPE_UNSPECIFIED: _ClassVar[EnumType]
    ENUM_TYPE_ENUM_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_NODE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_STRUCT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_TRAIT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_NODE_MODE: _ClassVar[EnumType]
    ENUM_TYPE_NODE_AREA: _ClassVar[EnumType]
    ENUM_TYPE_USER_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_ORGANIZATION_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_BENCH_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_PACKAGE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_ERROR_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_VARIABLE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_EDIT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_UPDATE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_CHANGE_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_CONDITIONAL_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_AGGREGATION_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_SORT_MODE: _ClassVar[EnumType]
    ENUM_TYPE_SORT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_JOIN_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_FUNCTION_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_EXPRESSION_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_RELATION_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_ATTRIBUTE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_QUERY_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_BENCH_ROLE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_ORGANIZATION_ROLE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_PACKAGE_ROLE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_SPACE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_BLOCK_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_CLOUD: _ClassVar[EnumType]
    ENUM_TYPE_REGION: _ClassVar[EnumType]
    ENUM_TYPE_AREA: _ClassVar[EnumType]
    ENUM_TYPE_CONTINENT: _ClassVar[EnumType]
    ENUM_TYPE_MACHINE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_DATABASE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_CLIENT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_ACTION_CARDINALITY: _ClassVar[EnumType]
    ENUM_TYPE_FLOW_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_FLOW_EDGE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_CURSOR_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_CURSOR_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_PROCESS_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_RUN_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_SPAN_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_SCHEDULE_FREQUENCY: _ClassVar[EnumType]
    ENUM_TYPE_INTERRUPTION_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_INTERRUPTION_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_INTERRUPTION_RESPONSE: _ClassVar[EnumType]
    ENUM_TYPE_TEXT_LINE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_TEXT_SPAN_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_CODE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_FILE_RETENTION_MODE: _ClassVar[EnumType]
    ENUM_TYPE_FILE_SOURCE: _ClassVar[EnumType]
    ENUM_TYPE_FILE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_FILE_FORMAT: _ClassVar[EnumType]
    ENUM_TYPE_ICON_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_LINK_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_PRIMITIVE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_TYPE_CARDINALITY: _ClassVar[EnumType]
    ENUM_TYPE_SCALAR_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_DEFAULT_FACTORY: _ClassVar[EnumType]
    ENUM_TYPE_STRING_FORMAT: _ClassVar[EnumType]
    ENUM_TYPE_NUMBER_FORMAT: _ClassVar[EnumType]
    ENUM_TYPE_FIELD_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_EDGE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_CASCADE_ACTION: _ClassVar[EnumType]
    ENUM_TYPE_DAY: _ClassVar[EnumType]
    ENUM_TYPE_MONTH: _ClassVar[EnumType]
    ENUM_TYPE_TIME_INTERVAL: _ClassVar[EnumType]
    ENUM_TYPE_RESOURCE_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_THREAD_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_MESSAGE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_MODEL_DEVELOPER: _ClassVar[EnumType]
    ENUM_TYPE_MODEL_PROVIDER: _ClassVar[EnumType]
    ENUM_TYPE_POSITION_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_COLOR_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_COLOR_SHADE: _ClassVar[EnumType]
    ENUM_TYPE_COLOR_HUE: _ClassVar[EnumType]
    ENUM_TYPE_FONT_WEIGHT: _ClassVar[EnumType]
    ENUM_TYPE_FONT_SIZE: _ClassVar[EnumType]
    ENUM_TYPE_FONT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_TEXT_ALIGN: _ClassVar[EnumType]
    ENUM_TYPE_TEXT_DECORATION: _ClassVar[EnumType]
    ENUM_TYPE_TEXT_TRANSFORM: _ClassVar[EnumType]
    ENUM_TYPE_SHADOW_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_SHADOW_POSITION: _ClassVar[EnumType]
    ENUM_TYPE_BORDER_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_GRADIENT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_FILL_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_FILL_POSITION: _ClassVar[EnumType]
    ENUM_TYPE_FILL_SIZE: _ClassVar[EnumType]
    ENUM_TYPE_LENGTH_UNIT: _ClassVar[EnumType]
    ENUM_TYPE_LAYOUT: _ClassVar[EnumType]
    ENUM_TYPE_DISTRIBUTE: _ClassVar[EnumType]
    ENUM_TYPE_ALIGN: _ClassVar[EnumType]
    ENUM_TYPE_DIRECTION: _ClassVar[EnumType]
    ENUM_TYPE_OVERFLOW: _ClassVar[EnumType]
    ENUM_TYPE_TRANSITION_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_SPRING_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_DIMENSION_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_THEME_COLOR: _ClassVar[EnumType]
    ENUM_TYPE_EFFECT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_REPEAT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_TEXT_SPLIT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_OFFSCREEN_BEHAVIOR: _ClassVar[EnumType]

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
    ERROR_TYPE_INTERRUPTION_CANCELLED: _ClassVar[ErrorType]
    ERROR_TYPE_MODEL_FAILED: _ClassVar[ErrorType]

class ExpressionType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    EXPRESSION_TYPE_UNSPECIFIED: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_LITERAL: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_ATTRIBUTE: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_CONDITION: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_FUNCTION: _ClassVar[ExpressionType]
    EXPRESSION_TYPE_AGGREGATION: _ClassVar[ExpressionType]

class FieldType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FIELD_TYPE_UNSPECIFIED: _ClassVar[FieldType]
    FIELD_TYPE_VARIABLE: _ClassVar[FieldType]
    FIELD_TYPE_INPUT: _ClassVar[FieldType]
    FIELD_TYPE_OUTPUT: _ClassVar[FieldType]

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

class FileRetentionMode(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FILE_RETENTION_MODE_UNSPECIFIED: _ClassVar[FileRetentionMode]
    FILE_RETENTION_MODE_AUTOMATIC: _ClassVar[FileRetentionMode]
    FILE_RETENTION_MODE_MANUAL: _ClassVar[FileRetentionMode]
    FILE_RETENTION_MODE_TIMED: _ClassVar[FileRetentionMode]

class FileSource(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FILE_SOURCE_UNSPECIFIED: _ClassVar[FileSource]
    FILE_SOURCE_BENCH: _ClassVar[FileSource]
    FILE_SOURCE_INLINE: _ClassVar[FileSource]
    FILE_SOURCE_EXTERNAL: _ClassVar[FileSource]

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

class FillPosition(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FILL_POSITION_UNSPECIFIED: _ClassVar[FillPosition]
    FILL_POSITION_TOP_LEFT: _ClassVar[FillPosition]
    FILL_POSITION_TOP_CENTER: _ClassVar[FillPosition]
    FILL_POSITION_TOP_RIGHT: _ClassVar[FillPosition]
    FILL_POSITION_LEFT: _ClassVar[FillPosition]
    FILL_POSITION_CENTER: _ClassVar[FillPosition]
    FILL_POSITION_RIGHT: _ClassVar[FillPosition]
    FILL_POSITION_BOTTOM_LEFT: _ClassVar[FillPosition]
    FILL_POSITION_BOTTOM_CENTER: _ClassVar[FillPosition]
    FILL_POSITION_BOTTOM_RIGHT: _ClassVar[FillPosition]

class FillSize(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FILL_SIZE_UNSPECIFIED: _ClassVar[FillSize]
    FILL_SIZE_FILL: _ClassVar[FillSize]
    FILL_SIZE_STRETCH: _ClassVar[FillSize]
    FILL_SIZE_FIT: _ClassVar[FillSize]
    FILL_SIZE_TILE: _ClassVar[FillSize]

class FillType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FILL_TYPE_UNSPECIFIED: _ClassVar[FillType]
    FILL_TYPE_SOLID: _ClassVar[FillType]
    FILL_TYPE_GRADIENT: _ClassVar[FillType]
    FILL_TYPE_IMAGE: _ClassVar[FillType]

class FlowEdgeType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FLOW_EDGE_TYPE_UNSPECIFIED: _ClassVar[FlowEdgeType]
    FLOW_EDGE_TYPE_MANUAL: _ClassVar[FlowEdgeType]
    FLOW_EDGE_TYPE_DECIDE: _ClassVar[FlowEdgeType]
    FLOW_EDGE_TYPE_REQUIRE: _ClassVar[FlowEdgeType]

class FlowType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FLOW_TYPE_UNSPECIFIED: _ClassVar[FlowType]
    FLOW_TYPE_ACTION: _ClassVar[FlowType]

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
    FONT_TYPE_STYLE: _ClassVar[FontType]
    FONT_TYPE_FIELD: _ClassVar[FontType]
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

class FunctionType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FUNCTION_TYPE_UNSPECIFIED: _ClassVar[FunctionType]
    FUNCTION_TYPE_ADD: _ClassVar[FunctionType]
    FUNCTION_TYPE_SUBTRACT: _ClassVar[FunctionType]
    FUNCTION_TYPE_MULTIPLY: _ClassVar[FunctionType]
    FUNCTION_TYPE_DIVIDE: _ClassVar[FunctionType]
    FUNCTION_TYPE_MODULO: _ClassVar[FunctionType]
    FUNCTION_TYPE_POWER: _ClassVar[FunctionType]

class GradientType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    GRADIENT_TYPE_UNSPECIFIED: _ClassVar[GradientType]
    GRADIENT_TYPE_STYLE: _ClassVar[GradientType]
    GRADIENT_TYPE_LINEAR: _ClassVar[GradientType]
    GRADIENT_TYPE_RADIAL: _ClassVar[GradientType]
    GRADIENT_TYPE_CONIC: _ClassVar[GradientType]

class IconType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ICON_TYPE_UNSPECIFIED: _ClassVar[IconType]
    ICON_TYPE_EMOJI: _ClassVar[IconType]
    ICON_TYPE_FONT_AWESOME: _ClassVar[IconType]
    ICON_TYPE_VS_CODE: _ClassVar[IconType]
    ICON_TYPE_FILE: _ClassVar[IconType]
    ICON_TYPE_FILE_URL: _ClassVar[IconType]

class InterruptionResponse(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    INTERRUPTION_RESPONSE_UNSPECIFIED: _ClassVar[InterruptionResponse]
    INTERRUPTION_RESPONSE_ACCEPT: _ClassVar[InterruptionResponse]
    INTERRUPTION_RESPONSE_REJECT: _ClassVar[InterruptionResponse]

class InterruptionStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    INTERRUPTION_STATUS_UNSPECIFIED: _ClassVar[InterruptionStatus]
    INTERRUPTION_STATUS_OPEN: _ClassVar[InterruptionStatus]
    INTERRUPTION_STATUS_CANCELLED: _ClassVar[InterruptionStatus]
    INTERRUPTION_STATUS_COMPLETED: _ClassVar[InterruptionStatus]

class InterruptionType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    INTERRUPTION_TYPE_UNSPECIFIED: _ClassVar[InterruptionType]
    INTERRUPTION_TYPE_PAUSE: _ClassVar[InterruptionType]
    INTERRUPTION_TYPE_YIELD: _ClassVar[InterruptionType]
    INTERRUPTION_TYPE_WAIT: _ClassVar[InterruptionType]

class JoinType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    JOIN_TYPE_UNSPECIFIED: _ClassVar[JoinType]
    JOIN_TYPE_LEFT: _ClassVar[JoinType]
    JOIN_TYPE_PARENT: _ClassVar[JoinType]
    JOIN_TYPE_CHILD: _ClassVar[JoinType]

class Layout(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    LAYOUT_UNSPECIFIED: _ClassVar[Layout]
    LAYOUT_STACK: _ClassVar[Layout]
    LAYOUT_GRID: _ClassVar[Layout]

class LengthUnit(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    LENGTH_UNIT_UNSPECIFIED: _ClassVar[LengthUnit]
    LENGTH_UNIT_PIXEL: _ClassVar[LengthUnit]
    LENGTH_UNIT_REM: _ClassVar[LengthUnit]
    LENGTH_UNIT_PERCENT: _ClassVar[LengthUnit]
    LENGTH_UNIT_FR: _ClassVar[LengthUnit]

class LinkType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    LINK_TYPE_UNSPECIFIED: _ClassVar[LinkType]
    LINK_TYPE_WEB: _ClassVar[LinkType]

class MachineType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MACHINE_TYPE_UNSPECIFIED: _ClassVar[MachineType]
    MACHINE_TYPE_RUNTIME: _ClassVar[MachineType]
    MACHINE_TYPE_UBUNTU: _ClassVar[MachineType]
    MACHINE_TYPE_MAC: _ClassVar[MachineType]
    MACHINE_TYPE_WINDOWS: _ClassVar[MachineType]
    MACHINE_TYPE_CUSTOM: _ClassVar[MachineType]

class MessageType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MESSAGE_TYPE_UNSPECIFIED: _ClassVar[MessageType]
    MESSAGE_TYPE_DEFAULT: _ClassVar[MessageType]
    MESSAGE_TYPE_JOIN: _ClassVar[MessageType]
    MESSAGE_TYPE_LEAVE: _ClassVar[MessageType]
    MESSAGE_TYPE_RESOURCE: _ClassVar[MessageType]
    MESSAGE_TYPE_RUN: _ClassVar[MessageType]
    MESSAGE_TYPE_THREAD: _ClassVar[MessageType]

class ModelDeveloper(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MODEL_DEVELOPER_UNSPECIFIED: _ClassVar[ModelDeveloper]
    MODEL_DEVELOPER_OPENAI: _ClassVar[ModelDeveloper]
    MODEL_DEVELOPER_ANTHROPIC: _ClassVar[ModelDeveloper]
    MODEL_DEVELOPER_GOOGLE: _ClassVar[ModelDeveloper]
    MODEL_DEVELOPER_XAI: _ClassVar[ModelDeveloper]

class ModelProvider(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MODEL_PROVIDER_UNSPECIFIED: _ClassVar[ModelProvider]
    MODEL_PROVIDER_OPENROUTER: _ClassVar[ModelProvider]
    MODEL_PROVIDER_OPENAI: _ClassVar[ModelProvider]
    MODEL_PROVIDER_ANTHROPIC: _ClassVar[ModelProvider]
    MODEL_PROVIDER_GOOGLE: _ClassVar[ModelProvider]
    MODEL_PROVIDER_XAI: _ClassVar[ModelProvider]

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

class NodeArea(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    NODE_AREA_UNSPECIFIED: _ClassVar[NodeArea]
    NODE_AREA_GLOBAL_POSTGRES: _ClassVar[NodeArea]
    NODE_AREA_MAIN_POSTGRES: _ClassVar[NodeArea]
    NODE_AREA_CUSTOM_POSTGRES: _ClassVar[NodeArea]

class NodeMode(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    NODE_MODE_UNSPECIFIED: _ClassVar[NodeMode]
    NODE_MODE_KERNEL: _ClassVar[NodeMode]
    NODE_MODE_SYSTEM: _ClassVar[NodeMode]
    NODE_MODE_BUILTIN: _ClassVar[NodeMode]
    NODE_MODE_MAIN: _ClassVar[NodeMode]
    NODE_MODE_TEST: _ClassVar[NodeMode]
    NODE_MODE_TEMPLATE: _ClassVar[NodeMode]
    NODE_MODE_ARCHIVE: _ClassVar[NodeMode]

class NodeType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    NODE_TYPE_UNSPECIFIED: _ClassVar[NodeType]
    NODE_TYPE_BENCH: _ClassVar[NodeType]
    NODE_TYPE_BENCH_MEMBERSHIP: _ClassVar[NodeType]
    NODE_TYPE_BENCH_INVITE: _ClassVar[NodeType]
    NODE_TYPE_PACKAGE: _ClassVar[NodeType]
    NODE_TYPE_PACKAGE_MEMBERSHIP: _ClassVar[NodeType]
    NODE_TYPE_PACKAGE_INVITE: _ClassVar[NodeType]
    NODE_TYPE_HANDLE: _ClassVar[NodeType]
    NODE_TYPE_USER: _ClassVar[NodeType]
    NODE_TYPE_FRIENDSHIP: _ClassVar[NodeType]
    NODE_TYPE_FRIENDSHIP_INVITE: _ClassVar[NodeType]
    NODE_TYPE_ORGANIZATION: _ClassVar[NodeType]
    NODE_TYPE_ORGANIZATION_MEMBERSHIP: _ClassVar[NodeType]
    NODE_TYPE_ORGANIZATION_INVITE: _ClassVar[NodeType]
    NODE_TYPE_CLIENT: _ClassVar[NodeType]
    NODE_TYPE_SPACE: _ClassVar[NodeType]
    NODE_TYPE_SCENE: _ClassVar[NodeType]
    NODE_TYPE_ROUTE: _ClassVar[NodeType]
    NODE_TYPE_PAGE: _ClassVar[NodeType]
    NODE_TYPE_BLOCK: _ClassVar[NodeType]
    NODE_TYPE_DATABASE: _ClassVar[NodeType]
    NODE_TYPE_MACHINE: _ClassVar[NodeType]
    NODE_TYPE_SERVICE: _ClassVar[NodeType]
    NODE_TYPE_ACTION: _ClassVar[NodeType]
    NODE_TYPE_FLOW: _ClassVar[NodeType]
    NODE_TYPE_FLOW_EDGE: _ClassVar[NodeType]
    NODE_TYPE_AGENT: _ClassVar[NodeType]
    NODE_TYPE_TASK: _ClassVar[NodeType]
    NODE_TYPE_CURSOR: _ClassVar[NodeType]
    NODE_TYPE_RUN: _ClassVar[NodeType]
    NODE_TYPE_SPAN: _ClassVar[NodeType]
    NODE_TYPE_INTERRUPTION: _ClassVar[NodeType]
    NODE_TYPE_SCHEMA: _ClassVar[NodeType]
    NODE_TYPE_FIELD: _ClassVar[NodeType]
    NODE_TYPE_FILE: _ClassVar[NodeType]
    NODE_TYPE_LINK: _ClassVar[NodeType]
    NODE_TYPE_CUSTOM_NODE_DEFINITION: _ClassVar[NodeType]
    NODE_TYPE_CUSTOM_NODE_INSTANCE: _ClassVar[NodeType]
    NODE_TYPE_THREAD: _ClassVar[NodeType]
    NODE_TYPE_MESSAGE: _ClassVar[NodeType]
    NODE_TYPE_FRAME_VIEW: _ClassVar[NodeType]
    NODE_TYPE_LABEL_VIEW: _ClassVar[NodeType]
    NODE_TYPE_SPLIT_VIEW: _ClassVar[NodeType]
    NODE_TYPE_TEXT_VIEW: _ClassVar[NodeType]
    NODE_TYPE_NUMBER_INPUT_VIEW: _ClassVar[NodeType]
    NODE_TYPE_SLIDER_INPUT_VIEW: _ClassVar[NodeType]
    NODE_TYPE_THREAD_VIEW: _ClassVar[NodeType]
    NODE_TYPE_WIZARD_VIEW: _ClassVar[NodeType]
    NODE_TYPE_THEME: _ClassVar[NodeType]
    NODE_TYPE_COLOR_STYLE: _ClassVar[NodeType]
    NODE_TYPE_FONT_STYLE: _ClassVar[NodeType]
    NODE_TYPE_BORDER_STYLE: _ClassVar[NodeType]
    NODE_TYPE_SHADOW_STYLE: _ClassVar[NodeType]
    NODE_TYPE_GRADIENT_STYLE: _ClassVar[NodeType]
    NODE_TYPE_TRANSITION_STYLE: _ClassVar[NodeType]
    NODE_TYPE_EFFECT_STYLE: _ClassVar[NodeType]

class NumberFormat(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    NUMBER_FORMAT_UNSPECIFIED: _ClassVar[NumberFormat]
    NUMBER_FORMAT_PERCENTAGE: _ClassVar[NumberFormat]
    NUMBER_FORMAT_ANGLE: _ClassVar[NumberFormat]
    NUMBER_FORMAT_CURRENCY: _ClassVar[NumberFormat]

class OffscreenBehavior(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    OFFSCREEN_BEHAVIOR_UNSPECIFIED: _ClassVar[OffscreenBehavior]
    OFFSCREEN_BEHAVIOR_PLAY: _ClassVar[OffscreenBehavior]
    OFFSCREEN_BEHAVIOR_PAUSE: _ClassVar[OffscreenBehavior]

class OrganizationRoleType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ORGANIZATION_ROLE_TYPE_UNSPECIFIED: _ClassVar[OrganizationRoleType]
    ORGANIZATION_ROLE_TYPE_ADMIN: _ClassVar[OrganizationRoleType]
    ORGANIZATION_ROLE_TYPE_MEMBER: _ClassVar[OrganizationRoleType]

class OrganizationStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ORGANIZATION_STATUS_UNSPECIFIED: _ClassVar[OrganizationStatus]
    ORGANIZATION_STATUS_CREATING: _ClassVar[OrganizationStatus]
    ORGANIZATION_STATUS_ACTIVE: _ClassVar[OrganizationStatus]

class Overflow(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    OVERFLOW_UNSPECIFIED: _ClassVar[Overflow]
    OVERFLOW_HIDDEN: _ClassVar[Overflow]
    OVERFLOW_VISIBLE: _ClassVar[Overflow]
    OVERFLOW_SCROLL: _ClassVar[Overflow]

class PackageRoleType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PACKAGE_ROLE_TYPE_UNSPECIFIED: _ClassVar[PackageRoleType]
    PACKAGE_ROLE_TYPE_MEMBER: _ClassVar[PackageRoleType]
    PACKAGE_ROLE_TYPE_ADMIN: _ClassVar[PackageRoleType]

class PackageType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PACKAGE_TYPE_UNSPECIFIED: _ClassVar[PackageType]
    PACKAGE_TYPE_HOME: _ClassVar[PackageType]
    PACKAGE_TYPE_APPLICATION: _ClassVar[PackageType]

class PositionType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    POSITION_TYPE_UNSPECIFIED: _ClassVar[PositionType]
    POSITION_TYPE_RELATIVE: _ClassVar[PositionType]
    POSITION_TYPE_ABSOLUTE: _ClassVar[PositionType]
    POSITION_TYPE_FIXED: _ClassVar[PositionType]
    POSITION_TYPE_STICKY: _ClassVar[PositionType]

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

class ProcessStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PROCESS_STATUS_UNSPECIFIED: _ClassVar[ProcessStatus]
    PROCESS_STATUS_CREATED: _ClassVar[ProcessStatus]
    PROCESS_STATUS_ASSIGNED: _ClassVar[ProcessStatus]
    PROCESS_STATUS_SCHEDULED: _ClassVar[ProcessStatus]
    PROCESS_STATUS_QUEUED: _ClassVar[ProcessStatus]
    PROCESS_STATUS_RUNNING: _ClassVar[ProcessStatus]
    PROCESS_STATUS_FAILING: _ClassVar[ProcessStatus]
    PROCESS_STATUS_PAUSED: _ClassVar[ProcessStatus]
    PROCESS_STATUS_YIELDED: _ClassVar[ProcessStatus]
    PROCESS_STATUS_WAITING: _ClassVar[ProcessStatus]
    PROCESS_STATUS_IDLE: _ClassVar[ProcessStatus]
    PROCESS_STATUS_CANCELLED: _ClassVar[ProcessStatus]
    PROCESS_STATUS_ABORTED: _ClassVar[ProcessStatus]
    PROCESS_STATUS_DIED: _ClassVar[ProcessStatus]
    PROCESS_STATUS_FAILED: _ClassVar[ProcessStatus]
    PROCESS_STATUS_COMPLETED: _ClassVar[ProcessStatus]
    PROCESS_STATUS_SKIPPED: _ClassVar[ProcessStatus]

class QueryType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    QUERY_TYPE_UNSPECIFIED: _ClassVar[QueryType]
    QUERY_TYPE_GET: _ClassVar[QueryType]
    QUERY_TYPE_SEARCH: _ClassVar[QueryType]
    QUERY_TYPE_AGGREGATE: _ClassVar[QueryType]

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

class RelationType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RELATION_TYPE_UNSPECIFIED: _ClassVar[RelationType]
    RELATION_TYPE_BUILTIN_NODE: _ClassVar[RelationType]
    RELATION_TYPE_CUSTOM_NODE: _ClassVar[RelationType]

class RepeatType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    REPEAT_TYPE_UNSPECIFIED: _ClassVar[RepeatType]
    REPEAT_TYPE_LOOP: _ClassVar[RepeatType]
    REPEAT_TYPE_REVERSE: _ClassVar[RepeatType]
    REPEAT_TYPE_MIRROR: _ClassVar[RepeatType]

class ResourceStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RESOURCE_STATUS_UNSPECIFIED: _ClassVar[ResourceStatus]
    RESOURCE_STATUS_PENDING: _ClassVar[ResourceStatus]
    RESOURCE_STATUS_CREATING: _ClassVar[ResourceStatus]
    RESOURCE_STATUS_RETRYING: _ClassVar[ResourceStatus]
    RESOURCE_STATUS_AVAILABLE: _ClassVar[ResourceStatus]
    RESOURCE_STATUS_SLEEPING: _ClassVar[ResourceStatus]
    RESOURCE_STATUS_UNAVAILABLE: _ClassVar[ResourceStatus]
    RESOURCE_STATUS_IMPAIRED: _ClassVar[ResourceStatus]
    RESOURCE_STATUS_OFFLINE: _ClassVar[ResourceStatus]
    RESOURCE_STATUS_FAILED: _ClassVar[ResourceStatus]

class RunType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RUN_TYPE_UNSPECIFIED: _ClassVar[RunType]
    RUN_TYPE_CODE: _ClassVar[RunType]
    RUN_TYPE_ACTION: _ClassVar[RunType]
    RUN_TYPE_FLOW: _ClassVar[RunType]
    RUN_TYPE_AGENT: _ClassVar[RunType]

class ScalarType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SCALAR_TYPE_UNSPECIFIED: _ClassVar[ScalarType]
    SCALAR_TYPE_PRIMITIVE: _ClassVar[ScalarType]
    SCALAR_TYPE_ENUM: _ClassVar[ScalarType]
    SCALAR_TYPE_NODE: _ClassVar[ScalarType]
    SCALAR_TYPE_STRUCT: _ClassVar[ScalarType]

class ScheduleFrequency(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SCHEDULE_FREQUENCY_UNSPECIFIED: _ClassVar[ScheduleFrequency]
    SCHEDULE_FREQUENCY_YEAR: _ClassVar[ScheduleFrequency]
    SCHEDULE_FREQUENCY_MONTH: _ClassVar[ScheduleFrequency]
    SCHEDULE_FREQUENCY_WEEK: _ClassVar[ScheduleFrequency]
    SCHEDULE_FREQUENCY_DAY: _ClassVar[ScheduleFrequency]
    SCHEDULE_FREQUENCY_HOUR: _ClassVar[ScheduleFrequency]
    SCHEDULE_FREQUENCY_MINUTE: _ClassVar[ScheduleFrequency]

class ShadowPosition(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SHADOW_POSITION_UNSPECIFIED: _ClassVar[ShadowPosition]
    SHADOW_POSITION_OUTSIDE: _ClassVar[ShadowPosition]
    SHADOW_POSITION_INSIDE: _ClassVar[ShadowPosition]

class ShadowType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SHADOW_TYPE_UNSPECIFIED: _ClassVar[ShadowType]
    SHADOW_TYPE_STYLE: _ClassVar[ShadowType]
    SHADOW_TYPE_FIELD: _ClassVar[ShadowType]
    SHADOW_TYPE_BOX: _ClassVar[ShadowType]
    SHADOW_TYPE_REALISTIC: _ClassVar[ShadowType]

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

class SpaceType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SPACE_TYPE_UNSPECIFIED: _ClassVar[SpaceType]
    SPACE_TYPE_BROWSER: _ClassVar[SpaceType]
    SPACE_TYPE_DESKTOP: _ClassVar[SpaceType]
    SPACE_TYPE_MOBILE: _ClassVar[SpaceType]

class SpanType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SPAN_TYPE_UNSPECIFIED: _ClassVar[SpanType]
    SPAN_TYPE_ATTEMPT: _ClassVar[SpanType]
    SPAN_TYPE_WAIT: _ClassVar[SpanType]
    SPAN_TYPE_ACQUIRE: _ClassVar[SpanType]
    SPAN_TYPE_CODE: _ClassVar[SpanType]
    SPAN_TYPE_AGENT_TURN: _ClassVar[SpanType]
    SPAN_TYPE_MODEL_PREPARE: _ClassVar[SpanType]
    SPAN_TYPE_MODEL_GENERATE: _ClassVar[SpanType]
    SPAN_TYPE_MODEL_PARSE: _ClassVar[SpanType]
    SPAN_TYPE_FILE_UPLOAD: _ClassVar[SpanType]
    SPAN_TYPE_FILE_PREPARE_UPLOAD: _ClassVar[SpanType]
    SPAN_TYPE_FILE_DOWNLOAD: _ClassVar[SpanType]
    SPAN_TYPE_FILE_PREPARE_DOWNLOAD: _ClassVar[SpanType]

class SpringType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SPRING_TYPE_UNSPECIFIED: _ClassVar[SpringType]
    SPRING_TYPE_TIME: _ClassVar[SpringType]
    SPRING_TYPE_PHYSICS: _ClassVar[SpringType]

class StringFormat(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    STRING_FORMAT_UNSPECIFIED: _ClassVar[StringFormat]
    STRING_FORMAT_NAME: _ClassVar[StringFormat]
    STRING_FORMAT_SLUG: _ClassVar[StringFormat]
    STRING_FORMAT_EMAIL: _ClassVar[StringFormat]
    STRING_FORMAT_UUID: _ClassVar[StringFormat]
    STRING_FORMAT_URL: _ClassVar[StringFormat]
    STRING_FORMAT_EMOJI: _ClassVar[StringFormat]
    STRING_FORMAT_MIME: _ClassVar[StringFormat]
    STRING_FORMAT_BASE64: _ClassVar[StringFormat]

class StructType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    STRUCT_TYPE_UNSPECIFIED: _ClassVar[StructType]
    STRUCT_TYPE_SCOPE: _ClassVar[StructType]
    STRUCT_TYPE_ORIGIN: _ClassVar[StructType]
    STRUCT_TYPE_NODE_REFERENCE: _ClassVar[StructType]
    STRUCT_TYPE_PROPERTY_REFERENCE: _ClassVar[StructType]
    STRUCT_TYPE_PROPERTY_INFO: _ClassVar[StructType]
    STRUCT_TYPE_TRAIT_INFO: _ClassVar[StructType]
    STRUCT_TYPE_NODE_INFO: _ClassVar[StructType]
    STRUCT_TYPE_STRUCT_INFO: _ClassVar[StructType]
    STRUCT_TYPE_ENUM_INFO: _ClassVar[StructType]
    STRUCT_TYPE_ENUM_OPTION_INFO: _ClassVar[StructType]
    STRUCT_TYPE_EDIT: _ClassVar[StructType]
    STRUCT_TYPE_CHANGE: _ClassVar[StructType]
    STRUCT_TYPE_CHANGE_RESULT: _ClassVar[StructType]
    STRUCT_TYPE_EXPRESSION: _ClassVar[StructType]
    STRUCT_TYPE_FUNCTION: _ClassVar[StructType]
    STRUCT_TYPE_JOIN: _ClassVar[StructType]
    STRUCT_TYPE_AGGREGATION: _ClassVar[StructType]
    STRUCT_TYPE_CONDITION: _ClassVar[StructType]
    STRUCT_TYPE_SORT: _ClassVar[StructType]
    STRUCT_TYPE_SELECT: _ClassVar[StructType]
    STRUCT_TYPE_RELATION_REFERENCE: _ClassVar[StructType]
    STRUCT_TYPE_ATTRIBUTE_REFERENCE: _ClassVar[StructType]
    STRUCT_TYPE_QUERY: _ClassVar[StructType]
    STRUCT_TYPE_QUERY_RESULT: _ClassVar[StructType]
    STRUCT_TYPE_QUERY_UPDATE: _ClassVar[StructType]
    STRUCT_TYPE_VARIABLE: _ClassVar[StructType]
    STRUCT_TYPE_SCHEDULE: _ClassVar[StructType]
    STRUCT_TYPE_ERROR: _ClassVar[StructType]
    STRUCT_TYPE_TYPE: _ClassVar[StructType]
    STRUCT_TYPE_NUMBER_CONSTRAINT: _ClassVar[StructType]
    STRUCT_TYPE_STRING_CONSTRAINT: _ClassVar[StructType]
    STRUCT_TYPE_COLLECTION_CONSTRAINT: _ClassVar[StructType]
    STRUCT_TYPE_NODE_CONSTRAINT: _ClassVar[StructType]
    STRUCT_TYPE_TEXT: _ClassVar[StructType]
    STRUCT_TYPE_TEXT_LINE: _ClassVar[StructType]
    STRUCT_TYPE_TEXT_SPAN: _ClassVar[StructType]
    STRUCT_TYPE_CODE: _ClassVar[StructType]
    STRUCT_TYPE_ICON: _ClassVar[StructType]
    STRUCT_TYPE_SELECTION: _ClassVar[StructType]
    STRUCT_TYPE_VALUE: _ClassVar[StructType]
    STRUCT_TYPE_VECTOR2: _ClassVar[StructType]
    STRUCT_TYPE_VECTOR3: _ClassVar[StructType]
    STRUCT_TYPE_VECTOR4: _ClassVar[StructType]
    STRUCT_TYPE_AXIS2: _ClassVar[StructType]
    STRUCT_TYPE_AXIS3: _ClassVar[StructType]
    STRUCT_TYPE_COLOR: _ClassVar[StructType]
    STRUCT_TYPE_SHADOW: _ClassVar[StructType]
    STRUCT_TYPE_BORDER: _ClassVar[StructType]
    STRUCT_TYPE_FONT: _ClassVar[StructType]
    STRUCT_TYPE_GRADIENT_STOP: _ClassVar[StructType]
    STRUCT_TYPE_GRADIENT: _ClassVar[StructType]
    STRUCT_TYPE_FILL: _ClassVar[StructType]
    STRUCT_TYPE_LENGTH: _ClassVar[StructType]
    STRUCT_TYPE_POSITION: _ClassVar[StructType]
    STRUCT_TYPE_DIMENSION: _ClassVar[StructType]
    STRUCT_TYPE_TRANSITION: _ClassVar[StructType]
    STRUCT_TYPE_EFFECT: _ClassVar[StructType]
    STRUCT_TYPE_GRID: _ClassVar[StructType]
    STRUCT_TYPE_GRID_SPAN: _ClassVar[StructType]
    STRUCT_TYPE_INSETS: _ClassVar[StructType]
    STRUCT_TYPE_CORNERS: _ClassVar[StructType]

class TextAlign(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TEXT_ALIGN_UNSPECIFIED: _ClassVar[TextAlign]
    TEXT_ALIGN_LEFT: _ClassVar[TextAlign]
    TEXT_ALIGN_CENTER: _ClassVar[TextAlign]
    TEXT_ALIGN_RIGHT: _ClassVar[TextAlign]
    TEXT_ALIGN_JUSTIFY: _ClassVar[TextAlign]

class TextDecoration(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TEXT_DECORATION_UNSPECIFIED: _ClassVar[TextDecoration]
    TEXT_DECORATION_NONE: _ClassVar[TextDecoration]
    TEXT_DECORATION_UNDERLINE: _ClassVar[TextDecoration]
    TEXT_DECORATION_STRIKETHROUGH: _ClassVar[TextDecoration]

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
    TEXT_LINE_TYPE_CODE: _ClassVar[TextLineType]
    TEXT_LINE_TYPE_NODE: _ClassVar[TextLineType]

class TextSpanType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TEXT_SPAN_TYPE_UNSPECIFIED: _ClassVar[TextSpanType]
    TEXT_SPAN_TYPE_TEXT: _ClassVar[TextSpanType]
    TEXT_SPAN_TYPE_HARD_BREAK: _ClassVar[TextSpanType]
    TEXT_SPAN_TYPE_MENTION: _ClassVar[TextSpanType]
    TEXT_SPAN_TYPE_LINK: _ClassVar[TextSpanType]
    TEXT_SPAN_TYPE_CITATION: _ClassVar[TextSpanType]
    TEXT_SPAN_TYPE_EQUATION: _ClassVar[TextSpanType]

class TextSplitType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TEXT_SPLIT_TYPE_UNSPECIFIED: _ClassVar[TextSplitType]
    TEXT_SPLIT_TYPE_CHAR: _ClassVar[TextSplitType]
    TEXT_SPLIT_TYPE_WORD: _ClassVar[TextSplitType]
    TEXT_SPLIT_TYPE_LINE: _ClassVar[TextSplitType]

class TextTransform(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TEXT_TRANSFORM_UNSPECIFIED: _ClassVar[TextTransform]
    TEXT_TRANSFORM_NONE: _ClassVar[TextTransform]
    TEXT_TRANSFORM_UPPERCASE: _ClassVar[TextTransform]
    TEXT_TRANSFORM_LOWERCASE: _ClassVar[TextTransform]
    TEXT_TRANSFORM_CAPITALIZE: _ClassVar[TextTransform]

class ThemeColor(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    THEME_COLOR_UNSPECIFIED: _ClassVar[ThemeColor]
    THEME_COLOR_PRIMARY: _ClassVar[ThemeColor]
    THEME_COLOR_SECONDARY: _ClassVar[ThemeColor]
    THEME_COLOR_ACCENT: _ClassVar[ThemeColor]
    THEME_COLOR_MUTED: _ClassVar[ThemeColor]
    THEME_COLOR_SUCCESS: _ClassVar[ThemeColor]
    THEME_COLOR_WARNING: _ClassVar[ThemeColor]
    THEME_COLOR_ERROR: _ClassVar[ThemeColor]

class ThreadStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    THREAD_STATUS_UNSPECIFIED: _ClassVar[ThreadStatus]
    THREAD_STATUS_OPEN: _ClassVar[ThreadStatus]
    THREAD_STATUS_CLOSED: _ClassVar[ThreadStatus]

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

class TraitType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TRAIT_TYPE_UNSPECIFIED: _ClassVar[TraitType]
    TRAIT_TYPE_GLOBAL: _ClassVar[TraitType]
    TRAIT_TYPE_CUSTOM: _ClassVar[TraitType]
    TRAIT_TYPE_MODAL: _ClassVar[TraitType]
    TRAIT_TYPE_ARCHIVABLE: _ClassVar[TraitType]
    TRAIT_TYPE_DELETABLE: _ClassVar[TraitType]
    TRAIT_TYPE_NAMED: _ClassVar[TraitType]
    TRAIT_TYPE_TITLED: _ClassVar[TraitType]
    TRAIT_TYPE_SLUG: _ClassVar[TraitType]
    TRAIT_TYPE_ICON: _ClassVar[TraitType]
    TRAIT_TYPE_ORDERED: _ClassVar[TraitType]
    TRAIT_TYPE_TEMPLATABLE: _ClassVar[TraitType]
    TRAIT_TYPE_EXTENSIBLE: _ClassVar[TraitType]
    TRAIT_TYPE_IN_BENCH: _ClassVar[TraitType]
    TRAIT_TYPE_IN_PACKAGE: _ClassVar[TraitType]
    TRAIT_TYPE_REGIONAL: _ClassVar[TraitType]
    TRAIT_TYPE_RESOURCE: _ClassVar[TraitType]
    TRAIT_TYPE_PROVISIONABLE: _ClassVar[TraitType]
    TRAIT_TYPE_OWNABLE: _ClassVar[TraitType]
    TRAIT_TYPE_JOINABLE: _ClassVar[TraitType]
    TRAIT_TYPE_SUBJECT: _ClassVar[TraitType]
    TRAIT_TYPE_MEMBERSHIP: _ClassVar[TraitType]
    TRAIT_TYPE_INVITE: _ClassVar[TraitType]
    TRAIT_TYPE_ROLE: _ClassVar[TraitType]
    TRAIT_TYPE_BLOCKABLE: _ClassVar[TraitType]
    TRAIT_TYPE_RUNNABLE: _ClassVar[TraitType]
    TRAIT_TYPE_PROCESSABLE: _ClassVar[TraitType]
    TRAIT_TYPE_COMPUTABLE: _ClassVar[TraitType]
    TRAIT_TYPE_VIEW: _ClassVar[TraitType]
    TRAIT_TYPE_CONTAINER_VIEW: _ClassVar[TraitType]
    TRAIT_TYPE_CONTENT_VIEW: _ClassVar[TraitType]
    TRAIT_TYPE_INPUT_VIEW: _ClassVar[TraitType]
    TRAIT_TYPE_NODE_VIEW: _ClassVar[TraitType]
    TRAIT_TYPE_INTERNAL_VIEW: _ClassVar[TraitType]
    TRAIT_TYPE_STYLE: _ClassVar[TraitType]

class TransitionType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TRANSITION_TYPE_UNSPECIFIED: _ClassVar[TransitionType]
    TRANSITION_TYPE_STYLE: _ClassVar[TransitionType]
    TRANSITION_TYPE_FIELD: _ClassVar[TransitionType]
    TRANSITION_TYPE_TWEEN: _ClassVar[TransitionType]
    TRANSITION_TYPE_SPRING: _ClassVar[TransitionType]

class TypeCardinality(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TYPE_CARDINALITY_UNSPECIFIED: _ClassVar[TypeCardinality]
    TYPE_CARDINALITY_SCALAR: _ClassVar[TypeCardinality]
    TYPE_CARDINALITY_LIST: _ClassVar[TypeCardinality]
    TYPE_CARDINALITY_MAP: _ClassVar[TypeCardinality]

class UpdateType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    UPDATE_TYPE_UNSPECIFIED: _ClassVar[UpdateType]
    UPDATE_TYPE_SET: _ClassVar[UpdateType]
    UPDATE_TYPE_CLEAR: _ClassVar[UpdateType]
    UPDATE_TYPE_MAP_SET: _ClassVar[UpdateType]
    UPDATE_TYPE_MAP_REMOVE: _ClassVar[UpdateType]

class UserStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    USER_STATUS_UNSPECIFIED: _ClassVar[UserStatus]
    USER_STATUS_CREATING: _ClassVar[UserStatus]
    USER_STATUS_ACTIVE: _ClassVar[UserStatus]

class VariableType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    VARIABLE_TYPE_UNSPECIFIED: _ClassVar[VariableType]
    VARIABLE_TYPE_FIELD: _ClassVar[VariableType]
ACTION_CARDINALITY_UNSPECIFIED: ActionCardinality
ACTION_CARDINALITY_UNARY: ActionCardinality
AGGREGATION_TYPE_UNSPECIFIED: AggregationType
AGGREGATION_TYPE_EXISTS: AggregationType
AGGREGATION_TYPE_COUNT: AggregationType
AGGREGATION_TYPE_SUM: AggregationType
AGGREGATION_TYPE_MIN: AggregationType
AGGREGATION_TYPE_MAX: AggregationType
AGGREGATION_TYPE_AVERAGE: AggregationType
AGGREGATION_TYPE_MEDIAN: AggregationType
AGGREGATION_TYPE_HISTOGRAM: AggregationType
ALIGN_UNSPECIFIED: Align
ALIGN_START: Align
ALIGN_CENTER: Align
ALIGN_END: Align
AREA_UNSPECIFIED: Area
AREA_EUROPE_CENTRAL: Area
AREA_NORTH_AMERICA_EAST: Area
AREA_NORTH_AMERICA_WEST: Area
AREA_SOUTH_AMERICA_EAST: Area
AREA_MIDDLE_EAST_CENTRAL: Area
AREA_MIDDLE_EAST_WEST: Area
AREA_AFRICA_SOUTH: Area
AREA_ASIA_WEST: Area
AREA_ASIA_SOUTH: Area
AREA_ASIA_EAST: Area
AREA_AUSTRALIA_SOUTH: Area
ATTRIBUTE_TYPE_UNSPECIFIED: AttributeType
ATTRIBUTE_TYPE_PROPERTY: AttributeType
ATTRIBUTE_TYPE_FIELD: AttributeType
ATTRIBUTE_TYPE_QUERY: AttributeType
BENCH_ROLE_TYPE_UNSPECIFIED: BenchRoleType
BENCH_ROLE_TYPE_ADMIN: BenchRoleType
BENCH_ROLE_TYPE_MEMBER: BenchRoleType
BENCH_STATUS_UNSPECIFIED: BenchStatus
BENCH_STATUS_CREATING: BenchStatus
BENCH_STATUS_ACTIVE: BenchStatus
BLOCK_TYPE_UNSPECIFIED: BlockType
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
BLOCK_TYPE_CODE: BlockType
BLOCK_TYPE_NODE: BlockType
BORDER_TYPE_UNSPECIFIED: BorderType
BORDER_TYPE_NONE: BorderType
BORDER_TYPE_STYLE: BorderType
BORDER_TYPE_FIELD: BorderType
BORDER_TYPE_SOLID: BorderType
BORDER_TYPE_DASHED: BorderType
BORDER_TYPE_DOTTED: BorderType
BORDER_TYPE_DOUBLE: BorderType
CASCADE_ACTION_UNSPECIFIED: CascadeAction
CASCADE_ACTION_RESTRICT: CascadeAction
CASCADE_ACTION_CASCADE: CascadeAction
CASCADE_ACTION_SET_NULL: CascadeAction
CHANGE_STATUS_UNSPECIFIED: ChangeStatus
CHANGE_STATUS_COMPLETED: ChangeStatus
CHANGE_STATUS_FAILED: ChangeStatus
CLIENT_TYPE_UNSPECIFIED: ClientType
CLIENT_TYPE_WEB: ClientType
CLIENT_TYPE_BROWSER_PLUGIN: ClientType
CLIENT_TYPE_DESKTOP: ClientType
CLIENT_TYPE_MOBILE: ClientType
CLIENT_TYPE_MACHINE: ClientType
CLOUD_UNSPECIFIED: Cloud
CLOUD_AWS: Cloud
CLOUD_AZURE: Cloud
CLOUD_GCP: Cloud
CLOUD_OCI: Cloud
CLOUD_ALIBABA: Cloud
CLOUD_HETZNER: Cloud
CLOUD_PRIVATE: Cloud
CODE_TYPE_UNSPECIFIED: CodeType
CODE_TYPE_SNIPPET: CodeType
CODE_TYPE_SCRIPT: CodeType
CODE_TYPE_FUNCTION: CodeType
COLOR_HUE_UNSPECIFIED: ColorHue
COLOR_HUE_GRAY: ColorHue
COLOR_HUE_RED: ColorHue
COLOR_HUE_ORANGE: ColorHue
COLOR_HUE_AMBER: ColorHue
COLOR_HUE_YELLOW: ColorHue
COLOR_HUE_LIME: ColorHue
COLOR_HUE_GREEN: ColorHue
COLOR_HUE_EMERALD: ColorHue
COLOR_HUE_TEAL: ColorHue
COLOR_HUE_CYAN: ColorHue
COLOR_HUE_SKY: ColorHue
COLOR_HUE_BLUE: ColorHue
COLOR_HUE_INDIGO: ColorHue
COLOR_HUE_VIOLET: ColorHue
COLOR_HUE_PURPLE: ColorHue
COLOR_HUE_FUCHSIA: ColorHue
COLOR_HUE_PINK: ColorHue
COLOR_HUE_ROSE: ColorHue
COLOR_SHADE_UNSPECIFIED: ColorShade
COLOR_SHADE_S25: ColorShade
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
COLOR_TYPE_BUILTIN: ColorType
COLOR_TYPE_STYLE: ColorType
COLOR_TYPE_FIELD: ColorType
COLOR_TYPE_RGB: ColorType
COLOR_TYPE_HSL: ColorType
COLOR_TYPE_P3: ColorType
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
CONDITIONAL_TYPE_CONTAINS: ConditionalType
CONDITIONAL_TYPE_NOT_CONTAINS: ConditionalType
CONDITIONAL_TYPE_IN: ConditionalType
CONDITIONAL_TYPE_NOT_IN: ConditionalType
CONDITIONAL_TYPE_EXISTS: ConditionalType
CONDITIONAL_TYPE_NOT_EXISTS: ConditionalType
CONTINENT_UNSPECIFIED: Continent
CONTINENT_EUROPE: Continent
CONTINENT_NORTH_AMERICA: Continent
CONTINENT_SOUTH_AMERICA: Continent
CONTINENT_MIDDLE_EAST: Continent
CONTINENT_AFRICA: Continent
CONTINENT_ASIA: Continent
CONTINENT_AUSTRALIA: Continent
CONTINENT_PRIVATE: Continent
CURSOR_STATUS_UNSPECIFIED: CursorStatus
CURSOR_STATUS_CREATED: CursorStatus
CURSOR_STATUS_WORKING: CursorStatus
CURSOR_STATUS_READING: CursorStatus
CURSOR_STATUS_WRITING: CursorStatus
CURSOR_STATUS_THINKING: CursorStatus
CURSOR_STATUS_WAITING: CursorStatus
CURSOR_STATUS_IDLE: CursorStatus
CURSOR_STATUS_CANCELLED: CursorStatus
CURSOR_STATUS_COMPLETED: CursorStatus
CURSOR_TYPE_UNSPECIFIED: CursorType
CURSOR_TYPE_THREAD: CursorType
CURSOR_TYPE_PAGE: CursorType
CURSOR_TYPE_TABLE: CursorType
CURSOR_TYPE_ACTION: CursorType
CURSOR_TYPE_WEB: CursorType
CURSOR_TYPE_CUSTOM: CursorType
DATABASE_TYPE_UNSPECIFIED: DatabaseType
DATABASE_TYPE_POSTGRES: DatabaseType
DAY_UNSPECIFIED: Day
DAY_MONDAY: Day
DAY_TUESDAY: Day
DAY_WEDNESDAY: Day
DAY_THURSDAY: Day
DAY_FRIDAY: Day
DAY_SATURDAY: Day
DAY_SUNDAY: Day
DEFAULT_FACTORY_UNSPECIFIED: DefaultFactory
DEFAULT_FACTORY_UUID: DefaultFactory
DEFAULT_FACTORY_NOW: DefaultFactory
DEFAULT_FACTORY_REGION: DefaultFactory
DIMENSION_TYPE_UNSPECIFIED: DimensionType
DIMENSION_TYPE_FIXED: DimensionType
DIMENSION_TYPE_FIT: DimensionType
DIMENSION_TYPE_FILL: DimensionType
DIRECTION_UNSPECIFIED: Direction
DIRECTION_HORIZONTAL: Direction
DIRECTION_VERTICAL: Direction
DISTRIBUTE_UNSPECIFIED: Distribute
DISTRIBUTE_START: Distribute
DISTRIBUTE_CENTER: Distribute
DISTRIBUTE_END: Distribute
DISTRIBUTE_SPACE_BETWEEN: Distribute
DISTRIBUTE_SPACE_AROUND: Distribute
DISTRIBUTE_SPACE_EVENLY: Distribute
EDGE_TYPE_UNSPECIFIED: EdgeType
EDGE_TYPE_NODE_PARENT: EdgeType
EDGE_TYPE_NODE_ANCESTOR: EdgeType
EDGE_TYPE_NODE_REGULAR: EdgeType
EDGE_TYPE_NODE_TEMPLATE: EdgeType
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
EFFECT_TYPE_UNSPECIFIED: EffectType
EFFECT_TYPE_NONE: EffectType
EFFECT_TYPE_STYLE: EffectType
EFFECT_TYPE_FIELD: EffectType
EFFECT_TYPE_APPEAR: EffectType
EFFECT_TYPE_ENTER: EffectType
EFFECT_TYPE_EXIT: EffectType
EFFECT_TYPE_HOVER: EffectType
EFFECT_TYPE_PRESS: EffectType
EFFECT_TYPE_DRAG: EffectType
EFFECT_TYPE_FOCUS: EffectType
EFFECT_TYPE_LOOP: EffectType
ENUM_TYPE_UNSPECIFIED: EnumType
ENUM_TYPE_ENUM_TYPE: EnumType
ENUM_TYPE_NODE_TYPE: EnumType
ENUM_TYPE_STRUCT_TYPE: EnumType
ENUM_TYPE_TRAIT_TYPE: EnumType
ENUM_TYPE_NODE_MODE: EnumType
ENUM_TYPE_NODE_AREA: EnumType
ENUM_TYPE_USER_STATUS: EnumType
ENUM_TYPE_ORGANIZATION_STATUS: EnumType
ENUM_TYPE_BENCH_STATUS: EnumType
ENUM_TYPE_PACKAGE_TYPE: EnumType
ENUM_TYPE_ERROR_TYPE: EnumType
ENUM_TYPE_VARIABLE_TYPE: EnumType
ENUM_TYPE_EDIT_TYPE: EnumType
ENUM_TYPE_UPDATE_TYPE: EnumType
ENUM_TYPE_CHANGE_STATUS: EnumType
ENUM_TYPE_CONDITIONAL_TYPE: EnumType
ENUM_TYPE_AGGREGATION_TYPE: EnumType
ENUM_TYPE_SORT_MODE: EnumType
ENUM_TYPE_SORT_TYPE: EnumType
ENUM_TYPE_JOIN_TYPE: EnumType
ENUM_TYPE_FUNCTION_TYPE: EnumType
ENUM_TYPE_EXPRESSION_TYPE: EnumType
ENUM_TYPE_RELATION_TYPE: EnumType
ENUM_TYPE_ATTRIBUTE_TYPE: EnumType
ENUM_TYPE_QUERY_TYPE: EnumType
ENUM_TYPE_BENCH_ROLE_TYPE: EnumType
ENUM_TYPE_ORGANIZATION_ROLE_TYPE: EnumType
ENUM_TYPE_PACKAGE_ROLE_TYPE: EnumType
ENUM_TYPE_SPACE_TYPE: EnumType
ENUM_TYPE_BLOCK_TYPE: EnumType
ENUM_TYPE_CLOUD: EnumType
ENUM_TYPE_REGION: EnumType
ENUM_TYPE_AREA: EnumType
ENUM_TYPE_CONTINENT: EnumType
ENUM_TYPE_MACHINE_TYPE: EnumType
ENUM_TYPE_DATABASE_TYPE: EnumType
ENUM_TYPE_CLIENT_TYPE: EnumType
ENUM_TYPE_ACTION_CARDINALITY: EnumType
ENUM_TYPE_FLOW_TYPE: EnumType
ENUM_TYPE_FLOW_EDGE_TYPE: EnumType
ENUM_TYPE_CURSOR_TYPE: EnumType
ENUM_TYPE_CURSOR_STATUS: EnumType
ENUM_TYPE_PROCESS_STATUS: EnumType
ENUM_TYPE_RUN_TYPE: EnumType
ENUM_TYPE_SPAN_TYPE: EnumType
ENUM_TYPE_SCHEDULE_FREQUENCY: EnumType
ENUM_TYPE_INTERRUPTION_TYPE: EnumType
ENUM_TYPE_INTERRUPTION_STATUS: EnumType
ENUM_TYPE_INTERRUPTION_RESPONSE: EnumType
ENUM_TYPE_TEXT_LINE_TYPE: EnumType
ENUM_TYPE_TEXT_SPAN_TYPE: EnumType
ENUM_TYPE_CODE_TYPE: EnumType
ENUM_TYPE_FILE_RETENTION_MODE: EnumType
ENUM_TYPE_FILE_SOURCE: EnumType
ENUM_TYPE_FILE_TYPE: EnumType
ENUM_TYPE_FILE_FORMAT: EnumType
ENUM_TYPE_ICON_TYPE: EnumType
ENUM_TYPE_LINK_TYPE: EnumType
ENUM_TYPE_PRIMITIVE_TYPE: EnumType
ENUM_TYPE_TYPE_CARDINALITY: EnumType
ENUM_TYPE_SCALAR_TYPE: EnumType
ENUM_TYPE_DEFAULT_FACTORY: EnumType
ENUM_TYPE_STRING_FORMAT: EnumType
ENUM_TYPE_NUMBER_FORMAT: EnumType
ENUM_TYPE_FIELD_TYPE: EnumType
ENUM_TYPE_EDGE_TYPE: EnumType
ENUM_TYPE_CASCADE_ACTION: EnumType
ENUM_TYPE_DAY: EnumType
ENUM_TYPE_MONTH: EnumType
ENUM_TYPE_TIME_INTERVAL: EnumType
ENUM_TYPE_RESOURCE_STATUS: EnumType
ENUM_TYPE_THREAD_STATUS: EnumType
ENUM_TYPE_MESSAGE_TYPE: EnumType
ENUM_TYPE_MODEL_DEVELOPER: EnumType
ENUM_TYPE_MODEL_PROVIDER: EnumType
ENUM_TYPE_POSITION_TYPE: EnumType
ENUM_TYPE_COLOR_TYPE: EnumType
ENUM_TYPE_COLOR_SHADE: EnumType
ENUM_TYPE_COLOR_HUE: EnumType
ENUM_TYPE_FONT_WEIGHT: EnumType
ENUM_TYPE_FONT_SIZE: EnumType
ENUM_TYPE_FONT_TYPE: EnumType
ENUM_TYPE_TEXT_ALIGN: EnumType
ENUM_TYPE_TEXT_DECORATION: EnumType
ENUM_TYPE_TEXT_TRANSFORM: EnumType
ENUM_TYPE_SHADOW_TYPE: EnumType
ENUM_TYPE_SHADOW_POSITION: EnumType
ENUM_TYPE_BORDER_TYPE: EnumType
ENUM_TYPE_GRADIENT_TYPE: EnumType
ENUM_TYPE_FILL_TYPE: EnumType
ENUM_TYPE_FILL_POSITION: EnumType
ENUM_TYPE_FILL_SIZE: EnumType
ENUM_TYPE_LENGTH_UNIT: EnumType
ENUM_TYPE_LAYOUT: EnumType
ENUM_TYPE_DISTRIBUTE: EnumType
ENUM_TYPE_ALIGN: EnumType
ENUM_TYPE_DIRECTION: EnumType
ENUM_TYPE_OVERFLOW: EnumType
ENUM_TYPE_TRANSITION_TYPE: EnumType
ENUM_TYPE_SPRING_TYPE: EnumType
ENUM_TYPE_DIMENSION_TYPE: EnumType
ENUM_TYPE_THEME_COLOR: EnumType
ENUM_TYPE_EFFECT_TYPE: EnumType
ENUM_TYPE_REPEAT_TYPE: EnumType
ENUM_TYPE_TEXT_SPLIT_TYPE: EnumType
ENUM_TYPE_OFFSCREEN_BEHAVIOR: EnumType
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
ERROR_TYPE_INTERRUPTION_CANCELLED: ErrorType
ERROR_TYPE_MODEL_FAILED: ErrorType
EXPRESSION_TYPE_UNSPECIFIED: ExpressionType
EXPRESSION_TYPE_LITERAL: ExpressionType
EXPRESSION_TYPE_ATTRIBUTE: ExpressionType
EXPRESSION_TYPE_CONDITION: ExpressionType
EXPRESSION_TYPE_FUNCTION: ExpressionType
EXPRESSION_TYPE_AGGREGATION: ExpressionType
FIELD_TYPE_UNSPECIFIED: FieldType
FIELD_TYPE_VARIABLE: FieldType
FIELD_TYPE_INPUT: FieldType
FIELD_TYPE_OUTPUT: FieldType
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
FILE_RETENTION_MODE_UNSPECIFIED: FileRetentionMode
FILE_RETENTION_MODE_AUTOMATIC: FileRetentionMode
FILE_RETENTION_MODE_MANUAL: FileRetentionMode
FILE_RETENTION_MODE_TIMED: FileRetentionMode
FILE_SOURCE_UNSPECIFIED: FileSource
FILE_SOURCE_BENCH: FileSource
FILE_SOURCE_INLINE: FileSource
FILE_SOURCE_EXTERNAL: FileSource
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
FILL_POSITION_UNSPECIFIED: FillPosition
FILL_POSITION_TOP_LEFT: FillPosition
FILL_POSITION_TOP_CENTER: FillPosition
FILL_POSITION_TOP_RIGHT: FillPosition
FILL_POSITION_LEFT: FillPosition
FILL_POSITION_CENTER: FillPosition
FILL_POSITION_RIGHT: FillPosition
FILL_POSITION_BOTTOM_LEFT: FillPosition
FILL_POSITION_BOTTOM_CENTER: FillPosition
FILL_POSITION_BOTTOM_RIGHT: FillPosition
FILL_SIZE_UNSPECIFIED: FillSize
FILL_SIZE_FILL: FillSize
FILL_SIZE_STRETCH: FillSize
FILL_SIZE_FIT: FillSize
FILL_SIZE_TILE: FillSize
FILL_TYPE_UNSPECIFIED: FillType
FILL_TYPE_SOLID: FillType
FILL_TYPE_GRADIENT: FillType
FILL_TYPE_IMAGE: FillType
FLOW_EDGE_TYPE_UNSPECIFIED: FlowEdgeType
FLOW_EDGE_TYPE_MANUAL: FlowEdgeType
FLOW_EDGE_TYPE_DECIDE: FlowEdgeType
FLOW_EDGE_TYPE_REQUIRE: FlowEdgeType
FLOW_TYPE_UNSPECIFIED: FlowType
FLOW_TYPE_ACTION: FlowType
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
FONT_TYPE_STYLE: FontType
FONT_TYPE_FIELD: FontType
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
FUNCTION_TYPE_UNSPECIFIED: FunctionType
FUNCTION_TYPE_ADD: FunctionType
FUNCTION_TYPE_SUBTRACT: FunctionType
FUNCTION_TYPE_MULTIPLY: FunctionType
FUNCTION_TYPE_DIVIDE: FunctionType
FUNCTION_TYPE_MODULO: FunctionType
FUNCTION_TYPE_POWER: FunctionType
GRADIENT_TYPE_UNSPECIFIED: GradientType
GRADIENT_TYPE_STYLE: GradientType
GRADIENT_TYPE_LINEAR: GradientType
GRADIENT_TYPE_RADIAL: GradientType
GRADIENT_TYPE_CONIC: GradientType
ICON_TYPE_UNSPECIFIED: IconType
ICON_TYPE_EMOJI: IconType
ICON_TYPE_FONT_AWESOME: IconType
ICON_TYPE_VS_CODE: IconType
ICON_TYPE_FILE: IconType
ICON_TYPE_FILE_URL: IconType
INTERRUPTION_RESPONSE_UNSPECIFIED: InterruptionResponse
INTERRUPTION_RESPONSE_ACCEPT: InterruptionResponse
INTERRUPTION_RESPONSE_REJECT: InterruptionResponse
INTERRUPTION_STATUS_UNSPECIFIED: InterruptionStatus
INTERRUPTION_STATUS_OPEN: InterruptionStatus
INTERRUPTION_STATUS_CANCELLED: InterruptionStatus
INTERRUPTION_STATUS_COMPLETED: InterruptionStatus
INTERRUPTION_TYPE_UNSPECIFIED: InterruptionType
INTERRUPTION_TYPE_PAUSE: InterruptionType
INTERRUPTION_TYPE_YIELD: InterruptionType
INTERRUPTION_TYPE_WAIT: InterruptionType
JOIN_TYPE_UNSPECIFIED: JoinType
JOIN_TYPE_LEFT: JoinType
JOIN_TYPE_PARENT: JoinType
JOIN_TYPE_CHILD: JoinType
LAYOUT_UNSPECIFIED: Layout
LAYOUT_STACK: Layout
LAYOUT_GRID: Layout
LENGTH_UNIT_UNSPECIFIED: LengthUnit
LENGTH_UNIT_PIXEL: LengthUnit
LENGTH_UNIT_REM: LengthUnit
LENGTH_UNIT_PERCENT: LengthUnit
LENGTH_UNIT_FR: LengthUnit
LINK_TYPE_UNSPECIFIED: LinkType
LINK_TYPE_WEB: LinkType
MACHINE_TYPE_UNSPECIFIED: MachineType
MACHINE_TYPE_RUNTIME: MachineType
MACHINE_TYPE_UBUNTU: MachineType
MACHINE_TYPE_MAC: MachineType
MACHINE_TYPE_WINDOWS: MachineType
MACHINE_TYPE_CUSTOM: MachineType
MESSAGE_TYPE_UNSPECIFIED: MessageType
MESSAGE_TYPE_DEFAULT: MessageType
MESSAGE_TYPE_JOIN: MessageType
MESSAGE_TYPE_LEAVE: MessageType
MESSAGE_TYPE_RESOURCE: MessageType
MESSAGE_TYPE_RUN: MessageType
MESSAGE_TYPE_THREAD: MessageType
MODEL_DEVELOPER_UNSPECIFIED: ModelDeveloper
MODEL_DEVELOPER_OPENAI: ModelDeveloper
MODEL_DEVELOPER_ANTHROPIC: ModelDeveloper
MODEL_DEVELOPER_GOOGLE: ModelDeveloper
MODEL_DEVELOPER_XAI: ModelDeveloper
MODEL_PROVIDER_UNSPECIFIED: ModelProvider
MODEL_PROVIDER_OPENROUTER: ModelProvider
MODEL_PROVIDER_OPENAI: ModelProvider
MODEL_PROVIDER_ANTHROPIC: ModelProvider
MODEL_PROVIDER_GOOGLE: ModelProvider
MODEL_PROVIDER_XAI: ModelProvider
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
NODE_AREA_UNSPECIFIED: NodeArea
NODE_AREA_GLOBAL_POSTGRES: NodeArea
NODE_AREA_MAIN_POSTGRES: NodeArea
NODE_AREA_CUSTOM_POSTGRES: NodeArea
NODE_MODE_UNSPECIFIED: NodeMode
NODE_MODE_KERNEL: NodeMode
NODE_MODE_SYSTEM: NodeMode
NODE_MODE_BUILTIN: NodeMode
NODE_MODE_MAIN: NodeMode
NODE_MODE_TEST: NodeMode
NODE_MODE_TEMPLATE: NodeMode
NODE_MODE_ARCHIVE: NodeMode
NODE_TYPE_UNSPECIFIED: NodeType
NODE_TYPE_BENCH: NodeType
NODE_TYPE_BENCH_MEMBERSHIP: NodeType
NODE_TYPE_BENCH_INVITE: NodeType
NODE_TYPE_PACKAGE: NodeType
NODE_TYPE_PACKAGE_MEMBERSHIP: NodeType
NODE_TYPE_PACKAGE_INVITE: NodeType
NODE_TYPE_HANDLE: NodeType
NODE_TYPE_USER: NodeType
NODE_TYPE_FRIENDSHIP: NodeType
NODE_TYPE_FRIENDSHIP_INVITE: NodeType
NODE_TYPE_ORGANIZATION: NodeType
NODE_TYPE_ORGANIZATION_MEMBERSHIP: NodeType
NODE_TYPE_ORGANIZATION_INVITE: NodeType
NODE_TYPE_CLIENT: NodeType
NODE_TYPE_SPACE: NodeType
NODE_TYPE_SCENE: NodeType
NODE_TYPE_ROUTE: NodeType
NODE_TYPE_PAGE: NodeType
NODE_TYPE_BLOCK: NodeType
NODE_TYPE_DATABASE: NodeType
NODE_TYPE_MACHINE: NodeType
NODE_TYPE_SERVICE: NodeType
NODE_TYPE_ACTION: NodeType
NODE_TYPE_FLOW: NodeType
NODE_TYPE_FLOW_EDGE: NodeType
NODE_TYPE_AGENT: NodeType
NODE_TYPE_TASK: NodeType
NODE_TYPE_CURSOR: NodeType
NODE_TYPE_RUN: NodeType
NODE_TYPE_SPAN: NodeType
NODE_TYPE_INTERRUPTION: NodeType
NODE_TYPE_SCHEMA: NodeType
NODE_TYPE_FIELD: NodeType
NODE_TYPE_FILE: NodeType
NODE_TYPE_LINK: NodeType
NODE_TYPE_CUSTOM_NODE_DEFINITION: NodeType
NODE_TYPE_CUSTOM_NODE_INSTANCE: NodeType
NODE_TYPE_THREAD: NodeType
NODE_TYPE_MESSAGE: NodeType
NODE_TYPE_FRAME_VIEW: NodeType
NODE_TYPE_LABEL_VIEW: NodeType
NODE_TYPE_SPLIT_VIEW: NodeType
NODE_TYPE_TEXT_VIEW: NodeType
NODE_TYPE_NUMBER_INPUT_VIEW: NodeType
NODE_TYPE_SLIDER_INPUT_VIEW: NodeType
NODE_TYPE_THREAD_VIEW: NodeType
NODE_TYPE_WIZARD_VIEW: NodeType
NODE_TYPE_THEME: NodeType
NODE_TYPE_COLOR_STYLE: NodeType
NODE_TYPE_FONT_STYLE: NodeType
NODE_TYPE_BORDER_STYLE: NodeType
NODE_TYPE_SHADOW_STYLE: NodeType
NODE_TYPE_GRADIENT_STYLE: NodeType
NODE_TYPE_TRANSITION_STYLE: NodeType
NODE_TYPE_EFFECT_STYLE: NodeType
NUMBER_FORMAT_UNSPECIFIED: NumberFormat
NUMBER_FORMAT_PERCENTAGE: NumberFormat
NUMBER_FORMAT_ANGLE: NumberFormat
NUMBER_FORMAT_CURRENCY: NumberFormat
OFFSCREEN_BEHAVIOR_UNSPECIFIED: OffscreenBehavior
OFFSCREEN_BEHAVIOR_PLAY: OffscreenBehavior
OFFSCREEN_BEHAVIOR_PAUSE: OffscreenBehavior
ORGANIZATION_ROLE_TYPE_UNSPECIFIED: OrganizationRoleType
ORGANIZATION_ROLE_TYPE_ADMIN: OrganizationRoleType
ORGANIZATION_ROLE_TYPE_MEMBER: OrganizationRoleType
ORGANIZATION_STATUS_UNSPECIFIED: OrganizationStatus
ORGANIZATION_STATUS_CREATING: OrganizationStatus
ORGANIZATION_STATUS_ACTIVE: OrganizationStatus
OVERFLOW_UNSPECIFIED: Overflow
OVERFLOW_HIDDEN: Overflow
OVERFLOW_VISIBLE: Overflow
OVERFLOW_SCROLL: Overflow
PACKAGE_ROLE_TYPE_UNSPECIFIED: PackageRoleType
PACKAGE_ROLE_TYPE_MEMBER: PackageRoleType
PACKAGE_ROLE_TYPE_ADMIN: PackageRoleType
PACKAGE_TYPE_UNSPECIFIED: PackageType
PACKAGE_TYPE_HOME: PackageType
PACKAGE_TYPE_APPLICATION: PackageType
POSITION_TYPE_UNSPECIFIED: PositionType
POSITION_TYPE_RELATIVE: PositionType
POSITION_TYPE_ABSOLUTE: PositionType
POSITION_TYPE_FIXED: PositionType
POSITION_TYPE_STICKY: PositionType
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
PROCESS_STATUS_UNSPECIFIED: ProcessStatus
PROCESS_STATUS_CREATED: ProcessStatus
PROCESS_STATUS_ASSIGNED: ProcessStatus
PROCESS_STATUS_SCHEDULED: ProcessStatus
PROCESS_STATUS_QUEUED: ProcessStatus
PROCESS_STATUS_RUNNING: ProcessStatus
PROCESS_STATUS_FAILING: ProcessStatus
PROCESS_STATUS_PAUSED: ProcessStatus
PROCESS_STATUS_YIELDED: ProcessStatus
PROCESS_STATUS_WAITING: ProcessStatus
PROCESS_STATUS_IDLE: ProcessStatus
PROCESS_STATUS_CANCELLED: ProcessStatus
PROCESS_STATUS_ABORTED: ProcessStatus
PROCESS_STATUS_DIED: ProcessStatus
PROCESS_STATUS_FAILED: ProcessStatus
PROCESS_STATUS_COMPLETED: ProcessStatus
PROCESS_STATUS_SKIPPED: ProcessStatus
QUERY_TYPE_UNSPECIFIED: QueryType
QUERY_TYPE_GET: QueryType
QUERY_TYPE_SEARCH: QueryType
QUERY_TYPE_AGGREGATE: QueryType
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
RELATION_TYPE_UNSPECIFIED: RelationType
RELATION_TYPE_BUILTIN_NODE: RelationType
RELATION_TYPE_CUSTOM_NODE: RelationType
REPEAT_TYPE_UNSPECIFIED: RepeatType
REPEAT_TYPE_LOOP: RepeatType
REPEAT_TYPE_REVERSE: RepeatType
REPEAT_TYPE_MIRROR: RepeatType
RESOURCE_STATUS_UNSPECIFIED: ResourceStatus
RESOURCE_STATUS_PENDING: ResourceStatus
RESOURCE_STATUS_CREATING: ResourceStatus
RESOURCE_STATUS_RETRYING: ResourceStatus
RESOURCE_STATUS_AVAILABLE: ResourceStatus
RESOURCE_STATUS_SLEEPING: ResourceStatus
RESOURCE_STATUS_UNAVAILABLE: ResourceStatus
RESOURCE_STATUS_IMPAIRED: ResourceStatus
RESOURCE_STATUS_OFFLINE: ResourceStatus
RESOURCE_STATUS_FAILED: ResourceStatus
RUN_TYPE_UNSPECIFIED: RunType
RUN_TYPE_CODE: RunType
RUN_TYPE_ACTION: RunType
RUN_TYPE_FLOW: RunType
RUN_TYPE_AGENT: RunType
SCALAR_TYPE_UNSPECIFIED: ScalarType
SCALAR_TYPE_PRIMITIVE: ScalarType
SCALAR_TYPE_ENUM: ScalarType
SCALAR_TYPE_NODE: ScalarType
SCALAR_TYPE_STRUCT: ScalarType
SCHEDULE_FREQUENCY_UNSPECIFIED: ScheduleFrequency
SCHEDULE_FREQUENCY_YEAR: ScheduleFrequency
SCHEDULE_FREQUENCY_MONTH: ScheduleFrequency
SCHEDULE_FREQUENCY_WEEK: ScheduleFrequency
SCHEDULE_FREQUENCY_DAY: ScheduleFrequency
SCHEDULE_FREQUENCY_HOUR: ScheduleFrequency
SCHEDULE_FREQUENCY_MINUTE: ScheduleFrequency
SHADOW_POSITION_UNSPECIFIED: ShadowPosition
SHADOW_POSITION_OUTSIDE: ShadowPosition
SHADOW_POSITION_INSIDE: ShadowPosition
SHADOW_TYPE_UNSPECIFIED: ShadowType
SHADOW_TYPE_STYLE: ShadowType
SHADOW_TYPE_FIELD: ShadowType
SHADOW_TYPE_BOX: ShadowType
SHADOW_TYPE_REALISTIC: ShadowType
SORT_MODE_UNSPECIFIED: SortMode
SORT_MODE_MAX: SortMode
SORT_MODE_MIN: SortMode
SORT_MODE_AVERAGE: SortMode
SORT_MODE_SUM: SortMode
SORT_MODE_MEDIAN: SortMode
SORT_TYPE_UNSPECIFIED: SortType
SORT_TYPE_ASCENDING: SortType
SORT_TYPE_DESCENDING: SortType
SPACE_TYPE_UNSPECIFIED: SpaceType
SPACE_TYPE_BROWSER: SpaceType
SPACE_TYPE_DESKTOP: SpaceType
SPACE_TYPE_MOBILE: SpaceType
SPAN_TYPE_UNSPECIFIED: SpanType
SPAN_TYPE_ATTEMPT: SpanType
SPAN_TYPE_WAIT: SpanType
SPAN_TYPE_ACQUIRE: SpanType
SPAN_TYPE_CODE: SpanType
SPAN_TYPE_AGENT_TURN: SpanType
SPAN_TYPE_MODEL_PREPARE: SpanType
SPAN_TYPE_MODEL_GENERATE: SpanType
SPAN_TYPE_MODEL_PARSE: SpanType
SPAN_TYPE_FILE_UPLOAD: SpanType
SPAN_TYPE_FILE_PREPARE_UPLOAD: SpanType
SPAN_TYPE_FILE_DOWNLOAD: SpanType
SPAN_TYPE_FILE_PREPARE_DOWNLOAD: SpanType
SPRING_TYPE_UNSPECIFIED: SpringType
SPRING_TYPE_TIME: SpringType
SPRING_TYPE_PHYSICS: SpringType
STRING_FORMAT_UNSPECIFIED: StringFormat
STRING_FORMAT_NAME: StringFormat
STRING_FORMAT_SLUG: StringFormat
STRING_FORMAT_EMAIL: StringFormat
STRING_FORMAT_UUID: StringFormat
STRING_FORMAT_URL: StringFormat
STRING_FORMAT_EMOJI: StringFormat
STRING_FORMAT_MIME: StringFormat
STRING_FORMAT_BASE64: StringFormat
STRUCT_TYPE_UNSPECIFIED: StructType
STRUCT_TYPE_SCOPE: StructType
STRUCT_TYPE_ORIGIN: StructType
STRUCT_TYPE_NODE_REFERENCE: StructType
STRUCT_TYPE_PROPERTY_REFERENCE: StructType
STRUCT_TYPE_PROPERTY_INFO: StructType
STRUCT_TYPE_TRAIT_INFO: StructType
STRUCT_TYPE_NODE_INFO: StructType
STRUCT_TYPE_STRUCT_INFO: StructType
STRUCT_TYPE_ENUM_INFO: StructType
STRUCT_TYPE_ENUM_OPTION_INFO: StructType
STRUCT_TYPE_EDIT: StructType
STRUCT_TYPE_CHANGE: StructType
STRUCT_TYPE_CHANGE_RESULT: StructType
STRUCT_TYPE_EXPRESSION: StructType
STRUCT_TYPE_FUNCTION: StructType
STRUCT_TYPE_JOIN: StructType
STRUCT_TYPE_AGGREGATION: StructType
STRUCT_TYPE_CONDITION: StructType
STRUCT_TYPE_SORT: StructType
STRUCT_TYPE_SELECT: StructType
STRUCT_TYPE_RELATION_REFERENCE: StructType
STRUCT_TYPE_ATTRIBUTE_REFERENCE: StructType
STRUCT_TYPE_QUERY: StructType
STRUCT_TYPE_QUERY_RESULT: StructType
STRUCT_TYPE_QUERY_UPDATE: StructType
STRUCT_TYPE_VARIABLE: StructType
STRUCT_TYPE_SCHEDULE: StructType
STRUCT_TYPE_ERROR: StructType
STRUCT_TYPE_TYPE: StructType
STRUCT_TYPE_NUMBER_CONSTRAINT: StructType
STRUCT_TYPE_STRING_CONSTRAINT: StructType
STRUCT_TYPE_COLLECTION_CONSTRAINT: StructType
STRUCT_TYPE_NODE_CONSTRAINT: StructType
STRUCT_TYPE_TEXT: StructType
STRUCT_TYPE_TEXT_LINE: StructType
STRUCT_TYPE_TEXT_SPAN: StructType
STRUCT_TYPE_CODE: StructType
STRUCT_TYPE_ICON: StructType
STRUCT_TYPE_SELECTION: StructType
STRUCT_TYPE_VALUE: StructType
STRUCT_TYPE_VECTOR2: StructType
STRUCT_TYPE_VECTOR3: StructType
STRUCT_TYPE_VECTOR4: StructType
STRUCT_TYPE_AXIS2: StructType
STRUCT_TYPE_AXIS3: StructType
STRUCT_TYPE_COLOR: StructType
STRUCT_TYPE_SHADOW: StructType
STRUCT_TYPE_BORDER: StructType
STRUCT_TYPE_FONT: StructType
STRUCT_TYPE_GRADIENT_STOP: StructType
STRUCT_TYPE_GRADIENT: StructType
STRUCT_TYPE_FILL: StructType
STRUCT_TYPE_LENGTH: StructType
STRUCT_TYPE_POSITION: StructType
STRUCT_TYPE_DIMENSION: StructType
STRUCT_TYPE_TRANSITION: StructType
STRUCT_TYPE_EFFECT: StructType
STRUCT_TYPE_GRID: StructType
STRUCT_TYPE_GRID_SPAN: StructType
STRUCT_TYPE_INSETS: StructType
STRUCT_TYPE_CORNERS: StructType
TEXT_ALIGN_UNSPECIFIED: TextAlign
TEXT_ALIGN_LEFT: TextAlign
TEXT_ALIGN_CENTER: TextAlign
TEXT_ALIGN_RIGHT: TextAlign
TEXT_ALIGN_JUSTIFY: TextAlign
TEXT_DECORATION_UNSPECIFIED: TextDecoration
TEXT_DECORATION_NONE: TextDecoration
TEXT_DECORATION_UNDERLINE: TextDecoration
TEXT_DECORATION_STRIKETHROUGH: TextDecoration
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
TEXT_LINE_TYPE_CODE: TextLineType
TEXT_LINE_TYPE_NODE: TextLineType
TEXT_SPAN_TYPE_UNSPECIFIED: TextSpanType
TEXT_SPAN_TYPE_TEXT: TextSpanType
TEXT_SPAN_TYPE_HARD_BREAK: TextSpanType
TEXT_SPAN_TYPE_MENTION: TextSpanType
TEXT_SPAN_TYPE_LINK: TextSpanType
TEXT_SPAN_TYPE_CITATION: TextSpanType
TEXT_SPAN_TYPE_EQUATION: TextSpanType
TEXT_SPLIT_TYPE_UNSPECIFIED: TextSplitType
TEXT_SPLIT_TYPE_CHAR: TextSplitType
TEXT_SPLIT_TYPE_WORD: TextSplitType
TEXT_SPLIT_TYPE_LINE: TextSplitType
TEXT_TRANSFORM_UNSPECIFIED: TextTransform
TEXT_TRANSFORM_NONE: TextTransform
TEXT_TRANSFORM_UPPERCASE: TextTransform
TEXT_TRANSFORM_LOWERCASE: TextTransform
TEXT_TRANSFORM_CAPITALIZE: TextTransform
THEME_COLOR_UNSPECIFIED: ThemeColor
THEME_COLOR_PRIMARY: ThemeColor
THEME_COLOR_SECONDARY: ThemeColor
THEME_COLOR_ACCENT: ThemeColor
THEME_COLOR_MUTED: ThemeColor
THEME_COLOR_SUCCESS: ThemeColor
THEME_COLOR_WARNING: ThemeColor
THEME_COLOR_ERROR: ThemeColor
THREAD_STATUS_UNSPECIFIED: ThreadStatus
THREAD_STATUS_OPEN: ThreadStatus
THREAD_STATUS_CLOSED: ThreadStatus
TIME_INTERVAL_UNSPECIFIED: TimeInterval
TIME_INTERVAL_SECOND: TimeInterval
TIME_INTERVAL_MINUTE: TimeInterval
TIME_INTERVAL_HOUR: TimeInterval
TIME_INTERVAL_DAY: TimeInterval
TIME_INTERVAL_WEEK: TimeInterval
TIME_INTERVAL_MONTH: TimeInterval
TIME_INTERVAL_YEAR: TimeInterval
TRAIT_TYPE_UNSPECIFIED: TraitType
TRAIT_TYPE_GLOBAL: TraitType
TRAIT_TYPE_CUSTOM: TraitType
TRAIT_TYPE_MODAL: TraitType
TRAIT_TYPE_ARCHIVABLE: TraitType
TRAIT_TYPE_DELETABLE: TraitType
TRAIT_TYPE_NAMED: TraitType
TRAIT_TYPE_TITLED: TraitType
TRAIT_TYPE_SLUG: TraitType
TRAIT_TYPE_ICON: TraitType
TRAIT_TYPE_ORDERED: TraitType
TRAIT_TYPE_TEMPLATABLE: TraitType
TRAIT_TYPE_EXTENSIBLE: TraitType
TRAIT_TYPE_IN_BENCH: TraitType
TRAIT_TYPE_IN_PACKAGE: TraitType
TRAIT_TYPE_REGIONAL: TraitType
TRAIT_TYPE_RESOURCE: TraitType
TRAIT_TYPE_PROVISIONABLE: TraitType
TRAIT_TYPE_OWNABLE: TraitType
TRAIT_TYPE_JOINABLE: TraitType
TRAIT_TYPE_SUBJECT: TraitType
TRAIT_TYPE_MEMBERSHIP: TraitType
TRAIT_TYPE_INVITE: TraitType
TRAIT_TYPE_ROLE: TraitType
TRAIT_TYPE_BLOCKABLE: TraitType
TRAIT_TYPE_RUNNABLE: TraitType
TRAIT_TYPE_PROCESSABLE: TraitType
TRAIT_TYPE_COMPUTABLE: TraitType
TRAIT_TYPE_VIEW: TraitType
TRAIT_TYPE_CONTAINER_VIEW: TraitType
TRAIT_TYPE_CONTENT_VIEW: TraitType
TRAIT_TYPE_INPUT_VIEW: TraitType
TRAIT_TYPE_NODE_VIEW: TraitType
TRAIT_TYPE_INTERNAL_VIEW: TraitType
TRAIT_TYPE_STYLE: TraitType
TRANSITION_TYPE_UNSPECIFIED: TransitionType
TRANSITION_TYPE_STYLE: TransitionType
TRANSITION_TYPE_FIELD: TransitionType
TRANSITION_TYPE_TWEEN: TransitionType
TRANSITION_TYPE_SPRING: TransitionType
TYPE_CARDINALITY_UNSPECIFIED: TypeCardinality
TYPE_CARDINALITY_SCALAR: TypeCardinality
TYPE_CARDINALITY_LIST: TypeCardinality
TYPE_CARDINALITY_MAP: TypeCardinality
UPDATE_TYPE_UNSPECIFIED: UpdateType
UPDATE_TYPE_SET: UpdateType
UPDATE_TYPE_CLEAR: UpdateType
UPDATE_TYPE_MAP_SET: UpdateType
UPDATE_TYPE_MAP_REMOVE: UpdateType
USER_STATUS_UNSPECIFIED: UserStatus
USER_STATUS_CREATING: UserStatus
USER_STATUS_ACTIVE: UserStatus
VARIABLE_TYPE_UNSPECIFIED: VariableType
VARIABLE_TYPE_FIELD: VariableType

class ActionData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "mode", "order_key", "name", "cardinality", "text", "model_developer", "model_provider", "model_id", "model_name", "max_attempts", "retry_interval", "backoff")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    CARDINALITY_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    MODEL_DEVELOPER_FIELD_NUMBER: _ClassVar[int]
    MODEL_PROVIDER_FIELD_NUMBER: _ClassVar[int]
    MODEL_ID_FIELD_NUMBER: _ClassVar[int]
    MODEL_NAME_FIELD_NUMBER: _ClassVar[int]
    MAX_ATTEMPTS_FIELD_NUMBER: _ClassVar[int]
    RETRY_INTERVAL_FIELD_NUMBER: _ClassVar[int]
    BACKOFF_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    name: str
    cardinality: ActionCardinality
    text: TextData
    model_developer: ModelDeveloper
    model_provider: ModelProvider
    model_id: str
    model_name: str
    max_attempts: int
    retry_interval: _duration_pb2.Duration
    backoff: float
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., cardinality: _Optional[_Union[ActionCardinality, str]] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., model_developer: _Optional[_Union[ModelDeveloper, str]] = ..., model_provider: _Optional[_Union[ModelProvider, str]] = ..., model_id: _Optional[str] = ..., model_name: _Optional[str] = ..., max_attempts: _Optional[int] = ..., retry_interval: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., backoff: _Optional[float] = ...) -> None: ...

class AgentData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "owned_by_ptr", "mode", "order_key", "name", "icon", "block_ptr", "page_ptr", "cursor_ptr", "status", "duration", "error", "interruption_ptr", "scheduled_at", "started_at", "active_at", "interrupted_at", "terminated_at", "requested_stop_at", "requested_pause_at", "requested_resume_at", "model_developer", "model_provider", "model_id", "model_name", "max_attempts", "retry_interval", "backoff")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    PAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CURSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    ERROR_FIELD_NUMBER: _ClassVar[int]
    INTERRUPTION_PTR_FIELD_NUMBER: _ClassVar[int]
    SCHEDULED_AT_FIELD_NUMBER: _ClassVar[int]
    STARTED_AT_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_AT_FIELD_NUMBER: _ClassVar[int]
    INTERRUPTED_AT_FIELD_NUMBER: _ClassVar[int]
    TERMINATED_AT_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_STOP_AT_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_PAUSE_AT_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_RESUME_AT_FIELD_NUMBER: _ClassVar[int]
    MODEL_DEVELOPER_FIELD_NUMBER: _ClassVar[int]
    MODEL_PROVIDER_FIELD_NUMBER: _ClassVar[int]
    MODEL_ID_FIELD_NUMBER: _ClassVar[int]
    MODEL_NAME_FIELD_NUMBER: _ClassVar[int]
    MAX_ATTEMPTS_FIELD_NUMBER: _ClassVar[int]
    RETRY_INTERVAL_FIELD_NUMBER: _ClassVar[int]
    BACKOFF_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    owned_by_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    name: str
    icon: IconData
    block_ptr: NodeReferenceData
    page_ptr: NodeReferenceData
    cursor_ptr: NodeReferenceData
    status: ProcessStatus
    duration: _duration_pb2.Duration
    error: ErrorData
    interruption_ptr: NodeReferenceData
    scheduled_at: _timestamp_pb2.Timestamp
    started_at: _timestamp_pb2.Timestamp
    active_at: _timestamp_pb2.Timestamp
    interrupted_at: _timestamp_pb2.Timestamp
    terminated_at: _timestamp_pb2.Timestamp
    requested_stop_at: _timestamp_pb2.Timestamp
    requested_pause_at: _timestamp_pb2.Timestamp
    requested_resume_at: _timestamp_pb2.Timestamp
    model_developer: ModelDeveloper
    model_provider: ModelProvider
    model_id: str
    model_name: str
    max_attempts: int
    retry_interval: _duration_pb2.Duration
    backoff: float
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., page_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., cursor_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., status: _Optional[_Union[ProcessStatus, str]] = ..., duration: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., error: _Optional[_Union[ErrorData, _Mapping]] = ..., interruption_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., scheduled_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., started_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., active_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., interrupted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., terminated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., requested_stop_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., requested_pause_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., requested_resume_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., model_developer: _Optional[_Union[ModelDeveloper, str]] = ..., model_provider: _Optional[_Union[ModelProvider, str]] = ..., model_id: _Optional[str] = ..., model_name: _Optional[str] = ..., max_attempts: _Optional[int] = ..., retry_interval: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., backoff: _Optional[float] = ...) -> None: ...

class AggregationData(_message.Message):
    __slots__ = ("metatype", "type", "expression", "distinct")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    EXPRESSION_FIELD_NUMBER: _ClassVar[int]
    DISTINCT_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: AggregationType
    expression: ExpressionData
    distinct: bool
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[AggregationType, str]] = ..., expression: _Optional[_Union[ExpressionData, _Mapping]] = ..., distinct: bool = ...) -> None: ...

class AttributeReferenceData(_message.Message):
    __slots__ = ("metatype", "type", "prop_ptr", "field_ptr", "name", "relation")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    PROP_PTR_FIELD_NUMBER: _ClassVar[int]
    FIELD_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    RELATION_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: AttributeType
    prop_ptr: PropertyReferenceData
    field_ptr: NodeReferenceData
    name: str
    relation: RelationReferenceData
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[AttributeType, str]] = ..., prop_ptr: _Optional[_Union[PropertyReferenceData, _Mapping]] = ..., field_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., name: _Optional[str] = ..., relation: _Optional[_Union[RelationReferenceData, _Mapping]] = ...) -> None: ...

class Axis2Data(_message.Message):
    __slots__ = ("metatype", "base", "x", "y")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    BASE_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    base: float
    x: float
    y: float
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., base: _Optional[float] = ..., x: _Optional[float] = ..., y: _Optional[float] = ...) -> None: ...

class Axis3Data(_message.Message):
    __slots__ = ("metatype", "base", "x", "y", "z")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    BASE_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    Z_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    base: float
    x: float
    y: float
    z: float
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., base: _Optional[float] = ..., x: _Optional[float] = ..., y: _Optional[float] = ..., z: _Optional[float] = ...) -> None: ...

class BenchData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "owned_by_ptr", "region", "name", "slug", "icon", "status", "handle_ptr", "database_ptr", "main_package_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    HANDLE_PTR_FIELD_NUMBER: _ClassVar[int]
    DATABASE_PTR_FIELD_NUMBER: _ClassVar[int]
    MAIN_PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    region: Region
    name: str
    slug: str
    icon: IconData
    status: BenchStatus
    handle_ptr: NodeReferenceData
    database_ptr: NodeReferenceData
    main_package_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., region: _Optional[_Union[Region, str]] = ..., name: _Optional[str] = ..., slug: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., status: _Optional[_Union[BenchStatus, str]] = ..., handle_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., database_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., main_package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class BenchInviteData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "member_ptr", "role_type")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    MEMBER_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLE_TYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    member_ptr: NodeReferenceData
    role_type: BenchRoleType
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., member_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., role_type: _Optional[_Union[BenchRoleType, str]] = ...) -> None: ...

class BenchMembershipData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "member_ptr", "role_type")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    MEMBER_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLE_TYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    member_ptr: NodeReferenceData
    role_type: BenchRoleType
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., member_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., role_type: _Optional[_Union[BenchRoleType, str]] = ...) -> None: ...

class BlockData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "mode", "order_key", "type", "name", "line", "node_ptr", "view_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    LINE_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    VIEW_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    type: BlockType
    name: str
    line: TextLineData
    node_ptr: NodeReferenceData
    view_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[BlockType, str]] = ..., name: _Optional[str] = ..., line: _Optional[_Union[TextLineData, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., view_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class BorderData(_message.Message):
    __slots__ = ("metatype", "type", "style_ptr", "color", "width")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    STYLE_PTR_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: BorderType
    style_ptr: NodeReferenceData
    color: ColorData
    width: InsetsData
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[BorderType, str]] = ..., style_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., color: _Optional[_Union[ColorData, _Mapping]] = ..., width: _Optional[_Union[InsetsData, _Mapping]] = ...) -> None: ...

class BorderStyleData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "mode", "order_key", "type", "name", "block_ptr", "style_ptr", "color", "width")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    STYLE_PTR_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    type: BorderType
    name: str
    block_ptr: NodeReferenceData
    style_ptr: NodeReferenceData
    color: ColorData
    width: InsetsData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[BorderType, str]] = ..., name: _Optional[str] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., style_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., color: _Optional[_Union[ColorData, _Mapping]] = ..., width: _Optional[_Union[InsetsData, _Mapping]] = ...) -> None: ...

class ChangeData(_message.Message):
    __slots__ = ("metatype", "id", "created_at", "edits")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    EDITS_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    id: str
    created_at: _timestamp_pb2.Timestamp
    edits: _containers.RepeatedCompositeFieldContainer[EditData]
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., id: _Optional[str] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., edits: _Optional[_Iterable[_Union[EditData, _Mapping]]] = ...) -> None: ...

class ChangeResultData(_message.Message):
    __slots__ = ("metatype", "id", "created_at", "status", "edits", "cascaded_edits", "epoch")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    EDITS_FIELD_NUMBER: _ClassVar[int]
    CASCADED_EDITS_FIELD_NUMBER: _ClassVar[int]
    EPOCH_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    id: str
    created_at: _timestamp_pb2.Timestamp
    status: ChangeStatus
    edits: _containers.RepeatedCompositeFieldContainer[EditData]
    cascaded_edits: _containers.RepeatedCompositeFieldContainer[EditData]
    epoch: int
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., id: _Optional[str] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., status: _Optional[_Union[ChangeStatus, str]] = ..., edits: _Optional[_Iterable[_Union[EditData, _Mapping]]] = ..., cascaded_edits: _Optional[_Iterable[_Union[EditData, _Mapping]]] = ..., epoch: _Optional[int] = ...) -> None: ...

class ClientData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "type", "name", "space_ptr", "machine_ptr", "user_ptr", "device_type", "device_name", "operating_system", "browser_name", "browser_version", "access_token", "seen_at", "logged_in_at", "cursor_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MACHINE_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_PTR_FIELD_NUMBER: _ClassVar[int]
    DEVICE_TYPE_FIELD_NUMBER: _ClassVar[int]
    DEVICE_NAME_FIELD_NUMBER: _ClassVar[int]
    OPERATING_SYSTEM_FIELD_NUMBER: _ClassVar[int]
    BROWSER_NAME_FIELD_NUMBER: _ClassVar[int]
    BROWSER_VERSION_FIELD_NUMBER: _ClassVar[int]
    ACCESS_TOKEN_FIELD_NUMBER: _ClassVar[int]
    SEEN_AT_FIELD_NUMBER: _ClassVar[int]
    LOGGED_IN_AT_FIELD_NUMBER: _ClassVar[int]
    CURSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    type: ClientType
    name: str
    space_ptr: NodeReferenceData
    machine_ptr: NodeReferenceData
    user_ptr: NodeReferenceData
    device_type: str
    device_name: str
    operating_system: str
    browser_name: str
    browser_version: str
    access_token: str
    seen_at: _timestamp_pb2.Timestamp
    logged_in_at: _timestamp_pb2.Timestamp
    cursor_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., type: _Optional[_Union[ClientType, str]] = ..., name: _Optional[str] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., machine_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., user_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., device_type: _Optional[str] = ..., device_name: _Optional[str] = ..., operating_system: _Optional[str] = ..., browser_name: _Optional[str] = ..., browser_version: _Optional[str] = ..., access_token: _Optional[str] = ..., seen_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., logged_in_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., cursor_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class CodeData(_message.Message):
    __slots__ = ("metatype", "language", "content")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    LANGUAGE_FIELD_NUMBER: _ClassVar[int]
    CONTENT_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    language: str
    content: str
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., language: _Optional[str] = ..., content: _Optional[str] = ...) -> None: ...

class CollectionConstraintData(_message.Message):
    __slots__ = ("metatype", "min_length", "max_length")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    MIN_LENGTH_FIELD_NUMBER: _ClassVar[int]
    MAX_LENGTH_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    min_length: int
    max_length: int
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., min_length: _Optional[int] = ..., max_length: _Optional[int] = ...) -> None: ...

class ColorData(_message.Message):
    __slots__ = ("metatype", "type", "style_ptr", "hue", "shade", "x", "y", "z", "alpha")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    STYLE_PTR_FIELD_NUMBER: _ClassVar[int]
    HUE_FIELD_NUMBER: _ClassVar[int]
    SHADE_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    Z_FIELD_NUMBER: _ClassVar[int]
    ALPHA_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: ColorType
    style_ptr: NodeReferenceData
    hue: ColorHue
    shade: ColorShade
    x: float
    y: float
    z: float
    alpha: float
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[ColorType, str]] = ..., style_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., hue: _Optional[_Union[ColorHue, str]] = ..., shade: _Optional[_Union[ColorShade, str]] = ..., x: _Optional[float] = ..., y: _Optional[float] = ..., z: _Optional[float] = ..., alpha: _Optional[float] = ...) -> None: ...

class ColorStyleData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "mode", "order_key", "type", "name", "block_ptr", "style_ptr", "hue", "shade", "x", "y", "z", "alpha", "dark")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    STYLE_PTR_FIELD_NUMBER: _ClassVar[int]
    HUE_FIELD_NUMBER: _ClassVar[int]
    SHADE_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    Z_FIELD_NUMBER: _ClassVar[int]
    ALPHA_FIELD_NUMBER: _ClassVar[int]
    DARK_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    type: ColorType
    name: str
    block_ptr: NodeReferenceData
    style_ptr: NodeReferenceData
    hue: ColorHue
    shade: ColorShade
    x: float
    y: float
    z: float
    alpha: float
    dark: ColorData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[ColorType, str]] = ..., name: _Optional[str] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., style_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., hue: _Optional[_Union[ColorHue, str]] = ..., shade: _Optional[_Union[ColorShade, str]] = ..., x: _Optional[float] = ..., y: _Optional[float] = ..., z: _Optional[float] = ..., alpha: _Optional[float] = ..., dark: _Optional[_Union[ColorData, _Mapping]] = ...) -> None: ...

class ConditionData(_message.Message):
    __slots__ = ("metatype", "type", "left", "right")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    LEFT_FIELD_NUMBER: _ClassVar[int]
    RIGHT_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: ConditionalType
    left: ExpressionData
    right: ExpressionData
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[ConditionalType, str]] = ..., left: _Optional[_Union[ExpressionData, _Mapping]] = ..., right: _Optional[_Union[ExpressionData, _Mapping]] = ...) -> None: ...

class CornersData(_message.Message):
    __slots__ = ("metatype", "base", "top_left", "top_right", "bottom_left", "bottom_right")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    BASE_FIELD_NUMBER: _ClassVar[int]
    TOP_LEFT_FIELD_NUMBER: _ClassVar[int]
    TOP_RIGHT_FIELD_NUMBER: _ClassVar[int]
    BOTTOM_LEFT_FIELD_NUMBER: _ClassVar[int]
    BOTTOM_RIGHT_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    base: int
    top_left: int
    top_right: int
    bottom_left: int
    bottom_right: int
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., base: _Optional[int] = ..., top_left: _Optional[int] = ..., top_right: _Optional[int] = ..., bottom_left: _Optional[int] = ..., bottom_right: _Optional[int] = ...) -> None: ...

class CursorData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "owned_by_ptr", "mode", "type", "title", "status", "started_at", "active_at", "seen_at", "terminated_at", "target_ptr", "selection", "focus", "filter", "sort", "url")
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
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    STARTED_AT_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_AT_FIELD_NUMBER: _ClassVar[int]
    SEEN_AT_FIELD_NUMBER: _ClassVar[int]
    TERMINATED_AT_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    SELECTION_FIELD_NUMBER: _ClassVar[int]
    FOCUS_FIELD_NUMBER: _ClassVar[int]
    FILTER_FIELD_NUMBER: _ClassVar[int]
    SORT_FIELD_NUMBER: _ClassVar[int]
    URL_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    owned_by_ptr: NodeReferenceData
    mode: NodeMode
    type: CursorType
    title: TextLineData
    status: CursorStatus
    started_at: _timestamp_pb2.Timestamp
    active_at: _timestamp_pb2.Timestamp
    seen_at: _timestamp_pb2.Timestamp
    terminated_at: _timestamp_pb2.Timestamp
    target_ptr: NodeReferenceData
    selection: SelectionData
    focus: SelectionData
    filter: ExpressionData
    sort: _containers.RepeatedCompositeFieldContainer[ExpressionData]
    url: str
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., type: _Optional[_Union[CursorType, str]] = ..., title: _Optional[_Union[TextLineData, _Mapping]] = ..., status: _Optional[_Union[CursorStatus, str]] = ..., started_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., active_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., seen_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., terminated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., selection: _Optional[_Union[SelectionData, _Mapping]] = ..., focus: _Optional[_Union[SelectionData, _Mapping]] = ..., filter: _Optional[_Union[ExpressionData, _Mapping]] = ..., sort: _Optional[_Iterable[_Union[ExpressionData, _Mapping]]] = ..., url: _Optional[str] = ...) -> None: ...

class CustomNodeDefinitionData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "owned_by_ptr", "mode", "order_key", "name", "block_ptr", "traits")
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
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    TRAITS_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    owned_by_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    name: str
    block_ptr: NodeReferenceData
    traits: _containers.RepeatedScalarFieldContainer[TraitType]
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., traits: _Optional[_Iterable[_Union[TraitType, str]]] = ...) -> None: ...

class CustomNodeInstanceData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "mode", "value", "definition_ptr")
    class ValueEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueData
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueData, _Mapping]] = ...) -> None: ...
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
    VALUE_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
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
    value: _containers.MessageMap[str, ValueData]
    definition_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., value: _Optional[_Mapping[str, ValueData]] = ..., definition_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class DatabaseData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "owned_by_ptr", "mode", "order_key", "region", "type", "name", "block_ptr", "status", "requested_activate_at", "requested_deactivate_at", "requested_reset_at", "requested_suspend_at", "requested_decommission_at", "active_at", "failed_at", "failed_attempts", "version", "schema_name", "external_name", "external_id", "sql_url")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_ACTIVATE_AT_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_DEACTIVATE_AT_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_RESET_AT_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_SUSPEND_AT_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_DECOMMISSION_AT_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_AT_FIELD_NUMBER: _ClassVar[int]
    FAILED_AT_FIELD_NUMBER: _ClassVar[int]
    FAILED_ATTEMPTS_FIELD_NUMBER: _ClassVar[int]
    VERSION_FIELD_NUMBER: _ClassVar[int]
    SCHEMA_NAME_FIELD_NUMBER: _ClassVar[int]
    EXTERNAL_NAME_FIELD_NUMBER: _ClassVar[int]
    EXTERNAL_ID_FIELD_NUMBER: _ClassVar[int]
    SQL_URL_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    region: Region
    type: DatabaseType
    name: str
    block_ptr: NodeReferenceData
    status: ResourceStatus
    requested_activate_at: _timestamp_pb2.Timestamp
    requested_deactivate_at: _timestamp_pb2.Timestamp
    requested_reset_at: _timestamp_pb2.Timestamp
    requested_suspend_at: _timestamp_pb2.Timestamp
    requested_decommission_at: _timestamp_pb2.Timestamp
    active_at: _timestamp_pb2.Timestamp
    failed_at: _timestamp_pb2.Timestamp
    failed_attempts: int
    version: str
    schema_name: str
    external_name: str
    external_id: str
    sql_url: str
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., region: _Optional[_Union[Region, str]] = ..., type: _Optional[_Union[DatabaseType, str]] = ..., name: _Optional[str] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., status: _Optional[_Union[ResourceStatus, str]] = ..., requested_activate_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., requested_deactivate_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., requested_reset_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., requested_suspend_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., requested_decommission_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., active_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., failed_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., failed_attempts: _Optional[int] = ..., version: _Optional[str] = ..., schema_name: _Optional[str] = ..., external_name: _Optional[str] = ..., external_id: _Optional[str] = ..., sql_url: _Optional[str] = ...) -> None: ...

class DimensionData(_message.Message):
    __slots__ = ("metatype", "type", "unit", "value")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    UNIT_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: DimensionType
    unit: LengthUnit
    value: float
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[DimensionType, str]] = ..., unit: _Optional[_Union[LengthUnit, str]] = ..., value: _Optional[float] = ...) -> None: ...

class EditData(_message.Message):
    __slots__ = ("metatype", "id", "type", "operation", "node_ptr", "path", "key", "value", "parent_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    OPERATION_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    PATH_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    id: str
    type: EditType
    operation: UpdateType
    node_ptr: NodeReferenceData
    path: str
    key: ValueData
    value: ValueData
    parent_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., id: _Optional[str] = ..., type: _Optional[_Union[EditType, str]] = ..., operation: _Optional[_Union[UpdateType, str]] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., path: _Optional[str] = ..., key: _Optional[_Union[ValueData, _Mapping]] = ..., value: _Optional[_Union[ValueData, _Mapping]] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class EffectData(_message.Message):
    __slots__ = ("metatype", "type", "style_ptr", "opacity", "offset", "scale", "rotate", "skew", "perspective", "delay", "duration", "threshold", "once", "repeat", "split", "offscreen", "transition")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    STYLE_PTR_FIELD_NUMBER: _ClassVar[int]
    OPACITY_FIELD_NUMBER: _ClassVar[int]
    OFFSET_FIELD_NUMBER: _ClassVar[int]
    SCALE_FIELD_NUMBER: _ClassVar[int]
    ROTATE_FIELD_NUMBER: _ClassVar[int]
    SKEW_FIELD_NUMBER: _ClassVar[int]
    PERSPECTIVE_FIELD_NUMBER: _ClassVar[int]
    DELAY_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    THRESHOLD_FIELD_NUMBER: _ClassVar[int]
    ONCE_FIELD_NUMBER: _ClassVar[int]
    REPEAT_FIELD_NUMBER: _ClassVar[int]
    SPLIT_FIELD_NUMBER: _ClassVar[int]
    OFFSCREEN_FIELD_NUMBER: _ClassVar[int]
    TRANSITION_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: EffectType
    style_ptr: NodeReferenceData
    opacity: float
    offset: Vector2Data
    scale: float
    rotate: Axis3Data
    skew: Vector2Data
    perspective: float
    delay: _duration_pb2.Duration
    duration: float
    threshold: float
    once: bool
    repeat: RepeatType
    split: TextSplitType
    offscreen: OffscreenBehavior
    transition: TransitionData
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[EffectType, str]] = ..., style_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., opacity: _Optional[float] = ..., offset: _Optional[_Union[Vector2Data, _Mapping]] = ..., scale: _Optional[float] = ..., rotate: _Optional[_Union[Axis3Data, _Mapping]] = ..., skew: _Optional[_Union[Vector2Data, _Mapping]] = ..., perspective: _Optional[float] = ..., delay: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., duration: _Optional[float] = ..., threshold: _Optional[float] = ..., once: bool = ..., repeat: _Optional[_Union[RepeatType, str]] = ..., split: _Optional[_Union[TextSplitType, str]] = ..., offscreen: _Optional[_Union[OffscreenBehavior, str]] = ..., transition: _Optional[_Union[TransitionData, _Mapping]] = ...) -> None: ...

class EffectStyleData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "mode", "order_key", "type", "name", "block_ptr", "style_ptr", "opacity", "offset", "scale", "rotate", "skew", "perspective", "delay", "duration", "threshold", "once", "repeat", "split", "offscreen", "transition")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    STYLE_PTR_FIELD_NUMBER: _ClassVar[int]
    OPACITY_FIELD_NUMBER: _ClassVar[int]
    OFFSET_FIELD_NUMBER: _ClassVar[int]
    SCALE_FIELD_NUMBER: _ClassVar[int]
    ROTATE_FIELD_NUMBER: _ClassVar[int]
    SKEW_FIELD_NUMBER: _ClassVar[int]
    PERSPECTIVE_FIELD_NUMBER: _ClassVar[int]
    DELAY_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    THRESHOLD_FIELD_NUMBER: _ClassVar[int]
    ONCE_FIELD_NUMBER: _ClassVar[int]
    REPEAT_FIELD_NUMBER: _ClassVar[int]
    SPLIT_FIELD_NUMBER: _ClassVar[int]
    OFFSCREEN_FIELD_NUMBER: _ClassVar[int]
    TRANSITION_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    type: EffectType
    name: str
    block_ptr: NodeReferenceData
    style_ptr: NodeReferenceData
    opacity: float
    offset: Vector2Data
    scale: float
    rotate: Axis3Data
    skew: Vector2Data
    perspective: float
    delay: _duration_pb2.Duration
    duration: float
    threshold: float
    once: bool
    repeat: RepeatType
    split: TextSplitType
    offscreen: OffscreenBehavior
    transition: TransitionData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[EffectType, str]] = ..., name: _Optional[str] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., style_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., opacity: _Optional[float] = ..., offset: _Optional[_Union[Vector2Data, _Mapping]] = ..., scale: _Optional[float] = ..., rotate: _Optional[_Union[Axis3Data, _Mapping]] = ..., skew: _Optional[_Union[Vector2Data, _Mapping]] = ..., perspective: _Optional[float] = ..., delay: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., duration: _Optional[float] = ..., threshold: _Optional[float] = ..., once: bool = ..., repeat: _Optional[_Union[RepeatType, str]] = ..., split: _Optional[_Union[TextSplitType, str]] = ..., offscreen: _Optional[_Union[OffscreenBehavior, str]] = ..., transition: _Optional[_Union[TransitionData, _Mapping]] = ...) -> None: ...

class EnumInfoData(_message.Message):
    __slots__ = ("metatype", "id", "type", "name", "icon", "description", "options")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    OPTIONS_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    id: int
    type: EnumType
    name: str
    icon: IconData
    description: str
    options: _containers.RepeatedCompositeFieldContainer[EnumOptionInfoData]
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., id: _Optional[int] = ..., type: _Optional[_Union[EnumType, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., description: _Optional[str] = ..., options: _Optional[_Iterable[_Union[EnumOptionInfoData, _Mapping]]] = ...) -> None: ...

class EnumOptionInfoData(_message.Message):
    __slots__ = ("metatype", "id", "type", "name", "icon", "description")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    id: int
    type: EnumType
    name: str
    icon: IconData
    description: str
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., id: _Optional[int] = ..., type: _Optional[_Union[EnumType, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., description: _Optional[str] = ...) -> None: ...

class ErrorData(_message.Message):
    __slots__ = ("metatype", "type", "title", "text", "nodes_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    NODES_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: ErrorType
    title: str
    text: str
    nodes_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[ErrorType, str]] = ..., title: _Optional[str] = ..., text: _Optional[str] = ..., nodes_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...) -> None: ...

class ExpressionData(_message.Message):
    __slots__ = ("metatype", "type", "literal", "attribute", "condition", "function", "aggregation")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    LITERAL_FIELD_NUMBER: _ClassVar[int]
    ATTRIBUTE_FIELD_NUMBER: _ClassVar[int]
    CONDITION_FIELD_NUMBER: _ClassVar[int]
    FUNCTION_FIELD_NUMBER: _ClassVar[int]
    AGGREGATION_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: ExpressionType
    literal: ValueData
    attribute: AttributeReferenceData
    condition: ConditionData
    function: FunctionData
    aggregation: AggregationData
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[ExpressionType, str]] = ..., literal: _Optional[_Union[ValueData, _Mapping]] = ..., attribute: _Optional[_Union[AttributeReferenceData, _Mapping]] = ..., condition: _Optional[_Union[ConditionData, _Mapping]] = ..., function: _Optional[_Union[FunctionData, _Mapping]] = ..., aggregation: _Optional[_Union[AggregationData, _Mapping]] = ...) -> None: ...

class FieldData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "mode", "order_key", "type", "name", "icon", "cardinality", "scalar_type", "primitive_type", "enum_type", "node_type", "struct_type", "base_type_ptr", "key_type", "is_required", "is_variable", "is_external", "default", "default_factory", "collection_constraint", "string_constraint", "number_constraint", "node_constraint", "edge_type", "cascade")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    CARDINALITY_FIELD_NUMBER: _ClassVar[int]
    SCALAR_TYPE_FIELD_NUMBER: _ClassVar[int]
    PRIMITIVE_TYPE_FIELD_NUMBER: _ClassVar[int]
    ENUM_TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    STRUCT_TYPE_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_PTR_FIELD_NUMBER: _ClassVar[int]
    KEY_TYPE_FIELD_NUMBER: _ClassVar[int]
    IS_REQUIRED_FIELD_NUMBER: _ClassVar[int]
    IS_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    IS_EXTERNAL_FIELD_NUMBER: _ClassVar[int]
    DEFAULT_FIELD_NUMBER: _ClassVar[int]
    DEFAULT_FACTORY_FIELD_NUMBER: _ClassVar[int]
    COLLECTION_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    STRING_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    NUMBER_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    NODE_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    EDGE_TYPE_FIELD_NUMBER: _ClassVar[int]
    CASCADE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    type: FieldType
    name: str
    icon: IconData
    cardinality: TypeCardinality
    scalar_type: ScalarType
    primitive_type: PrimitiveType
    enum_type: EnumType
    node_type: NodeType
    struct_type: StructType
    base_type_ptr: NodeReferenceData
    key_type: TypeData
    is_required: bool
    is_variable: bool
    is_external: bool
    default: ValueData
    default_factory: DefaultFactory
    collection_constraint: CollectionConstraintData
    string_constraint: StringConstraintData
    number_constraint: NumberConstraintData
    node_constraint: NodeConstraintData
    edge_type: EdgeType
    cascade: CascadeAction
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[FieldType, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., cardinality: _Optional[_Union[TypeCardinality, str]] = ..., scalar_type: _Optional[_Union[ScalarType, str]] = ..., primitive_type: _Optional[_Union[PrimitiveType, str]] = ..., enum_type: _Optional[_Union[EnumType, str]] = ..., node_type: _Optional[_Union[NodeType, str]] = ..., struct_type: _Optional[_Union[StructType, str]] = ..., base_type_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., key_type: _Optional[_Union[TypeData, _Mapping]] = ..., is_required: bool = ..., is_variable: bool = ..., is_external: bool = ..., default: _Optional[_Union[ValueData, _Mapping]] = ..., default_factory: _Optional[_Union[DefaultFactory, str]] = ..., collection_constraint: _Optional[_Union[CollectionConstraintData, _Mapping]] = ..., string_constraint: _Optional[_Union[StringConstraintData, _Mapping]] = ..., number_constraint: _Optional[_Union[NumberConstraintData, _Mapping]] = ..., node_constraint: _Optional[_Union[NodeConstraintData, _Mapping]] = ..., edge_type: _Optional[_Union[EdgeType, str]] = ..., cascade: _Optional[_Union[CascadeAction, str]] = ...) -> None: ...

class FileData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "owned_by_ptr", "mode", "order_key", "region", "type", "name", "block_ptr", "source", "mime_type", "format", "size", "sha256", "width", "height", "aspect_ratio", "codec", "duration", "url", "content_url", "thumbnail_url", "favicon_url", "thumbnail_width", "thumbnail_height", "content", "retention", "expires_at")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    SOURCE_FIELD_NUMBER: _ClassVar[int]
    MIME_TYPE_FIELD_NUMBER: _ClassVar[int]
    FORMAT_FIELD_NUMBER: _ClassVar[int]
    SIZE_FIELD_NUMBER: _ClassVar[int]
    SHA256_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    ASPECT_RATIO_FIELD_NUMBER: _ClassVar[int]
    CODEC_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    URL_FIELD_NUMBER: _ClassVar[int]
    CONTENT_URL_FIELD_NUMBER: _ClassVar[int]
    THUMBNAIL_URL_FIELD_NUMBER: _ClassVar[int]
    FAVICON_URL_FIELD_NUMBER: _ClassVar[int]
    THUMBNAIL_WIDTH_FIELD_NUMBER: _ClassVar[int]
    THUMBNAIL_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    CONTENT_FIELD_NUMBER: _ClassVar[int]
    RETENTION_FIELD_NUMBER: _ClassVar[int]
    EXPIRES_AT_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    region: Region
    type: FileType
    name: str
    block_ptr: NodeReferenceData
    source: FileSource
    mime_type: str
    format: FileFormat
    size: int
    sha256: str
    width: int
    height: int
    aspect_ratio: float
    codec: str
    duration: _duration_pb2.Duration
    url: str
    content_url: str
    thumbnail_url: str
    favicon_url: str
    thumbnail_width: int
    thumbnail_height: int
    content: bytes
    retention: FileRetentionMode
    expires_at: _timestamp_pb2.Timestamp
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., region: _Optional[_Union[Region, str]] = ..., type: _Optional[_Union[FileType, str]] = ..., name: _Optional[str] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., source: _Optional[_Union[FileSource, str]] = ..., mime_type: _Optional[str] = ..., format: _Optional[_Union[FileFormat, str]] = ..., size: _Optional[int] = ..., sha256: _Optional[str] = ..., width: _Optional[int] = ..., height: _Optional[int] = ..., aspect_ratio: _Optional[float] = ..., codec: _Optional[str] = ..., duration: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., url: _Optional[str] = ..., content_url: _Optional[str] = ..., thumbnail_url: _Optional[str] = ..., favicon_url: _Optional[str] = ..., thumbnail_width: _Optional[int] = ..., thumbnail_height: _Optional[int] = ..., content: _Optional[bytes] = ..., retention: _Optional[_Union[FileRetentionMode, str]] = ..., expires_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...) -> None: ...

class FillData(_message.Message):
    __slots__ = ("metatype", "type", "color", "gradient", "image_ptr", "position", "size")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    GRADIENT_FIELD_NUMBER: _ClassVar[int]
    IMAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    SIZE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: FillType
    color: ColorData
    gradient: GradientData
    image_ptr: NodeReferenceData
    position: FillPosition
    size: FillSize
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[FillType, str]] = ..., color: _Optional[_Union[ColorData, _Mapping]] = ..., gradient: _Optional[_Union[GradientData, _Mapping]] = ..., image_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., position: _Optional[_Union[FillPosition, str]] = ..., size: _Optional[_Union[FillSize, str]] = ...) -> None: ...

class FlowData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "owned_by_ptr", "mode", "order_key", "type", "name", "block_ptr", "model_developer", "model_provider", "model_id", "model_name", "max_attempts", "retry_interval", "backoff")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    MODEL_DEVELOPER_FIELD_NUMBER: _ClassVar[int]
    MODEL_PROVIDER_FIELD_NUMBER: _ClassVar[int]
    MODEL_ID_FIELD_NUMBER: _ClassVar[int]
    MODEL_NAME_FIELD_NUMBER: _ClassVar[int]
    MAX_ATTEMPTS_FIELD_NUMBER: _ClassVar[int]
    RETRY_INTERVAL_FIELD_NUMBER: _ClassVar[int]
    BACKOFF_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    type: FlowType
    name: str
    block_ptr: NodeReferenceData
    model_developer: ModelDeveloper
    model_provider: ModelProvider
    model_id: str
    model_name: str
    max_attempts: int
    retry_interval: _duration_pb2.Duration
    backoff: float
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[FlowType, str]] = ..., name: _Optional[str] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., model_developer: _Optional[_Union[ModelDeveloper, str]] = ..., model_provider: _Optional[_Union[ModelProvider, str]] = ..., model_id: _Optional[str] = ..., model_name: _Optional[str] = ..., max_attempts: _Optional[int] = ..., retry_interval: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., backoff: _Optional[float] = ...) -> None: ...

class FlowEdgeData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "template_ptr", "mode", "order_key", "type", "name", "source_ptr", "target_ptr", "model_developer", "model_provider", "model_id", "model_name", "max_attempts", "retry_interval", "backoff")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    MODEL_DEVELOPER_FIELD_NUMBER: _ClassVar[int]
    MODEL_PROVIDER_FIELD_NUMBER: _ClassVar[int]
    MODEL_ID_FIELD_NUMBER: _ClassVar[int]
    MODEL_NAME_FIELD_NUMBER: _ClassVar[int]
    MAX_ATTEMPTS_FIELD_NUMBER: _ClassVar[int]
    RETRY_INTERVAL_FIELD_NUMBER: _ClassVar[int]
    BACKOFF_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    template_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    type: FlowEdgeType
    name: str
    source_ptr: NodeReferenceData
    target_ptr: NodeReferenceData
    model_developer: ModelDeveloper
    model_provider: ModelProvider
    model_id: str
    model_name: str
    max_attempts: int
    retry_interval: _duration_pb2.Duration
    backoff: float
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[FlowEdgeType, str]] = ..., name: _Optional[str] = ..., source_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., model_developer: _Optional[_Union[ModelDeveloper, str]] = ..., model_provider: _Optional[_Union[ModelProvider, str]] = ..., model_id: _Optional[str] = ..., model_name: _Optional[str] = ..., max_attempts: _Optional[int] = ..., retry_interval: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., backoff: _Optional[float] = ...) -> None: ...

class FontData(_message.Message):
    __slots__ = ("metatype", "type", "style_ptr", "weight", "color", "size", "align", "line_height", "letter_spacing", "decoration", "transform")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    STYLE_PTR_FIELD_NUMBER: _ClassVar[int]
    WEIGHT_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    SIZE_FIELD_NUMBER: _ClassVar[int]
    ALIGN_FIELD_NUMBER: _ClassVar[int]
    LINE_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    LETTER_SPACING_FIELD_NUMBER: _ClassVar[int]
    DECORATION_FIELD_NUMBER: _ClassVar[int]
    TRANSFORM_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: FontType
    style_ptr: NodeReferenceData
    weight: FontWeight
    color: FillData
    size: FontSize
    align: TextAlign
    line_height: LengthData
    letter_spacing: LengthData
    decoration: TextDecoration
    transform: TextTransform
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[FontType, str]] = ..., style_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., weight: _Optional[_Union[FontWeight, str]] = ..., color: _Optional[_Union[FillData, _Mapping]] = ..., size: _Optional[_Union[FontSize, str]] = ..., align: _Optional[_Union[TextAlign, str]] = ..., line_height: _Optional[_Union[LengthData, _Mapping]] = ..., letter_spacing: _Optional[_Union[LengthData, _Mapping]] = ..., decoration: _Optional[_Union[TextDecoration, str]] = ..., transform: _Optional[_Union[TextTransform, str]] = ...) -> None: ...

class FontStyleData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "mode", "order_key", "type", "name", "block_ptr", "style_ptr", "weight", "color", "size", "align", "line_height", "letter_spacing", "decoration", "transform")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    STYLE_PTR_FIELD_NUMBER: _ClassVar[int]
    WEIGHT_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    SIZE_FIELD_NUMBER: _ClassVar[int]
    ALIGN_FIELD_NUMBER: _ClassVar[int]
    LINE_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    LETTER_SPACING_FIELD_NUMBER: _ClassVar[int]
    DECORATION_FIELD_NUMBER: _ClassVar[int]
    TRANSFORM_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    type: FontType
    name: str
    block_ptr: NodeReferenceData
    style_ptr: NodeReferenceData
    weight: FontWeight
    color: FillData
    size: FontSize
    align: TextAlign
    line_height: LengthData
    letter_spacing: LengthData
    decoration: TextDecoration
    transform: TextTransform
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[FontType, str]] = ..., name: _Optional[str] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., style_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., weight: _Optional[_Union[FontWeight, str]] = ..., color: _Optional[_Union[FillData, _Mapping]] = ..., size: _Optional[_Union[FontSize, str]] = ..., align: _Optional[_Union[TextAlign, str]] = ..., line_height: _Optional[_Union[LengthData, _Mapping]] = ..., letter_spacing: _Optional[_Union[LengthData, _Mapping]] = ..., decoration: _Optional[_Union[TextDecoration, str]] = ..., transform: _Optional[_Union[TextTransform, str]] = ...) -> None: ...

class FrameViewData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "mode", "value", "order_key", "name", "block_ptr", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "layout", "direction_value", "direction_variable", "distribute_value", "distribute_variable", "align_value", "align_variable", "gap_value", "gap_variable", "padding_value", "padding_variable", "grid", "grid_span", "aspect_ratio", "is_wrap", "is_visible_value", "is_visible_variable", "opacity_value", "opacity_variable", "fill_value", "fill_variable", "rotation_value", "rotation_variable", "skew_value", "skew_variable", "scale_value", "scale_variable", "shadow_value", "shadow_variable", "border_value", "border_variable", "radius_value", "radius_variable")
    class ValueEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueData
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueData, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    LAYOUT_FIELD_NUMBER: _ClassVar[int]
    DIRECTION_VALUE_FIELD_NUMBER: _ClassVar[int]
    DIRECTION_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    DISTRIBUTE_VALUE_FIELD_NUMBER: _ClassVar[int]
    DISTRIBUTE_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    ALIGN_VALUE_FIELD_NUMBER: _ClassVar[int]
    ALIGN_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    GAP_VALUE_FIELD_NUMBER: _ClassVar[int]
    GAP_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    PADDING_VALUE_FIELD_NUMBER: _ClassVar[int]
    PADDING_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    GRID_FIELD_NUMBER: _ClassVar[int]
    GRID_SPAN_FIELD_NUMBER: _ClassVar[int]
    ASPECT_RATIO_FIELD_NUMBER: _ClassVar[int]
    IS_WRAP_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_VALUE_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_VALUE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    FILL_VALUE_FIELD_NUMBER: _ClassVar[int]
    FILL_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    ROTATION_VALUE_FIELD_NUMBER: _ClassVar[int]
    ROTATION_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    SKEW_VALUE_FIELD_NUMBER: _ClassVar[int]
    SKEW_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    SCALE_VALUE_FIELD_NUMBER: _ClassVar[int]
    SCALE_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    SHADOW_VALUE_FIELD_NUMBER: _ClassVar[int]
    SHADOW_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    BORDER_VALUE_FIELD_NUMBER: _ClassVar[int]
    BORDER_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    RADIUS_VALUE_FIELD_NUMBER: _ClassVar[int]
    RADIUS_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    mode: NodeMode
    value: _containers.MessageMap[str, ValueData]
    order_key: str
    name: str
    block_ptr: NodeReferenceData
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    layout: Layout
    direction_value: Direction
    direction_variable: VariableData
    distribute_value: Distribute
    distribute_variable: VariableData
    align_value: Align
    align_variable: VariableData
    gap_value: Axis2Data
    gap_variable: VariableData
    padding_value: InsetsData
    padding_variable: VariableData
    grid: GridData
    grid_span: GridSpanData
    aspect_ratio: float
    is_wrap: bool
    is_visible_value: bool
    is_visible_variable: VariableData
    opacity_value: float
    opacity_variable: VariableData
    fill_value: FillData
    fill_variable: VariableData
    rotation_value: Axis3Data
    rotation_variable: VariableData
    skew_value: Vector2Data
    skew_variable: VariableData
    scale_value: float
    scale_variable: VariableData
    shadow_value: ShadowData
    shadow_variable: VariableData
    border_value: BorderData
    border_variable: VariableData
    radius_value: CornersData
    radius_variable: VariableData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., value: _Optional[_Mapping[str, ValueData]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ..., layout: _Optional[_Union[Layout, str]] = ..., direction_value: _Optional[_Union[Direction, str]] = ..., direction_variable: _Optional[_Union[VariableData, _Mapping]] = ..., distribute_value: _Optional[_Union[Distribute, str]] = ..., distribute_variable: _Optional[_Union[VariableData, _Mapping]] = ..., align_value: _Optional[_Union[Align, str]] = ..., align_variable: _Optional[_Union[VariableData, _Mapping]] = ..., gap_value: _Optional[_Union[Axis2Data, _Mapping]] = ..., gap_variable: _Optional[_Union[VariableData, _Mapping]] = ..., padding_value: _Optional[_Union[InsetsData, _Mapping]] = ..., padding_variable: _Optional[_Union[VariableData, _Mapping]] = ..., grid: _Optional[_Union[GridData, _Mapping]] = ..., grid_span: _Optional[_Union[GridSpanData, _Mapping]] = ..., aspect_ratio: _Optional[float] = ..., is_wrap: bool = ..., is_visible_value: bool = ..., is_visible_variable: _Optional[_Union[VariableData, _Mapping]] = ..., opacity_value: _Optional[float] = ..., opacity_variable: _Optional[_Union[VariableData, _Mapping]] = ..., fill_value: _Optional[_Union[FillData, _Mapping]] = ..., fill_variable: _Optional[_Union[VariableData, _Mapping]] = ..., rotation_value: _Optional[_Union[Axis3Data, _Mapping]] = ..., rotation_variable: _Optional[_Union[VariableData, _Mapping]] = ..., skew_value: _Optional[_Union[Vector2Data, _Mapping]] = ..., skew_variable: _Optional[_Union[VariableData, _Mapping]] = ..., scale_value: _Optional[float] = ..., scale_variable: _Optional[_Union[VariableData, _Mapping]] = ..., shadow_value: _Optional[_Union[ShadowData, _Mapping]] = ..., shadow_variable: _Optional[_Union[VariableData, _Mapping]] = ..., border_value: _Optional[_Union[BorderData, _Mapping]] = ..., border_variable: _Optional[_Union[VariableData, _Mapping]] = ..., radius_value: _Optional[_Union[CornersData, _Mapping]] = ..., radius_variable: _Optional[_Union[VariableData, _Mapping]] = ...) -> None: ...

class FriendshipData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "user_a_ptr", "user_b_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_A_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_B_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    user_a_ptr: NodeReferenceData
    user_b_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., user_a_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., user_b_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class FriendshipInviteData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "member_ptr", "inviter_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    MEMBER_PTR_FIELD_NUMBER: _ClassVar[int]
    INVITER_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    member_ptr: NodeReferenceData
    inviter_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., member_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., inviter_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class FunctionData(_message.Message):
    __slots__ = ("metatype", "type", "left", "right")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    LEFT_FIELD_NUMBER: _ClassVar[int]
    RIGHT_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: FunctionType
    left: ExpressionData
    right: ExpressionData
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[FunctionType, str]] = ..., left: _Optional[_Union[ExpressionData, _Mapping]] = ..., right: _Optional[_Union[ExpressionData, _Mapping]] = ...) -> None: ...

class GradientData(_message.Message):
    __slots__ = ("metatype", "type", "style_ptr", "angle", "stops", "center_anchor")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    STYLE_PTR_FIELD_NUMBER: _ClassVar[int]
    ANGLE_FIELD_NUMBER: _ClassVar[int]
    STOPS_FIELD_NUMBER: _ClassVar[int]
    CENTER_ANCHOR_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: GradientType
    style_ptr: NodeReferenceData
    angle: float
    stops: _containers.RepeatedCompositeFieldContainer[GradientStopData]
    center_anchor: Axis2Data
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[GradientType, str]] = ..., style_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., angle: _Optional[float] = ..., stops: _Optional[_Iterable[_Union[GradientStopData, _Mapping]]] = ..., center_anchor: _Optional[_Union[Axis2Data, _Mapping]] = ...) -> None: ...

class GradientStopData(_message.Message):
    __slots__ = ("metatype", "color", "position")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    color: ColorData
    position: float
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., color: _Optional[_Union[ColorData, _Mapping]] = ..., position: _Optional[float] = ...) -> None: ...

class GradientStyleData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "mode", "order_key", "type", "name", "block_ptr", "style_ptr", "angle", "stops", "center_anchor", "dark")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    STYLE_PTR_FIELD_NUMBER: _ClassVar[int]
    ANGLE_FIELD_NUMBER: _ClassVar[int]
    STOPS_FIELD_NUMBER: _ClassVar[int]
    CENTER_ANCHOR_FIELD_NUMBER: _ClassVar[int]
    DARK_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    type: GradientType
    name: str
    block_ptr: NodeReferenceData
    style_ptr: NodeReferenceData
    angle: float
    stops: _containers.RepeatedCompositeFieldContainer[GradientStopData]
    center_anchor: Axis2Data
    dark: GradientData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[GradientType, str]] = ..., name: _Optional[str] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., style_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., angle: _Optional[float] = ..., stops: _Optional[_Iterable[_Union[GradientStopData, _Mapping]]] = ..., center_anchor: _Optional[_Union[Axis2Data, _Mapping]] = ..., dark: _Optional[_Union[GradientData, _Mapping]] = ...) -> None: ...

class GridData(_message.Message):
    __slots__ = ("metatype", "columns", "rows", "column_width", "column_min_width", "row_height")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    COLUMNS_FIELD_NUMBER: _ClassVar[int]
    ROWS_FIELD_NUMBER: _ClassVar[int]
    COLUMN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    COLUMN_MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    ROW_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    columns: int
    rows: int
    column_width: DimensionData
    column_min_width: DimensionData
    row_height: DimensionData
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., columns: _Optional[int] = ..., rows: _Optional[int] = ..., column_width: _Optional[_Union[DimensionData, _Mapping]] = ..., column_min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., row_height: _Optional[_Union[DimensionData, _Mapping]] = ...) -> None: ...

class GridSpanData(_message.Message):
    __slots__ = ("metatype", "columns", "rows")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    COLUMNS_FIELD_NUMBER: _ClassVar[int]
    ROWS_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    columns: int
    rows: int
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., columns: _Optional[int] = ..., rows: _Optional[int] = ...) -> None: ...

class HandleData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "slug")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    slug: str
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., slug: _Optional[str] = ...) -> None: ...

class IconData(_message.Message):
    __slots__ = ("metatype", "type", "emoji", "fa_name", "vsc_name", "file_ptr", "file_url", "color")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    EMOJI_FIELD_NUMBER: _ClassVar[int]
    FA_NAME_FIELD_NUMBER: _ClassVar[int]
    VSC_NAME_FIELD_NUMBER: _ClassVar[int]
    FILE_PTR_FIELD_NUMBER: _ClassVar[int]
    FILE_URL_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: IconType
    emoji: str
    fa_name: str
    vsc_name: str
    file_ptr: NodeReferenceData
    file_url: str
    color: ColorData
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[IconType, str]] = ..., emoji: _Optional[str] = ..., fa_name: _Optional[str] = ..., vsc_name: _Optional[str] = ..., file_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., file_url: _Optional[str] = ..., color: _Optional[_Union[ColorData, _Mapping]] = ...) -> None: ...

class InsetsData(_message.Message):
    __slots__ = ("metatype", "base", "top", "left", "right", "bottom")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    BASE_FIELD_NUMBER: _ClassVar[int]
    TOP_FIELD_NUMBER: _ClassVar[int]
    LEFT_FIELD_NUMBER: _ClassVar[int]
    RIGHT_FIELD_NUMBER: _ClassVar[int]
    BOTTOM_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    base: int
    top: int
    left: int
    right: int
    bottom: int
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., base: _Optional[int] = ..., top: _Optional[int] = ..., left: _Optional[int] = ..., right: _Optional[int] = ..., bottom: _Optional[int] = ...) -> None: ...

class InterruptionData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "mode", "value", "type", "runnable_ptr", "span_ptr", "status", "duration", "closed_at", "response", "message_ptr", "task_ptr")
    class ValueEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueData
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueData, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    RUNNABLE_PTR_FIELD_NUMBER: _ClassVar[int]
    SPAN_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    CLOSED_AT_FIELD_NUMBER: _ClassVar[int]
    RESPONSE_FIELD_NUMBER: _ClassVar[int]
    MESSAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    TASK_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    mode: NodeMode
    value: _containers.MessageMap[str, ValueData]
    type: InterruptionType
    runnable_ptr: NodeReferenceData
    span_ptr: NodeReferenceData
    status: InterruptionStatus
    duration: _duration_pb2.Duration
    closed_at: _timestamp_pb2.Timestamp
    response: InterruptionResponse
    message_ptr: NodeReferenceData
    task_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., value: _Optional[_Mapping[str, ValueData]] = ..., type: _Optional[_Union[InterruptionType, str]] = ..., runnable_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., span_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., status: _Optional[_Union[InterruptionStatus, str]] = ..., duration: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., closed_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., response: _Optional[_Union[InterruptionResponse, str]] = ..., message_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., task_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class JoinData(_message.Message):
    __slots__ = ("metatype", "type", "relation", "on", "recursive")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    RELATION_FIELD_NUMBER: _ClassVar[int]
    ON_FIELD_NUMBER: _ClassVar[int]
    RECURSIVE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: JoinType
    relation: RelationReferenceData
    on: ConditionData
    recursive: bool
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[JoinType, str]] = ..., relation: _Optional[_Union[RelationReferenceData, _Mapping]] = ..., on: _Optional[_Union[ConditionData, _Mapping]] = ..., recursive: bool = ...) -> None: ...

class LabelViewData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "mode", "value", "order_key", "name", "block_ptr", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "layout", "direction_value", "direction_variable", "distribute_value", "distribute_variable", "align_value", "align_variable", "gap_value", "gap_variable", "padding_value", "padding_variable", "grid", "grid_span", "aspect_ratio", "is_wrap", "is_visible_value", "is_visible_variable", "opacity_value", "opacity_variable", "fill_value", "fill_variable", "rotation_value", "rotation_variable", "skew_value", "skew_variable", "scale_value", "scale_variable", "shadow_value", "shadow_variable", "border_value", "border_variable", "radius_value", "radius_variable")
    class ValueEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueData
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueData, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    LAYOUT_FIELD_NUMBER: _ClassVar[int]
    DIRECTION_VALUE_FIELD_NUMBER: _ClassVar[int]
    DIRECTION_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    DISTRIBUTE_VALUE_FIELD_NUMBER: _ClassVar[int]
    DISTRIBUTE_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    ALIGN_VALUE_FIELD_NUMBER: _ClassVar[int]
    ALIGN_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    GAP_VALUE_FIELD_NUMBER: _ClassVar[int]
    GAP_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    PADDING_VALUE_FIELD_NUMBER: _ClassVar[int]
    PADDING_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    GRID_FIELD_NUMBER: _ClassVar[int]
    GRID_SPAN_FIELD_NUMBER: _ClassVar[int]
    ASPECT_RATIO_FIELD_NUMBER: _ClassVar[int]
    IS_WRAP_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_VALUE_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_VALUE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    FILL_VALUE_FIELD_NUMBER: _ClassVar[int]
    FILL_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    ROTATION_VALUE_FIELD_NUMBER: _ClassVar[int]
    ROTATION_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    SKEW_VALUE_FIELD_NUMBER: _ClassVar[int]
    SKEW_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    SCALE_VALUE_FIELD_NUMBER: _ClassVar[int]
    SCALE_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    SHADOW_VALUE_FIELD_NUMBER: _ClassVar[int]
    SHADOW_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    BORDER_VALUE_FIELD_NUMBER: _ClassVar[int]
    BORDER_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    RADIUS_VALUE_FIELD_NUMBER: _ClassVar[int]
    RADIUS_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    mode: NodeMode
    value: _containers.MessageMap[str, ValueData]
    order_key: str
    name: str
    block_ptr: NodeReferenceData
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    layout: Layout
    direction_value: Direction
    direction_variable: VariableData
    distribute_value: Distribute
    distribute_variable: VariableData
    align_value: Align
    align_variable: VariableData
    gap_value: Axis2Data
    gap_variable: VariableData
    padding_value: InsetsData
    padding_variable: VariableData
    grid: GridData
    grid_span: GridSpanData
    aspect_ratio: float
    is_wrap: bool
    is_visible_value: bool
    is_visible_variable: VariableData
    opacity_value: float
    opacity_variable: VariableData
    fill_value: FillData
    fill_variable: VariableData
    rotation_value: Axis3Data
    rotation_variable: VariableData
    skew_value: Vector2Data
    skew_variable: VariableData
    scale_value: float
    scale_variable: VariableData
    shadow_value: ShadowData
    shadow_variable: VariableData
    border_value: BorderData
    border_variable: VariableData
    radius_value: CornersData
    radius_variable: VariableData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., value: _Optional[_Mapping[str, ValueData]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ..., layout: _Optional[_Union[Layout, str]] = ..., direction_value: _Optional[_Union[Direction, str]] = ..., direction_variable: _Optional[_Union[VariableData, _Mapping]] = ..., distribute_value: _Optional[_Union[Distribute, str]] = ..., distribute_variable: _Optional[_Union[VariableData, _Mapping]] = ..., align_value: _Optional[_Union[Align, str]] = ..., align_variable: _Optional[_Union[VariableData, _Mapping]] = ..., gap_value: _Optional[_Union[Axis2Data, _Mapping]] = ..., gap_variable: _Optional[_Union[VariableData, _Mapping]] = ..., padding_value: _Optional[_Union[InsetsData, _Mapping]] = ..., padding_variable: _Optional[_Union[VariableData, _Mapping]] = ..., grid: _Optional[_Union[GridData, _Mapping]] = ..., grid_span: _Optional[_Union[GridSpanData, _Mapping]] = ..., aspect_ratio: _Optional[float] = ..., is_wrap: bool = ..., is_visible_value: bool = ..., is_visible_variable: _Optional[_Union[VariableData, _Mapping]] = ..., opacity_value: _Optional[float] = ..., opacity_variable: _Optional[_Union[VariableData, _Mapping]] = ..., fill_value: _Optional[_Union[FillData, _Mapping]] = ..., fill_variable: _Optional[_Union[VariableData, _Mapping]] = ..., rotation_value: _Optional[_Union[Axis3Data, _Mapping]] = ..., rotation_variable: _Optional[_Union[VariableData, _Mapping]] = ..., skew_value: _Optional[_Union[Vector2Data, _Mapping]] = ..., skew_variable: _Optional[_Union[VariableData, _Mapping]] = ..., scale_value: _Optional[float] = ..., scale_variable: _Optional[_Union[VariableData, _Mapping]] = ..., shadow_value: _Optional[_Union[ShadowData, _Mapping]] = ..., shadow_variable: _Optional[_Union[VariableData, _Mapping]] = ..., border_value: _Optional[_Union[BorderData, _Mapping]] = ..., border_variable: _Optional[_Union[VariableData, _Mapping]] = ..., radius_value: _Optional[_Union[CornersData, _Mapping]] = ..., radius_variable: _Optional[_Union[VariableData, _Mapping]] = ...) -> None: ...

class LengthData(_message.Message):
    __slots__ = ("metatype", "unit", "value")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    UNIT_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    unit: LengthUnit
    value: float
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., unit: _Optional[_Union[LengthUnit, str]] = ..., value: _Optional[float] = ...) -> None: ...

class LinkData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "owned_by_ptr", "mode", "order_key", "type", "name", "title", "block_ptr", "url", "domain", "content_url", "thumbnail_url", "favicon_url", "thumbnail_width", "thumbnail_height", "content", "attribution", "attribution_tag", "published_at", "expires_at", "image_urls")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    URL_FIELD_NUMBER: _ClassVar[int]
    DOMAIN_FIELD_NUMBER: _ClassVar[int]
    CONTENT_URL_FIELD_NUMBER: _ClassVar[int]
    THUMBNAIL_URL_FIELD_NUMBER: _ClassVar[int]
    FAVICON_URL_FIELD_NUMBER: _ClassVar[int]
    THUMBNAIL_WIDTH_FIELD_NUMBER: _ClassVar[int]
    THUMBNAIL_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    CONTENT_FIELD_NUMBER: _ClassVar[int]
    ATTRIBUTION_FIELD_NUMBER: _ClassVar[int]
    ATTRIBUTION_TAG_FIELD_NUMBER: _ClassVar[int]
    PUBLISHED_AT_FIELD_NUMBER: _ClassVar[int]
    EXPIRES_AT_FIELD_NUMBER: _ClassVar[int]
    IMAGE_URLS_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    type: LinkType
    name: str
    title: TextLineData
    block_ptr: NodeReferenceData
    url: str
    domain: str
    content_url: str
    thumbnail_url: str
    favicon_url: str
    thumbnail_width: int
    thumbnail_height: int
    content: str
    attribution: str
    attribution_tag: str
    published_at: _timestamp_pb2.Timestamp
    expires_at: _timestamp_pb2.Timestamp
    image_urls: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[LinkType, str]] = ..., name: _Optional[str] = ..., title: _Optional[_Union[TextLineData, _Mapping]] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., url: _Optional[str] = ..., domain: _Optional[str] = ..., content_url: _Optional[str] = ..., thumbnail_url: _Optional[str] = ..., favicon_url: _Optional[str] = ..., thumbnail_width: _Optional[int] = ..., thumbnail_height: _Optional[int] = ..., content: _Optional[str] = ..., attribution: _Optional[str] = ..., attribution_tag: _Optional[str] = ..., published_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., expires_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., image_urls: _Optional[_Iterable[str]] = ...) -> None: ...

class MachineData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "owned_by_ptr", "mode", "order_key", "type", "name", "block_ptr", "status", "requested_activate_at", "requested_deactivate_at", "requested_reset_at", "requested_suspend_at", "requested_decommission_at", "active_at", "failed_at", "failed_attempts", "version", "external_name", "external_id", "image_id", "grpc_url", "vnc_url", "client_ptr", "cpu", "ram", "width", "height", "is_headless")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_ACTIVATE_AT_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_DEACTIVATE_AT_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_RESET_AT_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_SUSPEND_AT_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_DECOMMISSION_AT_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_AT_FIELD_NUMBER: _ClassVar[int]
    FAILED_AT_FIELD_NUMBER: _ClassVar[int]
    FAILED_ATTEMPTS_FIELD_NUMBER: _ClassVar[int]
    VERSION_FIELD_NUMBER: _ClassVar[int]
    EXTERNAL_NAME_FIELD_NUMBER: _ClassVar[int]
    EXTERNAL_ID_FIELD_NUMBER: _ClassVar[int]
    IMAGE_ID_FIELD_NUMBER: _ClassVar[int]
    GRPC_URL_FIELD_NUMBER: _ClassVar[int]
    VNC_URL_FIELD_NUMBER: _ClassVar[int]
    CLIENT_PTR_FIELD_NUMBER: _ClassVar[int]
    CPU_FIELD_NUMBER: _ClassVar[int]
    RAM_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    IS_HEADLESS_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    type: MachineType
    name: str
    block_ptr: NodeReferenceData
    status: ResourceStatus
    requested_activate_at: _timestamp_pb2.Timestamp
    requested_deactivate_at: _timestamp_pb2.Timestamp
    requested_reset_at: _timestamp_pb2.Timestamp
    requested_suspend_at: _timestamp_pb2.Timestamp
    requested_decommission_at: _timestamp_pb2.Timestamp
    active_at: _timestamp_pb2.Timestamp
    failed_at: _timestamp_pb2.Timestamp
    failed_attempts: int
    version: str
    external_name: str
    external_id: str
    image_id: str
    grpc_url: str
    vnc_url: str
    client_ptr: NodeReferenceData
    cpu: float
    ram: float
    width: int
    height: int
    is_headless: bool
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[MachineType, str]] = ..., name: _Optional[str] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., status: _Optional[_Union[ResourceStatus, str]] = ..., requested_activate_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., requested_deactivate_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., requested_reset_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., requested_suspend_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., requested_decommission_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., active_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., failed_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., failed_attempts: _Optional[int] = ..., version: _Optional[str] = ..., external_name: _Optional[str] = ..., external_id: _Optional[str] = ..., image_id: _Optional[str] = ..., grpc_url: _Optional[str] = ..., vnc_url: _Optional[str] = ..., client_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., cpu: _Optional[float] = ..., ram: _Optional[float] = ..., width: _Optional[int] = ..., height: _Optional[int] = ..., is_headless: bool = ...) -> None: ...

class MessageData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "owned_by_ptr", "mode", "type", "title", "thread_ptr", "edited_at", "reply_to_ptr", "forwarded_from_ptr", "text", "node_ptr", "model_developer", "model_provider", "model_id", "model_name")
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
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    THREAD_PTR_FIELD_NUMBER: _ClassVar[int]
    EDITED_AT_FIELD_NUMBER: _ClassVar[int]
    REPLY_TO_PTR_FIELD_NUMBER: _ClassVar[int]
    FORWARDED_FROM_PTR_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    MODEL_DEVELOPER_FIELD_NUMBER: _ClassVar[int]
    MODEL_PROVIDER_FIELD_NUMBER: _ClassVar[int]
    MODEL_ID_FIELD_NUMBER: _ClassVar[int]
    MODEL_NAME_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    owned_by_ptr: NodeReferenceData
    mode: NodeMode
    type: MessageType
    title: TextLineData
    thread_ptr: NodeReferenceData
    edited_at: _timestamp_pb2.Timestamp
    reply_to_ptr: NodeReferenceData
    forwarded_from_ptr: NodeReferenceData
    text: TextData
    node_ptr: NodeReferenceData
    model_developer: ModelDeveloper
    model_provider: ModelProvider
    model_id: str
    model_name: str
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., type: _Optional[_Union[MessageType, str]] = ..., title: _Optional[_Union[TextLineData, _Mapping]] = ..., thread_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., edited_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., reply_to_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., forwarded_from_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., model_developer: _Optional[_Union[ModelDeveloper, str]] = ..., model_provider: _Optional[_Union[ModelProvider, str]] = ..., model_id: _Optional[str] = ..., model_name: _Optional[str] = ...) -> None: ...

class NodeConstraintData(_message.Message):
    __slots__ = ("metatype", "node_types", "node_traits")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPES_FIELD_NUMBER: _ClassVar[int]
    NODE_TRAITS_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    node_types: _containers.RepeatedScalarFieldContainer[NodeType]
    node_traits: _containers.RepeatedScalarFieldContainer[TraitType]
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., node_types: _Optional[_Iterable[_Union[NodeType, str]]] = ..., node_traits: _Optional[_Iterable[_Union[TraitType, str]]] = ...) -> None: ...

class NodeInfoData(_message.Message):
    __slots__ = ("metatype", "id", "type", "name", "icon", "description", "properties", "traits")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    TRAITS_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    id: int
    type: NodeType
    name: str
    icon: IconData
    description: str
    properties: _containers.RepeatedCompositeFieldContainer[PropertyInfoData]
    traits: _containers.RepeatedScalarFieldContainer[TraitType]
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., id: _Optional[int] = ..., type: _Optional[_Union[NodeType, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., description: _Optional[str] = ..., properties: _Optional[_Iterable[_Union[PropertyInfoData, _Mapping]]] = ..., traits: _Optional[_Iterable[_Union[TraitType, str]]] = ...) -> None: ...

class NodeReferenceData(_message.Message):
    __slots__ = ("metatype", "node_type", "id", "bench_id", "definition_id")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    BENCH_ID_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_ID_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    node_type: NodeType
    id: str
    bench_id: str
    definition_id: str
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., node_type: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., bench_id: _Optional[str] = ..., definition_id: _Optional[str] = ...) -> None: ...

class NumberConstraintData(_message.Message):
    __slots__ = ("metatype", "format", "min_value", "max_value", "step_value", "precision", "scale")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    FORMAT_FIELD_NUMBER: _ClassVar[int]
    MIN_VALUE_FIELD_NUMBER: _ClassVar[int]
    MAX_VALUE_FIELD_NUMBER: _ClassVar[int]
    STEP_VALUE_FIELD_NUMBER: _ClassVar[int]
    PRECISION_FIELD_NUMBER: _ClassVar[int]
    SCALE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    format: NumberFormat
    min_value: float
    max_value: float
    step_value: float
    precision: int
    scale: int
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., format: _Optional[_Union[NumberFormat, str]] = ..., min_value: _Optional[float] = ..., max_value: _Optional[float] = ..., step_value: _Optional[float] = ..., precision: _Optional[int] = ..., scale: _Optional[int] = ...) -> None: ...

class NumberInputViewData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "mode", "order_key", "name", "block_ptr", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "is_visible_value", "is_visible_variable", "opacity_value", "opacity_variable", "value", "placeholder")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_VALUE_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_VALUE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    PLACEHOLDER_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    name: str
    block_ptr: NodeReferenceData
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    is_visible_value: bool
    is_visible_variable: VariableData
    opacity_value: float
    opacity_variable: VariableData
    value: str
    placeholder: str
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ..., is_visible_value: bool = ..., is_visible_variable: _Optional[_Union[VariableData, _Mapping]] = ..., opacity_value: _Optional[float] = ..., opacity_variable: _Optional[_Union[VariableData, _Mapping]] = ..., value: _Optional[str] = ..., placeholder: _Optional[str] = ...) -> None: ...

class OrganizationData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "region", "name", "slug", "icon", "status", "bench_ptr", "handle_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    HANDLE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    region: Region
    name: str
    slug: str
    icon: IconData
    status: OrganizationStatus
    bench_ptr: NodeReferenceData
    handle_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., region: _Optional[_Union[Region, str]] = ..., name: _Optional[str] = ..., slug: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., status: _Optional[_Union[OrganizationStatus, str]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., handle_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class OrganizationInviteData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "member_ptr", "role_type")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    MEMBER_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLE_TYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    member_ptr: NodeReferenceData
    role_type: OrganizationRoleType
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., member_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., role_type: _Optional[_Union[OrganizationRoleType, str]] = ...) -> None: ...

class OrganizationMembershipData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "member_ptr", "role_type")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    MEMBER_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLE_TYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    member_ptr: NodeReferenceData
    role_type: OrganizationRoleType
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., member_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., role_type: _Optional[_Union[OrganizationRoleType, str]] = ...) -> None: ...

class OriginData(_message.Message):
    __slots__ = ("metatype", "type", "id", "ck", "nonce")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    NONCE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: ClientType
    id: str
    ck: str
    nonce: str
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[ClientType, str]] = ..., id: _Optional[str] = ..., ck: _Optional[str] = ..., nonce: _Optional[str] = ...) -> None: ...

class PackageData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "owned_by_ptr", "mode", "type", "name", "slug", "icon", "main_page_ptr", "main_scene_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    MAIN_PAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    MAIN_SCENE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    mode: NodeMode
    type: PackageType
    name: str
    slug: str
    icon: IconData
    main_page_ptr: NodeReferenceData
    main_scene_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., type: _Optional[_Union[PackageType, str]] = ..., name: _Optional[str] = ..., slug: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., main_page_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., main_scene_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class PackageInviteData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "member_ptr", "role_type")
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
    MEMBER_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLE_TYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    member_ptr: NodeReferenceData
    role_type: PackageRoleType
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., member_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., role_type: _Optional[_Union[PackageRoleType, str]] = ...) -> None: ...

class PackageMembershipData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "member_ptr", "role_type")
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
    MEMBER_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLE_TYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    member_ptr: NodeReferenceData
    role_type: PackageRoleType
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., member_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., role_type: _Optional[_Union[PackageRoleType, str]] = ...) -> None: ...

class PageData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "owned_by_ptr", "mode", "order_key", "title", "slug", "icon", "block_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    title: TextLineData
    slug: str
    icon: IconData
    block_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., title: _Optional[_Union[TextLineData, _Mapping]] = ..., slug: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class PositionData(_message.Message):
    __slots__ = ("metatype", "type", "top", "left", "width", "height")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    TOP_FIELD_NUMBER: _ClassVar[int]
    LEFT_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: PositionType
    top: LengthData
    left: LengthData
    width: LengthData
    height: LengthData
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[PositionType, str]] = ..., top: _Optional[_Union[LengthData, _Mapping]] = ..., left: _Optional[_Union[LengthData, _Mapping]] = ..., width: _Optional[_Union[LengthData, _Mapping]] = ..., height: _Optional[_Union[LengthData, _Mapping]] = ...) -> None: ...

class PropertyInfoData(_message.Message):
    __slots__ = ("metatype", "id", "name", "icon", "description", "cardinality", "scalar_type", "primitive_type", "enum_type", "node_type", "struct_type", "base_type_ptr", "key_type", "is_required", "is_variable", "is_external", "default", "default_factory", "collection_constraint", "string_constraint", "number_constraint", "node_constraint", "node_is_customizable", "edge_type", "cascade", "is_wired", "is_stored", "is_repr", "is_hash", "is_eq", "is_managed", "is_computed")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    CARDINALITY_FIELD_NUMBER: _ClassVar[int]
    SCALAR_TYPE_FIELD_NUMBER: _ClassVar[int]
    PRIMITIVE_TYPE_FIELD_NUMBER: _ClassVar[int]
    ENUM_TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    STRUCT_TYPE_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_PTR_FIELD_NUMBER: _ClassVar[int]
    KEY_TYPE_FIELD_NUMBER: _ClassVar[int]
    IS_REQUIRED_FIELD_NUMBER: _ClassVar[int]
    IS_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    IS_EXTERNAL_FIELD_NUMBER: _ClassVar[int]
    DEFAULT_FIELD_NUMBER: _ClassVar[int]
    DEFAULT_FACTORY_FIELD_NUMBER: _ClassVar[int]
    COLLECTION_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    STRING_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    NUMBER_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    NODE_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    NODE_IS_CUSTOMIZABLE_FIELD_NUMBER: _ClassVar[int]
    EDGE_TYPE_FIELD_NUMBER: _ClassVar[int]
    CASCADE_FIELD_NUMBER: _ClassVar[int]
    IS_WIRED_FIELD_NUMBER: _ClassVar[int]
    IS_STORED_FIELD_NUMBER: _ClassVar[int]
    IS_REPR_FIELD_NUMBER: _ClassVar[int]
    IS_HASH_FIELD_NUMBER: _ClassVar[int]
    IS_EQ_FIELD_NUMBER: _ClassVar[int]
    IS_MANAGED_FIELD_NUMBER: _ClassVar[int]
    IS_COMPUTED_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    id: int
    name: str
    icon: IconData
    description: str
    cardinality: TypeCardinality
    scalar_type: ScalarType
    primitive_type: PrimitiveType
    enum_type: EnumType
    node_type: NodeType
    struct_type: StructType
    base_type_ptr: NodeReferenceData
    key_type: TypeData
    is_required: bool
    is_variable: bool
    is_external: bool
    default: ValueData
    default_factory: DefaultFactory
    collection_constraint: CollectionConstraintData
    string_constraint: StringConstraintData
    number_constraint: NumberConstraintData
    node_constraint: NodeConstraintData
    node_is_customizable: bool
    edge_type: EdgeType
    cascade: CascadeAction
    is_wired: bool
    is_stored: bool
    is_repr: bool
    is_hash: bool
    is_eq: bool
    is_managed: bool
    is_computed: bool
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., id: _Optional[int] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., description: _Optional[str] = ..., cardinality: _Optional[_Union[TypeCardinality, str]] = ..., scalar_type: _Optional[_Union[ScalarType, str]] = ..., primitive_type: _Optional[_Union[PrimitiveType, str]] = ..., enum_type: _Optional[_Union[EnumType, str]] = ..., node_type: _Optional[_Union[NodeType, str]] = ..., struct_type: _Optional[_Union[StructType, str]] = ..., base_type_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., key_type: _Optional[_Union[TypeData, _Mapping]] = ..., is_required: bool = ..., is_variable: bool = ..., is_external: bool = ..., default: _Optional[_Union[ValueData, _Mapping]] = ..., default_factory: _Optional[_Union[DefaultFactory, str]] = ..., collection_constraint: _Optional[_Union[CollectionConstraintData, _Mapping]] = ..., string_constraint: _Optional[_Union[StringConstraintData, _Mapping]] = ..., number_constraint: _Optional[_Union[NumberConstraintData, _Mapping]] = ..., node_constraint: _Optional[_Union[NodeConstraintData, _Mapping]] = ..., node_is_customizable: bool = ..., edge_type: _Optional[_Union[EdgeType, str]] = ..., cascade: _Optional[_Union[CascadeAction, str]] = ..., is_wired: bool = ..., is_stored: bool = ..., is_repr: bool = ..., is_hash: bool = ..., is_eq: bool = ..., is_managed: bool = ..., is_computed: bool = ...) -> None: ...

class PropertyReferenceData(_message.Message):
    __slots__ = ("metatype", "node_type", "struct_type", "id")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    STRUCT_TYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    node_type: NodeType
    struct_type: StructType
    id: int
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., node_type: _Optional[_Union[NodeType, str]] = ..., struct_type: _Optional[_Union[StructType, str]] = ..., id: _Optional[int] = ...) -> None: ...

class QueryData(_message.Message):
    __slots__ = ("metatype", "id", "type", "name", "relation", "join", "select", "subqueries", "is_live", "where", "having", "group_by", "aggregation", "sort", "limit", "offset", "count")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    RELATION_FIELD_NUMBER: _ClassVar[int]
    JOIN_FIELD_NUMBER: _ClassVar[int]
    SELECT_FIELD_NUMBER: _ClassVar[int]
    SUBQUERIES_FIELD_NUMBER: _ClassVar[int]
    IS_LIVE_FIELD_NUMBER: _ClassVar[int]
    WHERE_FIELD_NUMBER: _ClassVar[int]
    HAVING_FIELD_NUMBER: _ClassVar[int]
    GROUP_BY_FIELD_NUMBER: _ClassVar[int]
    AGGREGATION_FIELD_NUMBER: _ClassVar[int]
    SORT_FIELD_NUMBER: _ClassVar[int]
    LIMIT_FIELD_NUMBER: _ClassVar[int]
    OFFSET_FIELD_NUMBER: _ClassVar[int]
    COUNT_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    id: str
    type: QueryType
    name: str
    relation: RelationReferenceData
    join: JoinData
    select: SelectData
    subqueries: _containers.RepeatedCompositeFieldContainer[QueryData]
    is_live: bool
    where: ConditionData
    having: ConditionData
    group_by: _containers.RepeatedCompositeFieldContainer[ExpressionData]
    aggregation: AggregationData
    sort: _containers.RepeatedCompositeFieldContainer[SortData]
    limit: int
    offset: int
    count: bool
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., id: _Optional[str] = ..., type: _Optional[_Union[QueryType, str]] = ..., name: _Optional[str] = ..., relation: _Optional[_Union[RelationReferenceData, _Mapping]] = ..., join: _Optional[_Union[JoinData, _Mapping]] = ..., select: _Optional[_Union[SelectData, _Mapping]] = ..., subqueries: _Optional[_Iterable[_Union[QueryData, _Mapping]]] = ..., is_live: bool = ..., where: _Optional[_Union[ConditionData, _Mapping]] = ..., having: _Optional[_Union[ConditionData, _Mapping]] = ..., group_by: _Optional[_Iterable[_Union[ExpressionData, _Mapping]]] = ..., aggregation: _Optional[_Union[AggregationData, _Mapping]] = ..., sort: _Optional[_Iterable[_Union[SortData, _Mapping]]] = ..., limit: _Optional[int] = ..., offset: _Optional[int] = ..., count: bool = ...) -> None: ...

class QueryResultData(_message.Message):
    __slots__ = ("metatype", "epoch", "query")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    EPOCH_FIELD_NUMBER: _ClassVar[int]
    QUERY_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    epoch: int
    query: QueryData
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., epoch: _Optional[int] = ..., query: _Optional[_Union[QueryData, _Mapping]] = ...) -> None: ...

class QueryUpdateData(_message.Message):
    __slots__ = ("metatype", "epoch")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    EPOCH_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    epoch: int
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., epoch: _Optional[int] = ...) -> None: ...

class RelationReferenceData(_message.Message):
    __slots__ = ("metatype", "type", "node_type", "definition_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: RelationType
    node_type: NodeType
    definition_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[RelationType, str]] = ..., node_type: _Optional[_Union[NodeType, str]] = ..., definition_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class RouteData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "owned_by_ptr", "mode", "order_key", "name", "slug", "icon", "block_ptr", "scene_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    SCENE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    owned_by_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    name: str
    slug: str
    icon: IconData
    block_ptr: NodeReferenceData
    scene_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., slug: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., scene_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class RunData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "mode", "value", "type", "thread_ptr", "code", "runnable_ptr", "status", "duration", "error", "interruption_ptr", "scheduled_at", "started_at", "active_at", "interrupted_at", "terminated_at", "requested_stop_at", "requested_pause_at", "requested_resume_at", "model_developer", "model_provider", "model_id", "model_name")
    class ValueEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueData
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueData, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    THREAD_PTR_FIELD_NUMBER: _ClassVar[int]
    CODE_FIELD_NUMBER: _ClassVar[int]
    RUNNABLE_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    ERROR_FIELD_NUMBER: _ClassVar[int]
    INTERRUPTION_PTR_FIELD_NUMBER: _ClassVar[int]
    SCHEDULED_AT_FIELD_NUMBER: _ClassVar[int]
    STARTED_AT_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_AT_FIELD_NUMBER: _ClassVar[int]
    INTERRUPTED_AT_FIELD_NUMBER: _ClassVar[int]
    TERMINATED_AT_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_STOP_AT_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_PAUSE_AT_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_RESUME_AT_FIELD_NUMBER: _ClassVar[int]
    MODEL_DEVELOPER_FIELD_NUMBER: _ClassVar[int]
    MODEL_PROVIDER_FIELD_NUMBER: _ClassVar[int]
    MODEL_ID_FIELD_NUMBER: _ClassVar[int]
    MODEL_NAME_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    mode: NodeMode
    value: _containers.MessageMap[str, ValueData]
    type: RunType
    thread_ptr: NodeReferenceData
    code: CodeData
    runnable_ptr: NodeReferenceData
    status: ProcessStatus
    duration: _duration_pb2.Duration
    error: ErrorData
    interruption_ptr: NodeReferenceData
    scheduled_at: _timestamp_pb2.Timestamp
    started_at: _timestamp_pb2.Timestamp
    active_at: _timestamp_pb2.Timestamp
    interrupted_at: _timestamp_pb2.Timestamp
    terminated_at: _timestamp_pb2.Timestamp
    requested_stop_at: _timestamp_pb2.Timestamp
    requested_pause_at: _timestamp_pb2.Timestamp
    requested_resume_at: _timestamp_pb2.Timestamp
    model_developer: ModelDeveloper
    model_provider: ModelProvider
    model_id: str
    model_name: str
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., value: _Optional[_Mapping[str, ValueData]] = ..., type: _Optional[_Union[RunType, str]] = ..., thread_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., code: _Optional[_Union[CodeData, _Mapping]] = ..., runnable_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., status: _Optional[_Union[ProcessStatus, str]] = ..., duration: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., error: _Optional[_Union[ErrorData, _Mapping]] = ..., interruption_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., scheduled_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., started_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., active_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., interrupted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., terminated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., requested_stop_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., requested_pause_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., requested_resume_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., model_developer: _Optional[_Union[ModelDeveloper, str]] = ..., model_provider: _Optional[_Union[ModelProvider, str]] = ..., model_id: _Optional[str] = ..., model_name: _Optional[str] = ...) -> None: ...

class SceneData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "owned_by_ptr", "mode", "order_key", "name", "slug", "icon", "block_ptr", "root_view_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    ROOT_VIEW_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    name: str
    slug: str
    icon: IconData
    block_ptr: NodeReferenceData
    root_view_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., slug: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., root_view_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

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
    metatype: StructType
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
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., frequency: _Optional[_Union[ScheduleFrequency, str]] = ..., interval: _Optional[int] = ..., start: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., end: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., count: _Optional[int] = ..., week_start: _Optional[_Union[Day, str]] = ..., by_set_pos: _Optional[_Iterable[int]] = ..., by_month: _Optional[_Iterable[_Union[Month, str]]] = ..., by_month_day: _Optional[_Iterable[int]] = ..., by_year_day: _Optional[_Iterable[int]] = ..., by_easter: _Optional[_Iterable[int]] = ..., by_week_no: _Optional[_Iterable[int]] = ..., by_week_day: _Optional[_Iterable[_Union[Day, str]]] = ..., by_hour: _Optional[_Iterable[int]] = ..., by_minute: _Optional[_Iterable[int]] = ..., by_second: _Optional[_Iterable[int]] = ...) -> None: ...

class SchemaData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "mode", "order_key", "name", "block_ptr", "cardinality", "scalar_type", "primitive_type", "enum_type", "node_type", "struct_type", "base_type_ptr", "key_type", "is_required", "is_variable", "is_external", "default", "default_factory", "collection_constraint", "string_constraint", "number_constraint", "node_constraint")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    CARDINALITY_FIELD_NUMBER: _ClassVar[int]
    SCALAR_TYPE_FIELD_NUMBER: _ClassVar[int]
    PRIMITIVE_TYPE_FIELD_NUMBER: _ClassVar[int]
    ENUM_TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    STRUCT_TYPE_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_PTR_FIELD_NUMBER: _ClassVar[int]
    KEY_TYPE_FIELD_NUMBER: _ClassVar[int]
    IS_REQUIRED_FIELD_NUMBER: _ClassVar[int]
    IS_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    IS_EXTERNAL_FIELD_NUMBER: _ClassVar[int]
    DEFAULT_FIELD_NUMBER: _ClassVar[int]
    DEFAULT_FACTORY_FIELD_NUMBER: _ClassVar[int]
    COLLECTION_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    STRING_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    NUMBER_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    NODE_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    name: str
    block_ptr: NodeReferenceData
    cardinality: TypeCardinality
    scalar_type: ScalarType
    primitive_type: PrimitiveType
    enum_type: EnumType
    node_type: NodeType
    struct_type: StructType
    base_type_ptr: NodeReferenceData
    key_type: TypeData
    is_required: bool
    is_variable: bool
    is_external: bool
    default: ValueData
    default_factory: DefaultFactory
    collection_constraint: CollectionConstraintData
    string_constraint: StringConstraintData
    number_constraint: NumberConstraintData
    node_constraint: NodeConstraintData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., cardinality: _Optional[_Union[TypeCardinality, str]] = ..., scalar_type: _Optional[_Union[ScalarType, str]] = ..., primitive_type: _Optional[_Union[PrimitiveType, str]] = ..., enum_type: _Optional[_Union[EnumType, str]] = ..., node_type: _Optional[_Union[NodeType, str]] = ..., struct_type: _Optional[_Union[StructType, str]] = ..., base_type_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., key_type: _Optional[_Union[TypeData, _Mapping]] = ..., is_required: bool = ..., is_variable: bool = ..., is_external: bool = ..., default: _Optional[_Union[ValueData, _Mapping]] = ..., default_factory: _Optional[_Union[DefaultFactory, str]] = ..., collection_constraint: _Optional[_Union[CollectionConstraintData, _Mapping]] = ..., string_constraint: _Optional[_Union[StringConstraintData, _Mapping]] = ..., number_constraint: _Optional[_Union[NumberConstraintData, _Mapping]] = ..., node_constraint: _Optional[_Union[NodeConstraintData, _Mapping]] = ...) -> None: ...

class ScopeData(_message.Message):
    __slots__ = ("metatype", "region", "bench_id")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    BENCH_ID_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    region: Region
    bench_id: str
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., region: _Optional[_Union[Region, str]] = ..., bench_id: _Optional[str] = ...) -> None: ...

class SelectData(_message.Message):
    __slots__ = ("metatype", "attributes")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ATTRIBUTES_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    attributes: _containers.RepeatedCompositeFieldContainer[AttributeReferenceData]
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., attributes: _Optional[_Iterable[_Union[AttributeReferenceData, _Mapping]]] = ...) -> None: ...

class SelectionData(_message.Message):
    __slots__ = ("metatype", "nodes_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    NODES_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    nodes_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., nodes_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...) -> None: ...

class ServiceData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "owned_by_ptr", "mode", "order_key", "name", "block_ptr", "model_developer", "model_provider", "model_id", "model_name", "max_attempts", "retry_interval", "backoff")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    MODEL_DEVELOPER_FIELD_NUMBER: _ClassVar[int]
    MODEL_PROVIDER_FIELD_NUMBER: _ClassVar[int]
    MODEL_ID_FIELD_NUMBER: _ClassVar[int]
    MODEL_NAME_FIELD_NUMBER: _ClassVar[int]
    MAX_ATTEMPTS_FIELD_NUMBER: _ClassVar[int]
    RETRY_INTERVAL_FIELD_NUMBER: _ClassVar[int]
    BACKOFF_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    name: str
    block_ptr: NodeReferenceData
    model_developer: ModelDeveloper
    model_provider: ModelProvider
    model_id: str
    model_name: str
    max_attempts: int
    retry_interval: _duration_pb2.Duration
    backoff: float
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., model_developer: _Optional[_Union[ModelDeveloper, str]] = ..., model_provider: _Optional[_Union[ModelProvider, str]] = ..., model_id: _Optional[str] = ..., model_name: _Optional[str] = ..., max_attempts: _Optional[int] = ..., retry_interval: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., backoff: _Optional[float] = ...) -> None: ...

class ShadowData(_message.Message):
    __slots__ = ("metatype", "type", "style_ptr", "color", "position", "offset", "blur", "spread", "diffusion")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    STYLE_PTR_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    OFFSET_FIELD_NUMBER: _ClassVar[int]
    BLUR_FIELD_NUMBER: _ClassVar[int]
    SPREAD_FIELD_NUMBER: _ClassVar[int]
    DIFFUSION_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: ShadowType
    style_ptr: NodeReferenceData
    color: ColorData
    position: ShadowPosition
    offset: Axis2Data
    blur: int
    spread: int
    diffusion: float
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[ShadowType, str]] = ..., style_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., color: _Optional[_Union[ColorData, _Mapping]] = ..., position: _Optional[_Union[ShadowPosition, str]] = ..., offset: _Optional[_Union[Axis2Data, _Mapping]] = ..., blur: _Optional[int] = ..., spread: _Optional[int] = ..., diffusion: _Optional[float] = ...) -> None: ...

class ShadowStyleData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "mode", "order_key", "type", "name", "block_ptr", "style_ptr", "color", "position", "offset", "blur", "spread", "diffusion")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    STYLE_PTR_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    OFFSET_FIELD_NUMBER: _ClassVar[int]
    BLUR_FIELD_NUMBER: _ClassVar[int]
    SPREAD_FIELD_NUMBER: _ClassVar[int]
    DIFFUSION_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    type: ShadowType
    name: str
    block_ptr: NodeReferenceData
    style_ptr: NodeReferenceData
    color: ColorData
    position: ShadowPosition
    offset: Axis2Data
    blur: int
    spread: int
    diffusion: float
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[ShadowType, str]] = ..., name: _Optional[str] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., style_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., color: _Optional[_Union[ColorData, _Mapping]] = ..., position: _Optional[_Union[ShadowPosition, str]] = ..., offset: _Optional[_Union[Axis2Data, _Mapping]] = ..., blur: _Optional[int] = ..., spread: _Optional[int] = ..., diffusion: _Optional[float] = ...) -> None: ...

class SliderInputViewData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "mode", "order_key", "name", "block_ptr", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "is_visible_value", "is_visible_variable", "opacity_value", "opacity_variable", "value", "min_value", "max_value", "step")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_VALUE_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_VALUE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    MIN_VALUE_FIELD_NUMBER: _ClassVar[int]
    MAX_VALUE_FIELD_NUMBER: _ClassVar[int]
    STEP_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    name: str
    block_ptr: NodeReferenceData
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    is_visible_value: bool
    is_visible_variable: VariableData
    opacity_value: float
    opacity_variable: VariableData
    value: float
    min_value: float
    max_value: float
    step: float
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ..., is_visible_value: bool = ..., is_visible_variable: _Optional[_Union[VariableData, _Mapping]] = ..., opacity_value: _Optional[float] = ..., opacity_variable: _Optional[_Union[VariableData, _Mapping]] = ..., value: _Optional[float] = ..., min_value: _Optional[float] = ..., max_value: _Optional[float] = ..., step: _Optional[float] = ...) -> None: ...

class SortData(_message.Message):
    __slots__ = ("metatype", "type", "by", "mode")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    BY_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: SortType
    by: ExpressionData
    mode: SortMode
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[SortType, str]] = ..., by: _Optional[_Union[ExpressionData, _Mapping]] = ..., mode: _Optional[_Union[SortMode, str]] = ...) -> None: ...

class SpaceData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "owned_by_ptr", "mode", "order_key", "type", "name", "selection", "focus_ptr", "inspection_ptr", "container_ptr", "page_ptr", "thread_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SELECTION_FIELD_NUMBER: _ClassVar[int]
    FOCUS_PTR_FIELD_NUMBER: _ClassVar[int]
    INSPECTION_PTR_FIELD_NUMBER: _ClassVar[int]
    CONTAINER_PTR_FIELD_NUMBER: _ClassVar[int]
    PAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    THREAD_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    type: SpaceType
    name: str
    selection: SelectionData
    focus_ptr: NodeReferenceData
    inspection_ptr: NodeReferenceData
    container_ptr: NodeReferenceData
    page_ptr: NodeReferenceData
    thread_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[SpaceType, str]] = ..., name: _Optional[str] = ..., selection: _Optional[_Union[SelectionData, _Mapping]] = ..., focus_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., inspection_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., container_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., page_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., thread_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class SpanData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "mode", "type", "title", "status", "duration", "error", "interruption_ptr", "scheduled_at", "started_at", "active_at", "interrupted_at", "terminated_at", "requested_stop_at", "requested_pause_at", "requested_resume_at")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    ERROR_FIELD_NUMBER: _ClassVar[int]
    INTERRUPTION_PTR_FIELD_NUMBER: _ClassVar[int]
    SCHEDULED_AT_FIELD_NUMBER: _ClassVar[int]
    STARTED_AT_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_AT_FIELD_NUMBER: _ClassVar[int]
    INTERRUPTED_AT_FIELD_NUMBER: _ClassVar[int]
    TERMINATED_AT_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_STOP_AT_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_PAUSE_AT_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_RESUME_AT_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    mode: NodeMode
    type: SpanType
    title: str
    status: ProcessStatus
    duration: _duration_pb2.Duration
    error: ErrorData
    interruption_ptr: NodeReferenceData
    scheduled_at: _timestamp_pb2.Timestamp
    started_at: _timestamp_pb2.Timestamp
    active_at: _timestamp_pb2.Timestamp
    interrupted_at: _timestamp_pb2.Timestamp
    terminated_at: _timestamp_pb2.Timestamp
    requested_stop_at: _timestamp_pb2.Timestamp
    requested_pause_at: _timestamp_pb2.Timestamp
    requested_resume_at: _timestamp_pb2.Timestamp
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., type: _Optional[_Union[SpanType, str]] = ..., title: _Optional[str] = ..., status: _Optional[_Union[ProcessStatus, str]] = ..., duration: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., error: _Optional[_Union[ErrorData, _Mapping]] = ..., interruption_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., scheduled_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., started_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., active_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., interrupted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., terminated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., requested_stop_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., requested_pause_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., requested_resume_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...) -> None: ...

class SplitViewData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "mode", "value", "order_key", "name", "block_ptr", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "layout", "direction_value", "direction_variable", "distribute_value", "distribute_variable", "align_value", "align_variable", "gap_value", "gap_variable", "padding_value", "padding_variable", "grid", "grid_span", "aspect_ratio", "is_wrap", "is_visible_value", "is_visible_variable", "opacity_value", "opacity_variable", "fill_value", "fill_variable", "rotation_value", "rotation_variable", "skew_value", "skew_variable", "scale_value", "scale_variable", "shadow_value", "shadow_variable", "border_value", "border_variable", "radius_value", "radius_variable")
    class ValueEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueData
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueData, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    LAYOUT_FIELD_NUMBER: _ClassVar[int]
    DIRECTION_VALUE_FIELD_NUMBER: _ClassVar[int]
    DIRECTION_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    DISTRIBUTE_VALUE_FIELD_NUMBER: _ClassVar[int]
    DISTRIBUTE_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    ALIGN_VALUE_FIELD_NUMBER: _ClassVar[int]
    ALIGN_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    GAP_VALUE_FIELD_NUMBER: _ClassVar[int]
    GAP_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    PADDING_VALUE_FIELD_NUMBER: _ClassVar[int]
    PADDING_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    GRID_FIELD_NUMBER: _ClassVar[int]
    GRID_SPAN_FIELD_NUMBER: _ClassVar[int]
    ASPECT_RATIO_FIELD_NUMBER: _ClassVar[int]
    IS_WRAP_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_VALUE_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_VALUE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    FILL_VALUE_FIELD_NUMBER: _ClassVar[int]
    FILL_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    ROTATION_VALUE_FIELD_NUMBER: _ClassVar[int]
    ROTATION_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    SKEW_VALUE_FIELD_NUMBER: _ClassVar[int]
    SKEW_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    SCALE_VALUE_FIELD_NUMBER: _ClassVar[int]
    SCALE_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    SHADOW_VALUE_FIELD_NUMBER: _ClassVar[int]
    SHADOW_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    BORDER_VALUE_FIELD_NUMBER: _ClassVar[int]
    BORDER_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    RADIUS_VALUE_FIELD_NUMBER: _ClassVar[int]
    RADIUS_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    mode: NodeMode
    value: _containers.MessageMap[str, ValueData]
    order_key: str
    name: str
    block_ptr: NodeReferenceData
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    layout: Layout
    direction_value: Direction
    direction_variable: VariableData
    distribute_value: Distribute
    distribute_variable: VariableData
    align_value: Align
    align_variable: VariableData
    gap_value: Axis2Data
    gap_variable: VariableData
    padding_value: InsetsData
    padding_variable: VariableData
    grid: GridData
    grid_span: GridSpanData
    aspect_ratio: float
    is_wrap: bool
    is_visible_value: bool
    is_visible_variable: VariableData
    opacity_value: float
    opacity_variable: VariableData
    fill_value: FillData
    fill_variable: VariableData
    rotation_value: Axis3Data
    rotation_variable: VariableData
    skew_value: Vector2Data
    skew_variable: VariableData
    scale_value: float
    scale_variable: VariableData
    shadow_value: ShadowData
    shadow_variable: VariableData
    border_value: BorderData
    border_variable: VariableData
    radius_value: CornersData
    radius_variable: VariableData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., value: _Optional[_Mapping[str, ValueData]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ..., layout: _Optional[_Union[Layout, str]] = ..., direction_value: _Optional[_Union[Direction, str]] = ..., direction_variable: _Optional[_Union[VariableData, _Mapping]] = ..., distribute_value: _Optional[_Union[Distribute, str]] = ..., distribute_variable: _Optional[_Union[VariableData, _Mapping]] = ..., align_value: _Optional[_Union[Align, str]] = ..., align_variable: _Optional[_Union[VariableData, _Mapping]] = ..., gap_value: _Optional[_Union[Axis2Data, _Mapping]] = ..., gap_variable: _Optional[_Union[VariableData, _Mapping]] = ..., padding_value: _Optional[_Union[InsetsData, _Mapping]] = ..., padding_variable: _Optional[_Union[VariableData, _Mapping]] = ..., grid: _Optional[_Union[GridData, _Mapping]] = ..., grid_span: _Optional[_Union[GridSpanData, _Mapping]] = ..., aspect_ratio: _Optional[float] = ..., is_wrap: bool = ..., is_visible_value: bool = ..., is_visible_variable: _Optional[_Union[VariableData, _Mapping]] = ..., opacity_value: _Optional[float] = ..., opacity_variable: _Optional[_Union[VariableData, _Mapping]] = ..., fill_value: _Optional[_Union[FillData, _Mapping]] = ..., fill_variable: _Optional[_Union[VariableData, _Mapping]] = ..., rotation_value: _Optional[_Union[Axis3Data, _Mapping]] = ..., rotation_variable: _Optional[_Union[VariableData, _Mapping]] = ..., skew_value: _Optional[_Union[Vector2Data, _Mapping]] = ..., skew_variable: _Optional[_Union[VariableData, _Mapping]] = ..., scale_value: _Optional[float] = ..., scale_variable: _Optional[_Union[VariableData, _Mapping]] = ..., shadow_value: _Optional[_Union[ShadowData, _Mapping]] = ..., shadow_variable: _Optional[_Union[VariableData, _Mapping]] = ..., border_value: _Optional[_Union[BorderData, _Mapping]] = ..., border_variable: _Optional[_Union[VariableData, _Mapping]] = ..., radius_value: _Optional[_Union[CornersData, _Mapping]] = ..., radius_variable: _Optional[_Union[VariableData, _Mapping]] = ...) -> None: ...

class StringConstraintData(_message.Message):
    __slots__ = ("metatype", "format", "regex", "starts_with", "ends_with")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    FORMAT_FIELD_NUMBER: _ClassVar[int]
    REGEX_FIELD_NUMBER: _ClassVar[int]
    STARTS_WITH_FIELD_NUMBER: _ClassVar[int]
    ENDS_WITH_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    format: StringFormat
    regex: str
    starts_with: str
    ends_with: str
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., format: _Optional[_Union[StringFormat, str]] = ..., regex: _Optional[str] = ..., starts_with: _Optional[str] = ..., ends_with: _Optional[str] = ...) -> None: ...

class StructInfoData(_message.Message):
    __slots__ = ("metatype", "id", "type", "name", "icon", "description", "properties")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    id: int
    type: StructType
    name: str
    icon: IconData
    description: str
    properties: _containers.RepeatedCompositeFieldContainer[PropertyInfoData]
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., id: _Optional[int] = ..., type: _Optional[_Union[StructType, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., description: _Optional[str] = ..., properties: _Optional[_Iterable[_Union[PropertyInfoData, _Mapping]]] = ...) -> None: ...

class TaskData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "owned_by_ptr", "mode", "order_key", "title", "block_ptr", "due_at", "assigned_to_ptr", "status", "duration", "error", "interruption_ptr", "scheduled_at", "started_at", "active_at", "interrupted_at", "terminated_at", "requested_stop_at", "requested_pause_at", "requested_resume_at")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    DUE_AT_FIELD_NUMBER: _ClassVar[int]
    ASSIGNED_TO_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    ERROR_FIELD_NUMBER: _ClassVar[int]
    INTERRUPTION_PTR_FIELD_NUMBER: _ClassVar[int]
    SCHEDULED_AT_FIELD_NUMBER: _ClassVar[int]
    STARTED_AT_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_AT_FIELD_NUMBER: _ClassVar[int]
    INTERRUPTED_AT_FIELD_NUMBER: _ClassVar[int]
    TERMINATED_AT_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_STOP_AT_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_PAUSE_AT_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_RESUME_AT_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    title: TextLineData
    block_ptr: NodeReferenceData
    due_at: _timestamp_pb2.Timestamp
    assigned_to_ptr: NodeReferenceData
    status: ProcessStatus
    duration: _duration_pb2.Duration
    error: ErrorData
    interruption_ptr: NodeReferenceData
    scheduled_at: _timestamp_pb2.Timestamp
    started_at: _timestamp_pb2.Timestamp
    active_at: _timestamp_pb2.Timestamp
    interrupted_at: _timestamp_pb2.Timestamp
    terminated_at: _timestamp_pb2.Timestamp
    requested_stop_at: _timestamp_pb2.Timestamp
    requested_pause_at: _timestamp_pb2.Timestamp
    requested_resume_at: _timestamp_pb2.Timestamp
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., title: _Optional[_Union[TextLineData, _Mapping]] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., due_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., assigned_to_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., status: _Optional[_Union[ProcessStatus, str]] = ..., duration: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., error: _Optional[_Union[ErrorData, _Mapping]] = ..., interruption_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., scheduled_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., started_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., active_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., interrupted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., terminated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., requested_stop_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., requested_pause_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., requested_resume_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...) -> None: ...

class TextData(_message.Message):
    __slots__ = ("metatype", "lines")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    LINES_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    lines: _containers.RepeatedCompositeFieldContainer[TextLineData]
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., lines: _Optional[_Iterable[_Union[TextLineData, _Mapping]]] = ...) -> None: ...

class TextLineData(_message.Message):
    __slots__ = ("metatype", "type", "spans", "content", "color", "background_color", "is_bold", "is_italic", "is_strikethrough", "is_underline", "is_code", "is_spoiler", "language")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    SPANS_FIELD_NUMBER: _ClassVar[int]
    CONTENT_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    BACKGROUND_COLOR_FIELD_NUMBER: _ClassVar[int]
    IS_BOLD_FIELD_NUMBER: _ClassVar[int]
    IS_ITALIC_FIELD_NUMBER: _ClassVar[int]
    IS_STRIKETHROUGH_FIELD_NUMBER: _ClassVar[int]
    IS_UNDERLINE_FIELD_NUMBER: _ClassVar[int]
    IS_CODE_FIELD_NUMBER: _ClassVar[int]
    IS_SPOILER_FIELD_NUMBER: _ClassVar[int]
    LANGUAGE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: TextLineType
    spans: _containers.RepeatedCompositeFieldContainer[TextSpanData]
    content: str
    color: ColorHue
    background_color: ColorHue
    is_bold: bool
    is_italic: bool
    is_strikethrough: bool
    is_underline: bool
    is_code: bool
    is_spoiler: bool
    language: str
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[TextLineType, str]] = ..., spans: _Optional[_Iterable[_Union[TextSpanData, _Mapping]]] = ..., content: _Optional[str] = ..., color: _Optional[_Union[ColorHue, str]] = ..., background_color: _Optional[_Union[ColorHue, str]] = ..., is_bold: bool = ..., is_italic: bool = ..., is_strikethrough: bool = ..., is_underline: bool = ..., is_code: bool = ..., is_spoiler: bool = ..., language: _Optional[str] = ...) -> None: ...

class TextSpanData(_message.Message):
    __slots__ = ("metatype", "type", "content", "node_ptr", "url", "color", "background_color", "is_bold", "is_italic", "is_strikethrough", "is_underline", "is_code", "is_spoiler", "language")
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
    IS_SPOILER_FIELD_NUMBER: _ClassVar[int]
    LANGUAGE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: TextSpanType
    content: str
    node_ptr: NodeReferenceData
    url: str
    color: ColorHue
    background_color: ColorHue
    is_bold: bool
    is_italic: bool
    is_strikethrough: bool
    is_underline: bool
    is_code: bool
    is_spoiler: bool
    language: str
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[TextSpanType, str]] = ..., content: _Optional[str] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., url: _Optional[str] = ..., color: _Optional[_Union[ColorHue, str]] = ..., background_color: _Optional[_Union[ColorHue, str]] = ..., is_bold: bool = ..., is_italic: bool = ..., is_strikethrough: bool = ..., is_underline: bool = ..., is_code: bool = ..., is_spoiler: bool = ..., language: _Optional[str] = ...) -> None: ...

class TextViewData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "mode", "order_key", "name", "block_ptr", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "align", "is_visible_value", "is_visible_variable", "opacity_value", "opacity_variable", "user_select", "font", "color_value", "color_variable", "text_value", "text_variable")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    ALIGN_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_VALUE_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_VALUE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    USER_SELECT_FIELD_NUMBER: _ClassVar[int]
    FONT_FIELD_NUMBER: _ClassVar[int]
    COLOR_VALUE_FIELD_NUMBER: _ClassVar[int]
    COLOR_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    TEXT_VALUE_FIELD_NUMBER: _ClassVar[int]
    TEXT_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    name: str
    block_ptr: NodeReferenceData
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    align: Align
    is_visible_value: bool
    is_visible_variable: VariableData
    opacity_value: float
    opacity_variable: VariableData
    user_select: bool
    font: FontData
    color_value: FillData
    color_variable: VariableData
    text_value: str
    text_variable: VariableData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ..., align: _Optional[_Union[Align, str]] = ..., is_visible_value: bool = ..., is_visible_variable: _Optional[_Union[VariableData, _Mapping]] = ..., opacity_value: _Optional[float] = ..., opacity_variable: _Optional[_Union[VariableData, _Mapping]] = ..., user_select: bool = ..., font: _Optional[_Union[FontData, _Mapping]] = ..., color_value: _Optional[_Union[FillData, _Mapping]] = ..., color_variable: _Optional[_Union[VariableData, _Mapping]] = ..., text_value: _Optional[str] = ..., text_variable: _Optional[_Union[VariableData, _Mapping]] = ...) -> None: ...

class ThemeData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "mode", "order_key", "name", "icon", "block_ptr", "colors")
    class ColorsEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: int
        value: ColorData
        def __init__(self, key: _Optional[int] = ..., value: _Optional[_Union[ColorData, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    COLORS_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    mode: NodeMode
    order_key: str
    name: str
    icon: IconData
    block_ptr: NodeReferenceData
    colors: _containers.MessageMap[int, ColorData]
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., colors: _Optional[_Mapping[int, ColorData]] = ...) -> None: ...

class ThreadData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "owned_by_ptr", "mode", "title", "status", "duration", "error", "interruption_ptr", "scheduled_at", "started_at", "active_at", "interrupted_at", "terminated_at", "requested_stop_at", "requested_pause_at", "requested_resume_at", "model_developer", "model_provider", "model_id", "model_name")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    ERROR_FIELD_NUMBER: _ClassVar[int]
    INTERRUPTION_PTR_FIELD_NUMBER: _ClassVar[int]
    SCHEDULED_AT_FIELD_NUMBER: _ClassVar[int]
    STARTED_AT_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_AT_FIELD_NUMBER: _ClassVar[int]
    INTERRUPTED_AT_FIELD_NUMBER: _ClassVar[int]
    TERMINATED_AT_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_STOP_AT_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_PAUSE_AT_FIELD_NUMBER: _ClassVar[int]
    REQUESTED_RESUME_AT_FIELD_NUMBER: _ClassVar[int]
    MODEL_DEVELOPER_FIELD_NUMBER: _ClassVar[int]
    MODEL_PROVIDER_FIELD_NUMBER: _ClassVar[int]
    MODEL_ID_FIELD_NUMBER: _ClassVar[int]
    MODEL_NAME_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    owned_by_ptr: NodeReferenceData
    mode: NodeMode
    title: TextLineData
    status: ProcessStatus
    duration: _duration_pb2.Duration
    error: ErrorData
    interruption_ptr: NodeReferenceData
    scheduled_at: _timestamp_pb2.Timestamp
    started_at: _timestamp_pb2.Timestamp
    active_at: _timestamp_pb2.Timestamp
    interrupted_at: _timestamp_pb2.Timestamp
    terminated_at: _timestamp_pb2.Timestamp
    requested_stop_at: _timestamp_pb2.Timestamp
    requested_pause_at: _timestamp_pb2.Timestamp
    requested_resume_at: _timestamp_pb2.Timestamp
    model_developer: ModelDeveloper
    model_provider: ModelProvider
    model_id: str
    model_name: str
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., title: _Optional[_Union[TextLineData, _Mapping]] = ..., status: _Optional[_Union[ProcessStatus, str]] = ..., duration: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., error: _Optional[_Union[ErrorData, _Mapping]] = ..., interruption_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., scheduled_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., started_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., active_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., interrupted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., terminated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., requested_stop_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., requested_pause_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., requested_resume_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., model_developer: _Optional[_Union[ModelDeveloper, str]] = ..., model_provider: _Optional[_Union[ModelProvider, str]] = ..., model_id: _Optional[str] = ..., model_name: _Optional[str] = ...) -> None: ...

class ThreadViewData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "mode", "order_key", "name", "block_ptr", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "draft_text", "draft_reply_to_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    DRAFT_TEXT_FIELD_NUMBER: _ClassVar[int]
    DRAFT_REPLY_TO_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    name: str
    block_ptr: NodeReferenceData
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    draft_text: TextData
    draft_reply_to_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ..., draft_text: _Optional[_Union[TextData, _Mapping]] = ..., draft_reply_to_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class TraitInfoData(_message.Message):
    __slots__ = ("metatype", "id", "type", "name", "icon", "description", "properties", "nodes")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    NODES_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    id: int
    type: TraitType
    name: str
    icon: IconData
    description: str
    properties: _containers.RepeatedCompositeFieldContainer[PropertyInfoData]
    nodes: _containers.RepeatedScalarFieldContainer[NodeType]
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., id: _Optional[int] = ..., type: _Optional[_Union[TraitType, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., description: _Optional[str] = ..., properties: _Optional[_Iterable[_Union[PropertyInfoData, _Mapping]]] = ..., nodes: _Optional[_Iterable[_Union[NodeType, str]]] = ...) -> None: ...

class TransitionData(_message.Message):
    __slots__ = ("metatype", "type", "style_ptr", "delay", "duration", "ease", "stiffness", "damping", "mass", "bounce", "spring_type")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    STYLE_PTR_FIELD_NUMBER: _ClassVar[int]
    DELAY_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    EASE_FIELD_NUMBER: _ClassVar[int]
    STIFFNESS_FIELD_NUMBER: _ClassVar[int]
    DAMPING_FIELD_NUMBER: _ClassVar[int]
    MASS_FIELD_NUMBER: _ClassVar[int]
    BOUNCE_FIELD_NUMBER: _ClassVar[int]
    SPRING_TYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: TransitionType
    style_ptr: NodeReferenceData
    delay: float
    duration: float
    ease: _containers.RepeatedScalarFieldContainer[float]
    stiffness: float
    damping: float
    mass: float
    bounce: float
    spring_type: SpringType
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[TransitionType, str]] = ..., style_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., delay: _Optional[float] = ..., duration: _Optional[float] = ..., ease: _Optional[_Iterable[float]] = ..., stiffness: _Optional[float] = ..., damping: _Optional[float] = ..., mass: _Optional[float] = ..., bounce: _Optional[float] = ..., spring_type: _Optional[_Union[SpringType, str]] = ...) -> None: ...

class TransitionStyleData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "mode", "order_key", "type", "name", "block_ptr", "style_ptr", "delay", "duration", "ease", "stiffness", "damping", "mass", "bounce", "spring_type")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    STYLE_PTR_FIELD_NUMBER: _ClassVar[int]
    DELAY_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    EASE_FIELD_NUMBER: _ClassVar[int]
    STIFFNESS_FIELD_NUMBER: _ClassVar[int]
    DAMPING_FIELD_NUMBER: _ClassVar[int]
    MASS_FIELD_NUMBER: _ClassVar[int]
    BOUNCE_FIELD_NUMBER: _ClassVar[int]
    SPRING_TYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    type: TransitionType
    name: str
    block_ptr: NodeReferenceData
    style_ptr: NodeReferenceData
    delay: float
    duration: float
    ease: _containers.RepeatedScalarFieldContainer[float]
    stiffness: float
    damping: float
    mass: float
    bounce: float
    spring_type: SpringType
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[TransitionType, str]] = ..., name: _Optional[str] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., style_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., delay: _Optional[float] = ..., duration: _Optional[float] = ..., ease: _Optional[_Iterable[float]] = ..., stiffness: _Optional[float] = ..., damping: _Optional[float] = ..., mass: _Optional[float] = ..., bounce: _Optional[float] = ..., spring_type: _Optional[_Union[SpringType, str]] = ...) -> None: ...

class TypeData(_message.Message):
    __slots__ = ("metatype", "cardinality", "scalar_type", "primitive_type", "enum_type", "node_type", "struct_type", "base_type_ptr", "key_type", "is_required", "is_variable", "is_external", "default", "default_factory", "collection_constraint", "string_constraint", "number_constraint", "node_constraint")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    CARDINALITY_FIELD_NUMBER: _ClassVar[int]
    SCALAR_TYPE_FIELD_NUMBER: _ClassVar[int]
    PRIMITIVE_TYPE_FIELD_NUMBER: _ClassVar[int]
    ENUM_TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    STRUCT_TYPE_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_PTR_FIELD_NUMBER: _ClassVar[int]
    KEY_TYPE_FIELD_NUMBER: _ClassVar[int]
    IS_REQUIRED_FIELD_NUMBER: _ClassVar[int]
    IS_VARIABLE_FIELD_NUMBER: _ClassVar[int]
    IS_EXTERNAL_FIELD_NUMBER: _ClassVar[int]
    DEFAULT_FIELD_NUMBER: _ClassVar[int]
    DEFAULT_FACTORY_FIELD_NUMBER: _ClassVar[int]
    COLLECTION_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    STRING_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    NUMBER_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    NODE_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    cardinality: TypeCardinality
    scalar_type: ScalarType
    primitive_type: PrimitiveType
    enum_type: EnumType
    node_type: NodeType
    struct_type: StructType
    base_type_ptr: NodeReferenceData
    key_type: TypeData
    is_required: bool
    is_variable: bool
    is_external: bool
    default: ValueData
    default_factory: DefaultFactory
    collection_constraint: CollectionConstraintData
    string_constraint: StringConstraintData
    number_constraint: NumberConstraintData
    node_constraint: NodeConstraintData
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., cardinality: _Optional[_Union[TypeCardinality, str]] = ..., scalar_type: _Optional[_Union[ScalarType, str]] = ..., primitive_type: _Optional[_Union[PrimitiveType, str]] = ..., enum_type: _Optional[_Union[EnumType, str]] = ..., node_type: _Optional[_Union[NodeType, str]] = ..., struct_type: _Optional[_Union[StructType, str]] = ..., base_type_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., key_type: _Optional[_Union[TypeData, _Mapping]] = ..., is_required: bool = ..., is_variable: bool = ..., is_external: bool = ..., default: _Optional[_Union[ValueData, _Mapping]] = ..., default_factory: _Optional[_Union[DefaultFactory, str]] = ..., collection_constraint: _Optional[_Union[CollectionConstraintData, _Mapping]] = ..., string_constraint: _Optional[_Union[StringConstraintData, _Mapping]] = ..., number_constraint: _Optional[_Union[NumberConstraintData, _Mapping]] = ..., node_constraint: _Optional[_Union[NodeConstraintData, _Mapping]] = ...) -> None: ...

class UserData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "region", "name", "slug", "icon", "status", "last_logged_in_at", "is_staff", "bench_ptr", "handle_ptr", "cursor_ptr", "email", "password_salt", "password_hash")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    LAST_LOGGED_IN_AT_FIELD_NUMBER: _ClassVar[int]
    IS_STAFF_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    HANDLE_PTR_FIELD_NUMBER: _ClassVar[int]
    CURSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    EMAIL_FIELD_NUMBER: _ClassVar[int]
    PASSWORD_SALT_FIELD_NUMBER: _ClassVar[int]
    PASSWORD_HASH_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    region: Region
    name: str
    slug: str
    icon: IconData
    status: UserStatus
    last_logged_in_at: _timestamp_pb2.Timestamp
    is_staff: bool
    bench_ptr: NodeReferenceData
    handle_ptr: NodeReferenceData
    cursor_ptr: NodeReferenceData
    email: str
    password_salt: bytes
    password_hash: bytes
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., region: _Optional[_Union[Region, str]] = ..., name: _Optional[str] = ..., slug: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., status: _Optional[_Union[UserStatus, str]] = ..., last_logged_in_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., is_staff: bool = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., handle_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., cursor_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., email: _Optional[str] = ..., password_salt: _Optional[bytes] = ..., password_hash: _Optional[bytes] = ...) -> None: ...

class ValueData(_message.Message):
    __slots__ = ("metatype", "key", "value")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    key: str
    value: _struct_pb2.Value
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., key: _Optional[str] = ..., value: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ...) -> None: ...

class VariableData(_message.Message):
    __slots__ = ("metatype", "field_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    FIELD_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    field_ptr: NodeReferenceData
    node_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., field_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class Vector2Data(_message.Message):
    __slots__ = ("metatype", "x", "y")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    x: float
    y: float
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., x: _Optional[float] = ..., y: _Optional[float] = ...) -> None: ...

class Vector3Data(_message.Message):
    __slots__ = ("metatype", "x", "y", "z")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    Z_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    x: float
    y: float
    z: float
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., x: _Optional[float] = ..., y: _Optional[float] = ..., z: _Optional[float] = ...) -> None: ...

class Vector4Data(_message.Message):
    __slots__ = ("metatype", "x", "y", "z", "w")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    Z_FIELD_NUMBER: _ClassVar[int]
    W_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    x: float
    y: float
    z: float
    w: float
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., x: _Optional[float] = ..., y: _Optional[float] = ..., z: _Optional[float] = ..., w: _Optional[float] = ...) -> None: ...

class WizardViewData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "bench_ptr", "package_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "template_ptr", "mode", "order_key", "name", "block_ptr", "position", "width", "height", "min_width", "min_height", "max_width", "max_height")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    BENCH_PTR_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    BLOCK_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    bench_ptr: NodeReferenceData
    package_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    mode: NodeMode
    order_key: str
    name: str
    block_ptr: NodeReferenceData
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., bench_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., package_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., mode: _Optional[_Union[NodeMode, str]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., block_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ...) -> None: ...

class SomeNodeData(_message.Message):
    __slots__ = ("bench", "bench_membership", "bench_invite", "package", "package_membership", "package_invite", "handle", "user", "friendship", "friendship_invite", "organization", "organization_membership", "organization_invite", "client", "space", "scene", "route", "page", "block", "database", "machine", "service", "action", "flow", "flow_edge", "agent", "task", "cursor", "run", "span", "interruption", "schema", "field", "file", "link", "custom_node_definition", "custom_node_instance", "thread", "message", "frame_view", "label_view", "split_view", "text_view", "number_input_view", "slider_input_view", "thread_view", "wizard_view", "theme", "color_style", "font_style", "border_style", "shadow_style", "gradient_style", "transition_style", "effect_style")
    BENCH_FIELD_NUMBER: _ClassVar[int]
    BENCH_MEMBERSHIP_FIELD_NUMBER: _ClassVar[int]
    BENCH_INVITE_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_MEMBERSHIP_FIELD_NUMBER: _ClassVar[int]
    PACKAGE_INVITE_FIELD_NUMBER: _ClassVar[int]
    HANDLE_FIELD_NUMBER: _ClassVar[int]
    USER_FIELD_NUMBER: _ClassVar[int]
    FRIENDSHIP_FIELD_NUMBER: _ClassVar[int]
    FRIENDSHIP_INVITE_FIELD_NUMBER: _ClassVar[int]
    ORGANIZATION_FIELD_NUMBER: _ClassVar[int]
    ORGANIZATION_MEMBERSHIP_FIELD_NUMBER: _ClassVar[int]
    ORGANIZATION_INVITE_FIELD_NUMBER: _ClassVar[int]
    CLIENT_FIELD_NUMBER: _ClassVar[int]
    SPACE_FIELD_NUMBER: _ClassVar[int]
    SCENE_FIELD_NUMBER: _ClassVar[int]
    ROUTE_FIELD_NUMBER: _ClassVar[int]
    PAGE_FIELD_NUMBER: _ClassVar[int]
    BLOCK_FIELD_NUMBER: _ClassVar[int]
    DATABASE_FIELD_NUMBER: _ClassVar[int]
    MACHINE_FIELD_NUMBER: _ClassVar[int]
    SERVICE_FIELD_NUMBER: _ClassVar[int]
    ACTION_FIELD_NUMBER: _ClassVar[int]
    FLOW_FIELD_NUMBER: _ClassVar[int]
    FLOW_EDGE_FIELD_NUMBER: _ClassVar[int]
    AGENT_FIELD_NUMBER: _ClassVar[int]
    TASK_FIELD_NUMBER: _ClassVar[int]
    CURSOR_FIELD_NUMBER: _ClassVar[int]
    RUN_FIELD_NUMBER: _ClassVar[int]
    SPAN_FIELD_NUMBER: _ClassVar[int]
    INTERRUPTION_FIELD_NUMBER: _ClassVar[int]
    SCHEMA_FIELD_NUMBER: _ClassVar[int]
    FIELD_FIELD_NUMBER: _ClassVar[int]
    FILE_FIELD_NUMBER: _ClassVar[int]
    LINK_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_NODE_DEFINITION_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_NODE_INSTANCE_FIELD_NUMBER: _ClassVar[int]
    THREAD_FIELD_NUMBER: _ClassVar[int]
    MESSAGE_FIELD_NUMBER: _ClassVar[int]
    FRAME_VIEW_FIELD_NUMBER: _ClassVar[int]
    LABEL_VIEW_FIELD_NUMBER: _ClassVar[int]
    SPLIT_VIEW_FIELD_NUMBER: _ClassVar[int]
    TEXT_VIEW_FIELD_NUMBER: _ClassVar[int]
    NUMBER_INPUT_VIEW_FIELD_NUMBER: _ClassVar[int]
    SLIDER_INPUT_VIEW_FIELD_NUMBER: _ClassVar[int]
    THREAD_VIEW_FIELD_NUMBER: _ClassVar[int]
    WIZARD_VIEW_FIELD_NUMBER: _ClassVar[int]
    THEME_FIELD_NUMBER: _ClassVar[int]
    COLOR_STYLE_FIELD_NUMBER: _ClassVar[int]
    FONT_STYLE_FIELD_NUMBER: _ClassVar[int]
    BORDER_STYLE_FIELD_NUMBER: _ClassVar[int]
    SHADOW_STYLE_FIELD_NUMBER: _ClassVar[int]
    GRADIENT_STYLE_FIELD_NUMBER: _ClassVar[int]
    TRANSITION_STYLE_FIELD_NUMBER: _ClassVar[int]
    EFFECT_STYLE_FIELD_NUMBER: _ClassVar[int]
    bench: BenchData
    bench_membership: BenchMembershipData
    bench_invite: BenchInviteData
    package: PackageData
    package_membership: PackageMembershipData
    package_invite: PackageInviteData
    handle: HandleData
    user: UserData
    friendship: FriendshipData
    friendship_invite: FriendshipInviteData
    organization: OrganizationData
    organization_membership: OrganizationMembershipData
    organization_invite: OrganizationInviteData
    client: ClientData
    space: SpaceData
    scene: SceneData
    route: RouteData
    page: PageData
    block: BlockData
    database: DatabaseData
    machine: MachineData
    service: ServiceData
    action: ActionData
    flow: FlowData
    flow_edge: FlowEdgeData
    agent: AgentData
    task: TaskData
    cursor: CursorData
    run: RunData
    span: SpanData
    interruption: InterruptionData
    schema: SchemaData
    field: FieldData
    file: FileData
    link: LinkData
    custom_node_definition: CustomNodeDefinitionData
    custom_node_instance: CustomNodeInstanceData
    thread: ThreadData
    message: MessageData
    frame_view: FrameViewData
    label_view: LabelViewData
    split_view: SplitViewData
    text_view: TextViewData
    number_input_view: NumberInputViewData
    slider_input_view: SliderInputViewData
    thread_view: ThreadViewData
    wizard_view: WizardViewData
    theme: ThemeData
    color_style: ColorStyleData
    font_style: FontStyleData
    border_style: BorderStyleData
    shadow_style: ShadowStyleData
    gradient_style: GradientStyleData
    transition_style: TransitionStyleData
    effect_style: EffectStyleData
    def __init__(self, bench: _Optional[_Union[BenchData, _Mapping]] = ..., bench_membership: _Optional[_Union[BenchMembershipData, _Mapping]] = ..., bench_invite: _Optional[_Union[BenchInviteData, _Mapping]] = ..., package: _Optional[_Union[PackageData, _Mapping]] = ..., package_membership: _Optional[_Union[PackageMembershipData, _Mapping]] = ..., package_invite: _Optional[_Union[PackageInviteData, _Mapping]] = ..., handle: _Optional[_Union[HandleData, _Mapping]] = ..., user: _Optional[_Union[UserData, _Mapping]] = ..., friendship: _Optional[_Union[FriendshipData, _Mapping]] = ..., friendship_invite: _Optional[_Union[FriendshipInviteData, _Mapping]] = ..., organization: _Optional[_Union[OrganizationData, _Mapping]] = ..., organization_membership: _Optional[_Union[OrganizationMembershipData, _Mapping]] = ..., organization_invite: _Optional[_Union[OrganizationInviteData, _Mapping]] = ..., client: _Optional[_Union[ClientData, _Mapping]] = ..., space: _Optional[_Union[SpaceData, _Mapping]] = ..., scene: _Optional[_Union[SceneData, _Mapping]] = ..., route: _Optional[_Union[RouteData, _Mapping]] = ..., page: _Optional[_Union[PageData, _Mapping]] = ..., block: _Optional[_Union[BlockData, _Mapping]] = ..., database: _Optional[_Union[DatabaseData, _Mapping]] = ..., machine: _Optional[_Union[MachineData, _Mapping]] = ..., service: _Optional[_Union[ServiceData, _Mapping]] = ..., action: _Optional[_Union[ActionData, _Mapping]] = ..., flow: _Optional[_Union[FlowData, _Mapping]] = ..., flow_edge: _Optional[_Union[FlowEdgeData, _Mapping]] = ..., agent: _Optional[_Union[AgentData, _Mapping]] = ..., task: _Optional[_Union[TaskData, _Mapping]] = ..., cursor: _Optional[_Union[CursorData, _Mapping]] = ..., run: _Optional[_Union[RunData, _Mapping]] = ..., span: _Optional[_Union[SpanData, _Mapping]] = ..., interruption: _Optional[_Union[InterruptionData, _Mapping]] = ..., schema: _Optional[_Union[SchemaData, _Mapping]] = ..., field: _Optional[_Union[FieldData, _Mapping]] = ..., file: _Optional[_Union[FileData, _Mapping]] = ..., link: _Optional[_Union[LinkData, _Mapping]] = ..., custom_node_definition: _Optional[_Union[CustomNodeDefinitionData, _Mapping]] = ..., custom_node_instance: _Optional[_Union[CustomNodeInstanceData, _Mapping]] = ..., thread: _Optional[_Union[ThreadData, _Mapping]] = ..., message: _Optional[_Union[MessageData, _Mapping]] = ..., frame_view: _Optional[_Union[FrameViewData, _Mapping]] = ..., label_view: _Optional[_Union[LabelViewData, _Mapping]] = ..., split_view: _Optional[_Union[SplitViewData, _Mapping]] = ..., text_view: _Optional[_Union[TextViewData, _Mapping]] = ..., number_input_view: _Optional[_Union[NumberInputViewData, _Mapping]] = ..., slider_input_view: _Optional[_Union[SliderInputViewData, _Mapping]] = ..., thread_view: _Optional[_Union[ThreadViewData, _Mapping]] = ..., wizard_view: _Optional[_Union[WizardViewData, _Mapping]] = ..., theme: _Optional[_Union[ThemeData, _Mapping]] = ..., color_style: _Optional[_Union[ColorStyleData, _Mapping]] = ..., font_style: _Optional[_Union[FontStyleData, _Mapping]] = ..., border_style: _Optional[_Union[BorderStyleData, _Mapping]] = ..., shadow_style: _Optional[_Union[ShadowStyleData, _Mapping]] = ..., gradient_style: _Optional[_Union[GradientStyleData, _Mapping]] = ..., transition_style: _Optional[_Union[TransitionStyleData, _Mapping]] = ..., effect_style: _Optional[_Union[EffectStyleData, _Mapping]] = ...) -> None: ...
