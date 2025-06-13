
# type: ignore
# ruff: noqa

from typing import TYPE_CHECKING, Union, AsyncIterator, Mapping

if TYPE_CHECKING:
    from destack.language import Session, Session, IsSubject, Client



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

class Align(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ALIGN_UNSPECIFIED: _ClassVar[Align]
    ALIGN_START: _ClassVar[Align]
    ALIGN_CENTER: _ClassVar[Align]
    ALIGN_END: _ClassVar[Align]

class ArrowHeadType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ARROW_HEAD_TYPE_UNSPECIFIED: _ClassVar[ArrowHeadType]
    ARROW_HEAD_TYPE_ARROW: _ClassVar[ArrowHeadType]
    ARROW_HEAD_TYPE_TRIANGLE: _ClassVar[ArrowHeadType]
    ARROW_HEAD_TYPE_DOT: _ClassVar[ArrowHeadType]

class AttributeType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ATTRIBUTE_TYPE_UNSPECIFIED: _ClassVar[AttributeType]
    ATTRIBUTE_TYPE_PROPERTY: _ClassVar[AttributeType]
    ATTRIBUTE_TYPE_FIELD: _ClassVar[AttributeType]

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

class CanvasType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CANVAS_TYPE_UNSPECIFIED: _ClassVar[CanvasType]
    CANVAS_TYPE_SHAPE: _ClassVar[CanvasType]

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
    CLOUD_PRIVATE: _ClassVar[Cloud]
    CLOUD_AWS: _ClassVar[Cloud]
    CLOUD_AZURE: _ClassVar[Cloud]
    CLOUD_GCP: _ClassVar[Cloud]
    CLOUD_HETZNER: _ClassVar[Cloud]

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
    CONDITIONAL_TYPE_IN: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_NOT_IN: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_EXISTS: _ClassVar[ConditionalType]
    CONDITIONAL_TYPE_NOT_EXISTS: _ClassVar[ConditionalType]

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

class EdgeDirection(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    EDGE_DIRECTION_UNSPECIFIED: _ClassVar[EdgeDirection]
    EDGE_DIRECTION_PARENT: _ClassVar[EdgeDirection]
    EDGE_DIRECTION_CHILD: _ClassVar[EdgeDirection]
    EDGE_DIRECTION_SIDE: _ClassVar[EdgeDirection]

class EdgeType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    EDGE_TYPE_UNSPECIFIED: _ClassVar[EdgeType]
    EDGE_TYPE_PARENT: _ClassVar[EdgeType]
    EDGE_TYPE_ANCESTOR: _ClassVar[EdgeType]
    EDGE_TYPE_REGULAR: _ClassVar[EdgeType]
    EDGE_TYPE_TEMPLATE: _ClassVar[EdgeType]

class EditOperation(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    EDIT_OPERATION_UNSPECIFIED: _ClassVar[EditOperation]
    EDIT_OPERATION_SET: _ClassVar[EditOperation]
    EDIT_OPERATION_CLEAR: _ClassVar[EditOperation]

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
    ENUM_TYPE_SPACE_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_USER_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_FRIENDSHIP_INVITE_EVENT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_ORGANIZATION_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_CLIENT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_CONDITIONAL_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_AGGREGATION_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_SORT_MODE: _ClassVar[EnumType]
    ENUM_TYPE_SORT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_JOIN_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_FUNCTION_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_EXPRESSION_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_QUERY_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_QUERY_UPDATE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_MEMBERSHIP_EVENT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_MEMBERSHIP_PERMISSION: _ClassVar[EnumType]
    ENUM_TYPE_INVITE_EVENT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_ROLE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_ROLE_EVENT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_FOLDER_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_TEXT_LINE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_TEXT_SPAN_TYPE: _ClassVar[EnumType]
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
    ENUM_TYPE_EDGE_DIRECTION: _ClassVar[EnumType]
    ENUM_TYPE_CASCADE_ACTION: _ClassVar[EnumType]
    ENUM_TYPE_DAY: _ClassVar[EnumType]
    ENUM_TYPE_MONTH: _ClassVar[EnumType]
    ENUM_TYPE_TIME_INTERVAL: _ClassVar[EnumType]
    ENUM_TYPE_RESOURCE_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_ACTION_CARDINALITY: _ClassVar[EnumType]
    ENUM_TYPE_CURSOR_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_SCHEDULE_FREQUENCY: _ClassVar[EnumType]
    ENUM_TYPE_TIMER_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_TIMER_EVENT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_TRIGGER_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_TRIGGER_EVENT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_RUN_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_RUN_EVENT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_RUN_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_INTERRUPTION_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_INTERRUPTION_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_INTERRUPTION_RESPONSE: _ClassVar[EnumType]
    ENUM_TYPE_ENVIRONMENT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_THREAD_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_NOTIFICATION_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_NOTIFICATION_EVENT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_CLOUD: _ClassVar[EnumType]
    ENUM_TYPE_REGION: _ClassVar[EnumType]
    ENUM_TYPE_REGION_AREA: _ClassVar[EnumType]
    ENUM_TYPE_REGION_CONTINENT: _ClassVar[EnumType]
    ENUM_TYPE_TENANCY: _ClassVar[EnumType]
    ENUM_TYPE_DATABASE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_MACHINE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_MODEL_DEVELOPER: _ClassVar[EnumType]
    ENUM_TYPE_MODEL_PROVIDER: _ClassVar[EnumType]
    ENUM_TYPE_WINDOW_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_SCENE_EVENT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_LAYER_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_CANVAS_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_PLANE_SHAPE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_ARROW_HEAD_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_LINE_TYPE: _ClassVar[EnumType]
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
    ENUM_TYPE_ENUM_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_NODE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_STRUCT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_TRAIT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_RELATION_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_ATTRIBUTE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_PROPERTY_REFERENCE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_STORE_ZONE: _ClassVar[EnumType]
    ENUM_TYPE_STORE_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_STORE_IMPLEMENTATION: _ClassVar[EnumType]
    ENUM_TYPE_PLATFORM_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_RUNTIME_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_OPERATING_SYSTEM: _ClassVar[EnumType]
    ENUM_TYPE_ERROR_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_EDIT_TYPE: _ClassVar[EnumType]
    ENUM_TYPE_EDIT_OPERATION: _ClassVar[EnumType]
    ENUM_TYPE_CHANGE_STATUS: _ClassVar[EnumType]
    ENUM_TYPE_NODE_PERMISSION: _ClassVar[EnumType]

class EnvironmentType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ENVIRONMENT_TYPE_UNSPECIFIED: _ClassVar[EnvironmentType]
    ENVIRONMENT_TYPE_SYSTEM: _ClassVar[EnvironmentType]
    ENVIRONMENT_TYPE_DEVELOPMENT: _ClassVar[EnvironmentType]
    ENVIRONMENT_TYPE_TEST: _ClassVar[EnvironmentType]
    ENVIRONMENT_TYPE_STAGING: _ClassVar[EnvironmentType]
    ENVIRONMENT_TYPE_PRODUCTION: _ClassVar[EnvironmentType]

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
    FIELD_TYPE_MEMBER: _ClassVar[FieldType]
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
    FILE_SOURCE_SPACE: _ClassVar[FileSource]
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
    FILL_TYPE_STYLE: _ClassVar[FillType]
    FILL_TYPE_SOLID: _ClassVar[FillType]
    FILL_TYPE_GRADIENT: _ClassVar[FillType]
    FILL_TYPE_IMAGE: _ClassVar[FillType]

class FolderType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FOLDER_TYPE_UNSPECIFIED: _ClassVar[FolderType]
    FOLDER_TYPE_SYSTEM: _ClassVar[FolderType]
    FOLDER_TYPE_HOME: _ClassVar[FolderType]
    FOLDER_TYPE_GENERIC: _ClassVar[FolderType]
    FOLDER_TYPE_MODULE: _ClassVar[FolderType]
    FOLDER_TYPE_APP: _ClassVar[FolderType]

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

class FriendshipInviteEventType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FRIENDSHIP_INVITE_EVENT_TYPE_UNSPECIFIED: _ClassVar[FriendshipInviteEventType]
    FRIENDSHIP_INVITE_EVENT_TYPE_SENT: _ClassVar[FriendshipInviteEventType]
    FRIENDSHIP_INVITE_EVENT_TYPE_RESCINDED: _ClassVar[FriendshipInviteEventType]
    FRIENDSHIP_INVITE_EVENT_TYPE_ACCEPTED: _ClassVar[FriendshipInviteEventType]
    FRIENDSHIP_INVITE_EVENT_TYPE_REJECTED: _ClassVar[FriendshipInviteEventType]

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

class InviteEventType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    INVITE_EVENT_TYPE_UNSPECIFIED: _ClassVar[InviteEventType]
    INVITE_EVENT_TYPE_SENT: _ClassVar[InviteEventType]
    INVITE_EVENT_TYPE_RESCINDED: _ClassVar[InviteEventType]
    INVITE_EVENT_TYPE_ACCEPTED: _ClassVar[InviteEventType]
    INVITE_EVENT_TYPE_REJECTED: _ClassVar[InviteEventType]

class JoinType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    JOIN_TYPE_UNSPECIFIED: _ClassVar[JoinType]
    JOIN_TYPE_LEFT: _ClassVar[JoinType]
    JOIN_TYPE_PARENT: _ClassVar[JoinType]
    JOIN_TYPE_CHILD: _ClassVar[JoinType]

class LayerType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    LAYER_TYPE_UNSPECIFIED: _ClassVar[LayerType]
    LAYER_TYPE_GENERAL: _ClassVar[LayerType]
    LAYER_TYPE_SHAPE: _ClassVar[LayerType]

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

class LineType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    LINE_TYPE_UNSPECIFIED: _ClassVar[LineType]
    LINE_TYPE_SOLID: _ClassVar[LineType]
    LINE_TYPE_DASHED: _ClassVar[LineType]
    LINE_TYPE_DOTTED: _ClassVar[LineType]

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

class MembershipEventType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MEMBERSHIP_EVENT_TYPE_UNSPECIFIED: _ClassVar[MembershipEventType]
    MEMBERSHIP_EVENT_TYPE_JOIN: _ClassVar[MembershipEventType]
    MEMBERSHIP_EVENT_TYPE_LEAVE: _ClassVar[MembershipEventType]

class MembershipPermission(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MEMBERSHIP_PERMISSION_UNSPECIFIED: _ClassVar[MembershipPermission]
    MEMBERSHIP_PERMISSION_KICK: _ClassVar[MembershipPermission]
    MEMBERSHIP_PERMISSION_BAN: _ClassVar[MembershipPermission]

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

class NodePermission(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    NODE_PERMISSION_UNSPECIFIED: _ClassVar[NodePermission]
    NODE_PERMISSION_READ: _ClassVar[NodePermission]
    NODE_PERMISSION_ADD: _ClassVar[NodePermission]
    NODE_PERMISSION_UPDATE: _ClassVar[NodePermission]
    NODE_PERMISSION_REMOVE: _ClassVar[NodePermission]

class NodeType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    NODE_TYPE_UNSPECIFIED: _ClassVar[NodeType]
    NODE_TYPE_SPACE: _ClassVar[NodeType]
    NODE_TYPE_HANDLE: _ClassVar[NodeType]
    NODE_TYPE_USER: _ClassVar[NodeType]
    NODE_TYPE_FRIENDSHIP: _ClassVar[NodeType]
    NODE_TYPE_FRIENDSHIP_INVITE: _ClassVar[NodeType]
    NODE_TYPE_FRIENDSHIP_INVITE_EVENT: _ClassVar[NodeType]
    NODE_TYPE_ORGANIZATION: _ClassVar[NodeType]
    NODE_TYPE_TEAM: _ClassVar[NodeType]
    NODE_TYPE_CLIENT: _ClassVar[NodeType]
    NODE_TYPE_MEMBERSHIP: _ClassVar[NodeType]
    NODE_TYPE_MEMBERSHIP_EVENT: _ClassVar[NodeType]
    NODE_TYPE_INVITE: _ClassVar[NodeType]
    NODE_TYPE_INVITE_EVENT: _ClassVar[NodeType]
    NODE_TYPE_ROLE: _ClassVar[NodeType]
    NODE_TYPE_ROLE_EVENT: _ClassVar[NodeType]
    NODE_TYPE_AGENT: _ClassVar[NodeType]
    NODE_TYPE_FOLDER: _ClassVar[NodeType]
    NODE_TYPE_TAG: _ClassVar[NodeType]
    NODE_TYPE_TAGGING: _ClassVar[NodeType]
    NODE_TYPE_CUSTOM_ENTITY_DEFINITION: _ClassVar[NodeType]
    NODE_TYPE_CUSTOM_ENTITY: _ClassVar[NodeType]
    NODE_TYPE_CUSTOM_STRUCT_DEFINITION: _ClassVar[NodeType]
    NODE_TYPE_CUSTOM_ENUM_DEFINITION: _ClassVar[NodeType]
    NODE_TYPE_FIELD: _ClassVar[NodeType]
    NODE_TYPE_OPTION: _ClassVar[NodeType]
    NODE_TYPE_FILE: _ClassVar[NodeType]
    NODE_TYPE_LINK: _ClassVar[NodeType]
    NODE_TYPE_SCRIPT: _ClassVar[NodeType]
    NODE_TYPE_SERVICE: _ClassVar[NodeType]
    NODE_TYPE_ACTION: _ClassVar[NodeType]
    NODE_TYPE_ROUTE: _ClassVar[NodeType]
    NODE_TYPE_TRIGGER: _ClassVar[NodeType]
    NODE_TYPE_TRIGGER_EVENT: _ClassVar[NodeType]
    NODE_TYPE_TIMER: _ClassVar[NodeType]
    NODE_TYPE_TIMER_EVENT: _ClassVar[NodeType]
    NODE_TYPE_EVENT_CURSOR: _ClassVar[NodeType]
    NODE_TYPE_SCREEN_CURSOR: _ClassVar[NodeType]
    NODE_TYPE_THREAD_CURSOR: _ClassVar[NodeType]
    NODE_TYPE_RUN: _ClassVar[NodeType]
    NODE_TYPE_RUN_EVENT: _ClassVar[NodeType]
    NODE_TYPE_SPAN: _ClassVar[NodeType]
    NODE_TYPE_INTERRUPTION: _ClassVar[NodeType]
    NODE_TYPE_LOG: _ClassVar[NodeType]
    NODE_TYPE_GAUGE_METRIC: _ClassVar[NodeType]
    NODE_TYPE_GAUGE_MEASUREMENT: _ClassVar[NodeType]
    NODE_TYPE_COUNTER_METRIC: _ClassVar[NodeType]
    NODE_TYPE_COUNTER_MEASUREMENT: _ClassVar[NodeType]
    NODE_TYPE_HISTOGRAM_METRIC: _ClassVar[NodeType]
    NODE_TYPE_HISTOGRAM_MEASUREMENT: _ClassVar[NodeType]
    NODE_TYPE_CUSTOM_EVENT_DEFINITION: _ClassVar[NodeType]
    NODE_TYPE_CUSTOM_EVENT: _ClassVar[NodeType]
    NODE_TYPE_EDIT_EVENT: _ClassVar[NodeType]
    NODE_TYPE_ENVIRONMENT: _ClassVar[NodeType]
    NODE_TYPE_THREAD: _ClassVar[NodeType]
    NODE_TYPE_MESSAGE: _ClassVar[NodeType]
    NODE_TYPE_REACTION: _ClassVar[NodeType]
    NODE_TYPE_STAR: _ClassVar[NodeType]
    NODE_TYPE_FOLLOW: _ClassVar[NodeType]
    NODE_TYPE_NOTIFICATION: _ClassVar[NodeType]
    NODE_TYPE_NOTIFICATION_EVENT: _ClassVar[NodeType]
    NODE_TYPE_DATABASE: _ClassVar[NodeType]
    NODE_TYPE_MACHINE: _ClassVar[NodeType]
    NODE_TYPE_WINDOW: _ClassVar[NodeType]
    NODE_TYPE_SCENE: _ClassVar[NodeType]
    NODE_TYPE_SCENE_EVENT: _ClassVar[NodeType]
    NODE_TYPE_LAYER: _ClassVar[NodeType]
    NODE_TYPE_CUSTOM_VIEW_DEFINITION: _ClassVar[NodeType]
    NODE_TYPE_CUSTOM_VIEW: _ClassVar[NodeType]
    NODE_TYPE_FRAME_VIEW: _ClassVar[NodeType]
    NODE_TYPE_LABEL_VIEW: _ClassVar[NodeType]
    NODE_TYPE_SPLIT_VIEW: _ClassVar[NodeType]
    NODE_TYPE_TEXT_VIEW: _ClassVar[NodeType]
    NODE_TYPE_NUMBER_INPUT_VIEW: _ClassVar[NodeType]
    NODE_TYPE_SLIDER_INPUT_VIEW: _ClassVar[NodeType]
    NODE_TYPE_THREAD_VIEW: _ClassVar[NodeType]
    NODE_TYPE_WIZARD_VIEW: _ClassVar[NodeType]
    NODE_TYPE_CANVAS: _ClassVar[NodeType]
    NODE_TYPE_LINE_SHAPE: _ClassVar[NodeType]
    NODE_TYPE_PLANE_SHAPE: _ClassVar[NodeType]
    NODE_TYPE_ARROW_SHAPE: _ClassVar[NodeType]
    NODE_TYPE_ANNOTATION_SHAPE: _ClassVar[NodeType]
    NODE_TYPE_THEME: _ClassVar[NodeType]
    NODE_TYPE_COLOR_STYLE: _ClassVar[NodeType]
    NODE_TYPE_FILL_STYLE: _ClassVar[NodeType]
    NODE_TYPE_FONT_STYLE: _ClassVar[NodeType]
    NODE_TYPE_BORDER_STYLE: _ClassVar[NodeType]
    NODE_TYPE_SHADOW_STYLE: _ClassVar[NodeType]
    NODE_TYPE_GRADIENT_STYLE: _ClassVar[NodeType]
    NODE_TYPE_TRANSITION_STYLE: _ClassVar[NodeType]
    NODE_TYPE_EFFECT_STYLE: _ClassVar[NodeType]

class NotificationEventType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    NOTIFICATION_EVENT_TYPE_UNSPECIFIED: _ClassVar[NotificationEventType]
    NOTIFICATION_EVENT_TYPE_SENT: _ClassVar[NotificationEventType]
    NOTIFICATION_EVENT_TYPE_RESCINDED: _ClassVar[NotificationEventType]
    NOTIFICATION_EVENT_TYPE_READ: _ClassVar[NotificationEventType]
    NOTIFICATION_EVENT_TYPE_DISMISSED: _ClassVar[NotificationEventType]
    NOTIFICATION_EVENT_TYPE_EXPIRED: _ClassVar[NotificationEventType]

class NotificationStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    NOTIFICATION_STATUS_UNSPECIFIED: _ClassVar[NotificationStatus]
    NOTIFICATION_STATUS_UNREAD: _ClassVar[NotificationStatus]
    NOTIFICATION_STATUS_READ: _ClassVar[NotificationStatus]
    NOTIFICATION_STATUS_DISMISSED: _ClassVar[NotificationStatus]
    NOTIFICATION_STATUS_EXPIRED: _ClassVar[NotificationStatus]
    NOTIFICATION_STATUS_RESCINDED: _ClassVar[NotificationStatus]

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

class OperatingSystem(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    OPERATING_SYSTEM_UNSPECIFIED: _ClassVar[OperatingSystem]
    OPERATING_SYSTEM_LINUX: _ClassVar[OperatingSystem]
    OPERATING_SYSTEM_WINDOWS: _ClassVar[OperatingSystem]
    OPERATING_SYSTEM_MACOS: _ClassVar[OperatingSystem]
    OPERATING_SYSTEM_ANDROID: _ClassVar[OperatingSystem]
    OPERATING_SYSTEM_IOS: _ClassVar[OperatingSystem]

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

class PlaneShapeType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PLANE_SHAPE_TYPE_UNSPECIFIED: _ClassVar[PlaneShapeType]
    PLANE_SHAPE_TYPE_RECTANGLE: _ClassVar[PlaneShapeType]
    PLANE_SHAPE_TYPE_TRIANGLE: _ClassVar[PlaneShapeType]
    PLANE_SHAPE_TYPE_CIRCLE: _ClassVar[PlaneShapeType]
    PLANE_SHAPE_TYPE_ELLIPSE: _ClassVar[PlaneShapeType]
    PLANE_SHAPE_TYPE_POLYGON: _ClassVar[PlaneShapeType]

class PlatformType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PLATFORM_TYPE_UNSPECIFIED: _ClassVar[PlatformType]
    PLATFORM_TYPE_SERVER: _ClassVar[PlatformType]
    PLATFORM_TYPE_WEB: _ClassVar[PlatformType]

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
    PRIMITIVE_TYPE_DATETIME: _ClassVar[PrimitiveType]
    PRIMITIVE_TYPE_DATE: _ClassVar[PrimitiveType]
    PRIMITIVE_TYPE_TIME: _ClassVar[PrimitiveType]
    PRIMITIVE_TYPE_DURATION: _ClassVar[PrimitiveType]

class PropertyReferenceType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PROPERTY_REFERENCE_TYPE_UNSPECIFIED: _ClassVar[PropertyReferenceType]
    PROPERTY_REFERENCE_TYPE_NODE: _ClassVar[PropertyReferenceType]
    PROPERTY_REFERENCE_TYPE_TRAIT: _ClassVar[PropertyReferenceType]
    PROPERTY_REFERENCE_TYPE_STRUCT: _ClassVar[PropertyReferenceType]

class QueryType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    QUERY_TYPE_UNSPECIFIED: _ClassVar[QueryType]
    QUERY_TYPE_NODE: _ClassVar[QueryType]
    QUERY_TYPE_SCALAR: _ClassVar[QueryType]
    QUERY_TYPE_GROUPED_NODE: _ClassVar[QueryType]
    QUERY_TYPE_GROUPED_SCALAR: _ClassVar[QueryType]

class QueryUpdateType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    QUERY_UPDATE_TYPE_UNSPECIFIED: _ClassVar[QueryUpdateType]
    QUERY_UPDATE_TYPE_FULL_RESULT: _ClassVar[QueryUpdateType]
    QUERY_UPDATE_TYPE_PARTIAL_RESULT: _ClassVar[QueryUpdateType]

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

class RelationType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RELATION_TYPE_UNSPECIFIED: _ClassVar[RelationType]
    RELATION_TYPE_BUILTIN_NODE: _ClassVar[RelationType]
    RELATION_TYPE_CUSTOM_NODE: _ClassVar[RelationType]
    RELATION_TYPE_TRAIT: _ClassVar[RelationType]

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

class RoleEventType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ROLE_EVENT_TYPE_UNSPECIFIED: _ClassVar[RoleEventType]
    ROLE_EVENT_TYPE_ASSIGNED: _ClassVar[RoleEventType]
    ROLE_EVENT_TYPE_REMOVED: _ClassVar[RoleEventType]

class RoleType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ROLE_TYPE_UNSPECIFIED: _ClassVar[RoleType]
    ROLE_TYPE_SYSTEM: _ClassVar[RoleType]
    ROLE_TYPE_OWNER: _ClassVar[RoleType]
    ROLE_TYPE_ADMIN: _ClassVar[RoleType]
    ROLE_TYPE_DEVELOPER: _ClassVar[RoleType]
    ROLE_TYPE_USER: _ClassVar[RoleType]
    ROLE_TYPE_SPECTATOR: _ClassVar[RoleType]

class RunEventType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RUN_EVENT_TYPE_UNSPECIFIED: _ClassVar[RunEventType]
    RUN_EVENT_TYPE_SCHEDULED: _ClassVar[RunEventType]
    RUN_EVENT_TYPE_RUNNING: _ClassVar[RunEventType]
    RUN_EVENT_TYPE_REQUESTED_PAUSE: _ClassVar[RunEventType]
    RUN_EVENT_TYPE_PAUSED: _ClassVar[RunEventType]
    RUN_EVENT_TYPE_REQUESTED_RESUME: _ClassVar[RunEventType]
    RUN_EVENT_TYPE_RESUMED: _ClassVar[RunEventType]
    RUN_EVENT_TYPE_REQUESTED_CANCEL: _ClassVar[RunEventType]
    RUN_EVENT_TYPE_CANCELLED: _ClassVar[RunEventType]
    RUN_EVENT_TYPE_ABORTED: _ClassVar[RunEventType]
    RUN_EVENT_TYPE_FAILED: _ClassVar[RunEventType]
    RUN_EVENT_TYPE_COMPLETED: _ClassVar[RunEventType]

class RunStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RUN_STATUS_UNSPECIFIED: _ClassVar[RunStatus]
    RUN_STATUS_SCHEDULED: _ClassVar[RunStatus]
    RUN_STATUS_RUNNING: _ClassVar[RunStatus]
    RUN_STATUS_PAUSED: _ClassVar[RunStatus]
    RUN_STATUS_YIELDED: _ClassVar[RunStatus]
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
    RUN_TYPE_AGENT: _ClassVar[RunType]

class RuntimeType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RUNTIME_TYPE_UNSPECIFIED: _ClassVar[RuntimeType]
    RUNTIME_TYPE_PYTHON: _ClassVar[RuntimeType]
    RUNTIME_TYPE_JAVASCRIPT: _ClassVar[RuntimeType]

class ScalarType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SCALAR_TYPE_UNSPECIFIED: _ClassVar[ScalarType]
    SCALAR_TYPE_PRIMITIVE: _ClassVar[ScalarType]
    SCALAR_TYPE_ENUM: _ClassVar[ScalarType]
    SCALAR_TYPE_NODE_REFERENCE: _ClassVar[ScalarType]
    SCALAR_TYPE_NODE_VALUE: _ClassVar[ScalarType]
    SCALAR_TYPE_STRUCT: _ClassVar[ScalarType]

class SceneEventType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SCENE_EVENT_TYPE_UNSPECIFIED: _ClassVar[SceneEventType]
    SCENE_EVENT_TYPE_ENTERED: _ClassVar[SceneEventType]
    SCENE_EVENT_TYPE_EXITED: _ClassVar[SceneEventType]

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

class SpaceStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SPACE_STATUS_UNSPECIFIED: _ClassVar[SpaceStatus]
    SPACE_STATUS_CREATING: _ClassVar[SpaceStatus]
    SPACE_STATUS_QUEUED: _ClassVar[SpaceStatus]
    SPACE_STATUS_RUNNING: _ClassVar[SpaceStatus]
    SPACE_STATUS_PAUSED: _ClassVar[SpaceStatus]

class SpringType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SPRING_TYPE_UNSPECIFIED: _ClassVar[SpringType]
    SPRING_TYPE_TIME: _ClassVar[SpringType]
    SPRING_TYPE_PHYSICS: _ClassVar[SpringType]

class StoreImplementation(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    STORE_IMPLEMENTATION_UNSPECIFIED: _ClassVar[StoreImplementation]
    STORE_IMPLEMENTATION_POSTGRES: _ClassVar[StoreImplementation]
    STORE_IMPLEMENTATION_MEMORY: _ClassVar[StoreImplementation]

class StoreType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    STORE_TYPE_UNSPECIFIED: _ClassVar[StoreType]
    STORE_TYPE_GLOBAL_ENTITY: _ClassVar[StoreType]
    STORE_TYPE_SPATIAL_ENTITY: _ClassVar[StoreType]
    STORE_TYPE_LOCAL_MEMORY: _ClassVar[StoreType]

class StoreZone(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    STORE_ZONE_UNSPECIFIED: _ClassVar[StoreZone]
    STORE_ZONE_GLOBAL: _ClassVar[StoreZone]
    STORE_ZONE_SPATIAL: _ClassVar[StoreZone]
    STORE_ZONE_LOCAL: _ClassVar[StoreZone]

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
    STRUCT_TYPE_PROPERTY_DEFINITION: _ClassVar[StructType]
    STRUCT_TYPE_TRAIT_DEFINITION: _ClassVar[StructType]
    STRUCT_TYPE_NODE_DEFINITION: _ClassVar[StructType]
    STRUCT_TYPE_STRUCT_DEFINITION: _ClassVar[StructType]
    STRUCT_TYPE_ENUM_DEFINITION: _ClassVar[StructType]
    STRUCT_TYPE_ENUM_OPTION_DEFINITION: _ClassVar[StructType]
    STRUCT_TYPE_PERMISSION_DEFINITION: _ClassVar[StructType]
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
    STRUCT_TYPE_QUERY_RESULT_GROUP: _ClassVar[StructType]
    STRUCT_TYPE_QUERY_UPDATE: _ClassVar[StructType]
    STRUCT_TYPE_HISTOGRAM: _ClassVar[StructType]
    STRUCT_TYPE_VALUE: _ClassVar[StructType]
    STRUCT_TYPE_TYPE: _ClassVar[StructType]
    STRUCT_TYPE_NUMBER_CONSTRAINT: _ClassVar[StructType]
    STRUCT_TYPE_STRING_CONSTRAINT: _ClassVar[StructType]
    STRUCT_TYPE_COLLECTION_CONSTRAINT: _ClassVar[StructType]
    STRUCT_TYPE_NODE_CONSTRAINT: _ClassVar[StructType]
    STRUCT_TYPE_TEXT: _ClassVar[StructType]
    STRUCT_TYPE_TEXT_LINE: _ClassVar[StructType]
    STRUCT_TYPE_TEXT_SPAN: _ClassVar[StructType]
    STRUCT_TYPE_ICON: _ClassVar[StructType]
    STRUCT_TYPE_SELECTION: _ClassVar[StructType]
    STRUCT_TYPE_SCHEDULE: _ClassVar[StructType]
    STRUCT_TYPE_ERROR: _ClassVar[StructType]
    STRUCT_TYPE_DATABASE_INFO: _ClassVar[StructType]
    STRUCT_TYPE_CELL_INFO: _ClassVar[StructType]
    STRUCT_TYPE_VECTOR2: _ClassVar[StructType]
    STRUCT_TYPE_VECTOR3: _ClassVar[StructType]
    STRUCT_TYPE_VECTOR4: _ClassVar[StructType]
    STRUCT_TYPE_VECTOR2I: _ClassVar[StructType]
    STRUCT_TYPE_VECTOR3I: _ClassVar[StructType]
    STRUCT_TYPE_VECTOR4I: _ClassVar[StructType]
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

class Tenancy(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TENANCY_UNSPECIFIED: _ClassVar[Tenancy]
    TENANCY_DEDICATED: _ClassVar[Tenancy]
    TENANCY_SHARED: _ClassVar[Tenancy]

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

class TimerEventType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TIMER_EVENT_TYPE_UNSPECIFIED: _ClassVar[TimerEventType]
    TIMER_EVENT_TYPE_STARTED: _ClassVar[TimerEventType]
    TIMER_EVENT_TYPE_STOPPED: _ClassVar[TimerEventType]
    TIMER_EVENT_TYPE_EXPIRED: _ClassVar[TimerEventType]

class TimerType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TIMER_TYPE_UNSPECIFIED: _ClassVar[TimerType]
    TIMER_TYPE_ONCE: _ClassVar[TimerType]
    TIMER_TYPE_RECURRING: _ClassVar[TimerType]

class TraitType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TRAIT_TYPE_UNSPECIFIED: _ClassVar[TraitType]
    TRAIT_TYPE_GLOBAL: _ClassVar[TraitType]
    TRAIT_TYPE_SPATIAL: _ClassVar[TraitType]
    TRAIT_TYPE_ENTITY: _ClassVar[TraitType]
    TRAIT_TYPE_PARTICLE: _ClassVar[TraitType]
    TRAIT_TYPE_ANALYTIC: _ClassVar[TraitType]
    TRAIT_TYPE_INDEXED: _ClassVar[TraitType]
    TRAIT_TYPE_RESOURCE: _ClassVar[TraitType]
    TRAIT_TYPE_EVENT: _ClassVar[TraitType]
    TRAIT_TYPE_CUSTOM_NODE_DEFINITION: _ClassVar[TraitType]
    TRAIT_TYPE_CUSTOM_NODE: _ClassVar[TraitType]
    TRAIT_TYPE_FROZEN: _ClassVar[TraitType]
    TRAIT_TYPE_TRACKED: _ClassVar[TraitType]
    TRAIT_TYPE_ARCHIVABLE: _ClassVar[TraitType]
    TRAIT_TYPE_DELETABLE: _ClassVar[TraitType]
    TRAIT_TYPE_TEMPLATABLE: _ClassVar[TraitType]
    TRAIT_TYPE_EXTENSIBLE: _ClassVar[TraitType]
    TRAIT_TYPE_ORDERED: _ClassVar[TraitType]
    TRAIT_TYPE_HAS_NAME: _ClassVar[TraitType]
    TRAIT_TYPE_HAS_SLUG: _ClassVar[TraitType]
    TRAIT_TYPE_HAS_ICON: _ClassVar[TraitType]
    TRAIT_TYPE_OWNABLE: _ClassVar[TraitType]
    TRAIT_TYPE_JOINABLE: _ClassVar[TraitType]
    TRAIT_TYPE_SUBJECT: _ClassVar[TraitType]
    TRAIT_TYPE_OWNER: _ClassVar[TraitType]
    TRAIT_TYPE_MEMBERSHIP: _ClassVar[TraitType]
    TRAIT_TYPE_INVITE: _ClassVar[TraitType]
    TRAIT_TYPE_TAGGABLE: _ClassVar[TraitType]
    TRAIT_TYPE_TAG: _ClassVar[TraitType]
    TRAIT_TYPE_ACTIONABLE: _ClassVar[TraitType]
    TRAIT_TYPE_RUNNABLE: _ClassVar[TraitType]
    TRAIT_TYPE_SCRIPTABLE: _ClassVar[TraitType]
    TRAIT_TYPE_SOURCEABLE: _ClassVar[TraitType]
    TRAIT_TYPE_CURSOR: _ClassVar[TraitType]
    TRAIT_TYPE_METRIC: _ClassVar[TraitType]
    TRAIT_TYPE_MEASUREMENT: _ClassVar[TraitType]
    TRAIT_TYPE_STARABLE: _ClassVar[TraitType]
    TRAIT_TYPE_REACTABLE: _ClassVar[TraitType]
    TRAIT_TYPE_FOLLOWABLE: _ClassVar[TraitType]
    TRAIT_TYPE_FOLLOW: _ClassVar[TraitType]
    TRAIT_TYPE_VISUAL: _ClassVar[TraitType]
    TRAIT_TYPE_VIEW: _ClassVar[TraitType]
    TRAIT_TYPE_CONTAINER_VIEW: _ClassVar[TraitType]
    TRAIT_TYPE_CONTENT_VIEW: _ClassVar[TraitType]
    TRAIT_TYPE_INPUT_VIEW: _ClassVar[TraitType]
    TRAIT_TYPE_NODE_VIEW: _ClassVar[TraitType]
    TRAIT_TYPE_INTERNAL_VIEW: _ClassVar[TraitType]
    TRAIT_TYPE_STYLE: _ClassVar[TraitType]
    TRAIT_TYPE_SHAPE: _ClassVar[TraitType]

class TransitionType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TRANSITION_TYPE_UNSPECIFIED: _ClassVar[TransitionType]
    TRANSITION_TYPE_STYLE: _ClassVar[TransitionType]
    TRANSITION_TYPE_FIELD: _ClassVar[TransitionType]
    TRANSITION_TYPE_TWEEN: _ClassVar[TransitionType]
    TRANSITION_TYPE_SPRING: _ClassVar[TransitionType]

class TriggerEventType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TRIGGER_EVENT_TYPE_UNSPECIFIED: _ClassVar[TriggerEventType]
    TRIGGER_EVENT_TYPE_STARTED: _ClassVar[TriggerEventType]
    TRIGGER_EVENT_TYPE_TRIGGERED: _ClassVar[TriggerEventType]
    TRIGGER_EVENT_TYPE_STOPPED: _ClassVar[TriggerEventType]

class TriggerType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TRIGGER_TYPE_UNSPECIFIED: _ClassVar[TriggerType]
    TRIGGER_TYPE_EVENT: _ClassVar[TriggerType]

class TypeCardinality(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TYPE_CARDINALITY_UNSPECIFIED: _ClassVar[TypeCardinality]
    TYPE_CARDINALITY_SCALAR: _ClassVar[TypeCardinality]
    TYPE_CARDINALITY_LIST: _ClassVar[TypeCardinality]
    TYPE_CARDINALITY_MAP: _ClassVar[TypeCardinality]

class UserStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    USER_STATUS_UNSPECIFIED: _ClassVar[UserStatus]
    USER_STATUS_CREATING: _ClassVar[UserStatus]
    USER_STATUS_ACTIVE: _ClassVar[UserStatus]

class WindowType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    WINDOW_TYPE_UNSPECIFIED: _ClassVar[WindowType]
    WINDOW_TYPE_BROWSER: _ClassVar[WindowType]
    WINDOW_TYPE_DESKTOP: _ClassVar[WindowType]
    WINDOW_TYPE_MOBILE: _ClassVar[WindowType]
ACTION_CARDINALITY_UNSPECIFIED: ActionCardinality
ACTION_CARDINALITY_UNARY: ActionCardinality
AGGREGATION_TYPE_UNSPECIFIED: AggregationType
AGGREGATION_TYPE_EXISTS: AggregationType
AGGREGATION_TYPE_COUNT: AggregationType
AGGREGATION_TYPE_SUM: AggregationType
AGGREGATION_TYPE_MIN: AggregationType
AGGREGATION_TYPE_MAX: AggregationType
AGGREGATION_TYPE_AVERAGE: AggregationType
ALIGN_UNSPECIFIED: Align
ALIGN_START: Align
ALIGN_CENTER: Align
ALIGN_END: Align
ARROW_HEAD_TYPE_UNSPECIFIED: ArrowHeadType
ARROW_HEAD_TYPE_ARROW: ArrowHeadType
ARROW_HEAD_TYPE_TRIANGLE: ArrowHeadType
ARROW_HEAD_TYPE_DOT: ArrowHeadType
ATTRIBUTE_TYPE_UNSPECIFIED: AttributeType
ATTRIBUTE_TYPE_PROPERTY: AttributeType
ATTRIBUTE_TYPE_FIELD: AttributeType
BORDER_TYPE_UNSPECIFIED: BorderType
BORDER_TYPE_NONE: BorderType
BORDER_TYPE_STYLE: BorderType
BORDER_TYPE_FIELD: BorderType
BORDER_TYPE_SOLID: BorderType
BORDER_TYPE_DASHED: BorderType
BORDER_TYPE_DOTTED: BorderType
BORDER_TYPE_DOUBLE: BorderType
CANVAS_TYPE_UNSPECIFIED: CanvasType
CANVAS_TYPE_SHAPE: CanvasType
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
CLOUD_PRIVATE: Cloud
CLOUD_AWS: Cloud
CLOUD_AZURE: Cloud
CLOUD_GCP: Cloud
CLOUD_HETZNER: Cloud
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
CONDITIONAL_TYPE_IN: ConditionalType
CONDITIONAL_TYPE_NOT_IN: ConditionalType
CONDITIONAL_TYPE_EXISTS: ConditionalType
CONDITIONAL_TYPE_NOT_EXISTS: ConditionalType
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
EDGE_DIRECTION_UNSPECIFIED: EdgeDirection
EDGE_DIRECTION_PARENT: EdgeDirection
EDGE_DIRECTION_CHILD: EdgeDirection
EDGE_DIRECTION_SIDE: EdgeDirection
EDGE_TYPE_UNSPECIFIED: EdgeType
EDGE_TYPE_PARENT: EdgeType
EDGE_TYPE_ANCESTOR: EdgeType
EDGE_TYPE_REGULAR: EdgeType
EDGE_TYPE_TEMPLATE: EdgeType
EDIT_OPERATION_UNSPECIFIED: EditOperation
EDIT_OPERATION_SET: EditOperation
EDIT_OPERATION_CLEAR: EditOperation
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
ENUM_TYPE_SPACE_STATUS: EnumType
ENUM_TYPE_USER_STATUS: EnumType
ENUM_TYPE_FRIENDSHIP_INVITE_EVENT_TYPE: EnumType
ENUM_TYPE_ORGANIZATION_STATUS: EnumType
ENUM_TYPE_CLIENT_TYPE: EnumType
ENUM_TYPE_CONDITIONAL_TYPE: EnumType
ENUM_TYPE_AGGREGATION_TYPE: EnumType
ENUM_TYPE_SORT_MODE: EnumType
ENUM_TYPE_SORT_TYPE: EnumType
ENUM_TYPE_JOIN_TYPE: EnumType
ENUM_TYPE_FUNCTION_TYPE: EnumType
ENUM_TYPE_EXPRESSION_TYPE: EnumType
ENUM_TYPE_QUERY_TYPE: EnumType
ENUM_TYPE_QUERY_UPDATE_TYPE: EnumType
ENUM_TYPE_MEMBERSHIP_EVENT_TYPE: EnumType
ENUM_TYPE_MEMBERSHIP_PERMISSION: EnumType
ENUM_TYPE_INVITE_EVENT_TYPE: EnumType
ENUM_TYPE_ROLE_TYPE: EnumType
ENUM_TYPE_ROLE_EVENT_TYPE: EnumType
ENUM_TYPE_FOLDER_TYPE: EnumType
ENUM_TYPE_TEXT_LINE_TYPE: EnumType
ENUM_TYPE_TEXT_SPAN_TYPE: EnumType
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
ENUM_TYPE_EDGE_DIRECTION: EnumType
ENUM_TYPE_CASCADE_ACTION: EnumType
ENUM_TYPE_DAY: EnumType
ENUM_TYPE_MONTH: EnumType
ENUM_TYPE_TIME_INTERVAL: EnumType
ENUM_TYPE_RESOURCE_STATUS: EnumType
ENUM_TYPE_ACTION_CARDINALITY: EnumType
ENUM_TYPE_CURSOR_STATUS: EnumType
ENUM_TYPE_SCHEDULE_FREQUENCY: EnumType
ENUM_TYPE_TIMER_TYPE: EnumType
ENUM_TYPE_TIMER_EVENT_TYPE: EnumType
ENUM_TYPE_TRIGGER_TYPE: EnumType
ENUM_TYPE_TRIGGER_EVENT_TYPE: EnumType
ENUM_TYPE_RUN_STATUS: EnumType
ENUM_TYPE_RUN_EVENT_TYPE: EnumType
ENUM_TYPE_RUN_TYPE: EnumType
ENUM_TYPE_INTERRUPTION_TYPE: EnumType
ENUM_TYPE_INTERRUPTION_STATUS: EnumType
ENUM_TYPE_INTERRUPTION_RESPONSE: EnumType
ENUM_TYPE_ENVIRONMENT_TYPE: EnumType
ENUM_TYPE_THREAD_STATUS: EnumType
ENUM_TYPE_NOTIFICATION_STATUS: EnumType
ENUM_TYPE_NOTIFICATION_EVENT_TYPE: EnumType
ENUM_TYPE_CLOUD: EnumType
ENUM_TYPE_REGION: EnumType
ENUM_TYPE_REGION_AREA: EnumType
ENUM_TYPE_REGION_CONTINENT: EnumType
ENUM_TYPE_TENANCY: EnumType
ENUM_TYPE_DATABASE_TYPE: EnumType
ENUM_TYPE_MACHINE_TYPE: EnumType
ENUM_TYPE_MODEL_DEVELOPER: EnumType
ENUM_TYPE_MODEL_PROVIDER: EnumType
ENUM_TYPE_WINDOW_TYPE: EnumType
ENUM_TYPE_SCENE_EVENT_TYPE: EnumType
ENUM_TYPE_LAYER_TYPE: EnumType
ENUM_TYPE_CANVAS_TYPE: EnumType
ENUM_TYPE_PLANE_SHAPE_TYPE: EnumType
ENUM_TYPE_ARROW_HEAD_TYPE: EnumType
ENUM_TYPE_LINE_TYPE: EnumType
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
ENUM_TYPE_ENUM_TYPE: EnumType
ENUM_TYPE_NODE_TYPE: EnumType
ENUM_TYPE_STRUCT_TYPE: EnumType
ENUM_TYPE_TRAIT_TYPE: EnumType
ENUM_TYPE_RELATION_TYPE: EnumType
ENUM_TYPE_ATTRIBUTE_TYPE: EnumType
ENUM_TYPE_PROPERTY_REFERENCE_TYPE: EnumType
ENUM_TYPE_STORE_ZONE: EnumType
ENUM_TYPE_STORE_TYPE: EnumType
ENUM_TYPE_STORE_IMPLEMENTATION: EnumType
ENUM_TYPE_PLATFORM_TYPE: EnumType
ENUM_TYPE_RUNTIME_TYPE: EnumType
ENUM_TYPE_OPERATING_SYSTEM: EnumType
ENUM_TYPE_ERROR_TYPE: EnumType
ENUM_TYPE_EDIT_TYPE: EnumType
ENUM_TYPE_EDIT_OPERATION: EnumType
ENUM_TYPE_CHANGE_STATUS: EnumType
ENUM_TYPE_NODE_PERMISSION: EnumType
ENVIRONMENT_TYPE_UNSPECIFIED: EnvironmentType
ENVIRONMENT_TYPE_SYSTEM: EnvironmentType
ENVIRONMENT_TYPE_DEVELOPMENT: EnvironmentType
ENVIRONMENT_TYPE_TEST: EnvironmentType
ENVIRONMENT_TYPE_STAGING: EnvironmentType
ENVIRONMENT_TYPE_PRODUCTION: EnvironmentType
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
FIELD_TYPE_MEMBER: FieldType
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
FILE_SOURCE_SPACE: FileSource
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
FILL_TYPE_STYLE: FillType
FILL_TYPE_SOLID: FillType
FILL_TYPE_GRADIENT: FillType
FILL_TYPE_IMAGE: FillType
FOLDER_TYPE_UNSPECIFIED: FolderType
FOLDER_TYPE_SYSTEM: FolderType
FOLDER_TYPE_HOME: FolderType
FOLDER_TYPE_GENERIC: FolderType
FOLDER_TYPE_MODULE: FolderType
FOLDER_TYPE_APP: FolderType
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
FRIENDSHIP_INVITE_EVENT_TYPE_UNSPECIFIED: FriendshipInviteEventType
FRIENDSHIP_INVITE_EVENT_TYPE_SENT: FriendshipInviteEventType
FRIENDSHIP_INVITE_EVENT_TYPE_RESCINDED: FriendshipInviteEventType
FRIENDSHIP_INVITE_EVENT_TYPE_ACCEPTED: FriendshipInviteEventType
FRIENDSHIP_INVITE_EVENT_TYPE_REJECTED: FriendshipInviteEventType
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
INVITE_EVENT_TYPE_UNSPECIFIED: InviteEventType
INVITE_EVENT_TYPE_SENT: InviteEventType
INVITE_EVENT_TYPE_RESCINDED: InviteEventType
INVITE_EVENT_TYPE_ACCEPTED: InviteEventType
INVITE_EVENT_TYPE_REJECTED: InviteEventType
JOIN_TYPE_UNSPECIFIED: JoinType
JOIN_TYPE_LEFT: JoinType
JOIN_TYPE_PARENT: JoinType
JOIN_TYPE_CHILD: JoinType
LAYER_TYPE_UNSPECIFIED: LayerType
LAYER_TYPE_GENERAL: LayerType
LAYER_TYPE_SHAPE: LayerType
LAYOUT_UNSPECIFIED: Layout
LAYOUT_STACK: Layout
LAYOUT_GRID: Layout
LENGTH_UNIT_UNSPECIFIED: LengthUnit
LENGTH_UNIT_PIXEL: LengthUnit
LENGTH_UNIT_REM: LengthUnit
LENGTH_UNIT_PERCENT: LengthUnit
LENGTH_UNIT_FR: LengthUnit
LINE_TYPE_UNSPECIFIED: LineType
LINE_TYPE_SOLID: LineType
LINE_TYPE_DASHED: LineType
LINE_TYPE_DOTTED: LineType
LINK_TYPE_UNSPECIFIED: LinkType
LINK_TYPE_WEB: LinkType
MACHINE_TYPE_UNSPECIFIED: MachineType
MACHINE_TYPE_RUNTIME: MachineType
MACHINE_TYPE_UBUNTU: MachineType
MACHINE_TYPE_MAC: MachineType
MACHINE_TYPE_WINDOWS: MachineType
MACHINE_TYPE_CUSTOM: MachineType
MEMBERSHIP_EVENT_TYPE_UNSPECIFIED: MembershipEventType
MEMBERSHIP_EVENT_TYPE_JOIN: MembershipEventType
MEMBERSHIP_EVENT_TYPE_LEAVE: MembershipEventType
MEMBERSHIP_PERMISSION_UNSPECIFIED: MembershipPermission
MEMBERSHIP_PERMISSION_KICK: MembershipPermission
MEMBERSHIP_PERMISSION_BAN: MembershipPermission
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
NODE_PERMISSION_UNSPECIFIED: NodePermission
NODE_PERMISSION_READ: NodePermission
NODE_PERMISSION_ADD: NodePermission
NODE_PERMISSION_UPDATE: NodePermission
NODE_PERMISSION_REMOVE: NodePermission
NODE_TYPE_UNSPECIFIED: NodeType
NODE_TYPE_SPACE: NodeType
NODE_TYPE_HANDLE: NodeType
NODE_TYPE_USER: NodeType
NODE_TYPE_FRIENDSHIP: NodeType
NODE_TYPE_FRIENDSHIP_INVITE: NodeType
NODE_TYPE_FRIENDSHIP_INVITE_EVENT: NodeType
NODE_TYPE_ORGANIZATION: NodeType
NODE_TYPE_TEAM: NodeType
NODE_TYPE_CLIENT: NodeType
NODE_TYPE_MEMBERSHIP: NodeType
NODE_TYPE_MEMBERSHIP_EVENT: NodeType
NODE_TYPE_INVITE: NodeType
NODE_TYPE_INVITE_EVENT: NodeType
NODE_TYPE_ROLE: NodeType
NODE_TYPE_ROLE_EVENT: NodeType
NODE_TYPE_AGENT: NodeType
NODE_TYPE_FOLDER: NodeType
NODE_TYPE_TAG: NodeType
NODE_TYPE_TAGGING: NodeType
NODE_TYPE_CUSTOM_ENTITY_DEFINITION: NodeType
NODE_TYPE_CUSTOM_ENTITY: NodeType
NODE_TYPE_CUSTOM_STRUCT_DEFINITION: NodeType
NODE_TYPE_CUSTOM_ENUM_DEFINITION: NodeType
NODE_TYPE_FIELD: NodeType
NODE_TYPE_OPTION: NodeType
NODE_TYPE_FILE: NodeType
NODE_TYPE_LINK: NodeType
NODE_TYPE_SCRIPT: NodeType
NODE_TYPE_SERVICE: NodeType
NODE_TYPE_ACTION: NodeType
NODE_TYPE_ROUTE: NodeType
NODE_TYPE_TRIGGER: NodeType
NODE_TYPE_TRIGGER_EVENT: NodeType
NODE_TYPE_TIMER: NodeType
NODE_TYPE_TIMER_EVENT: NodeType
NODE_TYPE_EVENT_CURSOR: NodeType
NODE_TYPE_SCREEN_CURSOR: NodeType
NODE_TYPE_THREAD_CURSOR: NodeType
NODE_TYPE_RUN: NodeType
NODE_TYPE_RUN_EVENT: NodeType
NODE_TYPE_SPAN: NodeType
NODE_TYPE_INTERRUPTION: NodeType
NODE_TYPE_LOG: NodeType
NODE_TYPE_GAUGE_METRIC: NodeType
NODE_TYPE_GAUGE_MEASUREMENT: NodeType
NODE_TYPE_COUNTER_METRIC: NodeType
NODE_TYPE_COUNTER_MEASUREMENT: NodeType
NODE_TYPE_HISTOGRAM_METRIC: NodeType
NODE_TYPE_HISTOGRAM_MEASUREMENT: NodeType
NODE_TYPE_CUSTOM_EVENT_DEFINITION: NodeType
NODE_TYPE_CUSTOM_EVENT: NodeType
NODE_TYPE_EDIT_EVENT: NodeType
NODE_TYPE_ENVIRONMENT: NodeType
NODE_TYPE_THREAD: NodeType
NODE_TYPE_MESSAGE: NodeType
NODE_TYPE_REACTION: NodeType
NODE_TYPE_STAR: NodeType
NODE_TYPE_FOLLOW: NodeType
NODE_TYPE_NOTIFICATION: NodeType
NODE_TYPE_NOTIFICATION_EVENT: NodeType
NODE_TYPE_DATABASE: NodeType
NODE_TYPE_MACHINE: NodeType
NODE_TYPE_WINDOW: NodeType
NODE_TYPE_SCENE: NodeType
NODE_TYPE_SCENE_EVENT: NodeType
NODE_TYPE_LAYER: NodeType
NODE_TYPE_CUSTOM_VIEW_DEFINITION: NodeType
NODE_TYPE_CUSTOM_VIEW: NodeType
NODE_TYPE_FRAME_VIEW: NodeType
NODE_TYPE_LABEL_VIEW: NodeType
NODE_TYPE_SPLIT_VIEW: NodeType
NODE_TYPE_TEXT_VIEW: NodeType
NODE_TYPE_NUMBER_INPUT_VIEW: NodeType
NODE_TYPE_SLIDER_INPUT_VIEW: NodeType
NODE_TYPE_THREAD_VIEW: NodeType
NODE_TYPE_WIZARD_VIEW: NodeType
NODE_TYPE_CANVAS: NodeType
NODE_TYPE_LINE_SHAPE: NodeType
NODE_TYPE_PLANE_SHAPE: NodeType
NODE_TYPE_ARROW_SHAPE: NodeType
NODE_TYPE_ANNOTATION_SHAPE: NodeType
NODE_TYPE_THEME: NodeType
NODE_TYPE_COLOR_STYLE: NodeType
NODE_TYPE_FILL_STYLE: NodeType
NODE_TYPE_FONT_STYLE: NodeType
NODE_TYPE_BORDER_STYLE: NodeType
NODE_TYPE_SHADOW_STYLE: NodeType
NODE_TYPE_GRADIENT_STYLE: NodeType
NODE_TYPE_TRANSITION_STYLE: NodeType
NODE_TYPE_EFFECT_STYLE: NodeType
NOTIFICATION_EVENT_TYPE_UNSPECIFIED: NotificationEventType
NOTIFICATION_EVENT_TYPE_SENT: NotificationEventType
NOTIFICATION_EVENT_TYPE_RESCINDED: NotificationEventType
NOTIFICATION_EVENT_TYPE_READ: NotificationEventType
NOTIFICATION_EVENT_TYPE_DISMISSED: NotificationEventType
NOTIFICATION_EVENT_TYPE_EXPIRED: NotificationEventType
NOTIFICATION_STATUS_UNSPECIFIED: NotificationStatus
NOTIFICATION_STATUS_UNREAD: NotificationStatus
NOTIFICATION_STATUS_READ: NotificationStatus
NOTIFICATION_STATUS_DISMISSED: NotificationStatus
NOTIFICATION_STATUS_EXPIRED: NotificationStatus
NOTIFICATION_STATUS_RESCINDED: NotificationStatus
NUMBER_FORMAT_UNSPECIFIED: NumberFormat
NUMBER_FORMAT_PERCENTAGE: NumberFormat
NUMBER_FORMAT_ANGLE: NumberFormat
NUMBER_FORMAT_CURRENCY: NumberFormat
OFFSCREEN_BEHAVIOR_UNSPECIFIED: OffscreenBehavior
OFFSCREEN_BEHAVIOR_PLAY: OffscreenBehavior
OFFSCREEN_BEHAVIOR_PAUSE: OffscreenBehavior
OPERATING_SYSTEM_UNSPECIFIED: OperatingSystem
OPERATING_SYSTEM_LINUX: OperatingSystem
OPERATING_SYSTEM_WINDOWS: OperatingSystem
OPERATING_SYSTEM_MACOS: OperatingSystem
OPERATING_SYSTEM_ANDROID: OperatingSystem
OPERATING_SYSTEM_IOS: OperatingSystem
ORGANIZATION_STATUS_UNSPECIFIED: OrganizationStatus
ORGANIZATION_STATUS_CREATING: OrganizationStatus
ORGANIZATION_STATUS_ACTIVE: OrganizationStatus
OVERFLOW_UNSPECIFIED: Overflow
OVERFLOW_HIDDEN: Overflow
OVERFLOW_VISIBLE: Overflow
OVERFLOW_SCROLL: Overflow
PLANE_SHAPE_TYPE_UNSPECIFIED: PlaneShapeType
PLANE_SHAPE_TYPE_RECTANGLE: PlaneShapeType
PLANE_SHAPE_TYPE_TRIANGLE: PlaneShapeType
PLANE_SHAPE_TYPE_CIRCLE: PlaneShapeType
PLANE_SHAPE_TYPE_ELLIPSE: PlaneShapeType
PLANE_SHAPE_TYPE_POLYGON: PlaneShapeType
PLATFORM_TYPE_UNSPECIFIED: PlatformType
PLATFORM_TYPE_SERVER: PlatformType
PLATFORM_TYPE_WEB: PlatformType
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
PRIMITIVE_TYPE_DATETIME: PrimitiveType
PRIMITIVE_TYPE_DATE: PrimitiveType
PRIMITIVE_TYPE_TIME: PrimitiveType
PRIMITIVE_TYPE_DURATION: PrimitiveType
PROPERTY_REFERENCE_TYPE_UNSPECIFIED: PropertyReferenceType
PROPERTY_REFERENCE_TYPE_NODE: PropertyReferenceType
PROPERTY_REFERENCE_TYPE_TRAIT: PropertyReferenceType
PROPERTY_REFERENCE_TYPE_STRUCT: PropertyReferenceType
QUERY_TYPE_UNSPECIFIED: QueryType
QUERY_TYPE_NODE: QueryType
QUERY_TYPE_SCALAR: QueryType
QUERY_TYPE_GROUPED_NODE: QueryType
QUERY_TYPE_GROUPED_SCALAR: QueryType
QUERY_UPDATE_TYPE_UNSPECIFIED: QueryUpdateType
QUERY_UPDATE_TYPE_FULL_RESULT: QueryUpdateType
QUERY_UPDATE_TYPE_PARTIAL_RESULT: QueryUpdateType
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
RELATION_TYPE_UNSPECIFIED: RelationType
RELATION_TYPE_BUILTIN_NODE: RelationType
RELATION_TYPE_CUSTOM_NODE: RelationType
RELATION_TYPE_TRAIT: RelationType
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
ROLE_EVENT_TYPE_UNSPECIFIED: RoleEventType
ROLE_EVENT_TYPE_ASSIGNED: RoleEventType
ROLE_EVENT_TYPE_REMOVED: RoleEventType
ROLE_TYPE_UNSPECIFIED: RoleType
ROLE_TYPE_SYSTEM: RoleType
ROLE_TYPE_OWNER: RoleType
ROLE_TYPE_ADMIN: RoleType
ROLE_TYPE_DEVELOPER: RoleType
ROLE_TYPE_USER: RoleType
ROLE_TYPE_SPECTATOR: RoleType
RUN_EVENT_TYPE_UNSPECIFIED: RunEventType
RUN_EVENT_TYPE_SCHEDULED: RunEventType
RUN_EVENT_TYPE_RUNNING: RunEventType
RUN_EVENT_TYPE_REQUESTED_PAUSE: RunEventType
RUN_EVENT_TYPE_PAUSED: RunEventType
RUN_EVENT_TYPE_REQUESTED_RESUME: RunEventType
RUN_EVENT_TYPE_RESUMED: RunEventType
RUN_EVENT_TYPE_REQUESTED_CANCEL: RunEventType
RUN_EVENT_TYPE_CANCELLED: RunEventType
RUN_EVENT_TYPE_ABORTED: RunEventType
RUN_EVENT_TYPE_FAILED: RunEventType
RUN_EVENT_TYPE_COMPLETED: RunEventType
RUN_STATUS_UNSPECIFIED: RunStatus
RUN_STATUS_SCHEDULED: RunStatus
RUN_STATUS_RUNNING: RunStatus
RUN_STATUS_PAUSED: RunStatus
RUN_STATUS_YIELDED: RunStatus
RUN_STATUS_CANCELLED: RunStatus
RUN_STATUS_ABORTED: RunStatus
RUN_STATUS_FAILED: RunStatus
RUN_STATUS_COMPLETED: RunStatus
RUN_TYPE_UNSPECIFIED: RunType
RUN_TYPE_CODE: RunType
RUN_TYPE_ACTION: RunType
RUN_TYPE_FLOW: RunType
RUN_TYPE_AGENT: RunType
RUNTIME_TYPE_UNSPECIFIED: RuntimeType
RUNTIME_TYPE_PYTHON: RuntimeType
RUNTIME_TYPE_JAVASCRIPT: RuntimeType
SCALAR_TYPE_UNSPECIFIED: ScalarType
SCALAR_TYPE_PRIMITIVE: ScalarType
SCALAR_TYPE_ENUM: ScalarType
SCALAR_TYPE_NODE_REFERENCE: ScalarType
SCALAR_TYPE_NODE_VALUE: ScalarType
SCALAR_TYPE_STRUCT: ScalarType
SCENE_EVENT_TYPE_UNSPECIFIED: SceneEventType
SCENE_EVENT_TYPE_ENTERED: SceneEventType
SCENE_EVENT_TYPE_EXITED: SceneEventType
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
SPACE_STATUS_UNSPECIFIED: SpaceStatus
SPACE_STATUS_CREATING: SpaceStatus
SPACE_STATUS_QUEUED: SpaceStatus
SPACE_STATUS_RUNNING: SpaceStatus
SPACE_STATUS_PAUSED: SpaceStatus
SPRING_TYPE_UNSPECIFIED: SpringType
SPRING_TYPE_TIME: SpringType
SPRING_TYPE_PHYSICS: SpringType
STORE_IMPLEMENTATION_UNSPECIFIED: StoreImplementation
STORE_IMPLEMENTATION_POSTGRES: StoreImplementation
STORE_IMPLEMENTATION_MEMORY: StoreImplementation
STORE_TYPE_UNSPECIFIED: StoreType
STORE_TYPE_GLOBAL_ENTITY: StoreType
STORE_TYPE_SPATIAL_ENTITY: StoreType
STORE_TYPE_LOCAL_MEMORY: StoreType
STORE_ZONE_UNSPECIFIED: StoreZone
STORE_ZONE_GLOBAL: StoreZone
STORE_ZONE_SPATIAL: StoreZone
STORE_ZONE_LOCAL: StoreZone
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
STRUCT_TYPE_PROPERTY_DEFINITION: StructType
STRUCT_TYPE_TRAIT_DEFINITION: StructType
STRUCT_TYPE_NODE_DEFINITION: StructType
STRUCT_TYPE_STRUCT_DEFINITION: StructType
STRUCT_TYPE_ENUM_DEFINITION: StructType
STRUCT_TYPE_ENUM_OPTION_DEFINITION: StructType
STRUCT_TYPE_PERMISSION_DEFINITION: StructType
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
STRUCT_TYPE_QUERY_RESULT_GROUP: StructType
STRUCT_TYPE_QUERY_UPDATE: StructType
STRUCT_TYPE_HISTOGRAM: StructType
STRUCT_TYPE_VALUE: StructType
STRUCT_TYPE_TYPE: StructType
STRUCT_TYPE_NUMBER_CONSTRAINT: StructType
STRUCT_TYPE_STRING_CONSTRAINT: StructType
STRUCT_TYPE_COLLECTION_CONSTRAINT: StructType
STRUCT_TYPE_NODE_CONSTRAINT: StructType
STRUCT_TYPE_TEXT: StructType
STRUCT_TYPE_TEXT_LINE: StructType
STRUCT_TYPE_TEXT_SPAN: StructType
STRUCT_TYPE_ICON: StructType
STRUCT_TYPE_SELECTION: StructType
STRUCT_TYPE_SCHEDULE: StructType
STRUCT_TYPE_ERROR: StructType
STRUCT_TYPE_DATABASE_INFO: StructType
STRUCT_TYPE_CELL_INFO: StructType
STRUCT_TYPE_VECTOR2: StructType
STRUCT_TYPE_VECTOR3: StructType
STRUCT_TYPE_VECTOR4: StructType
STRUCT_TYPE_VECTOR2I: StructType
STRUCT_TYPE_VECTOR3I: StructType
STRUCT_TYPE_VECTOR4I: StructType
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
TENANCY_UNSPECIFIED: Tenancy
TENANCY_DEDICATED: Tenancy
TENANCY_SHARED: Tenancy
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
TIMER_EVENT_TYPE_UNSPECIFIED: TimerEventType
TIMER_EVENT_TYPE_STARTED: TimerEventType
TIMER_EVENT_TYPE_STOPPED: TimerEventType
TIMER_EVENT_TYPE_EXPIRED: TimerEventType
TIMER_TYPE_UNSPECIFIED: TimerType
TIMER_TYPE_ONCE: TimerType
TIMER_TYPE_RECURRING: TimerType
TRAIT_TYPE_UNSPECIFIED: TraitType
TRAIT_TYPE_GLOBAL: TraitType
TRAIT_TYPE_SPATIAL: TraitType
TRAIT_TYPE_ENTITY: TraitType
TRAIT_TYPE_PARTICLE: TraitType
TRAIT_TYPE_ANALYTIC: TraitType
TRAIT_TYPE_INDEXED: TraitType
TRAIT_TYPE_RESOURCE: TraitType
TRAIT_TYPE_EVENT: TraitType
TRAIT_TYPE_CUSTOM_NODE_DEFINITION: TraitType
TRAIT_TYPE_CUSTOM_NODE: TraitType
TRAIT_TYPE_FROZEN: TraitType
TRAIT_TYPE_TRACKED: TraitType
TRAIT_TYPE_ARCHIVABLE: TraitType
TRAIT_TYPE_DELETABLE: TraitType
TRAIT_TYPE_TEMPLATABLE: TraitType
TRAIT_TYPE_EXTENSIBLE: TraitType
TRAIT_TYPE_ORDERED: TraitType
TRAIT_TYPE_HAS_NAME: TraitType
TRAIT_TYPE_HAS_SLUG: TraitType
TRAIT_TYPE_HAS_ICON: TraitType
TRAIT_TYPE_OWNABLE: TraitType
TRAIT_TYPE_JOINABLE: TraitType
TRAIT_TYPE_SUBJECT: TraitType
TRAIT_TYPE_OWNER: TraitType
TRAIT_TYPE_MEMBERSHIP: TraitType
TRAIT_TYPE_INVITE: TraitType
TRAIT_TYPE_TAGGABLE: TraitType
TRAIT_TYPE_TAG: TraitType
TRAIT_TYPE_ACTIONABLE: TraitType
TRAIT_TYPE_RUNNABLE: TraitType
TRAIT_TYPE_SCRIPTABLE: TraitType
TRAIT_TYPE_SOURCEABLE: TraitType
TRAIT_TYPE_CURSOR: TraitType
TRAIT_TYPE_METRIC: TraitType
TRAIT_TYPE_MEASUREMENT: TraitType
TRAIT_TYPE_STARABLE: TraitType
TRAIT_TYPE_REACTABLE: TraitType
TRAIT_TYPE_FOLLOWABLE: TraitType
TRAIT_TYPE_FOLLOW: TraitType
TRAIT_TYPE_VISUAL: TraitType
TRAIT_TYPE_VIEW: TraitType
TRAIT_TYPE_CONTAINER_VIEW: TraitType
TRAIT_TYPE_CONTENT_VIEW: TraitType
TRAIT_TYPE_INPUT_VIEW: TraitType
TRAIT_TYPE_NODE_VIEW: TraitType
TRAIT_TYPE_INTERNAL_VIEW: TraitType
TRAIT_TYPE_STYLE: TraitType
TRAIT_TYPE_SHAPE: TraitType
TRANSITION_TYPE_UNSPECIFIED: TransitionType
TRANSITION_TYPE_STYLE: TransitionType
TRANSITION_TYPE_FIELD: TransitionType
TRANSITION_TYPE_TWEEN: TransitionType
TRANSITION_TYPE_SPRING: TransitionType
TRIGGER_EVENT_TYPE_UNSPECIFIED: TriggerEventType
TRIGGER_EVENT_TYPE_STARTED: TriggerEventType
TRIGGER_EVENT_TYPE_TRIGGERED: TriggerEventType
TRIGGER_EVENT_TYPE_STOPPED: TriggerEventType
TRIGGER_TYPE_UNSPECIFIED: TriggerType
TRIGGER_TYPE_EVENT: TriggerType
TYPE_CARDINALITY_UNSPECIFIED: TypeCardinality
TYPE_CARDINALITY_SCALAR: TypeCardinality
TYPE_CARDINALITY_LIST: TypeCardinality
TYPE_CARDINALITY_MAP: TypeCardinality
USER_STATUS_UNSPECIFIED: UserStatus
USER_STATUS_CREATING: UserStatus
USER_STATUS_ACTIVE: UserStatus
WINDOW_TYPE_UNSPECIFIED: WindowType
WINDOW_TYPE_BROWSER: WindowType
WINDOW_TYPE_DESKTOP: WindowType
WINDOW_TYPE_MOBILE: WindowType

class ActionData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "value", "order_key", "name", "cardinality", "text", "source_ptr")
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
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    CARDINALITY_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    value: _containers.MessageMap[str, ValueData]
    order_key: str
    name: str
    cardinality: ActionCardinality
    text: TextData
    source_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., value: _Optional[_Mapping[str, ValueData]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., cardinality: _Optional[_Union[ActionCardinality, str]] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., source_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class AgentData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "name", "slug", "icon", "cursor_ptr", "script_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    CURSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    name: str
    slug: str
    icon: IconData
    cursor_ptr: NodeReferenceData
    script_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., name: _Optional[str] = ..., slug: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., cursor_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., script_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class AggregationData(_message.Message):
    __slots__ = ("metatype", "type", "expression")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    EXPRESSION_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: AggregationType
    expression: ExpressionData
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[AggregationType, str]] = ..., expression: _Optional[_Union[ExpressionData, _Mapping]] = ...) -> None: ...

class AnnotationShapeData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "value", "order_key", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "layout", "direction", "distribute", "align", "gap", "padding", "grid", "grid_span", "aspect_ratio", "is_wrap", "is_visible", "opacity", "fill", "rotation", "skew", "scale", "shadow", "border", "radius", "text", "script_ptr")
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
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    LAYOUT_FIELD_NUMBER: _ClassVar[int]
    DIRECTION_FIELD_NUMBER: _ClassVar[int]
    DISTRIBUTE_FIELD_NUMBER: _ClassVar[int]
    ALIGN_FIELD_NUMBER: _ClassVar[int]
    GAP_FIELD_NUMBER: _ClassVar[int]
    PADDING_FIELD_NUMBER: _ClassVar[int]
    GRID_FIELD_NUMBER: _ClassVar[int]
    GRID_SPAN_FIELD_NUMBER: _ClassVar[int]
    ASPECT_RATIO_FIELD_NUMBER: _ClassVar[int]
    IS_WRAP_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_FIELD_NUMBER: _ClassVar[int]
    FILL_FIELD_NUMBER: _ClassVar[int]
    ROTATION_FIELD_NUMBER: _ClassVar[int]
    SKEW_FIELD_NUMBER: _ClassVar[int]
    SCALE_FIELD_NUMBER: _ClassVar[int]
    SHADOW_FIELD_NUMBER: _ClassVar[int]
    BORDER_FIELD_NUMBER: _ClassVar[int]
    RADIUS_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    value: _containers.MessageMap[str, ValueData]
    order_key: str
    name: str
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    layout: Layout
    direction: Direction
    distribute: Distribute
    align: Align
    gap: Axis2Data
    padding: InsetsData
    grid: GridData
    grid_span: GridSpanData
    aspect_ratio: float
    is_wrap: bool
    is_visible: bool
    opacity: float
    fill: FillData
    rotation: Axis3Data
    skew: Vector2Data
    scale: float
    shadow: ShadowData
    border: BorderData
    radius: CornersData
    text: TextData
    script_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., value: _Optional[_Mapping[str, ValueData]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ..., layout: _Optional[_Union[Layout, str]] = ..., direction: _Optional[_Union[Direction, str]] = ..., distribute: _Optional[_Union[Distribute, str]] = ..., align: _Optional[_Union[Align, str]] = ..., gap: _Optional[_Union[Axis2Data, _Mapping]] = ..., padding: _Optional[_Union[InsetsData, _Mapping]] = ..., grid: _Optional[_Union[GridData, _Mapping]] = ..., grid_span: _Optional[_Union[GridSpanData, _Mapping]] = ..., aspect_ratio: _Optional[float] = ..., is_wrap: bool = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., fill: _Optional[_Union[FillData, _Mapping]] = ..., rotation: _Optional[_Union[Axis3Data, _Mapping]] = ..., skew: _Optional[_Union[Vector2Data, _Mapping]] = ..., scale: _Optional[float] = ..., shadow: _Optional[_Union[ShadowData, _Mapping]] = ..., border: _Optional[_Union[BorderData, _Mapping]] = ..., radius: _Optional[_Union[CornersData, _Mapping]] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., script_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class ArrowShapeData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "order_key", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "align", "is_visible", "opacity", "start_type", "start", "end_type", "end", "script_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    ALIGN_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_FIELD_NUMBER: _ClassVar[int]
    START_TYPE_FIELD_NUMBER: _ClassVar[int]
    START_FIELD_NUMBER: _ClassVar[int]
    END_TYPE_FIELD_NUMBER: _ClassVar[int]
    END_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    order_key: str
    name: str
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    align: Align
    is_visible: bool
    opacity: float
    start_type: ArrowHeadType
    start: Vector2Data
    end_type: ArrowHeadType
    end: Vector2Data
    script_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ..., align: _Optional[_Union[Align, str]] = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., start_type: _Optional[_Union[ArrowHeadType, str]] = ..., start: _Optional[_Union[Vector2Data, _Mapping]] = ..., end_type: _Optional[_Union[ArrowHeadType, str]] = ..., end: _Optional[_Union[Vector2Data, _Mapping]] = ..., script_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class AttributeReferenceData(_message.Message):
    __slots__ = ("metatype", "type", "prop_ptr", "field_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    PROP_PTR_FIELD_NUMBER: _ClassVar[int]
    FIELD_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: AttributeType
    prop_ptr: PropertyReferenceData
    field_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[AttributeType, str]] = ..., prop_ptr: _Optional[_Union[PropertyReferenceData, _Mapping]] = ..., field_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

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
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "order_key", "type", "name", "style_ptr", "color", "width")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    STYLE_PTR_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    order_key: str
    type: BorderType
    name: str
    style_ptr: NodeReferenceData
    color: ColorData
    width: InsetsData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[BorderType, str]] = ..., name: _Optional[str] = ..., style_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., color: _Optional[_Union[ColorData, _Mapping]] = ..., width: _Optional[_Union[InsetsData, _Mapping]] = ...) -> None: ...

class CanvasData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "value", "order_key", "type", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "layout", "direction", "distribute", "align", "gap", "padding", "grid", "grid_span", "aspect_ratio", "is_wrap", "is_visible", "opacity", "fill", "rotation", "skew", "scale", "shadow", "border", "radius", "script_ptr")
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
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    LAYOUT_FIELD_NUMBER: _ClassVar[int]
    DIRECTION_FIELD_NUMBER: _ClassVar[int]
    DISTRIBUTE_FIELD_NUMBER: _ClassVar[int]
    ALIGN_FIELD_NUMBER: _ClassVar[int]
    GAP_FIELD_NUMBER: _ClassVar[int]
    PADDING_FIELD_NUMBER: _ClassVar[int]
    GRID_FIELD_NUMBER: _ClassVar[int]
    GRID_SPAN_FIELD_NUMBER: _ClassVar[int]
    ASPECT_RATIO_FIELD_NUMBER: _ClassVar[int]
    IS_WRAP_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_FIELD_NUMBER: _ClassVar[int]
    FILL_FIELD_NUMBER: _ClassVar[int]
    ROTATION_FIELD_NUMBER: _ClassVar[int]
    SKEW_FIELD_NUMBER: _ClassVar[int]
    SCALE_FIELD_NUMBER: _ClassVar[int]
    SHADOW_FIELD_NUMBER: _ClassVar[int]
    BORDER_FIELD_NUMBER: _ClassVar[int]
    RADIUS_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    value: _containers.MessageMap[str, ValueData]
    order_key: str
    type: CanvasType
    name: str
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    layout: Layout
    direction: Direction
    distribute: Distribute
    align: Align
    gap: Axis2Data
    padding: InsetsData
    grid: GridData
    grid_span: GridSpanData
    aspect_ratio: float
    is_wrap: bool
    is_visible: bool
    opacity: float
    fill: FillData
    rotation: Axis3Data
    skew: Vector2Data
    scale: float
    shadow: ShadowData
    border: BorderData
    radius: CornersData
    script_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., value: _Optional[_Mapping[str, ValueData]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[CanvasType, str]] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ..., layout: _Optional[_Union[Layout, str]] = ..., direction: _Optional[_Union[Direction, str]] = ..., distribute: _Optional[_Union[Distribute, str]] = ..., align: _Optional[_Union[Align, str]] = ..., gap: _Optional[_Union[Axis2Data, _Mapping]] = ..., padding: _Optional[_Union[InsetsData, _Mapping]] = ..., grid: _Optional[_Union[GridData, _Mapping]] = ..., grid_span: _Optional[_Union[GridSpanData, _Mapping]] = ..., aspect_ratio: _Optional[float] = ..., is_wrap: bool = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., fill: _Optional[_Union[FillData, _Mapping]] = ..., rotation: _Optional[_Union[Axis3Data, _Mapping]] = ..., skew: _Optional[_Union[Vector2Data, _Mapping]] = ..., scale: _Optional[float] = ..., shadow: _Optional[_Union[ShadowData, _Mapping]] = ..., border: _Optional[_Union[BorderData, _Mapping]] = ..., radius: _Optional[_Union[CornersData, _Mapping]] = ..., script_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class CellInfoData(_message.Message):
    __slots__ = ("metatype", "region", "name", "host")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    HOST_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    region: Region
    name: str
    host: str
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., region: _Optional[_Union[Region, str]] = ..., name: _Optional[str] = ..., host: _Optional[str] = ...) -> None: ...

class ChangeData(_message.Message):
    __slots__ = ("metatype", "id", "name", "created_at", "created_by_ptr", "origin", "edits")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ORIGIN_FIELD_NUMBER: _ClassVar[int]
    EDITS_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    id: str
    name: str
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    origin: OriginData
    edits: _containers.RepeatedCompositeFieldContainer[EditData]
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., id: _Optional[str] = ..., name: _Optional[str] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., origin: _Optional[_Union[OriginData, _Mapping]] = ..., edits: _Optional[_Iterable[_Union[EditData, _Mapping]]] = ...) -> None: ...

class ChangeResultData(_message.Message):
    __slots__ = ("metatype", "id", "created_at", "status", "edits", "cascaded_edits")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    EDITS_FIELD_NUMBER: _ClassVar[int]
    CASCADED_EDITS_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    id: str
    created_at: _timestamp_pb2.Timestamp
    status: ChangeStatus
    edits: _containers.RepeatedCompositeFieldContainer[EditData]
    cascaded_edits: _containers.RepeatedCompositeFieldContainer[EditData]
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., id: _Optional[str] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., status: _Optional[_Union[ChangeStatus, str]] = ..., edits: _Optional[_Iterable[_Union[EditData, _Mapping]]] = ..., cascaded_edits: _Optional[_Iterable[_Union[EditData, _Mapping]]] = ...) -> None: ...

class ClientData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "type", "name", "machine_ptr", "user_ptr", "device_type", "device_name", "operating_system", "browser_name", "browser_version", "access_token", "seen_at", "logged_in_at", "cursor_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
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
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    type: ClientType
    name: str
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
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., type: _Optional[_Union[ClientType, str]] = ..., name: _Optional[str] = ..., machine_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., user_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., device_type: _Optional[str] = ..., device_name: _Optional[str] = ..., operating_system: _Optional[str] = ..., browser_name: _Optional[str] = ..., browser_version: _Optional[str] = ..., access_token: _Optional[str] = ..., seen_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., logged_in_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., cursor_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

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
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "order_key", "type", "name", "style_ptr", "hue", "shade", "x", "y", "z", "alpha", "dark")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
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
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    order_key: str
    type: ColorType
    name: str
    style_ptr: NodeReferenceData
    hue: ColorHue
    shade: ColorShade
    x: float
    y: float
    z: float
    alpha: float
    dark: ColorData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[ColorType, str]] = ..., name: _Optional[str] = ..., style_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., hue: _Optional[_Union[ColorHue, str]] = ..., shade: _Optional[_Union[ColorShade, str]] = ..., x: _Optional[float] = ..., y: _Optional[float] = ..., z: _Optional[float] = ..., alpha: _Optional[float] = ..., dark: _Optional[_Union[ColorData, _Mapping]] = ...) -> None: ...

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

class CounterMeasurementData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "definition_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    definition_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class CounterMetricData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "order_key", "name", "source_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    order_key: str
    name: str
    source_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., source_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class CustomEntityData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "definition_ptr", "value")
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
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    definition_ptr: NodeReferenceData
    value: _containers.MessageMap[str, ValueData]
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., value: _Optional[_Mapping[str, ValueData]] = ...) -> None: ...

class CustomEntityDefinitionData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "order_key", "owned_by_ptr", "name", "traits", "script_ptr", "source_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TRAITS_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    order_key: str
    owned_by_ptr: NodeReferenceData
    name: str
    traits: _containers.RepeatedScalarFieldContainer[TraitType]
    script_ptr: NodeReferenceData
    source_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., name: _Optional[str] = ..., traits: _Optional[_Iterable[_Union[TraitType, str]]] = ..., script_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., source_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class CustomEnumDefinitionData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "value", "order_key", "name", "icon", "source_ptr")
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
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    value: _containers.MessageMap[str, ValueData]
    order_key: str
    name: str
    icon: IconData
    source_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., value: _Optional[_Mapping[str, ValueData]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., source_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class CustomEventData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "node_ptr", "definition_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    node_ptr: NodeReferenceData
    definition_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class CustomEventDefinitionData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "order_key", "name", "source_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    order_key: str
    name: str
    source_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., source_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class CustomStructDefinitionData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "value", "order_key", "name", "icon", "source_ptr")
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
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    value: _containers.MessageMap[str, ValueData]
    order_key: str
    name: str
    icon: IconData
    source_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., value: _Optional[_Mapping[str, ValueData]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., source_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class CustomViewData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "definition_ptr", "value", "order_key", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "layout", "direction", "distribute", "align", "gap", "padding", "grid", "grid_span", "aspect_ratio", "is_wrap", "is_visible", "opacity", "fill", "rotation", "skew", "scale", "shadow", "border", "radius", "script_ptr")
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
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    LAYOUT_FIELD_NUMBER: _ClassVar[int]
    DIRECTION_FIELD_NUMBER: _ClassVar[int]
    DISTRIBUTE_FIELD_NUMBER: _ClassVar[int]
    ALIGN_FIELD_NUMBER: _ClassVar[int]
    GAP_FIELD_NUMBER: _ClassVar[int]
    PADDING_FIELD_NUMBER: _ClassVar[int]
    GRID_FIELD_NUMBER: _ClassVar[int]
    GRID_SPAN_FIELD_NUMBER: _ClassVar[int]
    ASPECT_RATIO_FIELD_NUMBER: _ClassVar[int]
    IS_WRAP_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_FIELD_NUMBER: _ClassVar[int]
    FILL_FIELD_NUMBER: _ClassVar[int]
    ROTATION_FIELD_NUMBER: _ClassVar[int]
    SKEW_FIELD_NUMBER: _ClassVar[int]
    SCALE_FIELD_NUMBER: _ClassVar[int]
    SHADOW_FIELD_NUMBER: _ClassVar[int]
    BORDER_FIELD_NUMBER: _ClassVar[int]
    RADIUS_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    definition_ptr: NodeReferenceData
    value: _containers.MessageMap[str, ValueData]
    order_key: str
    name: str
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    layout: Layout
    direction: Direction
    distribute: Distribute
    align: Align
    gap: Axis2Data
    padding: InsetsData
    grid: GridData
    grid_span: GridSpanData
    aspect_ratio: float
    is_wrap: bool
    is_visible: bool
    opacity: float
    fill: FillData
    rotation: Axis3Data
    skew: Vector2Data
    scale: float
    shadow: ShadowData
    border: BorderData
    radius: CornersData
    script_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., value: _Optional[_Mapping[str, ValueData]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ..., layout: _Optional[_Union[Layout, str]] = ..., direction: _Optional[_Union[Direction, str]] = ..., distribute: _Optional[_Union[Distribute, str]] = ..., align: _Optional[_Union[Align, str]] = ..., gap: _Optional[_Union[Axis2Data, _Mapping]] = ..., padding: _Optional[_Union[InsetsData, _Mapping]] = ..., grid: _Optional[_Union[GridData, _Mapping]] = ..., grid_span: _Optional[_Union[GridSpanData, _Mapping]] = ..., aspect_ratio: _Optional[float] = ..., is_wrap: bool = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., fill: _Optional[_Union[FillData, _Mapping]] = ..., rotation: _Optional[_Union[Axis3Data, _Mapping]] = ..., skew: _Optional[_Union[Vector2Data, _Mapping]] = ..., scale: _Optional[float] = ..., shadow: _Optional[_Union[ShadowData, _Mapping]] = ..., border: _Optional[_Union[BorderData, _Mapping]] = ..., radius: _Optional[_Union[CornersData, _Mapping]] = ..., script_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class CustomViewDefinitionData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "value", "order_key", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "layout", "direction", "distribute", "align", "gap", "padding", "grid", "grid_span", "aspect_ratio", "is_wrap", "is_visible", "opacity", "fill", "rotation", "skew", "scale", "shadow", "border", "radius", "script_ptr")
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
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    LAYOUT_FIELD_NUMBER: _ClassVar[int]
    DIRECTION_FIELD_NUMBER: _ClassVar[int]
    DISTRIBUTE_FIELD_NUMBER: _ClassVar[int]
    ALIGN_FIELD_NUMBER: _ClassVar[int]
    GAP_FIELD_NUMBER: _ClassVar[int]
    PADDING_FIELD_NUMBER: _ClassVar[int]
    GRID_FIELD_NUMBER: _ClassVar[int]
    GRID_SPAN_FIELD_NUMBER: _ClassVar[int]
    ASPECT_RATIO_FIELD_NUMBER: _ClassVar[int]
    IS_WRAP_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_FIELD_NUMBER: _ClassVar[int]
    FILL_FIELD_NUMBER: _ClassVar[int]
    ROTATION_FIELD_NUMBER: _ClassVar[int]
    SKEW_FIELD_NUMBER: _ClassVar[int]
    SCALE_FIELD_NUMBER: _ClassVar[int]
    SHADOW_FIELD_NUMBER: _ClassVar[int]
    BORDER_FIELD_NUMBER: _ClassVar[int]
    RADIUS_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    value: _containers.MessageMap[str, ValueData]
    order_key: str
    name: str
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    layout: Layout
    direction: Direction
    distribute: Distribute
    align: Align
    gap: Axis2Data
    padding: InsetsData
    grid: GridData
    grid_span: GridSpanData
    aspect_ratio: float
    is_wrap: bool
    is_visible: bool
    opacity: float
    fill: FillData
    rotation: Axis3Data
    skew: Vector2Data
    scale: float
    shadow: ShadowData
    border: BorderData
    radius: CornersData
    script_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., value: _Optional[_Mapping[str, ValueData]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ..., layout: _Optional[_Union[Layout, str]] = ..., direction: _Optional[_Union[Direction, str]] = ..., distribute: _Optional[_Union[Distribute, str]] = ..., align: _Optional[_Union[Align, str]] = ..., gap: _Optional[_Union[Axis2Data, _Mapping]] = ..., padding: _Optional[_Union[InsetsData, _Mapping]] = ..., grid: _Optional[_Union[GridData, _Mapping]] = ..., grid_span: _Optional[_Union[GridSpanData, _Mapping]] = ..., aspect_ratio: _Optional[float] = ..., is_wrap: bool = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., fill: _Optional[_Union[FillData, _Mapping]] = ..., rotation: _Optional[_Union[Axis3Data, _Mapping]] = ..., skew: _Optional[_Union[Vector2Data, _Mapping]] = ..., scale: _Optional[float] = ..., shadow: _Optional[_Union[ShadowData, _Mapping]] = ..., border: _Optional[_Union[BorderData, _Mapping]] = ..., radius: _Optional[_Union[CornersData, _Mapping]] = ..., script_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class DatabaseData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "type", "name", "status", "target_status", "failed_at", "failed_attempts", "region", "cell_name", "external_name", "custom_schema_name", "tenancy", "connection_url")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    TARGET_STATUS_FIELD_NUMBER: _ClassVar[int]
    FAILED_AT_FIELD_NUMBER: _ClassVar[int]
    FAILED_ATTEMPTS_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    CELL_NAME_FIELD_NUMBER: _ClassVar[int]
    EXTERNAL_NAME_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_SCHEMA_NAME_FIELD_NUMBER: _ClassVar[int]
    TENANCY_FIELD_NUMBER: _ClassVar[int]
    CONNECTION_URL_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    type: DatabaseType
    name: str
    status: ResourceStatus
    target_status: _timestamp_pb2.Timestamp
    failed_at: _timestamp_pb2.Timestamp
    failed_attempts: int
    region: Region
    cell_name: str
    external_name: str
    custom_schema_name: str
    tenancy: Tenancy
    connection_url: str
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., type: _Optional[_Union[DatabaseType, str]] = ..., name: _Optional[str] = ..., status: _Optional[_Union[ResourceStatus, str]] = ..., target_status: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., failed_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., failed_attempts: _Optional[int] = ..., region: _Optional[_Union[Region, str]] = ..., cell_name: _Optional[str] = ..., external_name: _Optional[str] = ..., custom_schema_name: _Optional[str] = ..., tenancy: _Optional[_Union[Tenancy, str]] = ..., connection_url: _Optional[str] = ...) -> None: ...

class DatabaseInfoData(_message.Message):
    __slots__ = ("metatype", "type", "region", "cell_name", "external_name", "custom_schema_name", "tenancy", "connection_url")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    CELL_NAME_FIELD_NUMBER: _ClassVar[int]
    EXTERNAL_NAME_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_SCHEMA_NAME_FIELD_NUMBER: _ClassVar[int]
    TENANCY_FIELD_NUMBER: _ClassVar[int]
    CONNECTION_URL_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: DatabaseType
    region: Region
    cell_name: str
    external_name: str
    custom_schema_name: str
    tenancy: Tenancy
    connection_url: str
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[DatabaseType, str]] = ..., region: _Optional[_Union[Region, str]] = ..., cell_name: _Optional[str] = ..., external_name: _Optional[str] = ..., custom_schema_name: _Optional[str] = ..., tenancy: _Optional[_Union[Tenancy, str]] = ..., connection_url: _Optional[str] = ...) -> None: ...

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
    __slots__ = ("metatype", "id", "type", "operation", "node_ptr", "prop_ptr", "field_ptr", "key", "value", "undo")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    OPERATION_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    PROP_PTR_FIELD_NUMBER: _ClassVar[int]
    FIELD_PTR_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    UNDO_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    id: str
    type: EditType
    operation: EditOperation
    node_ptr: NodeReferenceData
    prop_ptr: PropertyReferenceData
    field_ptr: NodeReferenceData
    key: ValueData
    value: ValueData
    undo: EditData
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., id: _Optional[str] = ..., type: _Optional[_Union[EditType, str]] = ..., operation: _Optional[_Union[EditOperation, str]] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., prop_ptr: _Optional[_Union[PropertyReferenceData, _Mapping]] = ..., field_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., key: _Optional[_Union[ValueData, _Mapping]] = ..., value: _Optional[_Union[ValueData, _Mapping]] = ..., undo: _Optional[_Union[EditData, _Mapping]] = ...) -> None: ...

class EditEventData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "type", "operation", "node_ptr", "prop_ptr", "field_ptr", "key", "value")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    OPERATION_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    PROP_PTR_FIELD_NUMBER: _ClassVar[int]
    FIELD_PTR_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    type: EditType
    operation: EditOperation
    node_ptr: NodeReferenceData
    prop_ptr: PropertyReferenceData
    field_ptr: NodeReferenceData
    key: ValueData
    value: ValueData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., type: _Optional[_Union[EditType, str]] = ..., operation: _Optional[_Union[EditOperation, str]] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., prop_ptr: _Optional[_Union[PropertyReferenceData, _Mapping]] = ..., field_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., key: _Optional[_Union[ValueData, _Mapping]] = ..., value: _Optional[_Union[ValueData, _Mapping]] = ...) -> None: ...

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
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "order_key", "type", "name", "style_ptr", "opacity", "offset", "scale", "rotate", "skew", "perspective", "delay", "duration", "threshold", "once", "repeat", "split", "offscreen", "transition")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
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
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    order_key: str
    type: EffectType
    name: str
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
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[EffectType, str]] = ..., name: _Optional[str] = ..., style_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., opacity: _Optional[float] = ..., offset: _Optional[_Union[Vector2Data, _Mapping]] = ..., scale: _Optional[float] = ..., rotate: _Optional[_Union[Axis3Data, _Mapping]] = ..., skew: _Optional[_Union[Vector2Data, _Mapping]] = ..., perspective: _Optional[float] = ..., delay: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., duration: _Optional[float] = ..., threshold: _Optional[float] = ..., once: bool = ..., repeat: _Optional[_Union[RepeatType, str]] = ..., split: _Optional[_Union[TextSplitType, str]] = ..., offscreen: _Optional[_Union[OffscreenBehavior, str]] = ..., transition: _Optional[_Union[TransitionData, _Mapping]] = ...) -> None: ...

class EnumDefinitionData(_message.Message):
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
    options: _containers.RepeatedCompositeFieldContainer[EnumOptionDefinitionData]
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., id: _Optional[int] = ..., type: _Optional[_Union[EnumType, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., description: _Optional[str] = ..., options: _Optional[_Iterable[_Union[EnumOptionDefinitionData, _Mapping]]] = ...) -> None: ...

class EnumOptionDefinitionData(_message.Message):
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

class EnvironmentData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "name", "icon")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    name: str
    icon: IconData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ...) -> None: ...

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

class EventCursorData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "owned_by_ptr", "status", "active_at")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_AT_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    status: CursorStatus
    active_at: _timestamp_pb2.Timestamp
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., status: _Optional[_Union[CursorStatus, str]] = ..., active_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...) -> None: ...

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
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "order_key", "type", "name", "icon", "cardinality", "scalar_type", "primitive_type", "enum_type", "node_type", "node_definition_ptr", "struct_type", "base_type_ptr", "key_type", "is_required", "default", "default_factory", "collection_constraint", "string_constraint", "number_constraint", "node_constraint", "edge_type", "cascade", "source_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    CARDINALITY_FIELD_NUMBER: _ClassVar[int]
    SCALAR_TYPE_FIELD_NUMBER: _ClassVar[int]
    PRIMITIVE_TYPE_FIELD_NUMBER: _ClassVar[int]
    ENUM_TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    STRUCT_TYPE_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_PTR_FIELD_NUMBER: _ClassVar[int]
    KEY_TYPE_FIELD_NUMBER: _ClassVar[int]
    IS_REQUIRED_FIELD_NUMBER: _ClassVar[int]
    DEFAULT_FIELD_NUMBER: _ClassVar[int]
    DEFAULT_FACTORY_FIELD_NUMBER: _ClassVar[int]
    COLLECTION_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    STRING_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    NUMBER_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    NODE_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    EDGE_TYPE_FIELD_NUMBER: _ClassVar[int]
    CASCADE_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    order_key: str
    type: FieldType
    name: str
    icon: IconData
    cardinality: TypeCardinality
    scalar_type: ScalarType
    primitive_type: PrimitiveType
    enum_type: EnumType
    node_type: NodeType
    node_definition_ptr: NodeReferenceData
    struct_type: StructType
    base_type_ptr: NodeReferenceData
    key_type: TypeData
    is_required: bool
    default: ValueData
    default_factory: DefaultFactory
    collection_constraint: CollectionConstraintData
    string_constraint: StringConstraintData
    number_constraint: NumberConstraintData
    node_constraint: NodeConstraintData
    edge_type: EdgeType
    cascade: CascadeAction
    source_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[FieldType, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., cardinality: _Optional[_Union[TypeCardinality, str]] = ..., scalar_type: _Optional[_Union[ScalarType, str]] = ..., primitive_type: _Optional[_Union[PrimitiveType, str]] = ..., enum_type: _Optional[_Union[EnumType, str]] = ..., node_type: _Optional[_Union[NodeType, str]] = ..., node_definition_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., struct_type: _Optional[_Union[StructType, str]] = ..., base_type_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., key_type: _Optional[_Union[TypeData, _Mapping]] = ..., is_required: bool = ..., default: _Optional[_Union[ValueData, _Mapping]] = ..., default_factory: _Optional[_Union[DefaultFactory, str]] = ..., collection_constraint: _Optional[_Union[CollectionConstraintData, _Mapping]] = ..., string_constraint: _Optional[_Union[StringConstraintData, _Mapping]] = ..., number_constraint: _Optional[_Union[NumberConstraintData, _Mapping]] = ..., node_constraint: _Optional[_Union[NodeConstraintData, _Mapping]] = ..., edge_type: _Optional[_Union[EdgeType, str]] = ..., cascade: _Optional[_Union[CascadeAction, str]] = ..., source_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class FileData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "type", "name", "status", "target_status", "failed_at", "failed_attempts", "source", "mime_type", "format", "size", "sha256", "width", "height", "aspect_ratio", "codec", "duration", "url", "content_url", "thumbnail_url", "favicon_url", "thumbnail_width", "thumbnail_height", "content", "retention", "expires_at")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    TARGET_STATUS_FIELD_NUMBER: _ClassVar[int]
    FAILED_AT_FIELD_NUMBER: _ClassVar[int]
    FAILED_ATTEMPTS_FIELD_NUMBER: _ClassVar[int]
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
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    type: FileType
    name: str
    status: ResourceStatus
    target_status: _timestamp_pb2.Timestamp
    failed_at: _timestamp_pb2.Timestamp
    failed_attempts: int
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
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., type: _Optional[_Union[FileType, str]] = ..., name: _Optional[str] = ..., status: _Optional[_Union[ResourceStatus, str]] = ..., target_status: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., failed_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., failed_attempts: _Optional[int] = ..., source: _Optional[_Union[FileSource, str]] = ..., mime_type: _Optional[str] = ..., format: _Optional[_Union[FileFormat, str]] = ..., size: _Optional[int] = ..., sha256: _Optional[str] = ..., width: _Optional[int] = ..., height: _Optional[int] = ..., aspect_ratio: _Optional[float] = ..., codec: _Optional[str] = ..., duration: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., url: _Optional[str] = ..., content_url: _Optional[str] = ..., thumbnail_url: _Optional[str] = ..., favicon_url: _Optional[str] = ..., thumbnail_width: _Optional[int] = ..., thumbnail_height: _Optional[int] = ..., content: _Optional[bytes] = ..., retention: _Optional[_Union[FileRetentionMode, str]] = ..., expires_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...) -> None: ...

class FillData(_message.Message):
    __slots__ = ("metatype", "type", "style_ptr", "color", "gradient", "image_ptr", "position", "size")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    STYLE_PTR_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    GRADIENT_FIELD_NUMBER: _ClassVar[int]
    IMAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    SIZE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: FillType
    style_ptr: NodeReferenceData
    color: ColorData
    gradient: GradientData
    image_ptr: NodeReferenceData
    position: FillPosition
    size: FillSize
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[FillType, str]] = ..., style_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., color: _Optional[_Union[ColorData, _Mapping]] = ..., gradient: _Optional[_Union[GradientData, _Mapping]] = ..., image_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., position: _Optional[_Union[FillPosition, str]] = ..., size: _Optional[_Union[FillSize, str]] = ...) -> None: ...

class FillStyleData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "order_key", "type", "name", "style_ptr", "color", "gradient", "image_ptr", "position", "size")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    STYLE_PTR_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    GRADIENT_FIELD_NUMBER: _ClassVar[int]
    IMAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    SIZE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    order_key: str
    type: FillType
    name: str
    style_ptr: NodeReferenceData
    color: ColorData
    gradient: GradientData
    image_ptr: NodeReferenceData
    position: FillPosition
    size: FillSize
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[FillType, str]] = ..., name: _Optional[str] = ..., style_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., color: _Optional[_Union[ColorData, _Mapping]] = ..., gradient: _Optional[_Union[GradientData, _Mapping]] = ..., image_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., position: _Optional[_Union[FillPosition, str]] = ..., size: _Optional[_Union[FillSize, str]] = ...) -> None: ...

class FolderData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "order_key", "owned_by_ptr", "type", "name", "slug", "icon", "main_scene_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    MAIN_SCENE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    order_key: str
    owned_by_ptr: NodeReferenceData
    type: FolderType
    name: str
    slug: str
    icon: IconData
    main_scene_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., type: _Optional[_Union[FolderType, str]] = ..., name: _Optional[str] = ..., slug: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., main_scene_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class FollowData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "owned_by_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

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
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "order_key", "type", "name", "style_ptr", "weight", "color", "size", "align", "line_height", "letter_spacing", "decoration", "transform")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
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
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    order_key: str
    type: FontType
    name: str
    style_ptr: NodeReferenceData
    weight: FontWeight
    color: FillData
    size: FontSize
    align: TextAlign
    line_height: LengthData
    letter_spacing: LengthData
    decoration: TextDecoration
    transform: TextTransform
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[FontType, str]] = ..., name: _Optional[str] = ..., style_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., weight: _Optional[_Union[FontWeight, str]] = ..., color: _Optional[_Union[FillData, _Mapping]] = ..., size: _Optional[_Union[FontSize, str]] = ..., align: _Optional[_Union[TextAlign, str]] = ..., line_height: _Optional[_Union[LengthData, _Mapping]] = ..., letter_spacing: _Optional[_Union[LengthData, _Mapping]] = ..., decoration: _Optional[_Union[TextDecoration, str]] = ..., transform: _Optional[_Union[TextTransform, str]] = ...) -> None: ...

class FrameViewData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "value", "order_key", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "layout", "direction", "distribute", "align", "gap", "padding", "grid", "grid_span", "aspect_ratio", "is_wrap", "is_visible", "opacity", "fill", "rotation", "skew", "scale", "shadow", "border", "radius", "script_ptr")
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
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    LAYOUT_FIELD_NUMBER: _ClassVar[int]
    DIRECTION_FIELD_NUMBER: _ClassVar[int]
    DISTRIBUTE_FIELD_NUMBER: _ClassVar[int]
    ALIGN_FIELD_NUMBER: _ClassVar[int]
    GAP_FIELD_NUMBER: _ClassVar[int]
    PADDING_FIELD_NUMBER: _ClassVar[int]
    GRID_FIELD_NUMBER: _ClassVar[int]
    GRID_SPAN_FIELD_NUMBER: _ClassVar[int]
    ASPECT_RATIO_FIELD_NUMBER: _ClassVar[int]
    IS_WRAP_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_FIELD_NUMBER: _ClassVar[int]
    FILL_FIELD_NUMBER: _ClassVar[int]
    ROTATION_FIELD_NUMBER: _ClassVar[int]
    SKEW_FIELD_NUMBER: _ClassVar[int]
    SCALE_FIELD_NUMBER: _ClassVar[int]
    SHADOW_FIELD_NUMBER: _ClassVar[int]
    BORDER_FIELD_NUMBER: _ClassVar[int]
    RADIUS_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    value: _containers.MessageMap[str, ValueData]
    order_key: str
    name: str
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    layout: Layout
    direction: Direction
    distribute: Distribute
    align: Align
    gap: Axis2Data
    padding: InsetsData
    grid: GridData
    grid_span: GridSpanData
    aspect_ratio: float
    is_wrap: bool
    is_visible: bool
    opacity: float
    fill: FillData
    rotation: Axis3Data
    skew: Vector2Data
    scale: float
    shadow: ShadowData
    border: BorderData
    radius: CornersData
    script_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., value: _Optional[_Mapping[str, ValueData]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ..., layout: _Optional[_Union[Layout, str]] = ..., direction: _Optional[_Union[Direction, str]] = ..., distribute: _Optional[_Union[Distribute, str]] = ..., align: _Optional[_Union[Align, str]] = ..., gap: _Optional[_Union[Axis2Data, _Mapping]] = ..., padding: _Optional[_Union[InsetsData, _Mapping]] = ..., grid: _Optional[_Union[GridData, _Mapping]] = ..., grid_span: _Optional[_Union[GridSpanData, _Mapping]] = ..., aspect_ratio: _Optional[float] = ..., is_wrap: bool = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., fill: _Optional[_Union[FillData, _Mapping]] = ..., rotation: _Optional[_Union[Axis3Data, _Mapping]] = ..., skew: _Optional[_Union[Vector2Data, _Mapping]] = ..., scale: _Optional[float] = ..., shadow: _Optional[_Union[ShadowData, _Mapping]] = ..., border: _Optional[_Union[BorderData, _Mapping]] = ..., radius: _Optional[_Union[CornersData, _Mapping]] = ..., script_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

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
    __slots__ = ("metatype", "id", "parent_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "owned_by_ptr", "member_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MEMBER_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    member_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., member_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class FriendshipInviteEventData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "type", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    type: FriendshipInviteEventType
    node_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., type: _Optional[_Union[FriendshipInviteEventType, str]] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

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

class GaugeMeasurementData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "definition_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    definition_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class GaugeMetricData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "order_key", "name", "source_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    order_key: str
    name: str
    source_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., source_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

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
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "order_key", "type", "name", "style_ptr", "angle", "stops", "center_anchor", "dark")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    STYLE_PTR_FIELD_NUMBER: _ClassVar[int]
    ANGLE_FIELD_NUMBER: _ClassVar[int]
    STOPS_FIELD_NUMBER: _ClassVar[int]
    CENTER_ANCHOR_FIELD_NUMBER: _ClassVar[int]
    DARK_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    order_key: str
    type: GradientType
    name: str
    style_ptr: NodeReferenceData
    angle: float
    stops: _containers.RepeatedCompositeFieldContainer[GradientStopData]
    center_anchor: Axis2Data
    dark: GradientData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[GradientType, str]] = ..., name: _Optional[str] = ..., style_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., angle: _Optional[float] = ..., stops: _Optional[_Iterable[_Union[GradientStopData, _Mapping]]] = ..., center_anchor: _Optional[_Union[Axis2Data, _Mapping]] = ..., dark: _Optional[_Union[GradientData, _Mapping]] = ...) -> None: ...

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
    __slots__ = ("metatype", "id", "parent_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "slug")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    slug: str
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., slug: _Optional[str] = ...) -> None: ...

class HistogramData(_message.Message):
    __slots__ = ("metatype", "buckets", "counts")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    BUCKETS_FIELD_NUMBER: _ClassVar[int]
    COUNTS_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    buckets: _containers.RepeatedCompositeFieldContainer[ValueData]
    counts: _containers.RepeatedScalarFieldContainer[int]
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., buckets: _Optional[_Iterable[_Union[ValueData, _Mapping]]] = ..., counts: _Optional[_Iterable[int]] = ...) -> None: ...

class HistogramMeasurementData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "definition_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    definition_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class HistogramMetricData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "order_key", "name", "source_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    order_key: str
    name: str
    source_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., source_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

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
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "value", "type", "runnable_ptr", "span_ptr", "status", "duration", "closed_at", "response", "message_ptr")
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
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    RUNNABLE_PTR_FIELD_NUMBER: _ClassVar[int]
    SPAN_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    CLOSED_AT_FIELD_NUMBER: _ClassVar[int]
    RESPONSE_FIELD_NUMBER: _ClassVar[int]
    MESSAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    value: _containers.MessageMap[str, ValueData]
    type: InterruptionType
    runnable_ptr: NodeReferenceData
    span_ptr: NodeReferenceData
    status: InterruptionStatus
    duration: _duration_pb2.Duration
    closed_at: _timestamp_pb2.Timestamp
    response: InterruptionResponse
    message_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., value: _Optional[_Mapping[str, ValueData]] = ..., type: _Optional[_Union[InterruptionType, str]] = ..., runnable_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., span_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., status: _Optional[_Union[InterruptionStatus, str]] = ..., duration: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., closed_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., response: _Optional[_Union[InterruptionResponse, str]] = ..., message_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class InviteData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "owned_by_ptr", "member_ptr", "role_ptr", "role_type")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MEMBER_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLE_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLE_TYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    member_ptr: NodeReferenceData
    role_ptr: NodeReferenceData
    role_type: RoleType
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., member_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., role_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., role_type: _Optional[_Union[RoleType, str]] = ...) -> None: ...

class InviteEventData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "node_ptr", "joinable_ptr", "member_ptr", "role_ptr", "role_type")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    JOINABLE_PTR_FIELD_NUMBER: _ClassVar[int]
    MEMBER_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLE_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLE_TYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    node_ptr: NodeReferenceData
    joinable_ptr: NodeReferenceData
    member_ptr: NodeReferenceData
    role_ptr: NodeReferenceData
    role_type: RoleType
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., joinable_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., member_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., role_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., role_type: _Optional[_Union[RoleType, str]] = ...) -> None: ...

class JoinData(_message.Message):
    __slots__ = ("metatype", "type", "relation", "recursive", "depth", "on")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    RELATION_FIELD_NUMBER: _ClassVar[int]
    RECURSIVE_FIELD_NUMBER: _ClassVar[int]
    DEPTH_FIELD_NUMBER: _ClassVar[int]
    ON_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: JoinType
    relation: RelationReferenceData
    recursive: bool
    depth: int
    on: ConditionData
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[JoinType, str]] = ..., relation: _Optional[_Union[RelationReferenceData, _Mapping]] = ..., recursive: bool = ..., depth: _Optional[int] = ..., on: _Optional[_Union[ConditionData, _Mapping]] = ...) -> None: ...

class LabelViewData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "value", "order_key", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "layout", "direction", "distribute", "align", "gap", "padding", "grid", "grid_span", "aspect_ratio", "is_wrap", "is_visible", "opacity", "fill", "rotation", "skew", "scale", "shadow", "border", "radius", "script_ptr")
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
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    LAYOUT_FIELD_NUMBER: _ClassVar[int]
    DIRECTION_FIELD_NUMBER: _ClassVar[int]
    DISTRIBUTE_FIELD_NUMBER: _ClassVar[int]
    ALIGN_FIELD_NUMBER: _ClassVar[int]
    GAP_FIELD_NUMBER: _ClassVar[int]
    PADDING_FIELD_NUMBER: _ClassVar[int]
    GRID_FIELD_NUMBER: _ClassVar[int]
    GRID_SPAN_FIELD_NUMBER: _ClassVar[int]
    ASPECT_RATIO_FIELD_NUMBER: _ClassVar[int]
    IS_WRAP_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_FIELD_NUMBER: _ClassVar[int]
    FILL_FIELD_NUMBER: _ClassVar[int]
    ROTATION_FIELD_NUMBER: _ClassVar[int]
    SKEW_FIELD_NUMBER: _ClassVar[int]
    SCALE_FIELD_NUMBER: _ClassVar[int]
    SHADOW_FIELD_NUMBER: _ClassVar[int]
    BORDER_FIELD_NUMBER: _ClassVar[int]
    RADIUS_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    value: _containers.MessageMap[str, ValueData]
    order_key: str
    name: str
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    layout: Layout
    direction: Direction
    distribute: Distribute
    align: Align
    gap: Axis2Data
    padding: InsetsData
    grid: GridData
    grid_span: GridSpanData
    aspect_ratio: float
    is_wrap: bool
    is_visible: bool
    opacity: float
    fill: FillData
    rotation: Axis3Data
    skew: Vector2Data
    scale: float
    shadow: ShadowData
    border: BorderData
    radius: CornersData
    script_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., value: _Optional[_Mapping[str, ValueData]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ..., layout: _Optional[_Union[Layout, str]] = ..., direction: _Optional[_Union[Direction, str]] = ..., distribute: _Optional[_Union[Distribute, str]] = ..., align: _Optional[_Union[Align, str]] = ..., gap: _Optional[_Union[Axis2Data, _Mapping]] = ..., padding: _Optional[_Union[InsetsData, _Mapping]] = ..., grid: _Optional[_Union[GridData, _Mapping]] = ..., grid_span: _Optional[_Union[GridSpanData, _Mapping]] = ..., aspect_ratio: _Optional[float] = ..., is_wrap: bool = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., fill: _Optional[_Union[FillData, _Mapping]] = ..., rotation: _Optional[_Union[Axis3Data, _Mapping]] = ..., skew: _Optional[_Union[Vector2Data, _Mapping]] = ..., scale: _Optional[float] = ..., shadow: _Optional[_Union[ShadowData, _Mapping]] = ..., border: _Optional[_Union[BorderData, _Mapping]] = ..., radius: _Optional[_Union[CornersData, _Mapping]] = ..., script_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class LayerData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "value", "order_key", "owned_by_ptr", "type", "name", "icon", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "layout", "direction", "distribute", "align", "gap", "padding", "grid", "grid_span", "aspect_ratio", "is_wrap", "is_visible", "opacity", "fill", "rotation", "skew", "scale", "shadow", "border", "radius", "script_ptr")
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
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    LAYOUT_FIELD_NUMBER: _ClassVar[int]
    DIRECTION_FIELD_NUMBER: _ClassVar[int]
    DISTRIBUTE_FIELD_NUMBER: _ClassVar[int]
    ALIGN_FIELD_NUMBER: _ClassVar[int]
    GAP_FIELD_NUMBER: _ClassVar[int]
    PADDING_FIELD_NUMBER: _ClassVar[int]
    GRID_FIELD_NUMBER: _ClassVar[int]
    GRID_SPAN_FIELD_NUMBER: _ClassVar[int]
    ASPECT_RATIO_FIELD_NUMBER: _ClassVar[int]
    IS_WRAP_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_FIELD_NUMBER: _ClassVar[int]
    FILL_FIELD_NUMBER: _ClassVar[int]
    ROTATION_FIELD_NUMBER: _ClassVar[int]
    SKEW_FIELD_NUMBER: _ClassVar[int]
    SCALE_FIELD_NUMBER: _ClassVar[int]
    SHADOW_FIELD_NUMBER: _ClassVar[int]
    BORDER_FIELD_NUMBER: _ClassVar[int]
    RADIUS_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    value: _containers.MessageMap[str, ValueData]
    order_key: str
    owned_by_ptr: NodeReferenceData
    type: LayerType
    name: str
    icon: IconData
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    layout: Layout
    direction: Direction
    distribute: Distribute
    align: Align
    gap: Axis2Data
    padding: InsetsData
    grid: GridData
    grid_span: GridSpanData
    aspect_ratio: float
    is_wrap: bool
    is_visible: bool
    opacity: float
    fill: FillData
    rotation: Axis3Data
    skew: Vector2Data
    scale: float
    shadow: ShadowData
    border: BorderData
    radius: CornersData
    script_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., value: _Optional[_Mapping[str, ValueData]] = ..., order_key: _Optional[str] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., type: _Optional[_Union[LayerType, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ..., layout: _Optional[_Union[Layout, str]] = ..., direction: _Optional[_Union[Direction, str]] = ..., distribute: _Optional[_Union[Distribute, str]] = ..., align: _Optional[_Union[Align, str]] = ..., gap: _Optional[_Union[Axis2Data, _Mapping]] = ..., padding: _Optional[_Union[InsetsData, _Mapping]] = ..., grid: _Optional[_Union[GridData, _Mapping]] = ..., grid_span: _Optional[_Union[GridSpanData, _Mapping]] = ..., aspect_ratio: _Optional[float] = ..., is_wrap: bool = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., fill: _Optional[_Union[FillData, _Mapping]] = ..., rotation: _Optional[_Union[Axis3Data, _Mapping]] = ..., skew: _Optional[_Union[Vector2Data, _Mapping]] = ..., scale: _Optional[float] = ..., shadow: _Optional[_Union[ShadowData, _Mapping]] = ..., border: _Optional[_Union[BorderData, _Mapping]] = ..., radius: _Optional[_Union[CornersData, _Mapping]] = ..., script_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class LengthData(_message.Message):
    __slots__ = ("metatype", "unit", "value")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    UNIT_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    unit: LengthUnit
    value: float
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., unit: _Optional[_Union[LengthUnit, str]] = ..., value: _Optional[float] = ...) -> None: ...

class LineShapeData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "order_key", "type", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "align", "is_visible", "opacity", "points", "color", "script_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    ALIGN_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_FIELD_NUMBER: _ClassVar[int]
    POINTS_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    order_key: str
    type: LineType
    name: str
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    align: Align
    is_visible: bool
    opacity: float
    points: _containers.RepeatedCompositeFieldContainer[Vector2Data]
    color: ColorData
    script_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[LineType, str]] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ..., align: _Optional[_Union[Align, str]] = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., points: _Optional[_Iterable[_Union[Vector2Data, _Mapping]]] = ..., color: _Optional[_Union[ColorData, _Mapping]] = ..., script_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class LinkData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "type", "status", "target_status", "failed_at", "failed_attempts", "url", "domain", "content_url", "thumbnail_url", "favicon_url", "thumbnail_width", "thumbnail_height", "content", "attribution", "attribution_tag", "published_at", "expires_at", "image_urls")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    TARGET_STATUS_FIELD_NUMBER: _ClassVar[int]
    FAILED_AT_FIELD_NUMBER: _ClassVar[int]
    FAILED_ATTEMPTS_FIELD_NUMBER: _ClassVar[int]
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
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    type: LinkType
    status: ResourceStatus
    target_status: _timestamp_pb2.Timestamp
    failed_at: _timestamp_pb2.Timestamp
    failed_attempts: int
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
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., type: _Optional[_Union[LinkType, str]] = ..., status: _Optional[_Union[ResourceStatus, str]] = ..., target_status: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., failed_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., failed_attempts: _Optional[int] = ..., url: _Optional[str] = ..., domain: _Optional[str] = ..., content_url: _Optional[str] = ..., thumbnail_url: _Optional[str] = ..., favicon_url: _Optional[str] = ..., thumbnail_width: _Optional[int] = ..., thumbnail_height: _Optional[int] = ..., content: _Optional[str] = ..., attribution: _Optional[str] = ..., attribution_tag: _Optional[str] = ..., published_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., expires_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., image_urls: _Optional[_Iterable[str]] = ...) -> None: ...

class LogData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "content", "attributes")
    class AttributesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: _struct_pb2.Value
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    CONTENT_FIELD_NUMBER: _ClassVar[int]
    ATTRIBUTES_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    content: str
    attributes: _containers.MessageMap[str, _struct_pb2.Value]
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., content: _Optional[str] = ..., attributes: _Optional[_Mapping[str, _struct_pb2.Value]] = ...) -> None: ...

class MachineData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "type", "status", "target_status", "failed_at", "failed_attempts", "version", "external_name", "external_id", "image_id", "grpc_url", "vnc_url", "client_ptr", "cpu", "ram", "width", "height", "is_headless")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    TARGET_STATUS_FIELD_NUMBER: _ClassVar[int]
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
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    type: MachineType
    status: ResourceStatus
    target_status: _timestamp_pb2.Timestamp
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
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., type: _Optional[_Union[MachineType, str]] = ..., status: _Optional[_Union[ResourceStatus, str]] = ..., target_status: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., failed_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., failed_attempts: _Optional[int] = ..., version: _Optional[str] = ..., external_name: _Optional[str] = ..., external_id: _Optional[str] = ..., image_id: _Optional[str] = ..., grpc_url: _Optional[str] = ..., vnc_url: _Optional[str] = ..., client_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., cpu: _Optional[float] = ..., ram: _Optional[float] = ..., width: _Optional[int] = ..., height: _Optional[int] = ..., is_headless: bool = ...) -> None: ...

class MembershipData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "owned_by_ptr", "member_ptr", "role_ptr", "role_type")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MEMBER_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLE_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLE_TYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    member_ptr: NodeReferenceData
    role_ptr: NodeReferenceData
    role_type: RoleType
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., member_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., role_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., role_type: _Optional[_Union[RoleType, str]] = ...) -> None: ...

class MembershipEventData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "node_ptr", "joinable_ptr", "member_ptr", "role_ptr", "role_type")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    JOINABLE_PTR_FIELD_NUMBER: _ClassVar[int]
    MEMBER_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLE_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLE_TYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    node_ptr: NodeReferenceData
    joinable_ptr: NodeReferenceData
    member_ptr: NodeReferenceData
    role_ptr: NodeReferenceData
    role_type: RoleType
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., joinable_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., member_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., role_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., role_type: _Optional[_Union[RoleType, str]] = ...) -> None: ...

class MessageData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "owned_by_ptr", "thread_ptr", "edited_at", "reply_to_ptr", "forwarded_from_ptr", "text", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    THREAD_PTR_FIELD_NUMBER: _ClassVar[int]
    EDITED_AT_FIELD_NUMBER: _ClassVar[int]
    REPLY_TO_PTR_FIELD_NUMBER: _ClassVar[int]
    FORWARDED_FROM_PTR_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    thread_ptr: NodeReferenceData
    edited_at: _timestamp_pb2.Timestamp
    reply_to_ptr: NodeReferenceData
    forwarded_from_ptr: NodeReferenceData
    text: TextData
    node_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., thread_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., edited_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., reply_to_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., forwarded_from_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., text: _Optional[_Union[TextData, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class NodeConstraintData(_message.Message):
    __slots__ = ("metatype", "node_types", "node_traits")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPES_FIELD_NUMBER: _ClassVar[int]
    NODE_TRAITS_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    node_types: _containers.RepeatedScalarFieldContainer[NodeType]
    node_traits: _containers.RepeatedScalarFieldContainer[TraitType]
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., node_types: _Optional[_Iterable[_Union[NodeType, str]]] = ..., node_traits: _Optional[_Iterable[_Union[TraitType, str]]] = ...) -> None: ...

class NodeDefinitionData(_message.Message):
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
    properties: _containers.RepeatedCompositeFieldContainer[PropertyDefinitionData]
    traits: _containers.RepeatedScalarFieldContainer[TraitType]
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., id: _Optional[int] = ..., type: _Optional[_Union[NodeType, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., description: _Optional[str] = ..., properties: _Optional[_Iterable[_Union[PropertyDefinitionData, _Mapping]]] = ..., traits: _Optional[_Iterable[_Union[TraitType, str]]] = ...) -> None: ...

class NodeReferenceData(_message.Message):
    __slots__ = ("metatype", "node_type", "id", "space_id", "definition_id")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    SPACE_ID_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_ID_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    node_type: NodeType
    id: str
    space_id: str
    definition_id: str
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., node_type: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., space_id: _Optional[str] = ..., definition_id: _Optional[str] = ...) -> None: ...

class NotificationData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "owned_by_ptr", "status", "title", "text")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    status: NotificationStatus
    title: str
    text: TextData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., status: _Optional[_Union[NotificationStatus, str]] = ..., title: _Optional[str] = ..., text: _Optional[_Union[TextData, _Mapping]] = ...) -> None: ...

class NotificationEventData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "type", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    type: NotificationEventType
    node_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., type: _Optional[_Union[NotificationEventType, str]] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

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
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "order_key", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "is_visible", "opacity", "value", "placeholder", "script_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    PLACEHOLDER_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    order_key: str
    name: str
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    is_visible: bool
    opacity: float
    value: str
    placeholder: str
    script_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., value: _Optional[str] = ..., placeholder: _Optional[str] = ..., script_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class OptionData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "order_key", "name", "icon", "source_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    order_key: str
    name: str
    icon: IconData
    source_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., source_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class OrganizationData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "name", "slug", "icon", "status", "space_ptr", "handle_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    HANDLE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    name: str
    slug: str
    icon: IconData
    status: OrganizationStatus
    space_ptr: NodeReferenceData
    handle_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., name: _Optional[str] = ..., slug: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., status: _Optional[_Union[OrganizationStatus, str]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., handle_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

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

class PermissionDefinitionData(_message.Message):
    __slots__ = ("metatype", "id", "type", "name", "node_type", "icon")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    id: int
    type: EnumType
    name: str
    node_type: NodeType
    icon: IconData
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., id: _Optional[int] = ..., type: _Optional[_Union[EnumType, str]] = ..., name: _Optional[str] = ..., node_type: _Optional[_Union[NodeType, str]] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ...) -> None: ...

class PlaneShapeData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "value", "order_key", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "layout", "direction", "distribute", "align", "gap", "padding", "grid", "grid_span", "aspect_ratio", "is_wrap", "is_visible", "opacity", "fill", "rotation", "skew", "scale", "shadow", "border", "radius", "points", "script_ptr")
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
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    LAYOUT_FIELD_NUMBER: _ClassVar[int]
    DIRECTION_FIELD_NUMBER: _ClassVar[int]
    DISTRIBUTE_FIELD_NUMBER: _ClassVar[int]
    ALIGN_FIELD_NUMBER: _ClassVar[int]
    GAP_FIELD_NUMBER: _ClassVar[int]
    PADDING_FIELD_NUMBER: _ClassVar[int]
    GRID_FIELD_NUMBER: _ClassVar[int]
    GRID_SPAN_FIELD_NUMBER: _ClassVar[int]
    ASPECT_RATIO_FIELD_NUMBER: _ClassVar[int]
    IS_WRAP_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_FIELD_NUMBER: _ClassVar[int]
    FILL_FIELD_NUMBER: _ClassVar[int]
    ROTATION_FIELD_NUMBER: _ClassVar[int]
    SKEW_FIELD_NUMBER: _ClassVar[int]
    SCALE_FIELD_NUMBER: _ClassVar[int]
    SHADOW_FIELD_NUMBER: _ClassVar[int]
    BORDER_FIELD_NUMBER: _ClassVar[int]
    RADIUS_FIELD_NUMBER: _ClassVar[int]
    POINTS_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    value: _containers.MessageMap[str, ValueData]
    order_key: str
    name: str
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    layout: Layout
    direction: Direction
    distribute: Distribute
    align: Align
    gap: Axis2Data
    padding: InsetsData
    grid: GridData
    grid_span: GridSpanData
    aspect_ratio: float
    is_wrap: bool
    is_visible: bool
    opacity: float
    fill: FillData
    rotation: Axis3Data
    skew: Vector2Data
    scale: float
    shadow: ShadowData
    border: BorderData
    radius: CornersData
    points: _containers.RepeatedCompositeFieldContainer[Vector2Data]
    script_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., value: _Optional[_Mapping[str, ValueData]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ..., layout: _Optional[_Union[Layout, str]] = ..., direction: _Optional[_Union[Direction, str]] = ..., distribute: _Optional[_Union[Distribute, str]] = ..., align: _Optional[_Union[Align, str]] = ..., gap: _Optional[_Union[Axis2Data, _Mapping]] = ..., padding: _Optional[_Union[InsetsData, _Mapping]] = ..., grid: _Optional[_Union[GridData, _Mapping]] = ..., grid_span: _Optional[_Union[GridSpanData, _Mapping]] = ..., aspect_ratio: _Optional[float] = ..., is_wrap: bool = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., fill: _Optional[_Union[FillData, _Mapping]] = ..., rotation: _Optional[_Union[Axis3Data, _Mapping]] = ..., skew: _Optional[_Union[Vector2Data, _Mapping]] = ..., scale: _Optional[float] = ..., shadow: _Optional[_Union[ShadowData, _Mapping]] = ..., border: _Optional[_Union[BorderData, _Mapping]] = ..., radius: _Optional[_Union[CornersData, _Mapping]] = ..., points: _Optional[_Iterable[_Union[Vector2Data, _Mapping]]] = ..., script_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

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

class PropertyDefinitionData(_message.Message):
    __slots__ = ("metatype", "id", "name", "icon", "description", "cardinality", "scalar_type", "primitive_type", "enum_type", "node_type", "struct_type", "key_type", "is_required", "is_variable", "default", "default_factory", "collection_constraint", "string_constraint", "number_constraint", "node_constraint", "node_is_customizable", "edge_type", "cascade", "is_wired", "is_stored", "is_repr", "is_hash", "is_eq", "is_managed", "is_computed")
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
    KEY_TYPE_FIELD_NUMBER: _ClassVar[int]
    IS_REQUIRED_FIELD_NUMBER: _ClassVar[int]
    IS_VARIABLE_FIELD_NUMBER: _ClassVar[int]
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
    key_type: TypeData
    is_required: bool
    is_variable: bool
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
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., id: _Optional[int] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., description: _Optional[str] = ..., cardinality: _Optional[_Union[TypeCardinality, str]] = ..., scalar_type: _Optional[_Union[ScalarType, str]] = ..., primitive_type: _Optional[_Union[PrimitiveType, str]] = ..., enum_type: _Optional[_Union[EnumType, str]] = ..., node_type: _Optional[_Union[NodeType, str]] = ..., struct_type: _Optional[_Union[StructType, str]] = ..., key_type: _Optional[_Union[TypeData, _Mapping]] = ..., is_required: bool = ..., is_variable: bool = ..., default: _Optional[_Union[ValueData, _Mapping]] = ..., default_factory: _Optional[_Union[DefaultFactory, str]] = ..., collection_constraint: _Optional[_Union[CollectionConstraintData, _Mapping]] = ..., string_constraint: _Optional[_Union[StringConstraintData, _Mapping]] = ..., number_constraint: _Optional[_Union[NumberConstraintData, _Mapping]] = ..., node_constraint: _Optional[_Union[NodeConstraintData, _Mapping]] = ..., node_is_customizable: bool = ..., edge_type: _Optional[_Union[EdgeType, str]] = ..., cascade: _Optional[_Union[CascadeAction, str]] = ..., is_wired: bool = ..., is_stored: bool = ..., is_repr: bool = ..., is_hash: bool = ..., is_eq: bool = ..., is_managed: bool = ..., is_computed: bool = ...) -> None: ...

class PropertyReferenceData(_message.Message):
    __slots__ = ("metatype", "type", "node_type", "trait_type", "struct_type", "id")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    TRAIT_TYPE_FIELD_NUMBER: _ClassVar[int]
    STRUCT_TYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: PropertyReferenceType
    node_type: NodeType
    trait_type: TraitType
    struct_type: StructType
    id: int
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[PropertyReferenceType, str]] = ..., node_type: _Optional[_Union[NodeType, str]] = ..., trait_type: _Optional[_Union[TraitType, str]] = ..., struct_type: _Optional[_Union[StructType, str]] = ..., id: _Optional[int] = ...) -> None: ...

class QueryData(_message.Message):
    __slots__ = ("metatype", "id", "type", "name", "relation", "join", "select", "subqueries", "where", "having", "group_by", "aggregation", "sort", "limit", "offset")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    RELATION_FIELD_NUMBER: _ClassVar[int]
    JOIN_FIELD_NUMBER: _ClassVar[int]
    SELECT_FIELD_NUMBER: _ClassVar[int]
    SUBQUERIES_FIELD_NUMBER: _ClassVar[int]
    WHERE_FIELD_NUMBER: _ClassVar[int]
    HAVING_FIELD_NUMBER: _ClassVar[int]
    GROUP_BY_FIELD_NUMBER: _ClassVar[int]
    AGGREGATION_FIELD_NUMBER: _ClassVar[int]
    SORT_FIELD_NUMBER: _ClassVar[int]
    LIMIT_FIELD_NUMBER: _ClassVar[int]
    OFFSET_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    id: str
    type: QueryType
    name: str
    relation: RelationReferenceData
    join: JoinData
    select: SelectData
    subqueries: _containers.RepeatedCompositeFieldContainer[QueryData]
    where: ConditionData
    having: ConditionData
    group_by: _containers.RepeatedCompositeFieldContainer[ExpressionData]
    aggregation: AggregationData
    sort: _containers.RepeatedCompositeFieldContainer[SortData]
    limit: int
    offset: int
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., id: _Optional[str] = ..., type: _Optional[_Union[QueryType, str]] = ..., name: _Optional[str] = ..., relation: _Optional[_Union[RelationReferenceData, _Mapping]] = ..., join: _Optional[_Union[JoinData, _Mapping]] = ..., select: _Optional[_Union[SelectData, _Mapping]] = ..., subqueries: _Optional[_Iterable[_Union[QueryData, _Mapping]]] = ..., where: _Optional[_Union[ConditionData, _Mapping]] = ..., having: _Optional[_Union[ConditionData, _Mapping]] = ..., group_by: _Optional[_Iterable[_Union[ExpressionData, _Mapping]]] = ..., aggregation: _Optional[_Union[AggregationData, _Mapping]] = ..., sort: _Optional[_Iterable[_Union[SortData, _Mapping]]] = ..., limit: _Optional[int] = ..., offset: _Optional[int] = ...) -> None: ...

class QueryResultData(_message.Message):
    __slots__ = ("metatype", "id", "type", "groups", "subresults", "nodes", "count", "exists", "scalar")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    GROUPS_FIELD_NUMBER: _ClassVar[int]
    SUBRESULTS_FIELD_NUMBER: _ClassVar[int]
    NODES_FIELD_NUMBER: _ClassVar[int]
    COUNT_FIELD_NUMBER: _ClassVar[int]
    EXISTS_FIELD_NUMBER: _ClassVar[int]
    SCALAR_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    id: str
    type: QueryType
    groups: _containers.RepeatedCompositeFieldContainer[QueryResultGroupData]
    subresults: _containers.RepeatedCompositeFieldContainer[QueryResultData]
    nodes: _containers.RepeatedCompositeFieldContainer[ValueData]
    count: int
    exists: bool
    scalar: ValueData
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., id: _Optional[str] = ..., type: _Optional[_Union[QueryType, str]] = ..., groups: _Optional[_Iterable[_Union[QueryResultGroupData, _Mapping]]] = ..., subresults: _Optional[_Iterable[_Union[QueryResultData, _Mapping]]] = ..., nodes: _Optional[_Iterable[_Union[ValueData, _Mapping]]] = ..., count: _Optional[int] = ..., exists: bool = ..., scalar: _Optional[_Union[ValueData, _Mapping]] = ...) -> None: ...

class QueryResultGroupData(_message.Message):
    __slots__ = ("metatype", "type", "discriminator", "nodes", "count", "exists", "scalar")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    DISCRIMINATOR_FIELD_NUMBER: _ClassVar[int]
    NODES_FIELD_NUMBER: _ClassVar[int]
    COUNT_FIELD_NUMBER: _ClassVar[int]
    EXISTS_FIELD_NUMBER: _ClassVar[int]
    SCALAR_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: QueryType
    discriminator: ValueData
    nodes: _containers.RepeatedCompositeFieldContainer[ValueData]
    count: int
    exists: bool
    scalar: ValueData
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[QueryType, str]] = ..., discriminator: _Optional[_Union[ValueData, _Mapping]] = ..., nodes: _Optional[_Iterable[_Union[ValueData, _Mapping]]] = ..., count: _Optional[int] = ..., exists: bool = ..., scalar: _Optional[_Union[ValueData, _Mapping]] = ...) -> None: ...

class QueryUpdateData(_message.Message):
    __slots__ = ("metatype", "type", "result")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    RESULT_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: QueryUpdateType
    result: QueryResultData
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[QueryUpdateType, str]] = ..., result: _Optional[_Union[QueryResultData, _Mapping]] = ...) -> None: ...

class ReactionData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "owned_by_ptr", "content")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    CONTENT_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    content: str
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., content: _Optional[str] = ...) -> None: ...

class RelationReferenceData(_message.Message):
    __slots__ = ("metatype", "type", "node_type", "definition_ptr", "trait_type")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    TRAIT_TYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: RelationType
    node_type: NodeType
    definition_ptr: NodeReferenceData
    trait_type: TraitType
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[RelationType, str]] = ..., node_type: _Optional[_Union[NodeType, str]] = ..., definition_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., trait_type: _Optional[_Union[TraitType, str]] = ...) -> None: ...

class RoleData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "order_key", "type", "name", "slug", "icon")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    order_key: str
    type: RoleType
    name: str
    slug: str
    icon: IconData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[RoleType, str]] = ..., name: _Optional[str] = ..., slug: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ...) -> None: ...

class RoleEventData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "type", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    type: RoleEventType
    node_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., type: _Optional[_Union[RoleEventType, str]] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class RouteData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "order_key", "owned_by_ptr", "name", "scene_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SCENE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    order_key: str
    owned_by_ptr: NodeReferenceData
    name: str
    scene_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., name: _Optional[str] = ..., scene_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class RunData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "value", "type", "target_ptr", "status", "duration", "scheduled_at", "started_at", "seen_at", "interrupted_at", "terminated_at", "error", "interruption_ptr")
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
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    SCHEDULED_AT_FIELD_NUMBER: _ClassVar[int]
    STARTED_AT_FIELD_NUMBER: _ClassVar[int]
    SEEN_AT_FIELD_NUMBER: _ClassVar[int]
    INTERRUPTED_AT_FIELD_NUMBER: _ClassVar[int]
    TERMINATED_AT_FIELD_NUMBER: _ClassVar[int]
    ERROR_FIELD_NUMBER: _ClassVar[int]
    INTERRUPTION_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    value: _containers.MessageMap[str, ValueData]
    type: RunType
    target_ptr: NodeReferenceData
    status: RunStatus
    duration: _duration_pb2.Duration
    scheduled_at: _timestamp_pb2.Timestamp
    started_at: _timestamp_pb2.Timestamp
    seen_at: _timestamp_pb2.Timestamp
    interrupted_at: _timestamp_pb2.Timestamp
    terminated_at: _timestamp_pb2.Timestamp
    error: ErrorData
    interruption_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., value: _Optional[_Mapping[str, ValueData]] = ..., type: _Optional[_Union[RunType, str]] = ..., target_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., status: _Optional[_Union[RunStatus, str]] = ..., duration: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., scheduled_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., started_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., seen_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., interrupted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., terminated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., error: _Optional[_Union[ErrorData, _Mapping]] = ..., interruption_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class RunEventData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "type", "node_ptr", "target_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    type: RunEventType
    node_ptr: NodeReferenceData
    target_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., type: _Optional[_Union[RunEventType, str]] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class SceneData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "value", "order_key", "owned_by_ptr", "name", "icon", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "layout", "direction", "distribute", "align", "gap", "padding", "grid", "grid_span", "aspect_ratio", "is_wrap", "is_visible", "opacity", "fill", "rotation", "skew", "scale", "shadow", "border", "radius", "root_view_ptr", "script_ptr")
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
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    LAYOUT_FIELD_NUMBER: _ClassVar[int]
    DIRECTION_FIELD_NUMBER: _ClassVar[int]
    DISTRIBUTE_FIELD_NUMBER: _ClassVar[int]
    ALIGN_FIELD_NUMBER: _ClassVar[int]
    GAP_FIELD_NUMBER: _ClassVar[int]
    PADDING_FIELD_NUMBER: _ClassVar[int]
    GRID_FIELD_NUMBER: _ClassVar[int]
    GRID_SPAN_FIELD_NUMBER: _ClassVar[int]
    ASPECT_RATIO_FIELD_NUMBER: _ClassVar[int]
    IS_WRAP_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_FIELD_NUMBER: _ClassVar[int]
    FILL_FIELD_NUMBER: _ClassVar[int]
    ROTATION_FIELD_NUMBER: _ClassVar[int]
    SKEW_FIELD_NUMBER: _ClassVar[int]
    SCALE_FIELD_NUMBER: _ClassVar[int]
    SHADOW_FIELD_NUMBER: _ClassVar[int]
    BORDER_FIELD_NUMBER: _ClassVar[int]
    RADIUS_FIELD_NUMBER: _ClassVar[int]
    ROOT_VIEW_PTR_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    value: _containers.MessageMap[str, ValueData]
    order_key: str
    owned_by_ptr: NodeReferenceData
    name: str
    icon: IconData
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    layout: Layout
    direction: Direction
    distribute: Distribute
    align: Align
    gap: Axis2Data
    padding: InsetsData
    grid: GridData
    grid_span: GridSpanData
    aspect_ratio: float
    is_wrap: bool
    is_visible: bool
    opacity: float
    fill: FillData
    rotation: Axis3Data
    skew: Vector2Data
    scale: float
    shadow: ShadowData
    border: BorderData
    radius: CornersData
    root_view_ptr: NodeReferenceData
    script_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., value: _Optional[_Mapping[str, ValueData]] = ..., order_key: _Optional[str] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ..., layout: _Optional[_Union[Layout, str]] = ..., direction: _Optional[_Union[Direction, str]] = ..., distribute: _Optional[_Union[Distribute, str]] = ..., align: _Optional[_Union[Align, str]] = ..., gap: _Optional[_Union[Axis2Data, _Mapping]] = ..., padding: _Optional[_Union[InsetsData, _Mapping]] = ..., grid: _Optional[_Union[GridData, _Mapping]] = ..., grid_span: _Optional[_Union[GridSpanData, _Mapping]] = ..., aspect_ratio: _Optional[float] = ..., is_wrap: bool = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., fill: _Optional[_Union[FillData, _Mapping]] = ..., rotation: _Optional[_Union[Axis3Data, _Mapping]] = ..., skew: _Optional[_Union[Vector2Data, _Mapping]] = ..., scale: _Optional[float] = ..., shadow: _Optional[_Union[ShadowData, _Mapping]] = ..., border: _Optional[_Union[BorderData, _Mapping]] = ..., radius: _Optional[_Union[CornersData, _Mapping]] = ..., root_view_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., script_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class SceneEventData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "type", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    type: SceneEventType
    node_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., type: _Optional[_Union[SceneEventType, str]] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

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

class ScopeData(_message.Message):
    __slots__ = ("metatype", "region", "space_id")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    SPACE_ID_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    region: Region
    space_id: str
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., region: _Optional[_Union[Region, str]] = ..., space_id: _Optional[str] = ...) -> None: ...

class ScreenCursorData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "owned_by_ptr", "status", "active_at", "position")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_AT_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    status: CursorStatus
    active_at: _timestamp_pb2.Timestamp
    position: Vector2iData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., status: _Optional[_Union[CursorStatus, str]] = ..., active_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., position: _Optional[_Union[Vector2iData, _Mapping]] = ...) -> None: ...

class ScriptData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "value", "order_key", "name", "code")
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
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    CODE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    value: _containers.MessageMap[str, ValueData]
    order_key: str
    name: str
    code: str
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., value: _Optional[_Mapping[str, ValueData]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., code: _Optional[str] = ...) -> None: ...

class SelectData(_message.Message):
    __slots__ = ("metatype", "properties_ptr", "fields_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    PROPERTIES_PTR_FIELD_NUMBER: _ClassVar[int]
    FIELDS_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    properties_ptr: _containers.RepeatedCompositeFieldContainer[PropertyReferenceData]
    fields_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., properties_ptr: _Optional[_Iterable[_Union[PropertyReferenceData, _Mapping]]] = ..., fields_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...) -> None: ...

class SelectionData(_message.Message):
    __slots__ = ("metatype", "nodes_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    NODES_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    nodes_ptr: _containers.RepeatedCompositeFieldContainer[NodeReferenceData]
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., nodes_ptr: _Optional[_Iterable[_Union[NodeReferenceData, _Mapping]]] = ...) -> None: ...

class ServiceData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "value", "order_key", "owned_by_ptr", "name", "script_ptr", "source_ptr")
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
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    value: _containers.MessageMap[str, ValueData]
    order_key: str
    owned_by_ptr: NodeReferenceData
    name: str
    script_ptr: NodeReferenceData
    source_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., value: _Optional[_Mapping[str, ValueData]] = ..., order_key: _Optional[str] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., name: _Optional[str] = ..., script_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., source_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

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
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "order_key", "type", "name", "style_ptr", "color", "position", "offset", "blur", "spread", "diffusion")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
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
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    order_key: str
    type: ShadowType
    name: str
    style_ptr: NodeReferenceData
    color: ColorData
    position: ShadowPosition
    offset: Axis2Data
    blur: int
    spread: int
    diffusion: float
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[ShadowType, str]] = ..., name: _Optional[str] = ..., style_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., color: _Optional[_Union[ColorData, _Mapping]] = ..., position: _Optional[_Union[ShadowPosition, str]] = ..., offset: _Optional[_Union[Axis2Data, _Mapping]] = ..., blur: _Optional[int] = ..., spread: _Optional[int] = ..., diffusion: _Optional[float] = ...) -> None: ...

class SliderInputViewData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "order_key", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "is_visible", "opacity", "value", "min_value", "max_value", "step", "script_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    MIN_VALUE_FIELD_NUMBER: _ClassVar[int]
    MAX_VALUE_FIELD_NUMBER: _ClassVar[int]
    STEP_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    order_key: str
    name: str
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    is_visible: bool
    opacity: float
    value: float
    min_value: float
    max_value: float
    step: float
    script_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., value: _Optional[float] = ..., min_value: _Optional[float] = ..., max_value: _Optional[float] = ..., step: _Optional[float] = ..., script_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

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
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "owned_by_ptr", "name", "slug", "icon", "status", "handle_ptr", "system_folder_ptr", "home_folder_ptr", "region", "cell_name", "database_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    HANDLE_PTR_FIELD_NUMBER: _ClassVar[int]
    SYSTEM_FOLDER_PTR_FIELD_NUMBER: _ClassVar[int]
    HOME_FOLDER_PTR_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    CELL_NAME_FIELD_NUMBER: _ClassVar[int]
    DATABASE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    name: str
    slug: str
    icon: IconData
    status: SpaceStatus
    handle_ptr: NodeReferenceData
    system_folder_ptr: NodeReferenceData
    home_folder_ptr: NodeReferenceData
    region: Region
    cell_name: str
    database_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., name: _Optional[str] = ..., slug: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., status: _Optional[_Union[SpaceStatus, str]] = ..., handle_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., system_folder_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., home_folder_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., region: _Optional[_Union[Region, str]] = ..., cell_name: _Optional[str] = ..., database_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class SpanData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class SplitViewData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "value", "order_key", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "layout", "direction", "distribute", "align", "gap", "padding", "grid", "grid_span", "aspect_ratio", "is_wrap", "is_visible", "opacity", "fill", "rotation", "skew", "scale", "shadow", "border", "radius", "script_ptr")
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
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    LAYOUT_FIELD_NUMBER: _ClassVar[int]
    DIRECTION_FIELD_NUMBER: _ClassVar[int]
    DISTRIBUTE_FIELD_NUMBER: _ClassVar[int]
    ALIGN_FIELD_NUMBER: _ClassVar[int]
    GAP_FIELD_NUMBER: _ClassVar[int]
    PADDING_FIELD_NUMBER: _ClassVar[int]
    GRID_FIELD_NUMBER: _ClassVar[int]
    GRID_SPAN_FIELD_NUMBER: _ClassVar[int]
    ASPECT_RATIO_FIELD_NUMBER: _ClassVar[int]
    IS_WRAP_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_FIELD_NUMBER: _ClassVar[int]
    FILL_FIELD_NUMBER: _ClassVar[int]
    ROTATION_FIELD_NUMBER: _ClassVar[int]
    SKEW_FIELD_NUMBER: _ClassVar[int]
    SCALE_FIELD_NUMBER: _ClassVar[int]
    SHADOW_FIELD_NUMBER: _ClassVar[int]
    BORDER_FIELD_NUMBER: _ClassVar[int]
    RADIUS_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    value: _containers.MessageMap[str, ValueData]
    order_key: str
    name: str
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    layout: Layout
    direction: Direction
    distribute: Distribute
    align: Align
    gap: Axis2Data
    padding: InsetsData
    grid: GridData
    grid_span: GridSpanData
    aspect_ratio: float
    is_wrap: bool
    is_visible: bool
    opacity: float
    fill: FillData
    rotation: Axis3Data
    skew: Vector2Data
    scale: float
    shadow: ShadowData
    border: BorderData
    radius: CornersData
    script_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., value: _Optional[_Mapping[str, ValueData]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ..., layout: _Optional[_Union[Layout, str]] = ..., direction: _Optional[_Union[Direction, str]] = ..., distribute: _Optional[_Union[Distribute, str]] = ..., align: _Optional[_Union[Align, str]] = ..., gap: _Optional[_Union[Axis2Data, _Mapping]] = ..., padding: _Optional[_Union[InsetsData, _Mapping]] = ..., grid: _Optional[_Union[GridData, _Mapping]] = ..., grid_span: _Optional[_Union[GridSpanData, _Mapping]] = ..., aspect_ratio: _Optional[float] = ..., is_wrap: bool = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., fill: _Optional[_Union[FillData, _Mapping]] = ..., rotation: _Optional[_Union[Axis3Data, _Mapping]] = ..., skew: _Optional[_Union[Vector2Data, _Mapping]] = ..., scale: _Optional[float] = ..., shadow: _Optional[_Union[ShadowData, _Mapping]] = ..., border: _Optional[_Union[BorderData, _Mapping]] = ..., radius: _Optional[_Union[CornersData, _Mapping]] = ..., script_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class StarData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "owned_by_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

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

class StructDefinitionData(_message.Message):
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
    properties: _containers.RepeatedCompositeFieldContainer[PropertyDefinitionData]
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., id: _Optional[int] = ..., type: _Optional[_Union[StructType, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., description: _Optional[str] = ..., properties: _Optional[_Iterable[_Union[PropertyDefinitionData, _Mapping]]] = ...) -> None: ...

class TagData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "order_key", "name", "icon")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    order_key: str
    name: str
    icon: IconData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ...) -> None: ...

class TaggingData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "order_key", "tag_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TAG_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    order_key: str
    tag_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., tag_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class TeamData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "name", "slug", "icon")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    name: str
    slug: str
    icon: IconData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., name: _Optional[str] = ..., slug: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ...) -> None: ...

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
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "order_key", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "align", "is_visible", "opacity", "user_select", "font", "color", "text", "script_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    ALIGN_FIELD_NUMBER: _ClassVar[int]
    IS_VISIBLE_FIELD_NUMBER: _ClassVar[int]
    OPACITY_FIELD_NUMBER: _ClassVar[int]
    USER_SELECT_FIELD_NUMBER: _ClassVar[int]
    FONT_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    order_key: str
    name: str
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    align: Align
    is_visible: bool
    opacity: float
    user_select: bool
    font: FontData
    color: FillData
    text: str
    script_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ..., align: _Optional[_Union[Align, str]] = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., user_select: bool = ..., font: _Optional[_Union[FontData, _Mapping]] = ..., color: _Optional[_Union[FillData, _Mapping]] = ..., text: _Optional[str] = ..., script_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class ThemeData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "order_key", "name", "icon", "colors")
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
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    COLORS_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    order_key: str
    name: str
    icon: IconData
    colors: _containers.MessageMap[int, ColorData]
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., colors: _Optional[_Mapping[int, ColorData]] = ...) -> None: ...

class ThreadData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "owned_by_ptr", "name")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    name: str
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., name: _Optional[str] = ...) -> None: ...

class ThreadCursorData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "owned_by_ptr", "status", "active_at")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_AT_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    owned_by_ptr: NodeReferenceData
    status: CursorStatus
    active_at: _timestamp_pb2.Timestamp
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., status: _Optional[_Union[CursorStatus, str]] = ..., active_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...) -> None: ...

class ThreadViewData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "order_key", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "draft_text", "draft_reply_to_ptr", "script_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    DRAFT_TEXT_FIELD_NUMBER: _ClassVar[int]
    DRAFT_REPLY_TO_PTR_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    order_key: str
    name: str
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    draft_text: TextData
    draft_reply_to_ptr: NodeReferenceData
    script_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ..., draft_text: _Optional[_Union[TextData, _Mapping]] = ..., draft_reply_to_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., script_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class TimerData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "type", "name", "schedule")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SCHEDULE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    type: TimerType
    name: str
    schedule: ScheduleData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., type: _Optional[_Union[TimerType, str]] = ..., name: _Optional[str] = ..., schedule: _Optional[_Union[ScheduleData, _Mapping]] = ...) -> None: ...

class TimerEventData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "type", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    type: TimerEventType
    node_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., type: _Optional[_Union[TimerEventType, str]] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class TraitDefinitionData(_message.Message):
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
    properties: _containers.RepeatedCompositeFieldContainer[PropertyDefinitionData]
    nodes: _containers.RepeatedScalarFieldContainer[NodeType]
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., id: _Optional[int] = ..., type: _Optional[_Union[TraitType, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., description: _Optional[str] = ..., properties: _Optional[_Iterable[_Union[PropertyDefinitionData, _Mapping]]] = ..., nodes: _Optional[_Iterable[_Union[NodeType, str]]] = ...) -> None: ...

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
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "order_key", "type", "name", "style_ptr", "delay", "duration", "ease", "stiffness", "damping", "mass", "bounce", "spring_type")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
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
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    order_key: str
    type: TransitionType
    name: str
    style_ptr: NodeReferenceData
    delay: float
    duration: float
    ease: _containers.RepeatedScalarFieldContainer[float]
    stiffness: float
    damping: float
    mass: float
    bounce: float
    spring_type: SpringType
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[TransitionType, str]] = ..., name: _Optional[str] = ..., style_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., delay: _Optional[float] = ..., duration: _Optional[float] = ..., ease: _Optional[_Iterable[float]] = ..., stiffness: _Optional[float] = ..., damping: _Optional[float] = ..., mass: _Optional[float] = ..., bounce: _Optional[float] = ..., spring_type: _Optional[_Union[SpringType, str]] = ...) -> None: ...

class TriggerData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "type", "name", "event", "where", "target_ptr", "arguments")
    class ArgumentsEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueData
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueData, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    EVENT_FIELD_NUMBER: _ClassVar[int]
    WHERE_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    ARGUMENTS_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    type: TriggerType
    name: str
    event: RelationReferenceData
    where: ConditionData
    target_ptr: NodeReferenceData
    arguments: _containers.MessageMap[str, ValueData]
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., type: _Optional[_Union[TriggerType, str]] = ..., name: _Optional[str] = ..., event: _Optional[_Union[RelationReferenceData, _Mapping]] = ..., where: _Optional[_Union[ConditionData, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., arguments: _Optional[_Mapping[str, ValueData]] = ...) -> None: ...

class TriggerEventData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "type", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    type: TriggerEventType
    node_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., type: _Optional[_Union[TriggerEventType, str]] = ..., node_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class TypeData(_message.Message):
    __slots__ = ("metatype", "cardinality", "scalar_type", "primitive_type", "enum_type", "node_type", "node_definition_ptr", "struct_type", "base_type_ptr", "key_type", "is_required", "is_variable", "default", "default_factory", "collection_constraint", "string_constraint", "number_constraint", "node_constraint")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    CARDINALITY_FIELD_NUMBER: _ClassVar[int]
    SCALAR_TYPE_FIELD_NUMBER: _ClassVar[int]
    PRIMITIVE_TYPE_FIELD_NUMBER: _ClassVar[int]
    ENUM_TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    STRUCT_TYPE_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_PTR_FIELD_NUMBER: _ClassVar[int]
    KEY_TYPE_FIELD_NUMBER: _ClassVar[int]
    IS_REQUIRED_FIELD_NUMBER: _ClassVar[int]
    IS_VARIABLE_FIELD_NUMBER: _ClassVar[int]
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
    node_definition_ptr: NodeReferenceData
    struct_type: StructType
    base_type_ptr: NodeReferenceData
    key_type: TypeData
    is_required: bool
    is_variable: bool
    default: ValueData
    default_factory: DefaultFactory
    collection_constraint: CollectionConstraintData
    string_constraint: StringConstraintData
    number_constraint: NumberConstraintData
    node_constraint: NodeConstraintData
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., cardinality: _Optional[_Union[TypeCardinality, str]] = ..., scalar_type: _Optional[_Union[ScalarType, str]] = ..., primitive_type: _Optional[_Union[PrimitiveType, str]] = ..., enum_type: _Optional[_Union[EnumType, str]] = ..., node_type: _Optional[_Union[NodeType, str]] = ..., node_definition_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., struct_type: _Optional[_Union[StructType, str]] = ..., base_type_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., key_type: _Optional[_Union[TypeData, _Mapping]] = ..., is_required: bool = ..., is_variable: bool = ..., default: _Optional[_Union[ValueData, _Mapping]] = ..., default_factory: _Optional[_Union[DefaultFactory, str]] = ..., collection_constraint: _Optional[_Union[CollectionConstraintData, _Mapping]] = ..., string_constraint: _Optional[_Union[StringConstraintData, _Mapping]] = ..., number_constraint: _Optional[_Union[NumberConstraintData, _Mapping]] = ..., node_constraint: _Optional[_Union[NodeConstraintData, _Mapping]] = ...) -> None: ...

class UserData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "name", "slug", "icon", "status", "last_logged_in_at", "is_staff", "space_ptr", "handle_ptr", "cursor_ptr", "email", "password_salt", "password_hash")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    LAST_LOGGED_IN_AT_FIELD_NUMBER: _ClassVar[int]
    IS_STAFF_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
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
    name: str
    slug: str
    icon: IconData
    status: UserStatus
    last_logged_in_at: _timestamp_pb2.Timestamp
    is_staff: bool
    space_ptr: NodeReferenceData
    handle_ptr: NodeReferenceData
    cursor_ptr: NodeReferenceData
    email: str
    password_salt: bytes
    password_hash: bytes
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., name: _Optional[str] = ..., slug: _Optional[str] = ..., icon: _Optional[_Union[IconData, _Mapping]] = ..., status: _Optional[_Union[UserStatus, str]] = ..., last_logged_in_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., is_staff: bool = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., handle_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., cursor_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., email: _Optional[str] = ..., password_salt: _Optional[bytes] = ..., password_hash: _Optional[bytes] = ...) -> None: ...

class ValueData(_message.Message):
    __slots__ = ("metatype", "type", "value")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    type: TypeData
    value: _struct_pb2.Value
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., type: _Optional[_Union[TypeData, _Mapping]] = ..., value: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ...) -> None: ...

class Vector2Data(_message.Message):
    __slots__ = ("metatype", "x", "y")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    x: float
    y: float
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., x: _Optional[float] = ..., y: _Optional[float] = ...) -> None: ...

class Vector2iData(_message.Message):
    __slots__ = ("metatype", "x", "y")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    x: int
    y: int
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., x: _Optional[int] = ..., y: _Optional[int] = ...) -> None: ...

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

class Vector3iData(_message.Message):
    __slots__ = ("metatype", "x", "y", "z")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    Z_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    x: int
    y: int
    z: int
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., x: _Optional[int] = ..., y: _Optional[int] = ..., z: _Optional[int] = ...) -> None: ...

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

class Vector4iData(_message.Message):
    __slots__ = ("metatype", "x", "y", "z", "w")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    Z_FIELD_NUMBER: _ClassVar[int]
    W_FIELD_NUMBER: _ClassVar[int]
    metatype: StructType
    x: int
    y: int
    z: int
    w: int
    def __init__(self, metatype: _Optional[_Union[StructType, str]] = ..., x: _Optional[int] = ..., y: _Optional[int] = ..., z: _Optional[int] = ..., w: _Optional[int] = ...) -> None: ...

class WindowData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "order_key", "owned_by_ptr", "type", "name", "selection", "focus_ptr", "inspection_ptr", "container_ptr", "thread_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SELECTION_FIELD_NUMBER: _ClassVar[int]
    FOCUS_PTR_FIELD_NUMBER: _ClassVar[int]
    INSPECTION_PTR_FIELD_NUMBER: _ClassVar[int]
    CONTAINER_PTR_FIELD_NUMBER: _ClassVar[int]
    THREAD_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    order_key: str
    owned_by_ptr: NodeReferenceData
    type: WindowType
    name: str
    selection: SelectionData
    focus_ptr: NodeReferenceData
    inspection_ptr: NodeReferenceData
    container_ptr: NodeReferenceData
    thread_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., type: _Optional[_Union[WindowType, str]] = ..., name: _Optional[str] = ..., selection: _Optional[_Union[SelectionData, _Mapping]] = ..., focus_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., inspection_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., container_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., thread_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class WizardViewData(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "template_ptr", "order_key", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "script_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeType
    id: str
    parent_ptr: NodeReferenceData
    space_ptr: NodeReferenceData
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceData
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceData
    deleted_at: _timestamp_pb2.Timestamp
    template_ptr: NodeReferenceData
    order_key: str
    name: str
    position: PositionData
    width: DimensionData
    height: DimensionData
    min_width: DimensionData
    min_height: DimensionData
    max_width: DimensionData
    max_height: DimensionData
    script_ptr: NodeReferenceData
    def __init__(self, metatype: _Optional[_Union[NodeType, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionData, _Mapping]] = ..., width: _Optional[_Union[DimensionData, _Mapping]] = ..., height: _Optional[_Union[DimensionData, _Mapping]] = ..., min_width: _Optional[_Union[DimensionData, _Mapping]] = ..., min_height: _Optional[_Union[DimensionData, _Mapping]] = ..., max_width: _Optional[_Union[DimensionData, _Mapping]] = ..., max_height: _Optional[_Union[DimensionData, _Mapping]] = ..., script_ptr: _Optional[_Union[NodeReferenceData, _Mapping]] = ...) -> None: ...

class SomeNodeData(_message.Message):
    __slots__ = ("space", "handle", "user", "friendship", "friendship_invite", "friendship_invite_event", "organization", "team", "client", "membership", "membership_event", "invite", "invite_event", "role", "role_event", "agent", "folder", "tag", "tagging", "custom_entity_definition", "custom_entity", "custom_struct_definition", "custom_enum_definition", "field", "option", "file", "link", "script", "service", "action", "route", "trigger", "trigger_event", "timer", "timer_event", "event_cursor", "screen_cursor", "thread_cursor", "run", "run_event", "span", "interruption", "log", "gauge_metric", "gauge_measurement", "counter_metric", "counter_measurement", "histogram_metric", "histogram_measurement", "custom_event_definition", "custom_event", "edit_event", "environment", "thread", "message", "reaction", "star", "follow", "notification", "notification_event", "database", "machine", "window", "scene", "scene_event", "layer", "custom_view_definition", "custom_view", "frame_view", "label_view", "split_view", "text_view", "number_input_view", "slider_input_view", "thread_view", "wizard_view", "canvas", "line_shape", "plane_shape", "arrow_shape", "annotation_shape", "theme", "color_style", "fill_style", "font_style", "border_style", "shadow_style", "gradient_style", "transition_style", "effect_style")
    SPACE_FIELD_NUMBER: _ClassVar[int]
    HANDLE_FIELD_NUMBER: _ClassVar[int]
    USER_FIELD_NUMBER: _ClassVar[int]
    FRIENDSHIP_FIELD_NUMBER: _ClassVar[int]
    FRIENDSHIP_INVITE_FIELD_NUMBER: _ClassVar[int]
    FRIENDSHIP_INVITE_EVENT_FIELD_NUMBER: _ClassVar[int]
    ORGANIZATION_FIELD_NUMBER: _ClassVar[int]
    TEAM_FIELD_NUMBER: _ClassVar[int]
    CLIENT_FIELD_NUMBER: _ClassVar[int]
    MEMBERSHIP_FIELD_NUMBER: _ClassVar[int]
    MEMBERSHIP_EVENT_FIELD_NUMBER: _ClassVar[int]
    INVITE_FIELD_NUMBER: _ClassVar[int]
    INVITE_EVENT_FIELD_NUMBER: _ClassVar[int]
    ROLE_FIELD_NUMBER: _ClassVar[int]
    ROLE_EVENT_FIELD_NUMBER: _ClassVar[int]
    AGENT_FIELD_NUMBER: _ClassVar[int]
    FOLDER_FIELD_NUMBER: _ClassVar[int]
    TAG_FIELD_NUMBER: _ClassVar[int]
    TAGGING_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_ENTITY_DEFINITION_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_ENTITY_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_STRUCT_DEFINITION_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_ENUM_DEFINITION_FIELD_NUMBER: _ClassVar[int]
    FIELD_FIELD_NUMBER: _ClassVar[int]
    OPTION_FIELD_NUMBER: _ClassVar[int]
    FILE_FIELD_NUMBER: _ClassVar[int]
    LINK_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_FIELD_NUMBER: _ClassVar[int]
    SERVICE_FIELD_NUMBER: _ClassVar[int]
    ACTION_FIELD_NUMBER: _ClassVar[int]
    ROUTE_FIELD_NUMBER: _ClassVar[int]
    TRIGGER_FIELD_NUMBER: _ClassVar[int]
    TRIGGER_EVENT_FIELD_NUMBER: _ClassVar[int]
    TIMER_FIELD_NUMBER: _ClassVar[int]
    TIMER_EVENT_FIELD_NUMBER: _ClassVar[int]
    EVENT_CURSOR_FIELD_NUMBER: _ClassVar[int]
    SCREEN_CURSOR_FIELD_NUMBER: _ClassVar[int]
    THREAD_CURSOR_FIELD_NUMBER: _ClassVar[int]
    RUN_FIELD_NUMBER: _ClassVar[int]
    RUN_EVENT_FIELD_NUMBER: _ClassVar[int]
    SPAN_FIELD_NUMBER: _ClassVar[int]
    INTERRUPTION_FIELD_NUMBER: _ClassVar[int]
    LOG_FIELD_NUMBER: _ClassVar[int]
    GAUGE_METRIC_FIELD_NUMBER: _ClassVar[int]
    GAUGE_MEASUREMENT_FIELD_NUMBER: _ClassVar[int]
    COUNTER_METRIC_FIELD_NUMBER: _ClassVar[int]
    COUNTER_MEASUREMENT_FIELD_NUMBER: _ClassVar[int]
    HISTOGRAM_METRIC_FIELD_NUMBER: _ClassVar[int]
    HISTOGRAM_MEASUREMENT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_EVENT_DEFINITION_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_EVENT_FIELD_NUMBER: _ClassVar[int]
    EDIT_EVENT_FIELD_NUMBER: _ClassVar[int]
    ENVIRONMENT_FIELD_NUMBER: _ClassVar[int]
    THREAD_FIELD_NUMBER: _ClassVar[int]
    MESSAGE_FIELD_NUMBER: _ClassVar[int]
    REACTION_FIELD_NUMBER: _ClassVar[int]
    STAR_FIELD_NUMBER: _ClassVar[int]
    FOLLOW_FIELD_NUMBER: _ClassVar[int]
    NOTIFICATION_FIELD_NUMBER: _ClassVar[int]
    NOTIFICATION_EVENT_FIELD_NUMBER: _ClassVar[int]
    DATABASE_FIELD_NUMBER: _ClassVar[int]
    MACHINE_FIELD_NUMBER: _ClassVar[int]
    WINDOW_FIELD_NUMBER: _ClassVar[int]
    SCENE_FIELD_NUMBER: _ClassVar[int]
    SCENE_EVENT_FIELD_NUMBER: _ClassVar[int]
    LAYER_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VIEW_DEFINITION_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VIEW_FIELD_NUMBER: _ClassVar[int]
    FRAME_VIEW_FIELD_NUMBER: _ClassVar[int]
    LABEL_VIEW_FIELD_NUMBER: _ClassVar[int]
    SPLIT_VIEW_FIELD_NUMBER: _ClassVar[int]
    TEXT_VIEW_FIELD_NUMBER: _ClassVar[int]
    NUMBER_INPUT_VIEW_FIELD_NUMBER: _ClassVar[int]
    SLIDER_INPUT_VIEW_FIELD_NUMBER: _ClassVar[int]
    THREAD_VIEW_FIELD_NUMBER: _ClassVar[int]
    WIZARD_VIEW_FIELD_NUMBER: _ClassVar[int]
    CANVAS_FIELD_NUMBER: _ClassVar[int]
    LINE_SHAPE_FIELD_NUMBER: _ClassVar[int]
    PLANE_SHAPE_FIELD_NUMBER: _ClassVar[int]
    ARROW_SHAPE_FIELD_NUMBER: _ClassVar[int]
    ANNOTATION_SHAPE_FIELD_NUMBER: _ClassVar[int]
    THEME_FIELD_NUMBER: _ClassVar[int]
    COLOR_STYLE_FIELD_NUMBER: _ClassVar[int]
    FILL_STYLE_FIELD_NUMBER: _ClassVar[int]
    FONT_STYLE_FIELD_NUMBER: _ClassVar[int]
    BORDER_STYLE_FIELD_NUMBER: _ClassVar[int]
    SHADOW_STYLE_FIELD_NUMBER: _ClassVar[int]
    GRADIENT_STYLE_FIELD_NUMBER: _ClassVar[int]
    TRANSITION_STYLE_FIELD_NUMBER: _ClassVar[int]
    EFFECT_STYLE_FIELD_NUMBER: _ClassVar[int]
    space: SpaceData
    handle: HandleData
    user: UserData
    friendship: FriendshipData
    friendship_invite: FriendshipInviteData
    friendship_invite_event: FriendshipInviteEventData
    organization: OrganizationData
    team: TeamData
    client: ClientData
    membership: MembershipData
    membership_event: MembershipEventData
    invite: InviteData
    invite_event: InviteEventData
    role: RoleData
    role_event: RoleEventData
    agent: AgentData
    folder: FolderData
    tag: TagData
    tagging: TaggingData
    custom_entity_definition: CustomEntityDefinitionData
    custom_entity: CustomEntityData
    custom_struct_definition: CustomStructDefinitionData
    custom_enum_definition: CustomEnumDefinitionData
    field: FieldData
    option: OptionData
    file: FileData
    link: LinkData
    script: ScriptData
    service: ServiceData
    action: ActionData
    route: RouteData
    trigger: TriggerData
    trigger_event: TriggerEventData
    timer: TimerData
    timer_event: TimerEventData
    event_cursor: EventCursorData
    screen_cursor: ScreenCursorData
    thread_cursor: ThreadCursorData
    run: RunData
    run_event: RunEventData
    span: SpanData
    interruption: InterruptionData
    log: LogData
    gauge_metric: GaugeMetricData
    gauge_measurement: GaugeMeasurementData
    counter_metric: CounterMetricData
    counter_measurement: CounterMeasurementData
    histogram_metric: HistogramMetricData
    histogram_measurement: HistogramMeasurementData
    custom_event_definition: CustomEventDefinitionData
    custom_event: CustomEventData
    edit_event: EditEventData
    environment: EnvironmentData
    thread: ThreadData
    message: MessageData
    reaction: ReactionData
    star: StarData
    follow: FollowData
    notification: NotificationData
    notification_event: NotificationEventData
    database: DatabaseData
    machine: MachineData
    window: WindowData
    scene: SceneData
    scene_event: SceneEventData
    layer: LayerData
    custom_view_definition: CustomViewDefinitionData
    custom_view: CustomViewData
    frame_view: FrameViewData
    label_view: LabelViewData
    split_view: SplitViewData
    text_view: TextViewData
    number_input_view: NumberInputViewData
    slider_input_view: SliderInputViewData
    thread_view: ThreadViewData
    wizard_view: WizardViewData
    canvas: CanvasData
    line_shape: LineShapeData
    plane_shape: PlaneShapeData
    arrow_shape: ArrowShapeData
    annotation_shape: AnnotationShapeData
    theme: ThemeData
    color_style: ColorStyleData
    fill_style: FillStyleData
    font_style: FontStyleData
    border_style: BorderStyleData
    shadow_style: ShadowStyleData
    gradient_style: GradientStyleData
    transition_style: TransitionStyleData
    effect_style: EffectStyleData
    def __init__(self, space: _Optional[_Union[SpaceData, _Mapping]] = ..., handle: _Optional[_Union[HandleData, _Mapping]] = ..., user: _Optional[_Union[UserData, _Mapping]] = ..., friendship: _Optional[_Union[FriendshipData, _Mapping]] = ..., friendship_invite: _Optional[_Union[FriendshipInviteData, _Mapping]] = ..., friendship_invite_event: _Optional[_Union[FriendshipInviteEventData, _Mapping]] = ..., organization: _Optional[_Union[OrganizationData, _Mapping]] = ..., team: _Optional[_Union[TeamData, _Mapping]] = ..., client: _Optional[_Union[ClientData, _Mapping]] = ..., membership: _Optional[_Union[MembershipData, _Mapping]] = ..., membership_event: _Optional[_Union[MembershipEventData, _Mapping]] = ..., invite: _Optional[_Union[InviteData, _Mapping]] = ..., invite_event: _Optional[_Union[InviteEventData, _Mapping]] = ..., role: _Optional[_Union[RoleData, _Mapping]] = ..., role_event: _Optional[_Union[RoleEventData, _Mapping]] = ..., agent: _Optional[_Union[AgentData, _Mapping]] = ..., folder: _Optional[_Union[FolderData, _Mapping]] = ..., tag: _Optional[_Union[TagData, _Mapping]] = ..., tagging: _Optional[_Union[TaggingData, _Mapping]] = ..., custom_entity_definition: _Optional[_Union[CustomEntityDefinitionData, _Mapping]] = ..., custom_entity: _Optional[_Union[CustomEntityData, _Mapping]] = ..., custom_struct_definition: _Optional[_Union[CustomStructDefinitionData, _Mapping]] = ..., custom_enum_definition: _Optional[_Union[CustomEnumDefinitionData, _Mapping]] = ..., field: _Optional[_Union[FieldData, _Mapping]] = ..., option: _Optional[_Union[OptionData, _Mapping]] = ..., file: _Optional[_Union[FileData, _Mapping]] = ..., link: _Optional[_Union[LinkData, _Mapping]] = ..., script: _Optional[_Union[ScriptData, _Mapping]] = ..., service: _Optional[_Union[ServiceData, _Mapping]] = ..., action: _Optional[_Union[ActionData, _Mapping]] = ..., route: _Optional[_Union[RouteData, _Mapping]] = ..., trigger: _Optional[_Union[TriggerData, _Mapping]] = ..., trigger_event: _Optional[_Union[TriggerEventData, _Mapping]] = ..., timer: _Optional[_Union[TimerData, _Mapping]] = ..., timer_event: _Optional[_Union[TimerEventData, _Mapping]] = ..., event_cursor: _Optional[_Union[EventCursorData, _Mapping]] = ..., screen_cursor: _Optional[_Union[ScreenCursorData, _Mapping]] = ..., thread_cursor: _Optional[_Union[ThreadCursorData, _Mapping]] = ..., run: _Optional[_Union[RunData, _Mapping]] = ..., run_event: _Optional[_Union[RunEventData, _Mapping]] = ..., span: _Optional[_Union[SpanData, _Mapping]] = ..., interruption: _Optional[_Union[InterruptionData, _Mapping]] = ..., log: _Optional[_Union[LogData, _Mapping]] = ..., gauge_metric: _Optional[_Union[GaugeMetricData, _Mapping]] = ..., gauge_measurement: _Optional[_Union[GaugeMeasurementData, _Mapping]] = ..., counter_metric: _Optional[_Union[CounterMetricData, _Mapping]] = ..., counter_measurement: _Optional[_Union[CounterMeasurementData, _Mapping]] = ..., histogram_metric: _Optional[_Union[HistogramMetricData, _Mapping]] = ..., histogram_measurement: _Optional[_Union[HistogramMeasurementData, _Mapping]] = ..., custom_event_definition: _Optional[_Union[CustomEventDefinitionData, _Mapping]] = ..., custom_event: _Optional[_Union[CustomEventData, _Mapping]] = ..., edit_event: _Optional[_Union[EditEventData, _Mapping]] = ..., environment: _Optional[_Union[EnvironmentData, _Mapping]] = ..., thread: _Optional[_Union[ThreadData, _Mapping]] = ..., message: _Optional[_Union[MessageData, _Mapping]] = ..., reaction: _Optional[_Union[ReactionData, _Mapping]] = ..., star: _Optional[_Union[StarData, _Mapping]] = ..., follow: _Optional[_Union[FollowData, _Mapping]] = ..., notification: _Optional[_Union[NotificationData, _Mapping]] = ..., notification_event: _Optional[_Union[NotificationEventData, _Mapping]] = ..., database: _Optional[_Union[DatabaseData, _Mapping]] = ..., machine: _Optional[_Union[MachineData, _Mapping]] = ..., window: _Optional[_Union[WindowData, _Mapping]] = ..., scene: _Optional[_Union[SceneData, _Mapping]] = ..., scene_event: _Optional[_Union[SceneEventData, _Mapping]] = ..., layer: _Optional[_Union[LayerData, _Mapping]] = ..., custom_view_definition: _Optional[_Union[CustomViewDefinitionData, _Mapping]] = ..., custom_view: _Optional[_Union[CustomViewData, _Mapping]] = ..., frame_view: _Optional[_Union[FrameViewData, _Mapping]] = ..., label_view: _Optional[_Union[LabelViewData, _Mapping]] = ..., split_view: _Optional[_Union[SplitViewData, _Mapping]] = ..., text_view: _Optional[_Union[TextViewData, _Mapping]] = ..., number_input_view: _Optional[_Union[NumberInputViewData, _Mapping]] = ..., slider_input_view: _Optional[_Union[SliderInputViewData, _Mapping]] = ..., thread_view: _Optional[_Union[ThreadViewData, _Mapping]] = ..., wizard_view: _Optional[_Union[WizardViewData, _Mapping]] = ..., canvas: _Optional[_Union[CanvasData, _Mapping]] = ..., line_shape: _Optional[_Union[LineShapeData, _Mapping]] = ..., plane_shape: _Optional[_Union[PlaneShapeData, _Mapping]] = ..., arrow_shape: _Optional[_Union[ArrowShapeData, _Mapping]] = ..., annotation_shape: _Optional[_Union[AnnotationShapeData, _Mapping]] = ..., theme: _Optional[_Union[ThemeData, _Mapping]] = ..., color_style: _Optional[_Union[ColorStyleData, _Mapping]] = ..., fill_style: _Optional[_Union[FillStyleData, _Mapping]] = ..., font_style: _Optional[_Union[FontStyleData, _Mapping]] = ..., border_style: _Optional[_Union[BorderStyleData, _Mapping]] = ..., shadow_style: _Optional[_Union[ShadowStyleData, _Mapping]] = ..., gradient_style: _Optional[_Union[GradientStyleData, _Mapping]] = ..., transition_style: _Optional[_Union[TransitionStyleData, _Mapping]] = ..., effect_style: _Optional[_Union[EffectStyleData, _Mapping]] = ...) -> None: ...
