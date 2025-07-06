
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

class AggregationTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    AGGREGATION_TYPE_UNSPECIFIED: _ClassVar[AggregationTypeProto]
    AGGREGATION_TYPE_EXISTS: _ClassVar[AggregationTypeProto]
    AGGREGATION_TYPE_COUNT: _ClassVar[AggregationTypeProto]
    AGGREGATION_TYPE_SUM: _ClassVar[AggregationTypeProto]
    AGGREGATION_TYPE_MIN: _ClassVar[AggregationTypeProto]
    AGGREGATION_TYPE_MAX: _ClassVar[AggregationTypeProto]
    AGGREGATION_TYPE_AVERAGE: _ClassVar[AggregationTypeProto]

class AlignProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ALIGN_UNSPECIFIED: _ClassVar[AlignProto]
    ALIGN_START: _ClassVar[AlignProto]
    ALIGN_CENTER: _ClassVar[AlignProto]
    ALIGN_END: _ClassVar[AlignProto]

class ArrowHeadTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ARROW_HEAD_TYPE_UNSPECIFIED: _ClassVar[ArrowHeadTypeProto]
    ARROW_HEAD_TYPE_ARROW: _ClassVar[ArrowHeadTypeProto]
    ARROW_HEAD_TYPE_TRIANGLE: _ClassVar[ArrowHeadTypeProto]
    ARROW_HEAD_TYPE_DOT: _ClassVar[ArrowHeadTypeProto]

class BorderTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    BORDER_TYPE_UNSPECIFIED: _ClassVar[BorderTypeProto]
    BORDER_TYPE_STYLE: _ClassVar[BorderTypeProto]
    BORDER_TYPE_SOLID: _ClassVar[BorderTypeProto]
    BORDER_TYPE_DASHED: _ClassVar[BorderTypeProto]
    BORDER_TYPE_DOTTED: _ClassVar[BorderTypeProto]
    BORDER_TYPE_DOUBLE: _ClassVar[BorderTypeProto]

class CanvasTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CANVAS_TYPE_UNSPECIFIED: _ClassVar[CanvasTypeProto]
    CANVAS_TYPE_SHAPE: _ClassVar[CanvasTypeProto]

class CascadeActionProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CASCADE_ACTION_UNSPECIFIED: _ClassVar[CascadeActionProto]
    CASCADE_ACTION_RESTRICT: _ClassVar[CascadeActionProto]
    CASCADE_ACTION_CASCADE: _ClassVar[CascadeActionProto]
    CASCADE_ACTION_SET_NULL: _ClassVar[CascadeActionProto]

class ChangeDebounceProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CHANGE_DEBOUNCE_UNSPECIFIED: _ClassVar[ChangeDebounceProto]
    CHANGE_DEBOUNCE_LAZY: _ClassVar[ChangeDebounceProto]

class ChangeStatusProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CHANGE_STATUS_UNSPECIFIED: _ClassVar[ChangeStatusProto]
    CHANGE_STATUS_COMPLETED: _ClassVar[ChangeStatusProto]
    CHANGE_STATUS_SKIPPED: _ClassVar[ChangeStatusProto]
    CHANGE_STATUS_FAILED: _ClassVar[ChangeStatusProto]
    CHANGE_STATUS_REJECTED: _ClassVar[ChangeStatusProto]

class ClientTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CLIENT_TYPE_UNSPECIFIED: _ClassVar[ClientTypeProto]
    CLIENT_TYPE_WEB: _ClassVar[ClientTypeProto]
    CLIENT_TYPE_BROWSER_PLUGIN: _ClassVar[ClientTypeProto]
    CLIENT_TYPE_DESKTOP: _ClassVar[ClientTypeProto]
    CLIENT_TYPE_MOBILE: _ClassVar[ClientTypeProto]
    CLIENT_TYPE_MACHINE: _ClassVar[ClientTypeProto]

class CloudProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CLOUD_UNSPECIFIED: _ClassVar[CloudProto]
    CLOUD_PRIVATE: _ClassVar[CloudProto]
    CLOUD_AWS: _ClassVar[CloudProto]
    CLOUD_AZURE: _ClassVar[CloudProto]
    CLOUD_GCP: _ClassVar[CloudProto]
    CLOUD_HETZNER: _ClassVar[CloudProto]

class ColorHueProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    COLOR_HUE_UNSPECIFIED: _ClassVar[ColorHueProto]
    COLOR_HUE_GRAY: _ClassVar[ColorHueProto]
    COLOR_HUE_RED: _ClassVar[ColorHueProto]
    COLOR_HUE_ORANGE: _ClassVar[ColorHueProto]
    COLOR_HUE_AMBER: _ClassVar[ColorHueProto]
    COLOR_HUE_YELLOW: _ClassVar[ColorHueProto]
    COLOR_HUE_LIME: _ClassVar[ColorHueProto]
    COLOR_HUE_GREEN: _ClassVar[ColorHueProto]
    COLOR_HUE_EMERALD: _ClassVar[ColorHueProto]
    COLOR_HUE_TEAL: _ClassVar[ColorHueProto]
    COLOR_HUE_CYAN: _ClassVar[ColorHueProto]
    COLOR_HUE_SKY: _ClassVar[ColorHueProto]
    COLOR_HUE_BLUE: _ClassVar[ColorHueProto]
    COLOR_HUE_INDIGO: _ClassVar[ColorHueProto]
    COLOR_HUE_VIOLET: _ClassVar[ColorHueProto]
    COLOR_HUE_PURPLE: _ClassVar[ColorHueProto]
    COLOR_HUE_FUCHSIA: _ClassVar[ColorHueProto]
    COLOR_HUE_PINK: _ClassVar[ColorHueProto]
    COLOR_HUE_ROSE: _ClassVar[ColorHueProto]

class ColorIntentProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    COLOR_INTENT_UNSPECIFIED: _ClassVar[ColorIntentProto]
    COLOR_INTENT_PRIMARY: _ClassVar[ColorIntentProto]
    COLOR_INTENT_SECONDARY: _ClassVar[ColorIntentProto]
    COLOR_INTENT_NEUTRAL: _ClassVar[ColorIntentProto]
    COLOR_INTENT_SUCCESS: _ClassVar[ColorIntentProto]
    COLOR_INTENT_INFO: _ClassVar[ColorIntentProto]
    COLOR_INTENT_WARNING: _ClassVar[ColorIntentProto]
    COLOR_INTENT_ERROR: _ClassVar[ColorIntentProto]

class ColorShadeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    COLOR_SHADE_UNSPECIFIED: _ClassVar[ColorShadeProto]
    COLOR_SHADE_S25: _ClassVar[ColorShadeProto]
    COLOR_SHADE_S50: _ClassVar[ColorShadeProto]
    COLOR_SHADE_S100: _ClassVar[ColorShadeProto]
    COLOR_SHADE_S200: _ClassVar[ColorShadeProto]
    COLOR_SHADE_S300: _ClassVar[ColorShadeProto]
    COLOR_SHADE_S400: _ClassVar[ColorShadeProto]
    COLOR_SHADE_S500: _ClassVar[ColorShadeProto]
    COLOR_SHADE_S600: _ClassVar[ColorShadeProto]
    COLOR_SHADE_S700: _ClassVar[ColorShadeProto]
    COLOR_SHADE_S800: _ClassVar[ColorShadeProto]
    COLOR_SHADE_S900: _ClassVar[ColorShadeProto]
    COLOR_SHADE_S950: _ClassVar[ColorShadeProto]

class ColorTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    COLOR_TYPE_UNSPECIFIED: _ClassVar[ColorTypeProto]
    COLOR_TYPE_BUILTIN: _ClassVar[ColorTypeProto]
    COLOR_TYPE_RGB: _ClassVar[ColorTypeProto]
    COLOR_TYPE_HSL: _ClassVar[ColorTypeProto]
    COLOR_TYPE_P3: _ClassVar[ColorTypeProto]

class ConditionalTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CONDITIONAL_TYPE_UNSPECIFIED: _ClassVar[ConditionalTypeProto]
    CONDITIONAL_TYPE_NOT: _ClassVar[ConditionalTypeProto]
    CONDITIONAL_TYPE_AND: _ClassVar[ConditionalTypeProto]
    CONDITIONAL_TYPE_OR: _ClassVar[ConditionalTypeProto]
    CONDITIONAL_TYPE_EQUALS: _ClassVar[ConditionalTypeProto]
    CONDITIONAL_TYPE_NOT_EQUALS: _ClassVar[ConditionalTypeProto]
    CONDITIONAL_TYPE_GREATER_THAN: _ClassVar[ConditionalTypeProto]
    CONDITIONAL_TYPE_GREATER_THAN_OR_EQUALS: _ClassVar[ConditionalTypeProto]
    CONDITIONAL_TYPE_LESS_THAN: _ClassVar[ConditionalTypeProto]
    CONDITIONAL_TYPE_LESS_THAN_OR_EQUALS: _ClassVar[ConditionalTypeProto]
    CONDITIONAL_TYPE_MATCHES: _ClassVar[ConditionalTypeProto]
    CONDITIONAL_TYPE_STARTS_WITH: _ClassVar[ConditionalTypeProto]
    CONDITIONAL_TYPE_ENDS_WITH: _ClassVar[ConditionalTypeProto]
    CONDITIONAL_TYPE_IN: _ClassVar[ConditionalTypeProto]
    CONDITIONAL_TYPE_NOT_IN: _ClassVar[ConditionalTypeProto]
    CONDITIONAL_TYPE_EXISTS: _ClassVar[ConditionalTypeProto]
    CONDITIONAL_TYPE_NOT_EXISTS: _ClassVar[ConditionalTypeProto]

class CursorStatusProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CURSOR_STATUS_UNSPECIFIED: _ClassVar[CursorStatusProto]
    CURSOR_STATUS_CREATED: _ClassVar[CursorStatusProto]
    CURSOR_STATUS_WORKING: _ClassVar[CursorStatusProto]
    CURSOR_STATUS_READING: _ClassVar[CursorStatusProto]
    CURSOR_STATUS_WRITING: _ClassVar[CursorStatusProto]
    CURSOR_STATUS_THINKING: _ClassVar[CursorStatusProto]
    CURSOR_STATUS_WAITING: _ClassVar[CursorStatusProto]
    CURSOR_STATUS_IDLE: _ClassVar[CursorStatusProto]
    CURSOR_STATUS_CANCELLED: _ClassVar[CursorStatusProto]
    CURSOR_STATUS_COMPLETED: _ClassVar[CursorStatusProto]

class DatabaseTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    DATABASE_TYPE_UNSPECIFIED: _ClassVar[DatabaseTypeProto]
    DATABASE_TYPE_POSTGRES: _ClassVar[DatabaseTypeProto]

class DayOfWeekProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    DAY_OF_WEEK_UNSPECIFIED: _ClassVar[DayOfWeekProto]
    DAY_OF_WEEK_MONDAY: _ClassVar[DayOfWeekProto]
    DAY_OF_WEEK_TUESDAY: _ClassVar[DayOfWeekProto]
    DAY_OF_WEEK_WEDNESDAY: _ClassVar[DayOfWeekProto]
    DAY_OF_WEEK_THURSDAY: _ClassVar[DayOfWeekProto]
    DAY_OF_WEEK_FRIDAY: _ClassVar[DayOfWeekProto]
    DAY_OF_WEEK_SATURDAY: _ClassVar[DayOfWeekProto]
    DAY_OF_WEEK_SUNDAY: _ClassVar[DayOfWeekProto]

class DimensionTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    DIMENSION_TYPE_UNSPECIFIED: _ClassVar[DimensionTypeProto]
    DIMENSION_TYPE_FIXED: _ClassVar[DimensionTypeProto]
    DIMENSION_TYPE_FIT: _ClassVar[DimensionTypeProto]
    DIMENSION_TYPE_FILL: _ClassVar[DimensionTypeProto]

class DirectionProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    DIRECTION_UNSPECIFIED: _ClassVar[DirectionProto]
    DIRECTION_HORIZONTAL: _ClassVar[DirectionProto]
    DIRECTION_VERTICAL: _ClassVar[DirectionProto]

class DistributeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    DISTRIBUTE_UNSPECIFIED: _ClassVar[DistributeProto]
    DISTRIBUTE_START: _ClassVar[DistributeProto]
    DISTRIBUTE_CENTER: _ClassVar[DistributeProto]
    DISTRIBUTE_END: _ClassVar[DistributeProto]
    DISTRIBUTE_SPACE_BETWEEN: _ClassVar[DistributeProto]
    DISTRIBUTE_SPACE_AROUND: _ClassVar[DistributeProto]
    DISTRIBUTE_SPACE_EVENLY: _ClassVar[DistributeProto]

class EasingProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    EASING_UNSPECIFIED: _ClassVar[EasingProto]
    EASING_LINEAR: _ClassVar[EasingProto]
    EASING_EASE_IN_QUAD: _ClassVar[EasingProto]
    EASING_EASE_OUT_QUAD: _ClassVar[EasingProto]
    EASING_EASE_IN_OUT_QUAD: _ClassVar[EasingProto]
    EASING_EASE_IN_CUBIC: _ClassVar[EasingProto]
    EASING_EASE_OUT_CUBIC: _ClassVar[EasingProto]
    EASING_EASE_IN_OUT_CUBIC: _ClassVar[EasingProto]
    EASING_EASE_IN_QUART: _ClassVar[EasingProto]
    EASING_EASE_OUT_QUART: _ClassVar[EasingProto]
    EASING_EASE_IN_OUT_QUART: _ClassVar[EasingProto]
    EASING_EASE_IN_QUINT: _ClassVar[EasingProto]
    EASING_EASE_OUT_QUINT: _ClassVar[EasingProto]
    EASING_EASE_IN_OUT_QUINT: _ClassVar[EasingProto]
    EASING_EASE_IN_SINE: _ClassVar[EasingProto]
    EASING_EASE_OUT_SINE: _ClassVar[EasingProto]
    EASING_EASE_IN_OUT_SINE: _ClassVar[EasingProto]
    EASING_EASE_IN_EXPO: _ClassVar[EasingProto]
    EASING_EASE_OUT_EXPO: _ClassVar[EasingProto]
    EASING_EASE_IN_OUT_EXPO: _ClassVar[EasingProto]
    EASING_EASE_PEN: _ClassVar[EasingProto]

class EdgeDirectionProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    EDGE_DIRECTION_UNSPECIFIED: _ClassVar[EdgeDirectionProto]
    EDGE_DIRECTION_PARENT: _ClassVar[EdgeDirectionProto]
    EDGE_DIRECTION_CHILD: _ClassVar[EdgeDirectionProto]
    EDGE_DIRECTION_SIDE: _ClassVar[EdgeDirectionProto]

class EdgeTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    EDGE_TYPE_UNSPECIFIED: _ClassVar[EdgeTypeProto]
    EDGE_TYPE_PARENT: _ClassVar[EdgeTypeProto]
    EDGE_TYPE_REGULAR: _ClassVar[EdgeTypeProto]

class EditOperationProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    EDIT_OPERATION_UNSPECIFIED: _ClassVar[EditOperationProto]
    EDIT_OPERATION_SET: _ClassVar[EditOperationProto]
    EDIT_OPERATION_CLEAR: _ClassVar[EditOperationProto]

class EditTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    EDIT_TYPE_UNSPECIFIED: _ClassVar[EditTypeProto]
    EDIT_TYPE_CREATE: _ClassVar[EditTypeProto]
    EDIT_TYPE_UPSERT: _ClassVar[EditTypeProto]
    EDIT_TYPE_UPDATE: _ClassVar[EditTypeProto]
    EDIT_TYPE_MOVE: _ClassVar[EditTypeProto]
    EDIT_TYPE_ARCHIVE: _ClassVar[EditTypeProto]
    EDIT_TYPE_UNARCHIVE: _ClassVar[EditTypeProto]
    EDIT_TYPE_DELETE: _ClassVar[EditTypeProto]
    EDIT_TYPE_RESTORE: _ClassVar[EditTypeProto]
    EDIT_TYPE_ERASE: _ClassVar[EditTypeProto]

class EffectTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    EFFECT_TYPE_UNSPECIFIED: _ClassVar[EffectTypeProto]
    EFFECT_TYPE_APPEAR: _ClassVar[EffectTypeProto]
    EFFECT_TYPE_ENTER: _ClassVar[EffectTypeProto]
    EFFECT_TYPE_EXIT: _ClassVar[EffectTypeProto]
    EFFECT_TYPE_HOVER: _ClassVar[EffectTypeProto]
    EFFECT_TYPE_PRESS: _ClassVar[EffectTypeProto]
    EFFECT_TYPE_DRAG: _ClassVar[EffectTypeProto]
    EFFECT_TYPE_FOCUS: _ClassVar[EffectTypeProto]
    EFFECT_TYPE_LOOP: _ClassVar[EffectTypeProto]

class EntitlementTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ENTITLEMENT_TYPE_UNSPECIFIED: _ClassVar[EntitlementTypeProto]
    ENTITLEMENT_TYPE_PERMISSION: _ClassVar[EntitlementTypeProto]
    ENTITLEMENT_TYPE_ROLE: _ClassVar[EntitlementTypeProto]

class EnumTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ENUM_TYPE_UNSPECIFIED: _ClassVar[EnumTypeProto]
    ENUM_TYPE_ENUM_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_NODE_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_STRUCT_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_TRAIT_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_UNIVERSE_CATEGORY: _ClassVar[EnumTypeProto]
    ENUM_TYPE_NODE_DEFINITION_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_OBJECT_DEFINITION_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_STRUCT_DEFINITION_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_PROPERTY_REFERENCE_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_MATERIALIZATION: _ClassVar[EnumTypeProto]
    ENUM_TYPE_STORE_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_STORE_IMPLEMENTATION: _ClassVar[EnumTypeProto]
    ENUM_TYPE_PLATFORM_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_RUNTIME_LANGUAGE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_OPERATING_SYSTEM: _ClassVar[EnumTypeProto]
    ENUM_TYPE_EDIT_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_EDIT_OPERATION: _ClassVar[EnumTypeProto]
    ENUM_TYPE_CHANGE_STATUS: _ClassVar[EnumTypeProto]
    ENUM_TYPE_CHANGE_DEBOUNCE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_PRIMITIVE_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_TYPE_CARDINALITY: _ClassVar[EnumTypeProto]
    ENUM_TYPE_SCALAR_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_VALUE_FACTORY: _ClassVar[EnumTypeProto]
    ENUM_TYPE_STRING_FORMAT: _ClassVar[EnumTypeProto]
    ENUM_TYPE_NUMBER_FORMAT: _ClassVar[EnumTypeProto]
    ENUM_TYPE_PROPERTY_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_EDGE_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_EDGE_DIRECTION: _ClassVar[EnumTypeProto]
    ENUM_TYPE_CASCADE_ACTION: _ClassVar[EnumTypeProto]
    ENUM_TYPE_RESOURCE_STATUS: _ClassVar[EnumTypeProto]
    ENUM_TYPE_SNAPSHOT_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_SNAPSHOT_STATUS: _ClassVar[EnumTypeProto]
    ENUM_TYPE_CONDITIONAL_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_AGGREGATION_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_SORT_MODE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_SORT_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_JOIN_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_FUNCTION_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_EXPRESSION_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_QUERY_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_QUERY_UPDATE_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_SPACE_STATUS: _ClassVar[EnumTypeProto]
    ENUM_TYPE_USER_STATUS: _ClassVar[EnumTypeProto]
    ENUM_TYPE_ORGANIZATION_STATUS: _ClassVar[EnumTypeProto]
    ENUM_TYPE_CLIENT_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_FOLDER_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_ROLE_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_PERMISSION_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_SANCTION_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_ENTITLEMENT_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_FILE_RETENTION_MODE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_FILE_SOURCE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_FILE_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_FILE_FORMAT: _ClassVar[EnumTypeProto]
    ENUM_TYPE_TEXT_SPAN_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_ICON_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_METHOD_CARDINALITY: _ClassVar[EnumTypeProto]
    ENUM_TYPE_TRIGGER_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_TIMER_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_DAY_OF_WEEK: _ClassVar[EnumTypeProto]
    ENUM_TYPE_MONTH: _ClassVar[EnumTypeProto]
    ENUM_TYPE_SCHEDULE_FREQUENCY: _ClassVar[EnumTypeProto]
    ENUM_TYPE_CURSOR_STATUS: _ClassVar[EnumTypeProto]
    ENUM_TYPE_MODEL_DEVELOPER: _ClassVar[EnumTypeProto]
    ENUM_TYPE_MODEL_PROVIDER: _ClassVar[EnumTypeProto]
    ENUM_TYPE_CLOUD: _ClassVar[EnumTypeProto]
    ENUM_TYPE_REGION: _ClassVar[EnumTypeProto]
    ENUM_TYPE_REGION_AREA: _ClassVar[EnumTypeProto]
    ENUM_TYPE_REGION_CONTINENT: _ClassVar[EnumTypeProto]
    ENUM_TYPE_TENANCY: _ClassVar[EnumTypeProto]
    ENUM_TYPE_DATABASE_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_MACHINE_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_ENVIRONMENT_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_RUN_STATUS: _ClassVar[EnumTypeProto]
    ENUM_TYPE_LOG_LEVEL: _ClassVar[EnumTypeProto]
    ENUM_TYPE_THREAD_STATUS: _ClassVar[EnumTypeProto]
    ENUM_TYPE_NOTIFICATION_STATUS: _ClassVar[EnumTypeProto]
    ENUM_TYPE_WINDOW_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_LAYER_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_VARIANT_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_VARIANT_STATE_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_CANVAS_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_ARROW_HEAD_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_MODE_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_TOOL_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_MOUSE_BUTTON: _ClassVar[EnumTypeProto]
    ENUM_TYPE_COLOR_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_COLOR_SHADE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_COLOR_HUE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_COLOR_INTENT: _ClassVar[EnumTypeProto]
    ENUM_TYPE_FILL_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_FILL_POSITION: _ClassVar[EnumTypeProto]
    ENUM_TYPE_FILL_SIZE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_FONT_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_FONT_WEIGHT: _ClassVar[EnumTypeProto]
    ENUM_TYPE_FONT_SIZE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_TEXT_ALIGN: _ClassVar[EnumTypeProto]
    ENUM_TYPE_TEXT_DECORATION: _ClassVar[EnumTypeProto]
    ENUM_TYPE_TEXT_TRANSFORM: _ClassVar[EnumTypeProto]
    ENUM_TYPE_BORDER_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_SHADOW_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_SHADOW_POSITION: _ClassVar[EnumTypeProto]
    ENUM_TYPE_GRADIENT_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_TRANSITION_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_SPRING_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_EFFECT_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_STROKE_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_POSITION_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_LENGTH_UNIT: _ClassVar[EnumTypeProto]
    ENUM_TYPE_LAYOUT: _ClassVar[EnumTypeProto]
    ENUM_TYPE_DISTRIBUTE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_ALIGN: _ClassVar[EnumTypeProto]
    ENUM_TYPE_DIRECTION: _ClassVar[EnumTypeProto]
    ENUM_TYPE_OVERFLOW: _ClassVar[EnumTypeProto]
    ENUM_TYPE_DIMENSION_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_REPEAT_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_TEXT_SPLIT_TYPE: _ClassVar[EnumTypeProto]
    ENUM_TYPE_OFFSCREEN_BEHAVIOR: _ClassVar[EnumTypeProto]
    ENUM_TYPE_EASING: _ClassVar[EnumTypeProto]

class EnvironmentTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ENVIRONMENT_TYPE_UNSPECIFIED: _ClassVar[EnvironmentTypeProto]
    ENVIRONMENT_TYPE_SYSTEM: _ClassVar[EnvironmentTypeProto]
    ENVIRONMENT_TYPE_DEVELOPMENT: _ClassVar[EnvironmentTypeProto]
    ENVIRONMENT_TYPE_TEST: _ClassVar[EnvironmentTypeProto]
    ENVIRONMENT_TYPE_STAGING: _ClassVar[EnvironmentTypeProto]
    ENVIRONMENT_TYPE_PRODUCTION: _ClassVar[EnvironmentTypeProto]

class ExpressionTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    EXPRESSION_TYPE_UNSPECIFIED: _ClassVar[ExpressionTypeProto]
    EXPRESSION_TYPE_LITERAL: _ClassVar[ExpressionTypeProto]
    EXPRESSION_TYPE_ATTRIBUTE: _ClassVar[ExpressionTypeProto]
    EXPRESSION_TYPE_CONDITION: _ClassVar[ExpressionTypeProto]
    EXPRESSION_TYPE_FUNCTION: _ClassVar[ExpressionTypeProto]
    EXPRESSION_TYPE_AGGREGATION: _ClassVar[ExpressionTypeProto]

class FileFormatProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FILE_FORMAT_UNSPECIFIED: _ClassVar[FileFormatProto]
    FILE_FORMAT_TXT: _ClassVar[FileFormatProto]
    FILE_FORMAT_MARKDOWN: _ClassVar[FileFormatProto]
    FILE_FORMAT_RTF: _ClassVar[FileFormatProto]
    FILE_FORMAT_INI: _ClassVar[FileFormatProto]
    FILE_FORMAT_LOG: _ClassVar[FileFormatProto]
    FILE_FORMAT_PYTHON: _ClassVar[FileFormatProto]
    FILE_FORMAT_JAVASCRIPT: _ClassVar[FileFormatProto]
    FILE_FORMAT_TYPESCRIPT: _ClassVar[FileFormatProto]
    FILE_FORMAT_GO: _ClassVar[FileFormatProto]
    FILE_FORMAT_C_LANG: _ClassVar[FileFormatProto]
    FILE_FORMAT_CPP: _ClassVar[FileFormatProto]
    FILE_FORMAT_OBJECTIVE_C: _ClassVar[FileFormatProto]
    FILE_FORMAT_SWIFT: _ClassVar[FileFormatProto]
    FILE_FORMAT_RUBY: _ClassVar[FileFormatProto]
    FILE_FORMAT_PHP: _ClassVar[FileFormatProto]
    FILE_FORMAT_CSS: _ClassVar[FileFormatProto]
    FILE_FORMAT_JAVA: _ClassVar[FileFormatProto]
    FILE_FORMAT_KOTLIN: _ClassVar[FileFormatProto]
    FILE_FORMAT_RUST: _ClassVar[FileFormatProto]
    FILE_FORMAT_SCALA: _ClassVar[FileFormatProto]
    FILE_FORMAT_SHELL: _ClassVar[FileFormatProto]
    FILE_FORMAT_SQL: _ClassVar[FileFormatProto]
    FILE_FORMAT_POWERSHELL: _ClassVar[FileFormatProto]
    FILE_FORMAT_ASSEMBLY: _ClassVar[FileFormatProto]
    FILE_FORMAT_LATEX: _ClassVar[FileFormatProto]
    FILE_FORMAT_JPEG: _ClassVar[FileFormatProto]
    FILE_FORMAT_PNG: _ClassVar[FileFormatProto]
    FILE_FORMAT_GIF: _ClassVar[FileFormatProto]
    FILE_FORMAT_BMP: _ClassVar[FileFormatProto]
    FILE_FORMAT_TIFF: _ClassVar[FileFormatProto]
    FILE_FORMAT_WEBP: _ClassVar[FileFormatProto]
    FILE_FORMAT_SVG: _ClassVar[FileFormatProto]
    FILE_FORMAT_ICO: _ClassVar[FileFormatProto]
    FILE_FORMAT_RAW: _ClassVar[FileFormatProto]
    FILE_FORMAT_HEIC: _ClassVar[FileFormatProto]
    FILE_FORMAT_HEIF: _ClassVar[FileFormatProto]
    FILE_FORMAT_MP3: _ClassVar[FileFormatProto]
    FILE_FORMAT_WAV: _ClassVar[FileFormatProto]
    FILE_FORMAT_FLAC: _ClassVar[FileFormatProto]
    FILE_FORMAT_AAC: _ClassVar[FileFormatProto]
    FILE_FORMAT_OGG: _ClassVar[FileFormatProto]
    FILE_FORMAT_M4A: _ClassVar[FileFormatProto]
    FILE_FORMAT_WMA: _ClassVar[FileFormatProto]
    FILE_FORMAT_MP4: _ClassVar[FileFormatProto]
    FILE_FORMAT_WEBM: _ClassVar[FileFormatProto]
    FILE_FORMAT_AVI: _ClassVar[FileFormatProto]
    FILE_FORMAT_MOV: _ClassVar[FileFormatProto]
    FILE_FORMAT_WMV: _ClassVar[FileFormatProto]
    FILE_FORMAT_FLV: _ClassVar[FileFormatProto]
    FILE_FORMAT_MKV: _ClassVar[FileFormatProto]
    FILE_FORMAT_PDF: _ClassVar[FileFormatProto]
    FILE_FORMAT_DOCX: _ClassVar[FileFormatProto]
    FILE_FORMAT_PPTX: _ClassVar[FileFormatProto]
    FILE_FORMAT_ODT: _ClassVar[FileFormatProto]
    FILE_FORMAT_XLSX: _ClassVar[FileFormatProto]
    FILE_FORMAT_ODS: _ClassVar[FileFormatProto]
    FILE_FORMAT_EPUB: _ClassVar[FileFormatProto]
    FILE_FORMAT_MOBI: _ClassVar[FileFormatProto]
    FILE_FORMAT_CHM: _ClassVar[FileFormatProto]
    FILE_FORMAT_DOC: _ClassVar[FileFormatProto]
    FILE_FORMAT_XLS: _ClassVar[FileFormatProto]
    FILE_FORMAT_PPT: _ClassVar[FileFormatProto]
    FILE_FORMAT_HTML: _ClassVar[FileFormatProto]
    FILE_FORMAT_JSON: _ClassVar[FileFormatProto]
    FILE_FORMAT_YAML: _ClassVar[FileFormatProto]
    FILE_FORMAT_CSV: _ClassVar[FileFormatProto]
    FILE_FORMAT_XML: _ClassVar[FileFormatProto]
    FILE_FORMAT_TOML: _ClassVar[FileFormatProto]
    FILE_FORMAT_SQLITE: _ClassVar[FileFormatProto]
    FILE_FORMAT_PARQUET: _ClassVar[FileFormatProto]
    FILE_FORMAT_ZIP: _ClassVar[FileFormatProto]
    FILE_FORMAT_RAR: _ClassVar[FileFormatProto]
    FILE_FORMAT_TAR: _ClassVar[FileFormatProto]
    FILE_FORMAT_SEVENZIP: _ClassVar[FileFormatProto]
    FILE_FORMAT_CAB: _ClassVar[FileFormatProto]
    FILE_FORMAT_GZIP: _ClassVar[FileFormatProto]
    FILE_FORMAT_BZIP2: _ClassVar[FileFormatProto]
    FILE_FORMAT_XZ: _ClassVar[FileFormatProto]
    FILE_FORMAT_EXE: _ClassVar[FileFormatProto]
    FILE_FORMAT_APP_IMAGE: _ClassVar[FileFormatProto]
    FILE_FORMAT_APK: _ClassVar[FileFormatProto]
    FILE_FORMAT_DMG: _ClassVar[FileFormatProto]
    FILE_FORMAT_JAR: _ClassVar[FileFormatProto]
    FILE_FORMAT_MSI: _ClassVar[FileFormatProto]
    FILE_FORMAT_DEB: _ClassVar[FileFormatProto]
    FILE_FORMAT_RPM: _ClassVar[FileFormatProto]

class FileRetentionModeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FILE_RETENTION_MODE_UNSPECIFIED: _ClassVar[FileRetentionModeProto]
    FILE_RETENTION_MODE_AUTOMATIC: _ClassVar[FileRetentionModeProto]
    FILE_RETENTION_MODE_MANUAL: _ClassVar[FileRetentionModeProto]
    FILE_RETENTION_MODE_TIMED: _ClassVar[FileRetentionModeProto]

class FileSourceProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FILE_SOURCE_UNSPECIFIED: _ClassVar[FileSourceProto]
    FILE_SOURCE_SPACE: _ClassVar[FileSourceProto]
    FILE_SOURCE_INLINE: _ClassVar[FileSourceProto]
    FILE_SOURCE_EXTERNAL: _ClassVar[FileSourceProto]

class FileTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FILE_TYPE_UNSPECIFIED: _ClassVar[FileTypeProto]
    FILE_TYPE_TEXT: _ClassVar[FileTypeProto]
    FILE_TYPE_CODE: _ClassVar[FileTypeProto]
    FILE_TYPE_IMAGE: _ClassVar[FileTypeProto]
    FILE_TYPE_AUDIO: _ClassVar[FileTypeProto]
    FILE_TYPE_VIDEO: _ClassVar[FileTypeProto]
    FILE_TYPE_DOCUMENT: _ClassVar[FileTypeProto]
    FILE_TYPE_DATA: _ClassVar[FileTypeProto]
    FILE_TYPE_ARCHIVE: _ClassVar[FileTypeProto]
    FILE_TYPE_EXECUTABLE: _ClassVar[FileTypeProto]
    FILE_TYPE_GENERIC: _ClassVar[FileTypeProto]

class FillPositionProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FILL_POSITION_UNSPECIFIED: _ClassVar[FillPositionProto]
    FILL_POSITION_TOP_LEFT: _ClassVar[FillPositionProto]
    FILL_POSITION_TOP_CENTER: _ClassVar[FillPositionProto]
    FILL_POSITION_TOP_RIGHT: _ClassVar[FillPositionProto]
    FILL_POSITION_LEFT: _ClassVar[FillPositionProto]
    FILL_POSITION_CENTER: _ClassVar[FillPositionProto]
    FILL_POSITION_RIGHT: _ClassVar[FillPositionProto]
    FILL_POSITION_BOTTOM_LEFT: _ClassVar[FillPositionProto]
    FILL_POSITION_BOTTOM_CENTER: _ClassVar[FillPositionProto]
    FILL_POSITION_BOTTOM_RIGHT: _ClassVar[FillPositionProto]

class FillSizeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FILL_SIZE_UNSPECIFIED: _ClassVar[FillSizeProto]
    FILL_SIZE_FILL: _ClassVar[FillSizeProto]
    FILL_SIZE_STRETCH: _ClassVar[FillSizeProto]
    FILL_SIZE_FIT: _ClassVar[FillSizeProto]
    FILL_SIZE_TILE: _ClassVar[FillSizeProto]

class FillTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FILL_TYPE_UNSPECIFIED: _ClassVar[FillTypeProto]
    FILL_TYPE_SOLID: _ClassVar[FillTypeProto]
    FILL_TYPE_GRADIENT: _ClassVar[FillTypeProto]
    FILL_TYPE_IMAGE: _ClassVar[FillTypeProto]

class FolderTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FOLDER_TYPE_UNSPECIFIED: _ClassVar[FolderTypeProto]
    FOLDER_TYPE_SYSTEM: _ClassVar[FolderTypeProto]
    FOLDER_TYPE_HOME: _ClassVar[FolderTypeProto]
    FOLDER_TYPE_GENERAL: _ClassVar[FolderTypeProto]
    FOLDER_TYPE_MODULE: _ClassVar[FolderTypeProto]
    FOLDER_TYPE_APP: _ClassVar[FolderTypeProto]

class FontSizeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FONT_SIZE_UNSPECIFIED: _ClassVar[FontSizeProto]
    FONT_SIZE_XS: _ClassVar[FontSizeProto]
    FONT_SIZE_SM: _ClassVar[FontSizeProto]
    FONT_SIZE_BASE: _ClassVar[FontSizeProto]
    FONT_SIZE_LG: _ClassVar[FontSizeProto]
    FONT_SIZE_XL: _ClassVar[FontSizeProto]
    FONT_SIZE_XL2: _ClassVar[FontSizeProto]
    FONT_SIZE_XL3: _ClassVar[FontSizeProto]
    FONT_SIZE_XL4: _ClassVar[FontSizeProto]
    FONT_SIZE_XL5: _ClassVar[FontSizeProto]
    FONT_SIZE_XL6: _ClassVar[FontSizeProto]
    FONT_SIZE_XL7: _ClassVar[FontSizeProto]

class FontTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FONT_TYPE_UNSPECIFIED: _ClassVar[FontTypeProto]
    FONT_TYPE_SERIF: _ClassVar[FontTypeProto]
    FONT_TYPE_SANS: _ClassVar[FontTypeProto]
    FONT_TYPE_MONO: _ClassVar[FontTypeProto]

class FontWeightProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FONT_WEIGHT_UNSPECIFIED: _ClassVar[FontWeightProto]
    FONT_WEIGHT_THIN: _ClassVar[FontWeightProto]
    FONT_WEIGHT_EXTRA_LIGHT: _ClassVar[FontWeightProto]
    FONT_WEIGHT_LIGHT: _ClassVar[FontWeightProto]
    FONT_WEIGHT_NORMAL: _ClassVar[FontWeightProto]
    FONT_WEIGHT_MEDIUM: _ClassVar[FontWeightProto]
    FONT_WEIGHT_SEMI_BOLD: _ClassVar[FontWeightProto]
    FONT_WEIGHT_BOLD: _ClassVar[FontWeightProto]
    FONT_WEIGHT_EXTRA_BOLD: _ClassVar[FontWeightProto]
    FONT_WEIGHT_BLACK: _ClassVar[FontWeightProto]

class FunctionTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FUNCTION_TYPE_UNSPECIFIED: _ClassVar[FunctionTypeProto]
    FUNCTION_TYPE_ADD: _ClassVar[FunctionTypeProto]
    FUNCTION_TYPE_SUBTRACT: _ClassVar[FunctionTypeProto]
    FUNCTION_TYPE_MULTIPLY: _ClassVar[FunctionTypeProto]
    FUNCTION_TYPE_DIVIDE: _ClassVar[FunctionTypeProto]
    FUNCTION_TYPE_MODULO: _ClassVar[FunctionTypeProto]
    FUNCTION_TYPE_POWER: _ClassVar[FunctionTypeProto]

class GradientTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    GRADIENT_TYPE_UNSPECIFIED: _ClassVar[GradientTypeProto]
    GRADIENT_TYPE_LINEAR: _ClassVar[GradientTypeProto]
    GRADIENT_TYPE_RADIAL: _ClassVar[GradientTypeProto]
    GRADIENT_TYPE_CONIC: _ClassVar[GradientTypeProto]

class IconTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ICON_TYPE_UNSPECIFIED: _ClassVar[IconTypeProto]
    ICON_TYPE_EMOJI: _ClassVar[IconTypeProto]
    ICON_TYPE_FONT_AWESOME: _ClassVar[IconTypeProto]
    ICON_TYPE_VS_CODE: _ClassVar[IconTypeProto]
    ICON_TYPE_FILE: _ClassVar[IconTypeProto]
    ICON_TYPE_FILE_URL: _ClassVar[IconTypeProto]

class JoinTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    JOIN_TYPE_UNSPECIFIED: _ClassVar[JoinTypeProto]
    JOIN_TYPE_LEFT: _ClassVar[JoinTypeProto]
    JOIN_TYPE_PARENT: _ClassVar[JoinTypeProto]
    JOIN_TYPE_CHILD: _ClassVar[JoinTypeProto]

class LayerTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    LAYER_TYPE_UNSPECIFIED: _ClassVar[LayerTypeProto]
    LAYER_TYPE_GENERAL: _ClassVar[LayerTypeProto]
    LAYER_TYPE_SHAPE: _ClassVar[LayerTypeProto]

class LayoutProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    LAYOUT_UNSPECIFIED: _ClassVar[LayoutProto]
    LAYOUT_STACK: _ClassVar[LayoutProto]
    LAYOUT_GRID: _ClassVar[LayoutProto]

class LengthUnitProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    LENGTH_UNIT_UNSPECIFIED: _ClassVar[LengthUnitProto]
    LENGTH_UNIT_PIXEL: _ClassVar[LengthUnitProto]
    LENGTH_UNIT_REM: _ClassVar[LengthUnitProto]
    LENGTH_UNIT_PERCENT: _ClassVar[LengthUnitProto]
    LENGTH_UNIT_FR: _ClassVar[LengthUnitProto]

class LogLevelProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    LOG_LEVEL_UNSPECIFIED: _ClassVar[LogLevelProto]
    LOG_LEVEL_TRACE: _ClassVar[LogLevelProto]
    LOG_LEVEL_DEBUG: _ClassVar[LogLevelProto]
    LOG_LEVEL_INFO: _ClassVar[LogLevelProto]
    LOG_LEVEL_WARNING: _ClassVar[LogLevelProto]
    LOG_LEVEL_ERROR: _ClassVar[LogLevelProto]
    LOG_LEVEL_PANIC: _ClassVar[LogLevelProto]

class MachineTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MACHINE_TYPE_UNSPECIFIED: _ClassVar[MachineTypeProto]
    MACHINE_TYPE_RUNTIME: _ClassVar[MachineTypeProto]
    MACHINE_TYPE_UBUNTU: _ClassVar[MachineTypeProto]
    MACHINE_TYPE_MAC: _ClassVar[MachineTypeProto]
    MACHINE_TYPE_WINDOWS: _ClassVar[MachineTypeProto]
    MACHINE_TYPE_CUSTOM: _ClassVar[MachineTypeProto]

class MaterializationProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MATERIALIZATION_UNSPECIFIED: _ClassVar[MaterializationProto]
    MATERIALIZATION_PARTIAL: _ClassVar[MaterializationProto]
    MATERIALIZATION_FULL: _ClassVar[MaterializationProto]

class MethodCardinalityProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    METHOD_CARDINALITY_UNSPECIFIED: _ClassVar[MethodCardinalityProto]
    METHOD_CARDINALITY_UNARY: _ClassVar[MethodCardinalityProto]

class ModeTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MODE_TYPE_UNSPECIFIED: _ClassVar[ModeTypeProto]
    MODE_TYPE_EDIT: _ClassVar[ModeTypeProto]
    MODE_TYPE_DEBUG: _ClassVar[ModeTypeProto]
    MODE_TYPE_INSPECT: _ClassVar[ModeTypeProto]
    MODE_TYPE_PREVIEW: _ClassVar[ModeTypeProto]
    MODE_TYPE_USE: _ClassVar[ModeTypeProto]

class ModelDeveloperProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MODEL_DEVELOPER_UNSPECIFIED: _ClassVar[ModelDeveloperProto]
    MODEL_DEVELOPER_OPENAI: _ClassVar[ModelDeveloperProto]
    MODEL_DEVELOPER_ANTHROPIC: _ClassVar[ModelDeveloperProto]
    MODEL_DEVELOPER_GOOGLE: _ClassVar[ModelDeveloperProto]
    MODEL_DEVELOPER_XAI: _ClassVar[ModelDeveloperProto]

class ModelProviderProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MODEL_PROVIDER_UNSPECIFIED: _ClassVar[ModelProviderProto]
    MODEL_PROVIDER_OPENROUTER: _ClassVar[ModelProviderProto]
    MODEL_PROVIDER_OPENAI: _ClassVar[ModelProviderProto]
    MODEL_PROVIDER_ANTHROPIC: _ClassVar[ModelProviderProto]
    MODEL_PROVIDER_GOOGLE: _ClassVar[ModelProviderProto]
    MODEL_PROVIDER_XAI: _ClassVar[ModelProviderProto]

class MonthProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MONTH_UNSPECIFIED: _ClassVar[MonthProto]
    MONTH_JANUARY: _ClassVar[MonthProto]
    MONTH_FEBRUARY: _ClassVar[MonthProto]
    MONTH_MARCH: _ClassVar[MonthProto]
    MONTH_APRIL: _ClassVar[MonthProto]
    MONTH_MAY: _ClassVar[MonthProto]
    MONTH_JUNE: _ClassVar[MonthProto]
    MONTH_JULY: _ClassVar[MonthProto]
    MONTH_AUGUST: _ClassVar[MonthProto]
    MONTH_SEPTEMBER: _ClassVar[MonthProto]
    MONTH_OCTOBER: _ClassVar[MonthProto]
    MONTH_NOVEMBER: _ClassVar[MonthProto]
    MONTH_DECEMBER: _ClassVar[MonthProto]

class MouseButtonProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MOUSE_BUTTON_UNSPECIFIED: _ClassVar[MouseButtonProto]
    MOUSE_BUTTON_LEFT: _ClassVar[MouseButtonProto]
    MOUSE_BUTTON_RIGHT: _ClassVar[MouseButtonProto]
    MOUSE_BUTTON_MIDDLE: _ClassVar[MouseButtonProto]

class NodeDefinitionTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    NODE_DEFINITION_TYPE_UNSPECIFIED: _ClassVar[NodeDefinitionTypeProto]
    NODE_DEFINITION_TYPE_BUILTIN: _ClassVar[NodeDefinitionTypeProto]
    NODE_DEFINITION_TYPE_CUSTOM: _ClassVar[NodeDefinitionTypeProto]

class NodeTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    NODE_TYPE_UNSPECIFIED: _ClassVar[NodeTypeProto]
    NODE_TYPE_NODE: _ClassVar[NodeTypeProto]
    NODE_TYPE_ENTITY: _ClassVar[NodeTypeProto]
    NODE_TYPE_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_CUSTOM_ENTITY_DEFINITION: _ClassVar[NodeTypeProto]
    NODE_TYPE_CUSTOM_TRAIT_DEFINITION: _ClassVar[NodeTypeProto]
    NODE_TYPE_CUSTOM_EVENT_DEFINITION: _ClassVar[NodeTypeProto]
    NODE_TYPE_CUSTOM_STRUCT_DEFINITION: _ClassVar[NodeTypeProto]
    NODE_TYPE_CUSTOM_ENUM_DEFINITION: _ClassVar[NodeTypeProto]
    NODE_TYPE_CUSTOM_PROPERTY: _ClassVar[NodeTypeProto]
    NODE_TYPE_CUSTOM_PROPERTY_GROUP: _ClassVar[NodeTypeProto]
    NODE_TYPE_CUSTOM_OPTION: _ClassVar[NodeTypeProto]
    NODE_TYPE_CUSTOM_OPTION_GROUP: _ClassVar[NodeTypeProto]
    NODE_TYPE_RECORD: _ClassVar[NodeTypeProto]
    NODE_TYPE_RESOURCE: _ClassVar[NodeTypeProto]
    NODE_TYPE_METRIC: _ClassVar[NodeTypeProto]
    NODE_TYPE_SNAPSHOT: _ClassVar[NodeTypeProto]
    NODE_TYPE_SIGNAL: _ClassVar[NodeTypeProto]
    NODE_TYPE_EDIT_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_CHANGE_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_QUERY_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_MEASUREMENT_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_UNIVERSE: _ClassVar[NodeTypeProto]
    NODE_TYPE_SPACE: _ClassVar[NodeTypeProto]
    NODE_TYPE_HANDLE: _ClassVar[NodeTypeProto]
    NODE_TYPE_USER: _ClassVar[NodeTypeProto]
    NODE_TYPE_FRIENDSHIP: _ClassVar[NodeTypeProto]
    NODE_TYPE_FRIENDSHIP_INVITE: _ClassVar[NodeTypeProto]
    NODE_TYPE_FRIENDSHIP_INVITE_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_FRIENDSHIP_INVITE_SENT_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_FRIENDSHIP_INVITE_RESCINDED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_FRIENDSHIP_INVITE_ACCEPTED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_FRIENDSHIP_INVITE_REJECTED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_CLIENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_ORGANIZATION: _ClassVar[NodeTypeProto]
    NODE_TYPE_TEAM: _ClassVar[NodeTypeProto]
    NODE_TYPE_FOLDER: _ClassVar[NodeTypeProto]
    NODE_TYPE_TAG: _ClassVar[NodeTypeProto]
    NODE_TYPE_TAGGING: _ClassVar[NodeTypeProto]
    NODE_TYPE_BRANCH: _ClassVar[NodeTypeProto]
    NODE_TYPE_MEMBERSHIP: _ClassVar[NodeTypeProto]
    NODE_TYPE_MEMBERSHIP_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_MEMBERSHIP_JOINED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_MEMBERSHIP_LEFT_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_INVITE: _ClassVar[NodeTypeProto]
    NODE_TYPE_INVITE_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_INVITE_SENT_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_INVITE_RESCINDED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_INVITE_ACCEPTED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_INVITE_REJECTED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_ROLE: _ClassVar[NodeTypeProto]
    NODE_TYPE_ROLE_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_ROLE_ASSIGNED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_ROLE_UNASSIGNED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_PERMISSION: _ClassVar[NodeTypeProto]
    NODE_TYPE_SANCTION: _ClassVar[NodeTypeProto]
    NODE_TYPE_SANCTION_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_SANCTION_REQUESTED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_SANCTION_GRANTED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_SANCTION_REVOKED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_SANCTION_EXPIRED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_ENTITLEMENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_ENTITLEMENT_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_ENTITLEMENT_REQUESTED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_ENTITLEMENT_GRANTED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_ENTITLEMENT_REVOKED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_ENTITLEMENT_EXPIRED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_AGENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_FILE: _ClassVar[NodeTypeProto]
    NODE_TYPE_SERVICE: _ClassVar[NodeTypeProto]
    NODE_TYPE_SCRIPT: _ClassVar[NodeTypeProto]
    NODE_TYPE_METHOD: _ClassVar[NodeTypeProto]
    NODE_TYPE_ACTION: _ClassVar[NodeTypeProto]
    NODE_TYPE_TRIGGER: _ClassVar[NodeTypeProto]
    NODE_TYPE_TRIGGER_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_TIMER: _ClassVar[NodeTypeProto]
    NODE_TYPE_TIMER_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_TIMER_STARTED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_TIMER_COMPLETED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_TIMER_CANCELLED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_CURSOR: _ClassVar[NodeTypeProto]
    NODE_TYPE_EVENT_CURSOR: _ClassVar[NodeTypeProto]
    NODE_TYPE_SCREEN_CURSOR: _ClassVar[NodeTypeProto]
    NODE_TYPE_THREAD_CURSOR: _ClassVar[NodeTypeProto]
    NODE_TYPE_ROUTE: _ClassVar[NodeTypeProto]
    NODE_TYPE_DATABASE: _ClassVar[NodeTypeProto]
    NODE_TYPE_MACHINE: _ClassVar[NodeTypeProto]
    NODE_TYPE_ENVIRONMENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_RUN: _ClassVar[NodeTypeProto]
    NODE_TYPE_RUN_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_RUN_STARTED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_RUN_PAUSE_REQUESTED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_RUN_PAUSED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_RUN_RESUME_REQUESTED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_RUN_RESUMED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_RUN_STOP_REQUESTED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_RUN_FAILED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_RUN_COMPLETED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_SPAN_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_LOG_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_GAUGE_METRIC: _ClassVar[NodeTypeProto]
    NODE_TYPE_GAUGE_MEASUREMENT_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_COUNTER_METRIC: _ClassVar[NodeTypeProto]
    NODE_TYPE_COUNTER_MEASUREMENT_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_HISTOGRAM_METRIC: _ClassVar[NodeTypeProto]
    NODE_TYPE_HISTOGRAM_MEASUREMENT_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_THREAD: _ClassVar[NodeTypeProto]
    NODE_TYPE_MESSAGE: _ClassVar[NodeTypeProto]
    NODE_TYPE_REACTION: _ClassVar[NodeTypeProto]
    NODE_TYPE_STAR: _ClassVar[NodeTypeProto]
    NODE_TYPE_FOLLOW: _ClassVar[NodeTypeProto]
    NODE_TYPE_NOTIFICATION: _ClassVar[NodeTypeProto]
    NODE_TYPE_NOTIFICATION_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_NOTIFICATION_SENT_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_NOTIFICATION_RESCINDED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_NOTIFICATION_READ_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_NOTIFICATION_DISMISSED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_NOTIFICATION_EXPIRED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_WINDOW: _ClassVar[NodeTypeProto]
    NODE_TYPE_SCENE: _ClassVar[NodeTypeProto]
    NODE_TYPE_SCENE_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_LAYER: _ClassVar[NodeTypeProto]
    NODE_TYPE_VARIANT: _ClassVar[NodeTypeProto]
    NODE_TYPE_VIEW: _ClassVar[NodeTypeProto]
    NODE_TYPE_VIEW_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_VIEW_ENTERED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_VIEW_EXITED_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_CONTAINER_VIEW: _ClassVar[NodeTypeProto]
    NODE_TYPE_FRAME_VIEW: _ClassVar[NodeTypeProto]
    NODE_TYPE_LABEL_VIEW: _ClassVar[NodeTypeProto]
    NODE_TYPE_SPLIT_VIEW: _ClassVar[NodeTypeProto]
    NODE_TYPE_CONTENT_VIEW: _ClassVar[NodeTypeProto]
    NODE_TYPE_TEXT_VIEW: _ClassVar[NodeTypeProto]
    NODE_TYPE_INPUT_VIEW: _ClassVar[NodeTypeProto]
    NODE_TYPE_NUMBER_INPUT_VIEW: _ClassVar[NodeTypeProto]
    NODE_TYPE_SLIDER_INPUT_VIEW: _ClassVar[NodeTypeProto]
    NODE_TYPE_INTERNAL_VIEW: _ClassVar[NodeTypeProto]
    NODE_TYPE_CANVAS: _ClassVar[NodeTypeProto]
    NODE_TYPE_SHAPE: _ClassVar[NodeTypeProto]
    NODE_TYPE_LINE_SHAPE: _ClassVar[NodeTypeProto]
    NODE_TYPE_ARROW_SHAPE: _ClassVar[NodeTypeProto]
    NODE_TYPE_ANNOTATION_SHAPE: _ClassVar[NodeTypeProto]
    NODE_TYPE_INPUT_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_POINTER_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_POINTER_DOWN_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_POINTER_UP_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_POINTER_MOVE_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_POINTER_ENTER_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_POINTER_OVER_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_POINTER_LEAVE_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_POINTER_LONG_PRESS_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_MOUSE_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_CLICK_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_LEFT_CLICK_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_RIGHT_CLICK_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_MIDDLE_CLICK_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_DOUBLE_CLICK_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_WHEEL_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_KEYBOARD_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_KEY_DOWN_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_KEY_UP_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_KEY_PRESS_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_DRAG_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_DRAG_START_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_DRAG_END_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_DRAG_OVER_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_DRAG_ENTER_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_DRAG_LEAVE_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_DROP_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_CLIPBOARD_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_COPY_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_CUT_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_PASTE_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_FOCUS_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_FOCUS_IN_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_FOCUS_OUT_EVENT: _ClassVar[NodeTypeProto]
    NODE_TYPE_THEME: _ClassVar[NodeTypeProto]
    NODE_TYPE_PALETTE: _ClassVar[NodeTypeProto]
    NODE_TYPE_STYLE: _ClassVar[NodeTypeProto]
    NODE_TYPE_COLOR_STYLE: _ClassVar[NodeTypeProto]
    NODE_TYPE_FILL_STYLE: _ClassVar[NodeTypeProto]
    NODE_TYPE_FONT_STYLE: _ClassVar[NodeTypeProto]
    NODE_TYPE_BORDER_STYLE: _ClassVar[NodeTypeProto]
    NODE_TYPE_SHADOW_STYLE: _ClassVar[NodeTypeProto]
    NODE_TYPE_GRADIENT_STYLE: _ClassVar[NodeTypeProto]
    NODE_TYPE_TRANSITION_STYLE: _ClassVar[NodeTypeProto]
    NODE_TYPE_EFFECT_STYLE: _ClassVar[NodeTypeProto]
    NODE_TYPE_STROKE_STYLE: _ClassVar[NodeTypeProto]

class NotificationStatusProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    NOTIFICATION_STATUS_UNSPECIFIED: _ClassVar[NotificationStatusProto]
    NOTIFICATION_STATUS_UNREAD: _ClassVar[NotificationStatusProto]
    NOTIFICATION_STATUS_READ: _ClassVar[NotificationStatusProto]
    NOTIFICATION_STATUS_DISMISSED: _ClassVar[NotificationStatusProto]
    NOTIFICATION_STATUS_EXPIRED: _ClassVar[NotificationStatusProto]
    NOTIFICATION_STATUS_RESCINDED: _ClassVar[NotificationStatusProto]

class NumberFormatProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    NUMBER_FORMAT_UNSPECIFIED: _ClassVar[NumberFormatProto]
    NUMBER_FORMAT_PERCENTAGE: _ClassVar[NumberFormatProto]
    NUMBER_FORMAT_ANGLE: _ClassVar[NumberFormatProto]
    NUMBER_FORMAT_CURRENCY: _ClassVar[NumberFormatProto]

class ObjectDefinitionTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    OBJECT_DEFINITION_TYPE_UNSPECIFIED: _ClassVar[ObjectDefinitionTypeProto]
    OBJECT_DEFINITION_TYPE_BUILTIN_NODE: _ClassVar[ObjectDefinitionTypeProto]
    OBJECT_DEFINITION_TYPE_CUSTOM_NODE: _ClassVar[ObjectDefinitionTypeProto]
    OBJECT_DEFINITION_TYPE_BUILTIN_TRAIT: _ClassVar[ObjectDefinitionTypeProto]
    OBJECT_DEFINITION_TYPE_CUSTOM_TRAIT: _ClassVar[ObjectDefinitionTypeProto]
    OBJECT_DEFINITION_TYPE_BUILTIN_STRUCT: _ClassVar[ObjectDefinitionTypeProto]
    OBJECT_DEFINITION_TYPE_CUSTOM_STRUCT: _ClassVar[ObjectDefinitionTypeProto]

class OffscreenBehaviorProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    OFFSCREEN_BEHAVIOR_UNSPECIFIED: _ClassVar[OffscreenBehaviorProto]
    OFFSCREEN_BEHAVIOR_PLAY: _ClassVar[OffscreenBehaviorProto]
    OFFSCREEN_BEHAVIOR_PAUSE: _ClassVar[OffscreenBehaviorProto]

class OperatingSystemProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    OPERATING_SYSTEM_UNSPECIFIED: _ClassVar[OperatingSystemProto]
    OPERATING_SYSTEM_LINUX: _ClassVar[OperatingSystemProto]
    OPERATING_SYSTEM_WINDOWS: _ClassVar[OperatingSystemProto]
    OPERATING_SYSTEM_MACOS: _ClassVar[OperatingSystemProto]
    OPERATING_SYSTEM_ANDROID: _ClassVar[OperatingSystemProto]
    OPERATING_SYSTEM_IOS: _ClassVar[OperatingSystemProto]

class OrganizationStatusProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ORGANIZATION_STATUS_UNSPECIFIED: _ClassVar[OrganizationStatusProto]
    ORGANIZATION_STATUS_CREATING: _ClassVar[OrganizationStatusProto]
    ORGANIZATION_STATUS_ACTIVE: _ClassVar[OrganizationStatusProto]

class OverflowProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    OVERFLOW_UNSPECIFIED: _ClassVar[OverflowProto]
    OVERFLOW_HIDDEN: _ClassVar[OverflowProto]
    OVERFLOW_VISIBLE: _ClassVar[OverflowProto]
    OVERFLOW_SCROLL: _ClassVar[OverflowProto]

class PermissionTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PERMISSION_TYPE_UNSPECIFIED: _ClassVar[PermissionTypeProto]
    PERMISSION_TYPE_GENERAL: _ClassVar[PermissionTypeProto]

class PlatformTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PLATFORM_TYPE_UNSPECIFIED: _ClassVar[PlatformTypeProto]
    PLATFORM_TYPE_SYSTEM: _ClassVar[PlatformTypeProto]
    PLATFORM_TYPE_RUNTIME: _ClassVar[PlatformTypeProto]
    PLATFORM_TYPE_WEB: _ClassVar[PlatformTypeProto]

class PositionTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    POSITION_TYPE_UNSPECIFIED: _ClassVar[PositionTypeProto]
    POSITION_TYPE_RELATIVE: _ClassVar[PositionTypeProto]
    POSITION_TYPE_ABSOLUTE: _ClassVar[PositionTypeProto]
    POSITION_TYPE_FIXED: _ClassVar[PositionTypeProto]
    POSITION_TYPE_STICKY: _ClassVar[PositionTypeProto]

class PrimitiveTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PRIMITIVE_TYPE_UNSPECIFIED: _ClassVar[PrimitiveTypeProto]
    PRIMITIVE_TYPE_BOOLEAN: _ClassVar[PrimitiveTypeProto]
    PRIMITIVE_TYPE_INT16: _ClassVar[PrimitiveTypeProto]
    PRIMITIVE_TYPE_INT32: _ClassVar[PrimitiveTypeProto]
    PRIMITIVE_TYPE_INT64: _ClassVar[PrimitiveTypeProto]
    PRIMITIVE_TYPE_DECIMAL: _ClassVar[PrimitiveTypeProto]
    PRIMITIVE_TYPE_FLOAT32: _ClassVar[PrimitiveTypeProto]
    PRIMITIVE_TYPE_FLOAT64: _ClassVar[PrimitiveTypeProto]
    PRIMITIVE_TYPE_STRING: _ClassVar[PrimitiveTypeProto]
    PRIMITIVE_TYPE_UUID: _ClassVar[PrimitiveTypeProto]
    PRIMITIVE_TYPE_JSON: _ClassVar[PrimitiveTypeProto]
    PRIMITIVE_TYPE_BYTES: _ClassVar[PrimitiveTypeProto]
    PRIMITIVE_TYPE_DATETIME: _ClassVar[PrimitiveTypeProto]
    PRIMITIVE_TYPE_DATE: _ClassVar[PrimitiveTypeProto]
    PRIMITIVE_TYPE_TIME: _ClassVar[PrimitiveTypeProto]
    PRIMITIVE_TYPE_DURATION: _ClassVar[PrimitiveTypeProto]

class PropertyReferenceTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PROPERTY_REFERENCE_TYPE_UNSPECIFIED: _ClassVar[PropertyReferenceTypeProto]
    PROPERTY_REFERENCE_TYPE_BUILTIN: _ClassVar[PropertyReferenceTypeProto]
    PROPERTY_REFERENCE_TYPE_CUSTOM: _ClassVar[PropertyReferenceTypeProto]

class PropertyTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PROPERTY_TYPE_UNSPECIFIED: _ClassVar[PropertyTypeProto]
    PROPERTY_TYPE_MEMBER: _ClassVar[PropertyTypeProto]
    PROPERTY_TYPE_CONSTANT: _ClassVar[PropertyTypeProto]
    PROPERTY_TYPE_INPUT: _ClassVar[PropertyTypeProto]
    PROPERTY_TYPE_OUTPUT: _ClassVar[PropertyTypeProto]

class QueryTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    QUERY_TYPE_UNSPECIFIED: _ClassVar[QueryTypeProto]
    QUERY_TYPE_NODE: _ClassVar[QueryTypeProto]
    QUERY_TYPE_SCALAR: _ClassVar[QueryTypeProto]
    QUERY_TYPE_GROUPED_NODE: _ClassVar[QueryTypeProto]
    QUERY_TYPE_GROUPED_SCALAR: _ClassVar[QueryTypeProto]

class QueryUpdateTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    QUERY_UPDATE_TYPE_UNSPECIFIED: _ClassVar[QueryUpdateTypeProto]
    QUERY_UPDATE_TYPE_FULL_RESULT: _ClassVar[QueryUpdateTypeProto]
    QUERY_UPDATE_TYPE_PARTIAL_RESULT: _ClassVar[QueryUpdateTypeProto]

class RegionProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    REGION_UNSPECIFIED: _ClassVar[RegionProto]
    REGION_ZURICH: _ClassVar[RegionProto]
    REGION_FRANKFURT: _ClassVar[RegionProto]
    REGION_VIRGINIA: _ClassVar[RegionProto]
    REGION_OHIO: _ClassVar[RegionProto]
    REGION_OREGON: _ClassVar[RegionProto]
    REGION_SAO_PAULO: _ClassVar[RegionProto]
    REGION_CAPE_TOWN: _ClassVar[RegionProto]
    REGION_MUMBAI: _ClassVar[RegionProto]
    REGION_SINGAPORE: _ClassVar[RegionProto]
    REGION_TOKYO: _ClassVar[RegionProto]
    REGION_SYDNEY: _ClassVar[RegionProto]

class RegionAreaProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    REGION_AREA_UNSPECIFIED: _ClassVar[RegionAreaProto]
    REGION_AREA_EUROPE_CENTRAL: _ClassVar[RegionAreaProto]
    REGION_AREA_NORTH_AMERICA_EAST: _ClassVar[RegionAreaProto]
    REGION_AREA_NORTH_AMERICA_WEST: _ClassVar[RegionAreaProto]
    REGION_AREA_SOUTH_AMERICA_EAST: _ClassVar[RegionAreaProto]
    REGION_AREA_MIDDLE_EAST_CENTRAL: _ClassVar[RegionAreaProto]
    REGION_AREA_MIDDLE_EAST_WEST: _ClassVar[RegionAreaProto]
    REGION_AREA_AFRICA_SOUTH: _ClassVar[RegionAreaProto]
    REGION_AREA_ASIA_WEST: _ClassVar[RegionAreaProto]
    REGION_AREA_ASIA_SOUTH: _ClassVar[RegionAreaProto]
    REGION_AREA_ASIA_EAST: _ClassVar[RegionAreaProto]
    REGION_AREA_AUSTRALIA_SOUTH: _ClassVar[RegionAreaProto]

class RegionContinentProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    REGION_CONTINENT_UNSPECIFIED: _ClassVar[RegionContinentProto]
    REGION_CONTINENT_EUROPE: _ClassVar[RegionContinentProto]
    REGION_CONTINENT_NORTH_AMERICA: _ClassVar[RegionContinentProto]
    REGION_CONTINENT_SOUTH_AMERICA: _ClassVar[RegionContinentProto]
    REGION_CONTINENT_MIDDLE_EAST: _ClassVar[RegionContinentProto]
    REGION_CONTINENT_AFRICA: _ClassVar[RegionContinentProto]
    REGION_CONTINENT_ASIA: _ClassVar[RegionContinentProto]
    REGION_CONTINENT_AUSTRALIA: _ClassVar[RegionContinentProto]
    REGION_CONTINENT_PRIVATE: _ClassVar[RegionContinentProto]

class RepeatTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    REPEAT_TYPE_UNSPECIFIED: _ClassVar[RepeatTypeProto]
    REPEAT_TYPE_LOOP: _ClassVar[RepeatTypeProto]
    REPEAT_TYPE_REVERSE: _ClassVar[RepeatTypeProto]
    REPEAT_TYPE_MIRROR: _ClassVar[RepeatTypeProto]

class ResourceStatusProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RESOURCE_STATUS_UNSPECIFIED: _ClassVar[ResourceStatusProto]
    RESOURCE_STATUS_PENDING: _ClassVar[ResourceStatusProto]
    RESOURCE_STATUS_CREATING: _ClassVar[ResourceStatusProto]
    RESOURCE_STATUS_RETRYING: _ClassVar[ResourceStatusProto]
    RESOURCE_STATUS_AVAILABLE: _ClassVar[ResourceStatusProto]
    RESOURCE_STATUS_SLEEPING: _ClassVar[ResourceStatusProto]
    RESOURCE_STATUS_UNAVAILABLE: _ClassVar[ResourceStatusProto]
    RESOURCE_STATUS_IMPAIRED: _ClassVar[ResourceStatusProto]
    RESOURCE_STATUS_OFFLINE: _ClassVar[ResourceStatusProto]
    RESOURCE_STATUS_FAILED: _ClassVar[ResourceStatusProto]

class RoleTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ROLE_TYPE_UNSPECIFIED: _ClassVar[RoleTypeProto]
    ROLE_TYPE_SYSTEM: _ClassVar[RoleTypeProto]
    ROLE_TYPE_OWNER: _ClassVar[RoleTypeProto]
    ROLE_TYPE_ADMIN: _ClassVar[RoleTypeProto]
    ROLE_TYPE_DEVELOPER: _ClassVar[RoleTypeProto]
    ROLE_TYPE_USER: _ClassVar[RoleTypeProto]
    ROLE_TYPE_SPECTATOR: _ClassVar[RoleTypeProto]

class RunStatusProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RUN_STATUS_UNSPECIFIED: _ClassVar[RunStatusProto]
    RUN_STATUS_SCHEDULED: _ClassVar[RunStatusProto]
    RUN_STATUS_RUNNING: _ClassVar[RunStatusProto]
    RUN_STATUS_PAUSED: _ClassVar[RunStatusProto]
    RUN_STATUS_YIELDED: _ClassVar[RunStatusProto]
    RUN_STATUS_CANCELLED: _ClassVar[RunStatusProto]
    RUN_STATUS_ABORTED: _ClassVar[RunStatusProto]
    RUN_STATUS_FAILED: _ClassVar[RunStatusProto]
    RUN_STATUS_COMPLETED: _ClassVar[RunStatusProto]

class RuntimeLanguageProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RUNTIME_LANGUAGE_UNSPECIFIED: _ClassVar[RuntimeLanguageProto]
    RUNTIME_LANGUAGE_PYTHON: _ClassVar[RuntimeLanguageProto]
    RUNTIME_LANGUAGE_JAVASCRIPT: _ClassVar[RuntimeLanguageProto]

class SanctionTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SANCTION_TYPE_UNSPECIFIED: _ClassVar[SanctionTypeProto]
    SANCTION_TYPE_BAN: _ClassVar[SanctionTypeProto]
    SANCTION_TYPE_MUTE: _ClassVar[SanctionTypeProto]

class ScalarTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SCALAR_TYPE_UNSPECIFIED: _ClassVar[ScalarTypeProto]
    SCALAR_TYPE_PRIMITIVE: _ClassVar[ScalarTypeProto]
    SCALAR_TYPE_ENUM: _ClassVar[ScalarTypeProto]
    SCALAR_TYPE_NODE_REFERENCE: _ClassVar[ScalarTypeProto]
    SCALAR_TYPE_NODE_VALUE: _ClassVar[ScalarTypeProto]
    SCALAR_TYPE_STRUCT: _ClassVar[ScalarTypeProto]

class ScheduleFrequencyProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SCHEDULE_FREQUENCY_UNSPECIFIED: _ClassVar[ScheduleFrequencyProto]
    SCHEDULE_FREQUENCY_YEAR: _ClassVar[ScheduleFrequencyProto]
    SCHEDULE_FREQUENCY_MONTH: _ClassVar[ScheduleFrequencyProto]
    SCHEDULE_FREQUENCY_WEEK: _ClassVar[ScheduleFrequencyProto]
    SCHEDULE_FREQUENCY_DAY: _ClassVar[ScheduleFrequencyProto]
    SCHEDULE_FREQUENCY_HOUR: _ClassVar[ScheduleFrequencyProto]
    SCHEDULE_FREQUENCY_MINUTE: _ClassVar[ScheduleFrequencyProto]

class ShadowPositionProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SHADOW_POSITION_UNSPECIFIED: _ClassVar[ShadowPositionProto]
    SHADOW_POSITION_OUTSIDE: _ClassVar[ShadowPositionProto]
    SHADOW_POSITION_INSIDE: _ClassVar[ShadowPositionProto]

class ShadowTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SHADOW_TYPE_UNSPECIFIED: _ClassVar[ShadowTypeProto]
    SHADOW_TYPE_BOX: _ClassVar[ShadowTypeProto]
    SHADOW_TYPE_REALISTIC: _ClassVar[ShadowTypeProto]

class SnapshotStatusProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SNAPSHOT_STATUS_UNSPECIFIED: _ClassVar[SnapshotStatusProto]
    SNAPSHOT_STATUS_CREATING: _ClassVar[SnapshotStatusProto]
    SNAPSHOT_STATUS_ACTIVE: _ClassVar[SnapshotStatusProto]
    SNAPSHOT_STATUS_READONLY: _ClassVar[SnapshotStatusProto]

class SnapshotTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SNAPSHOT_TYPE_UNSPECIFIED: _ClassVar[SnapshotTypeProto]
    SNAPSHOT_TYPE_PARTIAL: _ClassVar[SnapshotTypeProto]
    SNAPSHOT_TYPE_FULL: _ClassVar[SnapshotTypeProto]

class SortModeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SORT_MODE_UNSPECIFIED: _ClassVar[SortModeProto]
    SORT_MODE_MAX: _ClassVar[SortModeProto]
    SORT_MODE_MIN: _ClassVar[SortModeProto]
    SORT_MODE_AVERAGE: _ClassVar[SortModeProto]
    SORT_MODE_SUM: _ClassVar[SortModeProto]
    SORT_MODE_MEDIAN: _ClassVar[SortModeProto]

class SortTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SORT_TYPE_UNSPECIFIED: _ClassVar[SortTypeProto]
    SORT_TYPE_ASCENDING: _ClassVar[SortTypeProto]
    SORT_TYPE_DESCENDING: _ClassVar[SortTypeProto]

class SpaceStatusProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SPACE_STATUS_UNSPECIFIED: _ClassVar[SpaceStatusProto]
    SPACE_STATUS_CREATING: _ClassVar[SpaceStatusProto]
    SPACE_STATUS_QUEUED: _ClassVar[SpaceStatusProto]
    SPACE_STATUS_RUNNING: _ClassVar[SpaceStatusProto]
    SPACE_STATUS_PAUSED: _ClassVar[SpaceStatusProto]

class SpringTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SPRING_TYPE_UNSPECIFIED: _ClassVar[SpringTypeProto]
    SPRING_TYPE_TIME: _ClassVar[SpringTypeProto]
    SPRING_TYPE_PHYSICS: _ClassVar[SpringTypeProto]

class StoreImplementationProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    STORE_IMPLEMENTATION_UNSPECIFIED: _ClassVar[StoreImplementationProto]
    STORE_IMPLEMENTATION_MEMORY: _ClassVar[StoreImplementationProto]
    STORE_IMPLEMENTATION_POSTGRES: _ClassVar[StoreImplementationProto]

class StoreTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    STORE_TYPE_UNSPECIFIED: _ClassVar[StoreTypeProto]
    STORE_TYPE_LOCAL_ENTITY: _ClassVar[StoreTypeProto]
    STORE_TYPE_LOCAL_EVENT: _ClassVar[StoreTypeProto]
    STORE_TYPE_GLOBAL_ENTITY_PRIMARY: _ClassVar[StoreTypeProto]
    STORE_TYPE_SPATIAL_ENTITY_PRIMARY: _ClassVar[StoreTypeProto]
    STORE_TYPE_SPATIAL_EVENT_PRIMARY: _ClassVar[StoreTypeProto]

class StringFormatProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    STRING_FORMAT_UNSPECIFIED: _ClassVar[StringFormatProto]
    STRING_FORMAT_NAME: _ClassVar[StringFormatProto]
    STRING_FORMAT_SLUG: _ClassVar[StringFormatProto]
    STRING_FORMAT_EMAIL: _ClassVar[StringFormatProto]
    STRING_FORMAT_UUID: _ClassVar[StringFormatProto]
    STRING_FORMAT_URL: _ClassVar[StringFormatProto]
    STRING_FORMAT_EMOJI: _ClassVar[StringFormatProto]
    STRING_FORMAT_MIME: _ClassVar[StringFormatProto]
    STRING_FORMAT_BASE64: _ClassVar[StringFormatProto]

class StrokeTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    STROKE_TYPE_UNSPECIFIED: _ClassVar[StrokeTypeProto]
    STROKE_TYPE_SOLID: _ClassVar[StrokeTypeProto]
    STROKE_TYPE_DASHED: _ClassVar[StrokeTypeProto]
    STROKE_TYPE_DOTTED: _ClassVar[StrokeTypeProto]
    STROKE_TYPE_FREEHAND: _ClassVar[StrokeTypeProto]

class StructDefinitionTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    STRUCT_DEFINITION_TYPE_UNSPECIFIED: _ClassVar[StructDefinitionTypeProto]
    STRUCT_DEFINITION_TYPE_BUILTIN_STRUCT: _ClassVar[StructDefinitionTypeProto]
    STRUCT_DEFINITION_TYPE_CUSTOM_STRUCT: _ClassVar[StructDefinitionTypeProto]
    STRUCT_DEFINITION_TYPE_BUILTIN_ENUM: _ClassVar[StructDefinitionTypeProto]
    STRUCT_DEFINITION_TYPE_CUSTOM_ENUM: _ClassVar[StructDefinitionTypeProto]

class StructTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    STRUCT_TYPE_UNSPECIFIED: _ClassVar[StructTypeProto]
    STRUCT_TYPE_STRUCT: _ClassVar[StructTypeProto]
    STRUCT_TYPE_CUSTOM_STRUCT: _ClassVar[StructTypeProto]
    STRUCT_TYPE_BUILTIN_DEFINITION: _ClassVar[StructTypeProto]
    STRUCT_TYPE_NODE_DEFINITION: _ClassVar[StructTypeProto]
    STRUCT_TYPE_TRAIT_DEFINITION: _ClassVar[StructTypeProto]
    STRUCT_TYPE_STRUCT_DEFINITION: _ClassVar[StructTypeProto]
    STRUCT_TYPE_ENUM_DEFINITION: _ClassVar[StructTypeProto]
    STRUCT_TYPE_PROPERTY_DEFINITION: _ClassVar[StructTypeProto]
    STRUCT_TYPE_PROPERTY_GROUP_DEFINITION: _ClassVar[StructTypeProto]
    STRUCT_TYPE_OPTION_DEFINITION: _ClassVar[StructTypeProto]
    STRUCT_TYPE_OPTION_GROUP_DEFINITION: _ClassVar[StructTypeProto]
    STRUCT_TYPE_CONSTANT_DEFINITION: _ClassVar[StructTypeProto]
    STRUCT_TYPE_METHOD_DEFINITION: _ClassVar[StructTypeProto]
    STRUCT_TYPE_ACTION_DEFINITION: _ClassVar[StructTypeProto]
    STRUCT_TYPE_PERMISSION_DEFINITION: _ClassVar[StructTypeProto]
    STRUCT_TYPE_NODE_DEFINITION_REFERENCE: _ClassVar[StructTypeProto]
    STRUCT_TYPE_OBJECT_DEFINITION_REFERENCE: _ClassVar[StructTypeProto]
    STRUCT_TYPE_STRUCT_DEFINITION_REFERENCE: _ClassVar[StructTypeProto]
    STRUCT_TYPE_NODE_REFERENCE: _ClassVar[StructTypeProto]
    STRUCT_TYPE_PROPERTY_REFERENCE: _ClassVar[StructTypeProto]
    STRUCT_TYPE_EDIT: _ClassVar[StructTypeProto]
    STRUCT_TYPE_CHANGE: _ClassVar[StructTypeProto]
    STRUCT_TYPE_CHANGE_RESULT: _ClassVar[StructTypeProto]
    STRUCT_TYPE_ORIGIN: _ClassVar[StructTypeProto]
    STRUCT_TYPE_EXPRESSION: _ClassVar[StructTypeProto]
    STRUCT_TYPE_FUNCTION: _ClassVar[StructTypeProto]
    STRUCT_TYPE_JOIN: _ClassVar[StructTypeProto]
    STRUCT_TYPE_AGGREGATION: _ClassVar[StructTypeProto]
    STRUCT_TYPE_CONDITION: _ClassVar[StructTypeProto]
    STRUCT_TYPE_SORT: _ClassVar[StructTypeProto]
    STRUCT_TYPE_SELECT: _ClassVar[StructTypeProto]
    STRUCT_TYPE_QUERY: _ClassVar[StructTypeProto]
    STRUCT_TYPE_QUERY_RESULT: _ClassVar[StructTypeProto]
    STRUCT_TYPE_QUERY_RESULT_GROUP: _ClassVar[StructTypeProto]
    STRUCT_TYPE_QUERY_UPDATE: _ClassVar[StructTypeProto]
    STRUCT_TYPE_HISTOGRAM: _ClassVar[StructTypeProto]
    STRUCT_TYPE_SELECTION: _ClassVar[StructTypeProto]
    STRUCT_TYPE_VALUE: _ClassVar[StructTypeProto]
    STRUCT_TYPE_TYPE: _ClassVar[StructTypeProto]
    STRUCT_TYPE_NUMBER_CONSTRAINT: _ClassVar[StructTypeProto]
    STRUCT_TYPE_STRING_CONSTRAINT: _ClassVar[StructTypeProto]
    STRUCT_TYPE_COLLECTION_CONSTRAINT: _ClassVar[StructTypeProto]
    STRUCT_TYPE_NODE_CONSTRAINT: _ClassVar[StructTypeProto]
    STRUCT_TYPE_VECTOR: _ClassVar[StructTypeProto]
    STRUCT_TYPE_VECTORF: _ClassVar[StructTypeProto]
    STRUCT_TYPE_VECTOR2F: _ClassVar[StructTypeProto]
    STRUCT_TYPE_VECTOR3F: _ClassVar[StructTypeProto]
    STRUCT_TYPE_VECTOR4F: _ClassVar[StructTypeProto]
    STRUCT_TYPE_VECTORI: _ClassVar[StructTypeProto]
    STRUCT_TYPE_VECTOR2I: _ClassVar[StructTypeProto]
    STRUCT_TYPE_VECTOR3I: _ClassVar[StructTypeProto]
    STRUCT_TYPE_VECTOR4I: _ClassVar[StructTypeProto]
    STRUCT_TYPE_TEXT: _ClassVar[StructTypeProto]
    STRUCT_TYPE_TEXT_SPAN: _ClassVar[StructTypeProto]
    STRUCT_TYPE_ICON: _ClassVar[StructTypeProto]
    STRUCT_TYPE_SCHEDULE: _ClassVar[StructTypeProto]
    STRUCT_TYPE_DATABASE_INFO: _ClassVar[StructTypeProto]
    STRUCT_TYPE_GALAXY_INFO: _ClassVar[StructTypeProto]
    STRUCT_TYPE_LINE: _ClassVar[StructTypeProto]
    STRUCT_TYPE_ARROW: _ClassVar[StructTypeProto]
    STRUCT_TYPE_LENGTH: _ClassVar[StructTypeProto]
    STRUCT_TYPE_POSITION: _ClassVar[StructTypeProto]
    STRUCT_TYPE_DIMENSION: _ClassVar[StructTypeProto]
    STRUCT_TYPE_GRID: _ClassVar[StructTypeProto]
    STRUCT_TYPE_GRID_SPAN: _ClassVar[StructTypeProto]
    STRUCT_TYPE_INSETS: _ClassVar[StructTypeProto]
    STRUCT_TYPE_CORNERS: _ClassVar[StructTypeProto]
    STRUCT_TYPE_AXIS2: _ClassVar[StructTypeProto]
    STRUCT_TYPE_AXIS3: _ClassVar[StructTypeProto]
    STRUCT_TYPE_COLOR: _ClassVar[StructTypeProto]
    STRUCT_TYPE_FILL: _ClassVar[StructTypeProto]
    STRUCT_TYPE_FONT: _ClassVar[StructTypeProto]
    STRUCT_TYPE_BORDER: _ClassVar[StructTypeProto]
    STRUCT_TYPE_SHADOW: _ClassVar[StructTypeProto]
    STRUCT_TYPE_GRADIENT: _ClassVar[StructTypeProto]
    STRUCT_TYPE_GRADIENT_STOP: _ClassVar[StructTypeProto]
    STRUCT_TYPE_TRANSITION: _ClassVar[StructTypeProto]
    STRUCT_TYPE_EFFECT: _ClassVar[StructTypeProto]
    STRUCT_TYPE_STROKE: _ClassVar[StructTypeProto]
    STRUCT_TYPE_STROKE_CAP: _ClassVar[StructTypeProto]
    STRUCT_TYPE_STROKE_PATH: _ClassVar[StructTypeProto]
    STRUCT_TYPE_STROKE_POINT: _ClassVar[StructTypeProto]

class TenancyProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TENANCY_UNSPECIFIED: _ClassVar[TenancyProto]
    TENANCY_DEDICATED: _ClassVar[TenancyProto]
    TENANCY_SHARED: _ClassVar[TenancyProto]

class TextAlignProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TEXT_ALIGN_UNSPECIFIED: _ClassVar[TextAlignProto]
    TEXT_ALIGN_LEFT: _ClassVar[TextAlignProto]
    TEXT_ALIGN_CENTER: _ClassVar[TextAlignProto]
    TEXT_ALIGN_RIGHT: _ClassVar[TextAlignProto]
    TEXT_ALIGN_JUSTIFY: _ClassVar[TextAlignProto]

class TextDecorationProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TEXT_DECORATION_UNSPECIFIED: _ClassVar[TextDecorationProto]
    TEXT_DECORATION_NONE: _ClassVar[TextDecorationProto]
    TEXT_DECORATION_UNDERLINE: _ClassVar[TextDecorationProto]
    TEXT_DECORATION_STRIKETHROUGH: _ClassVar[TextDecorationProto]

class TextSpanTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TEXT_SPAN_TYPE_UNSPECIFIED: _ClassVar[TextSpanTypeProto]
    TEXT_SPAN_TYPE_TEXT: _ClassVar[TextSpanTypeProto]
    TEXT_SPAN_TYPE_HARD_BREAK: _ClassVar[TextSpanTypeProto]
    TEXT_SPAN_TYPE_MENTION: _ClassVar[TextSpanTypeProto]
    TEXT_SPAN_TYPE_LINK: _ClassVar[TextSpanTypeProto]
    TEXT_SPAN_TYPE_CITATION: _ClassVar[TextSpanTypeProto]
    TEXT_SPAN_TYPE_EQUATION: _ClassVar[TextSpanTypeProto]

class TextSplitTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TEXT_SPLIT_TYPE_UNSPECIFIED: _ClassVar[TextSplitTypeProto]
    TEXT_SPLIT_TYPE_CHAR: _ClassVar[TextSplitTypeProto]
    TEXT_SPLIT_TYPE_WORD: _ClassVar[TextSplitTypeProto]
    TEXT_SPLIT_TYPE_LINE: _ClassVar[TextSplitTypeProto]

class TextTransformProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TEXT_TRANSFORM_UNSPECIFIED: _ClassVar[TextTransformProto]
    TEXT_TRANSFORM_NONE: _ClassVar[TextTransformProto]
    TEXT_TRANSFORM_UPPERCASE: _ClassVar[TextTransformProto]
    TEXT_TRANSFORM_LOWERCASE: _ClassVar[TextTransformProto]
    TEXT_TRANSFORM_CAPITALIZE: _ClassVar[TextTransformProto]

class ThreadStatusProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    THREAD_STATUS_UNSPECIFIED: _ClassVar[ThreadStatusProto]
    THREAD_STATUS_OPEN: _ClassVar[ThreadStatusProto]
    THREAD_STATUS_CLOSED: _ClassVar[ThreadStatusProto]

class TimerTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TIMER_TYPE_UNSPECIFIED: _ClassVar[TimerTypeProto]
    TIMER_TYPE_ONCE: _ClassVar[TimerTypeProto]
    TIMER_TYPE_RECURRING: _ClassVar[TimerTypeProto]

class ToolTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TOOL_TYPE_UNSPECIFIED: _ClassVar[ToolTypeProto]
    TOOL_TYPE_SELECT: _ClassVar[ToolTypeProto]
    TOOL_TYPE_DRAG: _ClassVar[ToolTypeProto]
    TOOL_TYPE_INSPECT: _ClassVar[ToolTypeProto]
    TOOL_TYPE_ANNOTATE: _ClassVar[ToolTypeProto]

class TraitTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TRAIT_TYPE_UNSPECIFIED: _ClassVar[TraitTypeProto]
    TRAIT_TYPE_GLOBAL: _ClassVar[TraitTypeProto]
    TRAIT_TYPE_SPATIAL: _ClassVar[TraitTypeProto]
    TRAIT_TYPE_ORDERED: _ClassVar[TraitTypeProto]
    TRAIT_TYPE_ARCHIVABLE: _ClassVar[TraitTypeProto]
    TRAIT_TYPE_DELETABLE: _ClassVar[TraitTypeProto]
    TRAIT_TYPE_CUSTOMIZABLE: _ClassVar[TraitTypeProto]
    TRAIT_TYPE_EXTENSIBLE: _ClassVar[TraitTypeProto]
    TRAIT_TYPE_IRREVERSIBLE: _ClassVar[TraitTypeProto]
    TRAIT_TYPE_TAGGABLE: _ClassVar[TraitTypeProto]
    TRAIT_TYPE_OWNABLE: _ClassVar[TraitTypeProto]
    TRAIT_TYPE_OWNER: _ClassVar[TraitTypeProto]
    TRAIT_TYPE_JOINABLE: _ClassVar[TraitTypeProto]
    TRAIT_TYPE_SUBJECT: _ClassVar[TraitTypeProto]
    TRAIT_TYPE_RUNNABLE: _ClassVar[TraitTypeProto]
    TRAIT_TYPE_SCRIPTABLE: _ClassVar[TraitTypeProto]
    TRAIT_TYPE_SOURCEABLE: _ClassVar[TraitTypeProto]
    TRAIT_TYPE_STARABLE: _ClassVar[TraitTypeProto]
    TRAIT_TYPE_REACTABLE: _ClassVar[TraitTypeProto]
    TRAIT_TYPE_FOLLOWABLE: _ClassVar[TraitTypeProto]

class TransitionTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TRANSITION_TYPE_UNSPECIFIED: _ClassVar[TransitionTypeProto]
    TRANSITION_TYPE_TWEEN: _ClassVar[TransitionTypeProto]
    TRANSITION_TYPE_SPRING: _ClassVar[TransitionTypeProto]

class TriggerTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TRIGGER_TYPE_UNSPECIFIED: _ClassVar[TriggerTypeProto]
    TRIGGER_TYPE_EVENT: _ClassVar[TriggerTypeProto]

class TypeCardinalityProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TYPE_CARDINALITY_UNSPECIFIED: _ClassVar[TypeCardinalityProto]
    TYPE_CARDINALITY_SCALAR: _ClassVar[TypeCardinalityProto]
    TYPE_CARDINALITY_LIST: _ClassVar[TypeCardinalityProto]
    TYPE_CARDINALITY_MAP: _ClassVar[TypeCardinalityProto]

class UniverseCategoryProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    UNIVERSE_CATEGORY_UNSPECIFIED: _ClassVar[UniverseCategoryProto]
    UNIVERSE_CATEGORY_META: _ClassVar[UniverseCategoryProto]
    UNIVERSE_CATEGORY_UNIVERSE: _ClassVar[UniverseCategoryProto]
    UNIVERSE_CATEGORY_SPACE: _ClassVar[UniverseCategoryProto]
    UNIVERSE_CATEGORY_ACCESS: _ClassVar[UniverseCategoryProto]
    UNIVERSE_CATEGORY_DATA: _ClassVar[UniverseCategoryProto]
    UNIVERSE_CATEGORY_LOGIC: _ClassVar[UniverseCategoryProto]
    UNIVERSE_CATEGORY_INTELLIGENCE: _ClassVar[UniverseCategoryProto]
    UNIVERSE_CATEGORY_INFRASTRUCTURE: _ClassVar[UniverseCategoryProto]
    UNIVERSE_CATEGORY_DEPLOYMENT: _ClassVar[UniverseCategoryProto]
    UNIVERSE_CATEGORY_OBSERVABILITY: _ClassVar[UniverseCategoryProto]
    UNIVERSE_CATEGORY_OPTIMIZATION: _ClassVar[UniverseCategoryProto]
    UNIVERSE_CATEGORY_SOCIAL: _ClassVar[UniverseCategoryProto]
    UNIVERSE_CATEGORY_FINANCE: _ClassVar[UniverseCategoryProto]
    UNIVERSE_CATEGORY_SCENE: _ClassVar[UniverseCategoryProto]
    UNIVERSE_CATEGORY_VIEW: _ClassVar[UniverseCategoryProto]
    UNIVERSE_CATEGORY_CANVAS: _ClassVar[UniverseCategoryProto]
    UNIVERSE_CATEGORY_INTERACTION: _ClassVar[UniverseCategoryProto]
    UNIVERSE_CATEGORY_ANIMATION: _ClassVar[UniverseCategoryProto]
    UNIVERSE_CATEGORY_STYLE: _ClassVar[UniverseCategoryProto]

class UserStatusProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    USER_STATUS_UNSPECIFIED: _ClassVar[UserStatusProto]
    USER_STATUS_CREATING: _ClassVar[UserStatusProto]
    USER_STATUS_ACTIVE: _ClassVar[UserStatusProto]

class ValueFactoryProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    VALUE_FACTORY_UNSPECIFIED: _ClassVar[ValueFactoryProto]
    VALUE_FACTORY_UUID: _ClassVar[ValueFactoryProto]
    VALUE_FACTORY_NOW: _ClassVar[ValueFactoryProto]
    VALUE_FACTORY_REGION: _ClassVar[ValueFactoryProto]
    VALUE_FACTORY_SELF: _ClassVar[ValueFactoryProto]

class VariantStateTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    VARIANT_STATE_TYPE_UNSPECIFIED: _ClassVar[VariantStateTypeProto]
    VARIANT_STATE_TYPE_LOADING: _ClassVar[VariantStateTypeProto]
    VARIANT_STATE_TYPE_ERROR: _ClassVar[VariantStateTypeProto]

class VariantTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    VARIANT_TYPE_UNSPECIFIED: _ClassVar[VariantTypeProto]
    VARIANT_TYPE_DYNAMIC: _ClassVar[VariantTypeProto]
    VARIANT_TYPE_BREAKPOINT: _ClassVar[VariantTypeProto]
    VARIANT_TYPE_PLATFORM: _ClassVar[VariantTypeProto]

class WindowTypeProto(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    WINDOW_TYPE_UNSPECIFIED: _ClassVar[WindowTypeProto]
    WINDOW_TYPE_BROWSER: _ClassVar[WindowTypeProto]
    WINDOW_TYPE_DESKTOP: _ClassVar[WindowTypeProto]
    WINDOW_TYPE_MOBILE: _ClassVar[WindowTypeProto]
AGGREGATION_TYPE_UNSPECIFIED: AggregationTypeProto
AGGREGATION_TYPE_EXISTS: AggregationTypeProto
AGGREGATION_TYPE_COUNT: AggregationTypeProto
AGGREGATION_TYPE_SUM: AggregationTypeProto
AGGREGATION_TYPE_MIN: AggregationTypeProto
AGGREGATION_TYPE_MAX: AggregationTypeProto
AGGREGATION_TYPE_AVERAGE: AggregationTypeProto
ALIGN_UNSPECIFIED: AlignProto
ALIGN_START: AlignProto
ALIGN_CENTER: AlignProto
ALIGN_END: AlignProto
ARROW_HEAD_TYPE_UNSPECIFIED: ArrowHeadTypeProto
ARROW_HEAD_TYPE_ARROW: ArrowHeadTypeProto
ARROW_HEAD_TYPE_TRIANGLE: ArrowHeadTypeProto
ARROW_HEAD_TYPE_DOT: ArrowHeadTypeProto
BORDER_TYPE_UNSPECIFIED: BorderTypeProto
BORDER_TYPE_STYLE: BorderTypeProto
BORDER_TYPE_SOLID: BorderTypeProto
BORDER_TYPE_DASHED: BorderTypeProto
BORDER_TYPE_DOTTED: BorderTypeProto
BORDER_TYPE_DOUBLE: BorderTypeProto
CANVAS_TYPE_UNSPECIFIED: CanvasTypeProto
CANVAS_TYPE_SHAPE: CanvasTypeProto
CASCADE_ACTION_UNSPECIFIED: CascadeActionProto
CASCADE_ACTION_RESTRICT: CascadeActionProto
CASCADE_ACTION_CASCADE: CascadeActionProto
CASCADE_ACTION_SET_NULL: CascadeActionProto
CHANGE_DEBOUNCE_UNSPECIFIED: ChangeDebounceProto
CHANGE_DEBOUNCE_LAZY: ChangeDebounceProto
CHANGE_STATUS_UNSPECIFIED: ChangeStatusProto
CHANGE_STATUS_COMPLETED: ChangeStatusProto
CHANGE_STATUS_SKIPPED: ChangeStatusProto
CHANGE_STATUS_FAILED: ChangeStatusProto
CHANGE_STATUS_REJECTED: ChangeStatusProto
CLIENT_TYPE_UNSPECIFIED: ClientTypeProto
CLIENT_TYPE_WEB: ClientTypeProto
CLIENT_TYPE_BROWSER_PLUGIN: ClientTypeProto
CLIENT_TYPE_DESKTOP: ClientTypeProto
CLIENT_TYPE_MOBILE: ClientTypeProto
CLIENT_TYPE_MACHINE: ClientTypeProto
CLOUD_UNSPECIFIED: CloudProto
CLOUD_PRIVATE: CloudProto
CLOUD_AWS: CloudProto
CLOUD_AZURE: CloudProto
CLOUD_GCP: CloudProto
CLOUD_HETZNER: CloudProto
COLOR_HUE_UNSPECIFIED: ColorHueProto
COLOR_HUE_GRAY: ColorHueProto
COLOR_HUE_RED: ColorHueProto
COLOR_HUE_ORANGE: ColorHueProto
COLOR_HUE_AMBER: ColorHueProto
COLOR_HUE_YELLOW: ColorHueProto
COLOR_HUE_LIME: ColorHueProto
COLOR_HUE_GREEN: ColorHueProto
COLOR_HUE_EMERALD: ColorHueProto
COLOR_HUE_TEAL: ColorHueProto
COLOR_HUE_CYAN: ColorHueProto
COLOR_HUE_SKY: ColorHueProto
COLOR_HUE_BLUE: ColorHueProto
COLOR_HUE_INDIGO: ColorHueProto
COLOR_HUE_VIOLET: ColorHueProto
COLOR_HUE_PURPLE: ColorHueProto
COLOR_HUE_FUCHSIA: ColorHueProto
COLOR_HUE_PINK: ColorHueProto
COLOR_HUE_ROSE: ColorHueProto
COLOR_INTENT_UNSPECIFIED: ColorIntentProto
COLOR_INTENT_PRIMARY: ColorIntentProto
COLOR_INTENT_SECONDARY: ColorIntentProto
COLOR_INTENT_NEUTRAL: ColorIntentProto
COLOR_INTENT_SUCCESS: ColorIntentProto
COLOR_INTENT_INFO: ColorIntentProto
COLOR_INTENT_WARNING: ColorIntentProto
COLOR_INTENT_ERROR: ColorIntentProto
COLOR_SHADE_UNSPECIFIED: ColorShadeProto
COLOR_SHADE_S25: ColorShadeProto
COLOR_SHADE_S50: ColorShadeProto
COLOR_SHADE_S100: ColorShadeProto
COLOR_SHADE_S200: ColorShadeProto
COLOR_SHADE_S300: ColorShadeProto
COLOR_SHADE_S400: ColorShadeProto
COLOR_SHADE_S500: ColorShadeProto
COLOR_SHADE_S600: ColorShadeProto
COLOR_SHADE_S700: ColorShadeProto
COLOR_SHADE_S800: ColorShadeProto
COLOR_SHADE_S900: ColorShadeProto
COLOR_SHADE_S950: ColorShadeProto
COLOR_TYPE_UNSPECIFIED: ColorTypeProto
COLOR_TYPE_BUILTIN: ColorTypeProto
COLOR_TYPE_RGB: ColorTypeProto
COLOR_TYPE_HSL: ColorTypeProto
COLOR_TYPE_P3: ColorTypeProto
CONDITIONAL_TYPE_UNSPECIFIED: ConditionalTypeProto
CONDITIONAL_TYPE_NOT: ConditionalTypeProto
CONDITIONAL_TYPE_AND: ConditionalTypeProto
CONDITIONAL_TYPE_OR: ConditionalTypeProto
CONDITIONAL_TYPE_EQUALS: ConditionalTypeProto
CONDITIONAL_TYPE_NOT_EQUALS: ConditionalTypeProto
CONDITIONAL_TYPE_GREATER_THAN: ConditionalTypeProto
CONDITIONAL_TYPE_GREATER_THAN_OR_EQUALS: ConditionalTypeProto
CONDITIONAL_TYPE_LESS_THAN: ConditionalTypeProto
CONDITIONAL_TYPE_LESS_THAN_OR_EQUALS: ConditionalTypeProto
CONDITIONAL_TYPE_MATCHES: ConditionalTypeProto
CONDITIONAL_TYPE_STARTS_WITH: ConditionalTypeProto
CONDITIONAL_TYPE_ENDS_WITH: ConditionalTypeProto
CONDITIONAL_TYPE_IN: ConditionalTypeProto
CONDITIONAL_TYPE_NOT_IN: ConditionalTypeProto
CONDITIONAL_TYPE_EXISTS: ConditionalTypeProto
CONDITIONAL_TYPE_NOT_EXISTS: ConditionalTypeProto
CURSOR_STATUS_UNSPECIFIED: CursorStatusProto
CURSOR_STATUS_CREATED: CursorStatusProto
CURSOR_STATUS_WORKING: CursorStatusProto
CURSOR_STATUS_READING: CursorStatusProto
CURSOR_STATUS_WRITING: CursorStatusProto
CURSOR_STATUS_THINKING: CursorStatusProto
CURSOR_STATUS_WAITING: CursorStatusProto
CURSOR_STATUS_IDLE: CursorStatusProto
CURSOR_STATUS_CANCELLED: CursorStatusProto
CURSOR_STATUS_COMPLETED: CursorStatusProto
DATABASE_TYPE_UNSPECIFIED: DatabaseTypeProto
DATABASE_TYPE_POSTGRES: DatabaseTypeProto
DAY_OF_WEEK_UNSPECIFIED: DayOfWeekProto
DAY_OF_WEEK_MONDAY: DayOfWeekProto
DAY_OF_WEEK_TUESDAY: DayOfWeekProto
DAY_OF_WEEK_WEDNESDAY: DayOfWeekProto
DAY_OF_WEEK_THURSDAY: DayOfWeekProto
DAY_OF_WEEK_FRIDAY: DayOfWeekProto
DAY_OF_WEEK_SATURDAY: DayOfWeekProto
DAY_OF_WEEK_SUNDAY: DayOfWeekProto
DIMENSION_TYPE_UNSPECIFIED: DimensionTypeProto
DIMENSION_TYPE_FIXED: DimensionTypeProto
DIMENSION_TYPE_FIT: DimensionTypeProto
DIMENSION_TYPE_FILL: DimensionTypeProto
DIRECTION_UNSPECIFIED: DirectionProto
DIRECTION_HORIZONTAL: DirectionProto
DIRECTION_VERTICAL: DirectionProto
DISTRIBUTE_UNSPECIFIED: DistributeProto
DISTRIBUTE_START: DistributeProto
DISTRIBUTE_CENTER: DistributeProto
DISTRIBUTE_END: DistributeProto
DISTRIBUTE_SPACE_BETWEEN: DistributeProto
DISTRIBUTE_SPACE_AROUND: DistributeProto
DISTRIBUTE_SPACE_EVENLY: DistributeProto
EASING_UNSPECIFIED: EasingProto
EASING_LINEAR: EasingProto
EASING_EASE_IN_QUAD: EasingProto
EASING_EASE_OUT_QUAD: EasingProto
EASING_EASE_IN_OUT_QUAD: EasingProto
EASING_EASE_IN_CUBIC: EasingProto
EASING_EASE_OUT_CUBIC: EasingProto
EASING_EASE_IN_OUT_CUBIC: EasingProto
EASING_EASE_IN_QUART: EasingProto
EASING_EASE_OUT_QUART: EasingProto
EASING_EASE_IN_OUT_QUART: EasingProto
EASING_EASE_IN_QUINT: EasingProto
EASING_EASE_OUT_QUINT: EasingProto
EASING_EASE_IN_OUT_QUINT: EasingProto
EASING_EASE_IN_SINE: EasingProto
EASING_EASE_OUT_SINE: EasingProto
EASING_EASE_IN_OUT_SINE: EasingProto
EASING_EASE_IN_EXPO: EasingProto
EASING_EASE_OUT_EXPO: EasingProto
EASING_EASE_IN_OUT_EXPO: EasingProto
EASING_EASE_PEN: EasingProto
EDGE_DIRECTION_UNSPECIFIED: EdgeDirectionProto
EDGE_DIRECTION_PARENT: EdgeDirectionProto
EDGE_DIRECTION_CHILD: EdgeDirectionProto
EDGE_DIRECTION_SIDE: EdgeDirectionProto
EDGE_TYPE_UNSPECIFIED: EdgeTypeProto
EDGE_TYPE_PARENT: EdgeTypeProto
EDGE_TYPE_REGULAR: EdgeTypeProto
EDIT_OPERATION_UNSPECIFIED: EditOperationProto
EDIT_OPERATION_SET: EditOperationProto
EDIT_OPERATION_CLEAR: EditOperationProto
EDIT_TYPE_UNSPECIFIED: EditTypeProto
EDIT_TYPE_CREATE: EditTypeProto
EDIT_TYPE_UPSERT: EditTypeProto
EDIT_TYPE_UPDATE: EditTypeProto
EDIT_TYPE_MOVE: EditTypeProto
EDIT_TYPE_ARCHIVE: EditTypeProto
EDIT_TYPE_UNARCHIVE: EditTypeProto
EDIT_TYPE_DELETE: EditTypeProto
EDIT_TYPE_RESTORE: EditTypeProto
EDIT_TYPE_ERASE: EditTypeProto
EFFECT_TYPE_UNSPECIFIED: EffectTypeProto
EFFECT_TYPE_APPEAR: EffectTypeProto
EFFECT_TYPE_ENTER: EffectTypeProto
EFFECT_TYPE_EXIT: EffectTypeProto
EFFECT_TYPE_HOVER: EffectTypeProto
EFFECT_TYPE_PRESS: EffectTypeProto
EFFECT_TYPE_DRAG: EffectTypeProto
EFFECT_TYPE_FOCUS: EffectTypeProto
EFFECT_TYPE_LOOP: EffectTypeProto
ENTITLEMENT_TYPE_UNSPECIFIED: EntitlementTypeProto
ENTITLEMENT_TYPE_PERMISSION: EntitlementTypeProto
ENTITLEMENT_TYPE_ROLE: EntitlementTypeProto
ENUM_TYPE_UNSPECIFIED: EnumTypeProto
ENUM_TYPE_ENUM_TYPE: EnumTypeProto
ENUM_TYPE_NODE_TYPE: EnumTypeProto
ENUM_TYPE_STRUCT_TYPE: EnumTypeProto
ENUM_TYPE_TRAIT_TYPE: EnumTypeProto
ENUM_TYPE_UNIVERSE_CATEGORY: EnumTypeProto
ENUM_TYPE_NODE_DEFINITION_TYPE: EnumTypeProto
ENUM_TYPE_OBJECT_DEFINITION_TYPE: EnumTypeProto
ENUM_TYPE_STRUCT_DEFINITION_TYPE: EnumTypeProto
ENUM_TYPE_PROPERTY_REFERENCE_TYPE: EnumTypeProto
ENUM_TYPE_MATERIALIZATION: EnumTypeProto
ENUM_TYPE_STORE_TYPE: EnumTypeProto
ENUM_TYPE_STORE_IMPLEMENTATION: EnumTypeProto
ENUM_TYPE_PLATFORM_TYPE: EnumTypeProto
ENUM_TYPE_RUNTIME_LANGUAGE: EnumTypeProto
ENUM_TYPE_OPERATING_SYSTEM: EnumTypeProto
ENUM_TYPE_EDIT_TYPE: EnumTypeProto
ENUM_TYPE_EDIT_OPERATION: EnumTypeProto
ENUM_TYPE_CHANGE_STATUS: EnumTypeProto
ENUM_TYPE_CHANGE_DEBOUNCE: EnumTypeProto
ENUM_TYPE_PRIMITIVE_TYPE: EnumTypeProto
ENUM_TYPE_TYPE_CARDINALITY: EnumTypeProto
ENUM_TYPE_SCALAR_TYPE: EnumTypeProto
ENUM_TYPE_VALUE_FACTORY: EnumTypeProto
ENUM_TYPE_STRING_FORMAT: EnumTypeProto
ENUM_TYPE_NUMBER_FORMAT: EnumTypeProto
ENUM_TYPE_PROPERTY_TYPE: EnumTypeProto
ENUM_TYPE_EDGE_TYPE: EnumTypeProto
ENUM_TYPE_EDGE_DIRECTION: EnumTypeProto
ENUM_TYPE_CASCADE_ACTION: EnumTypeProto
ENUM_TYPE_RESOURCE_STATUS: EnumTypeProto
ENUM_TYPE_SNAPSHOT_TYPE: EnumTypeProto
ENUM_TYPE_SNAPSHOT_STATUS: EnumTypeProto
ENUM_TYPE_CONDITIONAL_TYPE: EnumTypeProto
ENUM_TYPE_AGGREGATION_TYPE: EnumTypeProto
ENUM_TYPE_SORT_MODE: EnumTypeProto
ENUM_TYPE_SORT_TYPE: EnumTypeProto
ENUM_TYPE_JOIN_TYPE: EnumTypeProto
ENUM_TYPE_FUNCTION_TYPE: EnumTypeProto
ENUM_TYPE_EXPRESSION_TYPE: EnumTypeProto
ENUM_TYPE_QUERY_TYPE: EnumTypeProto
ENUM_TYPE_QUERY_UPDATE_TYPE: EnumTypeProto
ENUM_TYPE_SPACE_STATUS: EnumTypeProto
ENUM_TYPE_USER_STATUS: EnumTypeProto
ENUM_TYPE_ORGANIZATION_STATUS: EnumTypeProto
ENUM_TYPE_CLIENT_TYPE: EnumTypeProto
ENUM_TYPE_FOLDER_TYPE: EnumTypeProto
ENUM_TYPE_ROLE_TYPE: EnumTypeProto
ENUM_TYPE_PERMISSION_TYPE: EnumTypeProto
ENUM_TYPE_SANCTION_TYPE: EnumTypeProto
ENUM_TYPE_ENTITLEMENT_TYPE: EnumTypeProto
ENUM_TYPE_FILE_RETENTION_MODE: EnumTypeProto
ENUM_TYPE_FILE_SOURCE: EnumTypeProto
ENUM_TYPE_FILE_TYPE: EnumTypeProto
ENUM_TYPE_FILE_FORMAT: EnumTypeProto
ENUM_TYPE_TEXT_SPAN_TYPE: EnumTypeProto
ENUM_TYPE_ICON_TYPE: EnumTypeProto
ENUM_TYPE_METHOD_CARDINALITY: EnumTypeProto
ENUM_TYPE_TRIGGER_TYPE: EnumTypeProto
ENUM_TYPE_TIMER_TYPE: EnumTypeProto
ENUM_TYPE_DAY_OF_WEEK: EnumTypeProto
ENUM_TYPE_MONTH: EnumTypeProto
ENUM_TYPE_SCHEDULE_FREQUENCY: EnumTypeProto
ENUM_TYPE_CURSOR_STATUS: EnumTypeProto
ENUM_TYPE_MODEL_DEVELOPER: EnumTypeProto
ENUM_TYPE_MODEL_PROVIDER: EnumTypeProto
ENUM_TYPE_CLOUD: EnumTypeProto
ENUM_TYPE_REGION: EnumTypeProto
ENUM_TYPE_REGION_AREA: EnumTypeProto
ENUM_TYPE_REGION_CONTINENT: EnumTypeProto
ENUM_TYPE_TENANCY: EnumTypeProto
ENUM_TYPE_DATABASE_TYPE: EnumTypeProto
ENUM_TYPE_MACHINE_TYPE: EnumTypeProto
ENUM_TYPE_ENVIRONMENT_TYPE: EnumTypeProto
ENUM_TYPE_RUN_STATUS: EnumTypeProto
ENUM_TYPE_LOG_LEVEL: EnumTypeProto
ENUM_TYPE_THREAD_STATUS: EnumTypeProto
ENUM_TYPE_NOTIFICATION_STATUS: EnumTypeProto
ENUM_TYPE_WINDOW_TYPE: EnumTypeProto
ENUM_TYPE_LAYER_TYPE: EnumTypeProto
ENUM_TYPE_VARIANT_TYPE: EnumTypeProto
ENUM_TYPE_VARIANT_STATE_TYPE: EnumTypeProto
ENUM_TYPE_CANVAS_TYPE: EnumTypeProto
ENUM_TYPE_ARROW_HEAD_TYPE: EnumTypeProto
ENUM_TYPE_MODE_TYPE: EnumTypeProto
ENUM_TYPE_TOOL_TYPE: EnumTypeProto
ENUM_TYPE_MOUSE_BUTTON: EnumTypeProto
ENUM_TYPE_COLOR_TYPE: EnumTypeProto
ENUM_TYPE_COLOR_SHADE: EnumTypeProto
ENUM_TYPE_COLOR_HUE: EnumTypeProto
ENUM_TYPE_COLOR_INTENT: EnumTypeProto
ENUM_TYPE_FILL_TYPE: EnumTypeProto
ENUM_TYPE_FILL_POSITION: EnumTypeProto
ENUM_TYPE_FILL_SIZE: EnumTypeProto
ENUM_TYPE_FONT_TYPE: EnumTypeProto
ENUM_TYPE_FONT_WEIGHT: EnumTypeProto
ENUM_TYPE_FONT_SIZE: EnumTypeProto
ENUM_TYPE_TEXT_ALIGN: EnumTypeProto
ENUM_TYPE_TEXT_DECORATION: EnumTypeProto
ENUM_TYPE_TEXT_TRANSFORM: EnumTypeProto
ENUM_TYPE_BORDER_TYPE: EnumTypeProto
ENUM_TYPE_SHADOW_TYPE: EnumTypeProto
ENUM_TYPE_SHADOW_POSITION: EnumTypeProto
ENUM_TYPE_GRADIENT_TYPE: EnumTypeProto
ENUM_TYPE_TRANSITION_TYPE: EnumTypeProto
ENUM_TYPE_SPRING_TYPE: EnumTypeProto
ENUM_TYPE_EFFECT_TYPE: EnumTypeProto
ENUM_TYPE_STROKE_TYPE: EnumTypeProto
ENUM_TYPE_POSITION_TYPE: EnumTypeProto
ENUM_TYPE_LENGTH_UNIT: EnumTypeProto
ENUM_TYPE_LAYOUT: EnumTypeProto
ENUM_TYPE_DISTRIBUTE: EnumTypeProto
ENUM_TYPE_ALIGN: EnumTypeProto
ENUM_TYPE_DIRECTION: EnumTypeProto
ENUM_TYPE_OVERFLOW: EnumTypeProto
ENUM_TYPE_DIMENSION_TYPE: EnumTypeProto
ENUM_TYPE_REPEAT_TYPE: EnumTypeProto
ENUM_TYPE_TEXT_SPLIT_TYPE: EnumTypeProto
ENUM_TYPE_OFFSCREEN_BEHAVIOR: EnumTypeProto
ENUM_TYPE_EASING: EnumTypeProto
ENVIRONMENT_TYPE_UNSPECIFIED: EnvironmentTypeProto
ENVIRONMENT_TYPE_SYSTEM: EnvironmentTypeProto
ENVIRONMENT_TYPE_DEVELOPMENT: EnvironmentTypeProto
ENVIRONMENT_TYPE_TEST: EnvironmentTypeProto
ENVIRONMENT_TYPE_STAGING: EnvironmentTypeProto
ENVIRONMENT_TYPE_PRODUCTION: EnvironmentTypeProto
EXPRESSION_TYPE_UNSPECIFIED: ExpressionTypeProto
EXPRESSION_TYPE_LITERAL: ExpressionTypeProto
EXPRESSION_TYPE_ATTRIBUTE: ExpressionTypeProto
EXPRESSION_TYPE_CONDITION: ExpressionTypeProto
EXPRESSION_TYPE_FUNCTION: ExpressionTypeProto
EXPRESSION_TYPE_AGGREGATION: ExpressionTypeProto
FILE_FORMAT_UNSPECIFIED: FileFormatProto
FILE_FORMAT_TXT: FileFormatProto
FILE_FORMAT_MARKDOWN: FileFormatProto
FILE_FORMAT_RTF: FileFormatProto
FILE_FORMAT_INI: FileFormatProto
FILE_FORMAT_LOG: FileFormatProto
FILE_FORMAT_PYTHON: FileFormatProto
FILE_FORMAT_JAVASCRIPT: FileFormatProto
FILE_FORMAT_TYPESCRIPT: FileFormatProto
FILE_FORMAT_GO: FileFormatProto
FILE_FORMAT_C_LANG: FileFormatProto
FILE_FORMAT_CPP: FileFormatProto
FILE_FORMAT_OBJECTIVE_C: FileFormatProto
FILE_FORMAT_SWIFT: FileFormatProto
FILE_FORMAT_RUBY: FileFormatProto
FILE_FORMAT_PHP: FileFormatProto
FILE_FORMAT_CSS: FileFormatProto
FILE_FORMAT_JAVA: FileFormatProto
FILE_FORMAT_KOTLIN: FileFormatProto
FILE_FORMAT_RUST: FileFormatProto
FILE_FORMAT_SCALA: FileFormatProto
FILE_FORMAT_SHELL: FileFormatProto
FILE_FORMAT_SQL: FileFormatProto
FILE_FORMAT_POWERSHELL: FileFormatProto
FILE_FORMAT_ASSEMBLY: FileFormatProto
FILE_FORMAT_LATEX: FileFormatProto
FILE_FORMAT_JPEG: FileFormatProto
FILE_FORMAT_PNG: FileFormatProto
FILE_FORMAT_GIF: FileFormatProto
FILE_FORMAT_BMP: FileFormatProto
FILE_FORMAT_TIFF: FileFormatProto
FILE_FORMAT_WEBP: FileFormatProto
FILE_FORMAT_SVG: FileFormatProto
FILE_FORMAT_ICO: FileFormatProto
FILE_FORMAT_RAW: FileFormatProto
FILE_FORMAT_HEIC: FileFormatProto
FILE_FORMAT_HEIF: FileFormatProto
FILE_FORMAT_MP3: FileFormatProto
FILE_FORMAT_WAV: FileFormatProto
FILE_FORMAT_FLAC: FileFormatProto
FILE_FORMAT_AAC: FileFormatProto
FILE_FORMAT_OGG: FileFormatProto
FILE_FORMAT_M4A: FileFormatProto
FILE_FORMAT_WMA: FileFormatProto
FILE_FORMAT_MP4: FileFormatProto
FILE_FORMAT_WEBM: FileFormatProto
FILE_FORMAT_AVI: FileFormatProto
FILE_FORMAT_MOV: FileFormatProto
FILE_FORMAT_WMV: FileFormatProto
FILE_FORMAT_FLV: FileFormatProto
FILE_FORMAT_MKV: FileFormatProto
FILE_FORMAT_PDF: FileFormatProto
FILE_FORMAT_DOCX: FileFormatProto
FILE_FORMAT_PPTX: FileFormatProto
FILE_FORMAT_ODT: FileFormatProto
FILE_FORMAT_XLSX: FileFormatProto
FILE_FORMAT_ODS: FileFormatProto
FILE_FORMAT_EPUB: FileFormatProto
FILE_FORMAT_MOBI: FileFormatProto
FILE_FORMAT_CHM: FileFormatProto
FILE_FORMAT_DOC: FileFormatProto
FILE_FORMAT_XLS: FileFormatProto
FILE_FORMAT_PPT: FileFormatProto
FILE_FORMAT_HTML: FileFormatProto
FILE_FORMAT_JSON: FileFormatProto
FILE_FORMAT_YAML: FileFormatProto
FILE_FORMAT_CSV: FileFormatProto
FILE_FORMAT_XML: FileFormatProto
FILE_FORMAT_TOML: FileFormatProto
FILE_FORMAT_SQLITE: FileFormatProto
FILE_FORMAT_PARQUET: FileFormatProto
FILE_FORMAT_ZIP: FileFormatProto
FILE_FORMAT_RAR: FileFormatProto
FILE_FORMAT_TAR: FileFormatProto
FILE_FORMAT_SEVENZIP: FileFormatProto
FILE_FORMAT_CAB: FileFormatProto
FILE_FORMAT_GZIP: FileFormatProto
FILE_FORMAT_BZIP2: FileFormatProto
FILE_FORMAT_XZ: FileFormatProto
FILE_FORMAT_EXE: FileFormatProto
FILE_FORMAT_APP_IMAGE: FileFormatProto
FILE_FORMAT_APK: FileFormatProto
FILE_FORMAT_DMG: FileFormatProto
FILE_FORMAT_JAR: FileFormatProto
FILE_FORMAT_MSI: FileFormatProto
FILE_FORMAT_DEB: FileFormatProto
FILE_FORMAT_RPM: FileFormatProto
FILE_RETENTION_MODE_UNSPECIFIED: FileRetentionModeProto
FILE_RETENTION_MODE_AUTOMATIC: FileRetentionModeProto
FILE_RETENTION_MODE_MANUAL: FileRetentionModeProto
FILE_RETENTION_MODE_TIMED: FileRetentionModeProto
FILE_SOURCE_UNSPECIFIED: FileSourceProto
FILE_SOURCE_SPACE: FileSourceProto
FILE_SOURCE_INLINE: FileSourceProto
FILE_SOURCE_EXTERNAL: FileSourceProto
FILE_TYPE_UNSPECIFIED: FileTypeProto
FILE_TYPE_TEXT: FileTypeProto
FILE_TYPE_CODE: FileTypeProto
FILE_TYPE_IMAGE: FileTypeProto
FILE_TYPE_AUDIO: FileTypeProto
FILE_TYPE_VIDEO: FileTypeProto
FILE_TYPE_DOCUMENT: FileTypeProto
FILE_TYPE_DATA: FileTypeProto
FILE_TYPE_ARCHIVE: FileTypeProto
FILE_TYPE_EXECUTABLE: FileTypeProto
FILE_TYPE_GENERIC: FileTypeProto
FILL_POSITION_UNSPECIFIED: FillPositionProto
FILL_POSITION_TOP_LEFT: FillPositionProto
FILL_POSITION_TOP_CENTER: FillPositionProto
FILL_POSITION_TOP_RIGHT: FillPositionProto
FILL_POSITION_LEFT: FillPositionProto
FILL_POSITION_CENTER: FillPositionProto
FILL_POSITION_RIGHT: FillPositionProto
FILL_POSITION_BOTTOM_LEFT: FillPositionProto
FILL_POSITION_BOTTOM_CENTER: FillPositionProto
FILL_POSITION_BOTTOM_RIGHT: FillPositionProto
FILL_SIZE_UNSPECIFIED: FillSizeProto
FILL_SIZE_FILL: FillSizeProto
FILL_SIZE_STRETCH: FillSizeProto
FILL_SIZE_FIT: FillSizeProto
FILL_SIZE_TILE: FillSizeProto
FILL_TYPE_UNSPECIFIED: FillTypeProto
FILL_TYPE_SOLID: FillTypeProto
FILL_TYPE_GRADIENT: FillTypeProto
FILL_TYPE_IMAGE: FillTypeProto
FOLDER_TYPE_UNSPECIFIED: FolderTypeProto
FOLDER_TYPE_SYSTEM: FolderTypeProto
FOLDER_TYPE_HOME: FolderTypeProto
FOLDER_TYPE_GENERAL: FolderTypeProto
FOLDER_TYPE_MODULE: FolderTypeProto
FOLDER_TYPE_APP: FolderTypeProto
FONT_SIZE_UNSPECIFIED: FontSizeProto
FONT_SIZE_XS: FontSizeProto
FONT_SIZE_SM: FontSizeProto
FONT_SIZE_BASE: FontSizeProto
FONT_SIZE_LG: FontSizeProto
FONT_SIZE_XL: FontSizeProto
FONT_SIZE_XL2: FontSizeProto
FONT_SIZE_XL3: FontSizeProto
FONT_SIZE_XL4: FontSizeProto
FONT_SIZE_XL5: FontSizeProto
FONT_SIZE_XL6: FontSizeProto
FONT_SIZE_XL7: FontSizeProto
FONT_TYPE_UNSPECIFIED: FontTypeProto
FONT_TYPE_SERIF: FontTypeProto
FONT_TYPE_SANS: FontTypeProto
FONT_TYPE_MONO: FontTypeProto
FONT_WEIGHT_UNSPECIFIED: FontWeightProto
FONT_WEIGHT_THIN: FontWeightProto
FONT_WEIGHT_EXTRA_LIGHT: FontWeightProto
FONT_WEIGHT_LIGHT: FontWeightProto
FONT_WEIGHT_NORMAL: FontWeightProto
FONT_WEIGHT_MEDIUM: FontWeightProto
FONT_WEIGHT_SEMI_BOLD: FontWeightProto
FONT_WEIGHT_BOLD: FontWeightProto
FONT_WEIGHT_EXTRA_BOLD: FontWeightProto
FONT_WEIGHT_BLACK: FontWeightProto
FUNCTION_TYPE_UNSPECIFIED: FunctionTypeProto
FUNCTION_TYPE_ADD: FunctionTypeProto
FUNCTION_TYPE_SUBTRACT: FunctionTypeProto
FUNCTION_TYPE_MULTIPLY: FunctionTypeProto
FUNCTION_TYPE_DIVIDE: FunctionTypeProto
FUNCTION_TYPE_MODULO: FunctionTypeProto
FUNCTION_TYPE_POWER: FunctionTypeProto
GRADIENT_TYPE_UNSPECIFIED: GradientTypeProto
GRADIENT_TYPE_LINEAR: GradientTypeProto
GRADIENT_TYPE_RADIAL: GradientTypeProto
GRADIENT_TYPE_CONIC: GradientTypeProto
ICON_TYPE_UNSPECIFIED: IconTypeProto
ICON_TYPE_EMOJI: IconTypeProto
ICON_TYPE_FONT_AWESOME: IconTypeProto
ICON_TYPE_VS_CODE: IconTypeProto
ICON_TYPE_FILE: IconTypeProto
ICON_TYPE_FILE_URL: IconTypeProto
JOIN_TYPE_UNSPECIFIED: JoinTypeProto
JOIN_TYPE_LEFT: JoinTypeProto
JOIN_TYPE_PARENT: JoinTypeProto
JOIN_TYPE_CHILD: JoinTypeProto
LAYER_TYPE_UNSPECIFIED: LayerTypeProto
LAYER_TYPE_GENERAL: LayerTypeProto
LAYER_TYPE_SHAPE: LayerTypeProto
LAYOUT_UNSPECIFIED: LayoutProto
LAYOUT_STACK: LayoutProto
LAYOUT_GRID: LayoutProto
LENGTH_UNIT_UNSPECIFIED: LengthUnitProto
LENGTH_UNIT_PIXEL: LengthUnitProto
LENGTH_UNIT_REM: LengthUnitProto
LENGTH_UNIT_PERCENT: LengthUnitProto
LENGTH_UNIT_FR: LengthUnitProto
LOG_LEVEL_UNSPECIFIED: LogLevelProto
LOG_LEVEL_TRACE: LogLevelProto
LOG_LEVEL_DEBUG: LogLevelProto
LOG_LEVEL_INFO: LogLevelProto
LOG_LEVEL_WARNING: LogLevelProto
LOG_LEVEL_ERROR: LogLevelProto
LOG_LEVEL_PANIC: LogLevelProto
MACHINE_TYPE_UNSPECIFIED: MachineTypeProto
MACHINE_TYPE_RUNTIME: MachineTypeProto
MACHINE_TYPE_UBUNTU: MachineTypeProto
MACHINE_TYPE_MAC: MachineTypeProto
MACHINE_TYPE_WINDOWS: MachineTypeProto
MACHINE_TYPE_CUSTOM: MachineTypeProto
MATERIALIZATION_UNSPECIFIED: MaterializationProto
MATERIALIZATION_PARTIAL: MaterializationProto
MATERIALIZATION_FULL: MaterializationProto
METHOD_CARDINALITY_UNSPECIFIED: MethodCardinalityProto
METHOD_CARDINALITY_UNARY: MethodCardinalityProto
MODE_TYPE_UNSPECIFIED: ModeTypeProto
MODE_TYPE_EDIT: ModeTypeProto
MODE_TYPE_DEBUG: ModeTypeProto
MODE_TYPE_INSPECT: ModeTypeProto
MODE_TYPE_PREVIEW: ModeTypeProto
MODE_TYPE_USE: ModeTypeProto
MODEL_DEVELOPER_UNSPECIFIED: ModelDeveloperProto
MODEL_DEVELOPER_OPENAI: ModelDeveloperProto
MODEL_DEVELOPER_ANTHROPIC: ModelDeveloperProto
MODEL_DEVELOPER_GOOGLE: ModelDeveloperProto
MODEL_DEVELOPER_XAI: ModelDeveloperProto
MODEL_PROVIDER_UNSPECIFIED: ModelProviderProto
MODEL_PROVIDER_OPENROUTER: ModelProviderProto
MODEL_PROVIDER_OPENAI: ModelProviderProto
MODEL_PROVIDER_ANTHROPIC: ModelProviderProto
MODEL_PROVIDER_GOOGLE: ModelProviderProto
MODEL_PROVIDER_XAI: ModelProviderProto
MONTH_UNSPECIFIED: MonthProto
MONTH_JANUARY: MonthProto
MONTH_FEBRUARY: MonthProto
MONTH_MARCH: MonthProto
MONTH_APRIL: MonthProto
MONTH_MAY: MonthProto
MONTH_JUNE: MonthProto
MONTH_JULY: MonthProto
MONTH_AUGUST: MonthProto
MONTH_SEPTEMBER: MonthProto
MONTH_OCTOBER: MonthProto
MONTH_NOVEMBER: MonthProto
MONTH_DECEMBER: MonthProto
MOUSE_BUTTON_UNSPECIFIED: MouseButtonProto
MOUSE_BUTTON_LEFT: MouseButtonProto
MOUSE_BUTTON_RIGHT: MouseButtonProto
MOUSE_BUTTON_MIDDLE: MouseButtonProto
NODE_DEFINITION_TYPE_UNSPECIFIED: NodeDefinitionTypeProto
NODE_DEFINITION_TYPE_BUILTIN: NodeDefinitionTypeProto
NODE_DEFINITION_TYPE_CUSTOM: NodeDefinitionTypeProto
NODE_TYPE_UNSPECIFIED: NodeTypeProto
NODE_TYPE_NODE: NodeTypeProto
NODE_TYPE_ENTITY: NodeTypeProto
NODE_TYPE_EVENT: NodeTypeProto
NODE_TYPE_CUSTOM_ENTITY_DEFINITION: NodeTypeProto
NODE_TYPE_CUSTOM_TRAIT_DEFINITION: NodeTypeProto
NODE_TYPE_CUSTOM_EVENT_DEFINITION: NodeTypeProto
NODE_TYPE_CUSTOM_STRUCT_DEFINITION: NodeTypeProto
NODE_TYPE_CUSTOM_ENUM_DEFINITION: NodeTypeProto
NODE_TYPE_CUSTOM_PROPERTY: NodeTypeProto
NODE_TYPE_CUSTOM_PROPERTY_GROUP: NodeTypeProto
NODE_TYPE_CUSTOM_OPTION: NodeTypeProto
NODE_TYPE_CUSTOM_OPTION_GROUP: NodeTypeProto
NODE_TYPE_RECORD: NodeTypeProto
NODE_TYPE_RESOURCE: NodeTypeProto
NODE_TYPE_METRIC: NodeTypeProto
NODE_TYPE_SNAPSHOT: NodeTypeProto
NODE_TYPE_SIGNAL: NodeTypeProto
NODE_TYPE_EDIT_EVENT: NodeTypeProto
NODE_TYPE_CHANGE_EVENT: NodeTypeProto
NODE_TYPE_QUERY_EVENT: NodeTypeProto
NODE_TYPE_MEASUREMENT_EVENT: NodeTypeProto
NODE_TYPE_UNIVERSE: NodeTypeProto
NODE_TYPE_SPACE: NodeTypeProto
NODE_TYPE_HANDLE: NodeTypeProto
NODE_TYPE_USER: NodeTypeProto
NODE_TYPE_FRIENDSHIP: NodeTypeProto
NODE_TYPE_FRIENDSHIP_INVITE: NodeTypeProto
NODE_TYPE_FRIENDSHIP_INVITE_EVENT: NodeTypeProto
NODE_TYPE_FRIENDSHIP_INVITE_SENT_EVENT: NodeTypeProto
NODE_TYPE_FRIENDSHIP_INVITE_RESCINDED_EVENT: NodeTypeProto
NODE_TYPE_FRIENDSHIP_INVITE_ACCEPTED_EVENT: NodeTypeProto
NODE_TYPE_FRIENDSHIP_INVITE_REJECTED_EVENT: NodeTypeProto
NODE_TYPE_CLIENT: NodeTypeProto
NODE_TYPE_ORGANIZATION: NodeTypeProto
NODE_TYPE_TEAM: NodeTypeProto
NODE_TYPE_FOLDER: NodeTypeProto
NODE_TYPE_TAG: NodeTypeProto
NODE_TYPE_TAGGING: NodeTypeProto
NODE_TYPE_BRANCH: NodeTypeProto
NODE_TYPE_MEMBERSHIP: NodeTypeProto
NODE_TYPE_MEMBERSHIP_EVENT: NodeTypeProto
NODE_TYPE_MEMBERSHIP_JOINED_EVENT: NodeTypeProto
NODE_TYPE_MEMBERSHIP_LEFT_EVENT: NodeTypeProto
NODE_TYPE_INVITE: NodeTypeProto
NODE_TYPE_INVITE_EVENT: NodeTypeProto
NODE_TYPE_INVITE_SENT_EVENT: NodeTypeProto
NODE_TYPE_INVITE_RESCINDED_EVENT: NodeTypeProto
NODE_TYPE_INVITE_ACCEPTED_EVENT: NodeTypeProto
NODE_TYPE_INVITE_REJECTED_EVENT: NodeTypeProto
NODE_TYPE_ROLE: NodeTypeProto
NODE_TYPE_ROLE_EVENT: NodeTypeProto
NODE_TYPE_ROLE_ASSIGNED_EVENT: NodeTypeProto
NODE_TYPE_ROLE_UNASSIGNED_EVENT: NodeTypeProto
NODE_TYPE_PERMISSION: NodeTypeProto
NODE_TYPE_SANCTION: NodeTypeProto
NODE_TYPE_SANCTION_EVENT: NodeTypeProto
NODE_TYPE_SANCTION_REQUESTED_EVENT: NodeTypeProto
NODE_TYPE_SANCTION_GRANTED_EVENT: NodeTypeProto
NODE_TYPE_SANCTION_REVOKED_EVENT: NodeTypeProto
NODE_TYPE_SANCTION_EXPIRED_EVENT: NodeTypeProto
NODE_TYPE_ENTITLEMENT: NodeTypeProto
NODE_TYPE_ENTITLEMENT_EVENT: NodeTypeProto
NODE_TYPE_ENTITLEMENT_REQUESTED_EVENT: NodeTypeProto
NODE_TYPE_ENTITLEMENT_GRANTED_EVENT: NodeTypeProto
NODE_TYPE_ENTITLEMENT_REVOKED_EVENT: NodeTypeProto
NODE_TYPE_ENTITLEMENT_EXPIRED_EVENT: NodeTypeProto
NODE_TYPE_AGENT: NodeTypeProto
NODE_TYPE_FILE: NodeTypeProto
NODE_TYPE_SERVICE: NodeTypeProto
NODE_TYPE_SCRIPT: NodeTypeProto
NODE_TYPE_METHOD: NodeTypeProto
NODE_TYPE_ACTION: NodeTypeProto
NODE_TYPE_TRIGGER: NodeTypeProto
NODE_TYPE_TRIGGER_EVENT: NodeTypeProto
NODE_TYPE_TIMER: NodeTypeProto
NODE_TYPE_TIMER_EVENT: NodeTypeProto
NODE_TYPE_TIMER_STARTED_EVENT: NodeTypeProto
NODE_TYPE_TIMER_COMPLETED_EVENT: NodeTypeProto
NODE_TYPE_TIMER_CANCELLED_EVENT: NodeTypeProto
NODE_TYPE_CURSOR: NodeTypeProto
NODE_TYPE_EVENT_CURSOR: NodeTypeProto
NODE_TYPE_SCREEN_CURSOR: NodeTypeProto
NODE_TYPE_THREAD_CURSOR: NodeTypeProto
NODE_TYPE_ROUTE: NodeTypeProto
NODE_TYPE_DATABASE: NodeTypeProto
NODE_TYPE_MACHINE: NodeTypeProto
NODE_TYPE_ENVIRONMENT: NodeTypeProto
NODE_TYPE_RUN: NodeTypeProto
NODE_TYPE_RUN_EVENT: NodeTypeProto
NODE_TYPE_RUN_STARTED_EVENT: NodeTypeProto
NODE_TYPE_RUN_PAUSE_REQUESTED_EVENT: NodeTypeProto
NODE_TYPE_RUN_PAUSED_EVENT: NodeTypeProto
NODE_TYPE_RUN_RESUME_REQUESTED_EVENT: NodeTypeProto
NODE_TYPE_RUN_RESUMED_EVENT: NodeTypeProto
NODE_TYPE_RUN_STOP_REQUESTED_EVENT: NodeTypeProto
NODE_TYPE_RUN_FAILED_EVENT: NodeTypeProto
NODE_TYPE_RUN_COMPLETED_EVENT: NodeTypeProto
NODE_TYPE_SPAN_EVENT: NodeTypeProto
NODE_TYPE_LOG_EVENT: NodeTypeProto
NODE_TYPE_GAUGE_METRIC: NodeTypeProto
NODE_TYPE_GAUGE_MEASUREMENT_EVENT: NodeTypeProto
NODE_TYPE_COUNTER_METRIC: NodeTypeProto
NODE_TYPE_COUNTER_MEASUREMENT_EVENT: NodeTypeProto
NODE_TYPE_HISTOGRAM_METRIC: NodeTypeProto
NODE_TYPE_HISTOGRAM_MEASUREMENT_EVENT: NodeTypeProto
NODE_TYPE_THREAD: NodeTypeProto
NODE_TYPE_MESSAGE: NodeTypeProto
NODE_TYPE_REACTION: NodeTypeProto
NODE_TYPE_STAR: NodeTypeProto
NODE_TYPE_FOLLOW: NodeTypeProto
NODE_TYPE_NOTIFICATION: NodeTypeProto
NODE_TYPE_NOTIFICATION_EVENT: NodeTypeProto
NODE_TYPE_NOTIFICATION_SENT_EVENT: NodeTypeProto
NODE_TYPE_NOTIFICATION_RESCINDED_EVENT: NodeTypeProto
NODE_TYPE_NOTIFICATION_READ_EVENT: NodeTypeProto
NODE_TYPE_NOTIFICATION_DISMISSED_EVENT: NodeTypeProto
NODE_TYPE_NOTIFICATION_EXPIRED_EVENT: NodeTypeProto
NODE_TYPE_WINDOW: NodeTypeProto
NODE_TYPE_SCENE: NodeTypeProto
NODE_TYPE_SCENE_EVENT: NodeTypeProto
NODE_TYPE_LAYER: NodeTypeProto
NODE_TYPE_VARIANT: NodeTypeProto
NODE_TYPE_VIEW: NodeTypeProto
NODE_TYPE_VIEW_EVENT: NodeTypeProto
NODE_TYPE_VIEW_ENTERED_EVENT: NodeTypeProto
NODE_TYPE_VIEW_EXITED_EVENT: NodeTypeProto
NODE_TYPE_CONTAINER_VIEW: NodeTypeProto
NODE_TYPE_FRAME_VIEW: NodeTypeProto
NODE_TYPE_LABEL_VIEW: NodeTypeProto
NODE_TYPE_SPLIT_VIEW: NodeTypeProto
NODE_TYPE_CONTENT_VIEW: NodeTypeProto
NODE_TYPE_TEXT_VIEW: NodeTypeProto
NODE_TYPE_INPUT_VIEW: NodeTypeProto
NODE_TYPE_NUMBER_INPUT_VIEW: NodeTypeProto
NODE_TYPE_SLIDER_INPUT_VIEW: NodeTypeProto
NODE_TYPE_INTERNAL_VIEW: NodeTypeProto
NODE_TYPE_CANVAS: NodeTypeProto
NODE_TYPE_SHAPE: NodeTypeProto
NODE_TYPE_LINE_SHAPE: NodeTypeProto
NODE_TYPE_ARROW_SHAPE: NodeTypeProto
NODE_TYPE_ANNOTATION_SHAPE: NodeTypeProto
NODE_TYPE_INPUT_EVENT: NodeTypeProto
NODE_TYPE_POINTER_EVENT: NodeTypeProto
NODE_TYPE_POINTER_DOWN_EVENT: NodeTypeProto
NODE_TYPE_POINTER_UP_EVENT: NodeTypeProto
NODE_TYPE_POINTER_MOVE_EVENT: NodeTypeProto
NODE_TYPE_POINTER_ENTER_EVENT: NodeTypeProto
NODE_TYPE_POINTER_OVER_EVENT: NodeTypeProto
NODE_TYPE_POINTER_LEAVE_EVENT: NodeTypeProto
NODE_TYPE_POINTER_LONG_PRESS_EVENT: NodeTypeProto
NODE_TYPE_MOUSE_EVENT: NodeTypeProto
NODE_TYPE_CLICK_EVENT: NodeTypeProto
NODE_TYPE_LEFT_CLICK_EVENT: NodeTypeProto
NODE_TYPE_RIGHT_CLICK_EVENT: NodeTypeProto
NODE_TYPE_MIDDLE_CLICK_EVENT: NodeTypeProto
NODE_TYPE_DOUBLE_CLICK_EVENT: NodeTypeProto
NODE_TYPE_WHEEL_EVENT: NodeTypeProto
NODE_TYPE_KEYBOARD_EVENT: NodeTypeProto
NODE_TYPE_KEY_DOWN_EVENT: NodeTypeProto
NODE_TYPE_KEY_UP_EVENT: NodeTypeProto
NODE_TYPE_KEY_PRESS_EVENT: NodeTypeProto
NODE_TYPE_DRAG_EVENT: NodeTypeProto
NODE_TYPE_DRAG_START_EVENT: NodeTypeProto
NODE_TYPE_DRAG_END_EVENT: NodeTypeProto
NODE_TYPE_DRAG_OVER_EVENT: NodeTypeProto
NODE_TYPE_DRAG_ENTER_EVENT: NodeTypeProto
NODE_TYPE_DRAG_LEAVE_EVENT: NodeTypeProto
NODE_TYPE_DROP_EVENT: NodeTypeProto
NODE_TYPE_CLIPBOARD_EVENT: NodeTypeProto
NODE_TYPE_COPY_EVENT: NodeTypeProto
NODE_TYPE_CUT_EVENT: NodeTypeProto
NODE_TYPE_PASTE_EVENT: NodeTypeProto
NODE_TYPE_FOCUS_EVENT: NodeTypeProto
NODE_TYPE_FOCUS_IN_EVENT: NodeTypeProto
NODE_TYPE_FOCUS_OUT_EVENT: NodeTypeProto
NODE_TYPE_THEME: NodeTypeProto
NODE_TYPE_PALETTE: NodeTypeProto
NODE_TYPE_STYLE: NodeTypeProto
NODE_TYPE_COLOR_STYLE: NodeTypeProto
NODE_TYPE_FILL_STYLE: NodeTypeProto
NODE_TYPE_FONT_STYLE: NodeTypeProto
NODE_TYPE_BORDER_STYLE: NodeTypeProto
NODE_TYPE_SHADOW_STYLE: NodeTypeProto
NODE_TYPE_GRADIENT_STYLE: NodeTypeProto
NODE_TYPE_TRANSITION_STYLE: NodeTypeProto
NODE_TYPE_EFFECT_STYLE: NodeTypeProto
NODE_TYPE_STROKE_STYLE: NodeTypeProto
NOTIFICATION_STATUS_UNSPECIFIED: NotificationStatusProto
NOTIFICATION_STATUS_UNREAD: NotificationStatusProto
NOTIFICATION_STATUS_READ: NotificationStatusProto
NOTIFICATION_STATUS_DISMISSED: NotificationStatusProto
NOTIFICATION_STATUS_EXPIRED: NotificationStatusProto
NOTIFICATION_STATUS_RESCINDED: NotificationStatusProto
NUMBER_FORMAT_UNSPECIFIED: NumberFormatProto
NUMBER_FORMAT_PERCENTAGE: NumberFormatProto
NUMBER_FORMAT_ANGLE: NumberFormatProto
NUMBER_FORMAT_CURRENCY: NumberFormatProto
OBJECT_DEFINITION_TYPE_UNSPECIFIED: ObjectDefinitionTypeProto
OBJECT_DEFINITION_TYPE_BUILTIN_NODE: ObjectDefinitionTypeProto
OBJECT_DEFINITION_TYPE_CUSTOM_NODE: ObjectDefinitionTypeProto
OBJECT_DEFINITION_TYPE_BUILTIN_TRAIT: ObjectDefinitionTypeProto
OBJECT_DEFINITION_TYPE_CUSTOM_TRAIT: ObjectDefinitionTypeProto
OBJECT_DEFINITION_TYPE_BUILTIN_STRUCT: ObjectDefinitionTypeProto
OBJECT_DEFINITION_TYPE_CUSTOM_STRUCT: ObjectDefinitionTypeProto
OFFSCREEN_BEHAVIOR_UNSPECIFIED: OffscreenBehaviorProto
OFFSCREEN_BEHAVIOR_PLAY: OffscreenBehaviorProto
OFFSCREEN_BEHAVIOR_PAUSE: OffscreenBehaviorProto
OPERATING_SYSTEM_UNSPECIFIED: OperatingSystemProto
OPERATING_SYSTEM_LINUX: OperatingSystemProto
OPERATING_SYSTEM_WINDOWS: OperatingSystemProto
OPERATING_SYSTEM_MACOS: OperatingSystemProto
OPERATING_SYSTEM_ANDROID: OperatingSystemProto
OPERATING_SYSTEM_IOS: OperatingSystemProto
ORGANIZATION_STATUS_UNSPECIFIED: OrganizationStatusProto
ORGANIZATION_STATUS_CREATING: OrganizationStatusProto
ORGANIZATION_STATUS_ACTIVE: OrganizationStatusProto
OVERFLOW_UNSPECIFIED: OverflowProto
OVERFLOW_HIDDEN: OverflowProto
OVERFLOW_VISIBLE: OverflowProto
OVERFLOW_SCROLL: OverflowProto
PERMISSION_TYPE_UNSPECIFIED: PermissionTypeProto
PERMISSION_TYPE_GENERAL: PermissionTypeProto
PLATFORM_TYPE_UNSPECIFIED: PlatformTypeProto
PLATFORM_TYPE_SYSTEM: PlatformTypeProto
PLATFORM_TYPE_RUNTIME: PlatformTypeProto
PLATFORM_TYPE_WEB: PlatformTypeProto
POSITION_TYPE_UNSPECIFIED: PositionTypeProto
POSITION_TYPE_RELATIVE: PositionTypeProto
POSITION_TYPE_ABSOLUTE: PositionTypeProto
POSITION_TYPE_FIXED: PositionTypeProto
POSITION_TYPE_STICKY: PositionTypeProto
PRIMITIVE_TYPE_UNSPECIFIED: PrimitiveTypeProto
PRIMITIVE_TYPE_BOOLEAN: PrimitiveTypeProto
PRIMITIVE_TYPE_INT16: PrimitiveTypeProto
PRIMITIVE_TYPE_INT32: PrimitiveTypeProto
PRIMITIVE_TYPE_INT64: PrimitiveTypeProto
PRIMITIVE_TYPE_DECIMAL: PrimitiveTypeProto
PRIMITIVE_TYPE_FLOAT32: PrimitiveTypeProto
PRIMITIVE_TYPE_FLOAT64: PrimitiveTypeProto
PRIMITIVE_TYPE_STRING: PrimitiveTypeProto
PRIMITIVE_TYPE_UUID: PrimitiveTypeProto
PRIMITIVE_TYPE_JSON: PrimitiveTypeProto
PRIMITIVE_TYPE_BYTES: PrimitiveTypeProto
PRIMITIVE_TYPE_DATETIME: PrimitiveTypeProto
PRIMITIVE_TYPE_DATE: PrimitiveTypeProto
PRIMITIVE_TYPE_TIME: PrimitiveTypeProto
PRIMITIVE_TYPE_DURATION: PrimitiveTypeProto
PROPERTY_REFERENCE_TYPE_UNSPECIFIED: PropertyReferenceTypeProto
PROPERTY_REFERENCE_TYPE_BUILTIN: PropertyReferenceTypeProto
PROPERTY_REFERENCE_TYPE_CUSTOM: PropertyReferenceTypeProto
PROPERTY_TYPE_UNSPECIFIED: PropertyTypeProto
PROPERTY_TYPE_MEMBER: PropertyTypeProto
PROPERTY_TYPE_CONSTANT: PropertyTypeProto
PROPERTY_TYPE_INPUT: PropertyTypeProto
PROPERTY_TYPE_OUTPUT: PropertyTypeProto
QUERY_TYPE_UNSPECIFIED: QueryTypeProto
QUERY_TYPE_NODE: QueryTypeProto
QUERY_TYPE_SCALAR: QueryTypeProto
QUERY_TYPE_GROUPED_NODE: QueryTypeProto
QUERY_TYPE_GROUPED_SCALAR: QueryTypeProto
QUERY_UPDATE_TYPE_UNSPECIFIED: QueryUpdateTypeProto
QUERY_UPDATE_TYPE_FULL_RESULT: QueryUpdateTypeProto
QUERY_UPDATE_TYPE_PARTIAL_RESULT: QueryUpdateTypeProto
REGION_UNSPECIFIED: RegionProto
REGION_ZURICH: RegionProto
REGION_FRANKFURT: RegionProto
REGION_VIRGINIA: RegionProto
REGION_OHIO: RegionProto
REGION_OREGON: RegionProto
REGION_SAO_PAULO: RegionProto
REGION_CAPE_TOWN: RegionProto
REGION_MUMBAI: RegionProto
REGION_SINGAPORE: RegionProto
REGION_TOKYO: RegionProto
REGION_SYDNEY: RegionProto
REGION_AREA_UNSPECIFIED: RegionAreaProto
REGION_AREA_EUROPE_CENTRAL: RegionAreaProto
REGION_AREA_NORTH_AMERICA_EAST: RegionAreaProto
REGION_AREA_NORTH_AMERICA_WEST: RegionAreaProto
REGION_AREA_SOUTH_AMERICA_EAST: RegionAreaProto
REGION_AREA_MIDDLE_EAST_CENTRAL: RegionAreaProto
REGION_AREA_MIDDLE_EAST_WEST: RegionAreaProto
REGION_AREA_AFRICA_SOUTH: RegionAreaProto
REGION_AREA_ASIA_WEST: RegionAreaProto
REGION_AREA_ASIA_SOUTH: RegionAreaProto
REGION_AREA_ASIA_EAST: RegionAreaProto
REGION_AREA_AUSTRALIA_SOUTH: RegionAreaProto
REGION_CONTINENT_UNSPECIFIED: RegionContinentProto
REGION_CONTINENT_EUROPE: RegionContinentProto
REGION_CONTINENT_NORTH_AMERICA: RegionContinentProto
REGION_CONTINENT_SOUTH_AMERICA: RegionContinentProto
REGION_CONTINENT_MIDDLE_EAST: RegionContinentProto
REGION_CONTINENT_AFRICA: RegionContinentProto
REGION_CONTINENT_ASIA: RegionContinentProto
REGION_CONTINENT_AUSTRALIA: RegionContinentProto
REGION_CONTINENT_PRIVATE: RegionContinentProto
REPEAT_TYPE_UNSPECIFIED: RepeatTypeProto
REPEAT_TYPE_LOOP: RepeatTypeProto
REPEAT_TYPE_REVERSE: RepeatTypeProto
REPEAT_TYPE_MIRROR: RepeatTypeProto
RESOURCE_STATUS_UNSPECIFIED: ResourceStatusProto
RESOURCE_STATUS_PENDING: ResourceStatusProto
RESOURCE_STATUS_CREATING: ResourceStatusProto
RESOURCE_STATUS_RETRYING: ResourceStatusProto
RESOURCE_STATUS_AVAILABLE: ResourceStatusProto
RESOURCE_STATUS_SLEEPING: ResourceStatusProto
RESOURCE_STATUS_UNAVAILABLE: ResourceStatusProto
RESOURCE_STATUS_IMPAIRED: ResourceStatusProto
RESOURCE_STATUS_OFFLINE: ResourceStatusProto
RESOURCE_STATUS_FAILED: ResourceStatusProto
ROLE_TYPE_UNSPECIFIED: RoleTypeProto
ROLE_TYPE_SYSTEM: RoleTypeProto
ROLE_TYPE_OWNER: RoleTypeProto
ROLE_TYPE_ADMIN: RoleTypeProto
ROLE_TYPE_DEVELOPER: RoleTypeProto
ROLE_TYPE_USER: RoleTypeProto
ROLE_TYPE_SPECTATOR: RoleTypeProto
RUN_STATUS_UNSPECIFIED: RunStatusProto
RUN_STATUS_SCHEDULED: RunStatusProto
RUN_STATUS_RUNNING: RunStatusProto
RUN_STATUS_PAUSED: RunStatusProto
RUN_STATUS_YIELDED: RunStatusProto
RUN_STATUS_CANCELLED: RunStatusProto
RUN_STATUS_ABORTED: RunStatusProto
RUN_STATUS_FAILED: RunStatusProto
RUN_STATUS_COMPLETED: RunStatusProto
RUNTIME_LANGUAGE_UNSPECIFIED: RuntimeLanguageProto
RUNTIME_LANGUAGE_PYTHON: RuntimeLanguageProto
RUNTIME_LANGUAGE_JAVASCRIPT: RuntimeLanguageProto
SANCTION_TYPE_UNSPECIFIED: SanctionTypeProto
SANCTION_TYPE_BAN: SanctionTypeProto
SANCTION_TYPE_MUTE: SanctionTypeProto
SCALAR_TYPE_UNSPECIFIED: ScalarTypeProto
SCALAR_TYPE_PRIMITIVE: ScalarTypeProto
SCALAR_TYPE_ENUM: ScalarTypeProto
SCALAR_TYPE_NODE_REFERENCE: ScalarTypeProto
SCALAR_TYPE_NODE_VALUE: ScalarTypeProto
SCALAR_TYPE_STRUCT: ScalarTypeProto
SCHEDULE_FREQUENCY_UNSPECIFIED: ScheduleFrequencyProto
SCHEDULE_FREQUENCY_YEAR: ScheduleFrequencyProto
SCHEDULE_FREQUENCY_MONTH: ScheduleFrequencyProto
SCHEDULE_FREQUENCY_WEEK: ScheduleFrequencyProto
SCHEDULE_FREQUENCY_DAY: ScheduleFrequencyProto
SCHEDULE_FREQUENCY_HOUR: ScheduleFrequencyProto
SCHEDULE_FREQUENCY_MINUTE: ScheduleFrequencyProto
SHADOW_POSITION_UNSPECIFIED: ShadowPositionProto
SHADOW_POSITION_OUTSIDE: ShadowPositionProto
SHADOW_POSITION_INSIDE: ShadowPositionProto
SHADOW_TYPE_UNSPECIFIED: ShadowTypeProto
SHADOW_TYPE_BOX: ShadowTypeProto
SHADOW_TYPE_REALISTIC: ShadowTypeProto
SNAPSHOT_STATUS_UNSPECIFIED: SnapshotStatusProto
SNAPSHOT_STATUS_CREATING: SnapshotStatusProto
SNAPSHOT_STATUS_ACTIVE: SnapshotStatusProto
SNAPSHOT_STATUS_READONLY: SnapshotStatusProto
SNAPSHOT_TYPE_UNSPECIFIED: SnapshotTypeProto
SNAPSHOT_TYPE_PARTIAL: SnapshotTypeProto
SNAPSHOT_TYPE_FULL: SnapshotTypeProto
SORT_MODE_UNSPECIFIED: SortModeProto
SORT_MODE_MAX: SortModeProto
SORT_MODE_MIN: SortModeProto
SORT_MODE_AVERAGE: SortModeProto
SORT_MODE_SUM: SortModeProto
SORT_MODE_MEDIAN: SortModeProto
SORT_TYPE_UNSPECIFIED: SortTypeProto
SORT_TYPE_ASCENDING: SortTypeProto
SORT_TYPE_DESCENDING: SortTypeProto
SPACE_STATUS_UNSPECIFIED: SpaceStatusProto
SPACE_STATUS_CREATING: SpaceStatusProto
SPACE_STATUS_QUEUED: SpaceStatusProto
SPACE_STATUS_RUNNING: SpaceStatusProto
SPACE_STATUS_PAUSED: SpaceStatusProto
SPRING_TYPE_UNSPECIFIED: SpringTypeProto
SPRING_TYPE_TIME: SpringTypeProto
SPRING_TYPE_PHYSICS: SpringTypeProto
STORE_IMPLEMENTATION_UNSPECIFIED: StoreImplementationProto
STORE_IMPLEMENTATION_MEMORY: StoreImplementationProto
STORE_IMPLEMENTATION_POSTGRES: StoreImplementationProto
STORE_TYPE_UNSPECIFIED: StoreTypeProto
STORE_TYPE_LOCAL_ENTITY: StoreTypeProto
STORE_TYPE_LOCAL_EVENT: StoreTypeProto
STORE_TYPE_GLOBAL_ENTITY_PRIMARY: StoreTypeProto
STORE_TYPE_SPATIAL_ENTITY_PRIMARY: StoreTypeProto
STORE_TYPE_SPATIAL_EVENT_PRIMARY: StoreTypeProto
STRING_FORMAT_UNSPECIFIED: StringFormatProto
STRING_FORMAT_NAME: StringFormatProto
STRING_FORMAT_SLUG: StringFormatProto
STRING_FORMAT_EMAIL: StringFormatProto
STRING_FORMAT_UUID: StringFormatProto
STRING_FORMAT_URL: StringFormatProto
STRING_FORMAT_EMOJI: StringFormatProto
STRING_FORMAT_MIME: StringFormatProto
STRING_FORMAT_BASE64: StringFormatProto
STROKE_TYPE_UNSPECIFIED: StrokeTypeProto
STROKE_TYPE_SOLID: StrokeTypeProto
STROKE_TYPE_DASHED: StrokeTypeProto
STROKE_TYPE_DOTTED: StrokeTypeProto
STROKE_TYPE_FREEHAND: StrokeTypeProto
STRUCT_DEFINITION_TYPE_UNSPECIFIED: StructDefinitionTypeProto
STRUCT_DEFINITION_TYPE_BUILTIN_STRUCT: StructDefinitionTypeProto
STRUCT_DEFINITION_TYPE_CUSTOM_STRUCT: StructDefinitionTypeProto
STRUCT_DEFINITION_TYPE_BUILTIN_ENUM: StructDefinitionTypeProto
STRUCT_DEFINITION_TYPE_CUSTOM_ENUM: StructDefinitionTypeProto
STRUCT_TYPE_UNSPECIFIED: StructTypeProto
STRUCT_TYPE_STRUCT: StructTypeProto
STRUCT_TYPE_CUSTOM_STRUCT: StructTypeProto
STRUCT_TYPE_BUILTIN_DEFINITION: StructTypeProto
STRUCT_TYPE_NODE_DEFINITION: StructTypeProto
STRUCT_TYPE_TRAIT_DEFINITION: StructTypeProto
STRUCT_TYPE_STRUCT_DEFINITION: StructTypeProto
STRUCT_TYPE_ENUM_DEFINITION: StructTypeProto
STRUCT_TYPE_PROPERTY_DEFINITION: StructTypeProto
STRUCT_TYPE_PROPERTY_GROUP_DEFINITION: StructTypeProto
STRUCT_TYPE_OPTION_DEFINITION: StructTypeProto
STRUCT_TYPE_OPTION_GROUP_DEFINITION: StructTypeProto
STRUCT_TYPE_CONSTANT_DEFINITION: StructTypeProto
STRUCT_TYPE_METHOD_DEFINITION: StructTypeProto
STRUCT_TYPE_ACTION_DEFINITION: StructTypeProto
STRUCT_TYPE_PERMISSION_DEFINITION: StructTypeProto
STRUCT_TYPE_NODE_DEFINITION_REFERENCE: StructTypeProto
STRUCT_TYPE_OBJECT_DEFINITION_REFERENCE: StructTypeProto
STRUCT_TYPE_STRUCT_DEFINITION_REFERENCE: StructTypeProto
STRUCT_TYPE_NODE_REFERENCE: StructTypeProto
STRUCT_TYPE_PROPERTY_REFERENCE: StructTypeProto
STRUCT_TYPE_EDIT: StructTypeProto
STRUCT_TYPE_CHANGE: StructTypeProto
STRUCT_TYPE_CHANGE_RESULT: StructTypeProto
STRUCT_TYPE_ORIGIN: StructTypeProto
STRUCT_TYPE_EXPRESSION: StructTypeProto
STRUCT_TYPE_FUNCTION: StructTypeProto
STRUCT_TYPE_JOIN: StructTypeProto
STRUCT_TYPE_AGGREGATION: StructTypeProto
STRUCT_TYPE_CONDITION: StructTypeProto
STRUCT_TYPE_SORT: StructTypeProto
STRUCT_TYPE_SELECT: StructTypeProto
STRUCT_TYPE_QUERY: StructTypeProto
STRUCT_TYPE_QUERY_RESULT: StructTypeProto
STRUCT_TYPE_QUERY_RESULT_GROUP: StructTypeProto
STRUCT_TYPE_QUERY_UPDATE: StructTypeProto
STRUCT_TYPE_HISTOGRAM: StructTypeProto
STRUCT_TYPE_SELECTION: StructTypeProto
STRUCT_TYPE_VALUE: StructTypeProto
STRUCT_TYPE_TYPE: StructTypeProto
STRUCT_TYPE_NUMBER_CONSTRAINT: StructTypeProto
STRUCT_TYPE_STRING_CONSTRAINT: StructTypeProto
STRUCT_TYPE_COLLECTION_CONSTRAINT: StructTypeProto
STRUCT_TYPE_NODE_CONSTRAINT: StructTypeProto
STRUCT_TYPE_VECTOR: StructTypeProto
STRUCT_TYPE_VECTORF: StructTypeProto
STRUCT_TYPE_VECTOR2F: StructTypeProto
STRUCT_TYPE_VECTOR3F: StructTypeProto
STRUCT_TYPE_VECTOR4F: StructTypeProto
STRUCT_TYPE_VECTORI: StructTypeProto
STRUCT_TYPE_VECTOR2I: StructTypeProto
STRUCT_TYPE_VECTOR3I: StructTypeProto
STRUCT_TYPE_VECTOR4I: StructTypeProto
STRUCT_TYPE_TEXT: StructTypeProto
STRUCT_TYPE_TEXT_SPAN: StructTypeProto
STRUCT_TYPE_ICON: StructTypeProto
STRUCT_TYPE_SCHEDULE: StructTypeProto
STRUCT_TYPE_DATABASE_INFO: StructTypeProto
STRUCT_TYPE_GALAXY_INFO: StructTypeProto
STRUCT_TYPE_LINE: StructTypeProto
STRUCT_TYPE_ARROW: StructTypeProto
STRUCT_TYPE_LENGTH: StructTypeProto
STRUCT_TYPE_POSITION: StructTypeProto
STRUCT_TYPE_DIMENSION: StructTypeProto
STRUCT_TYPE_GRID: StructTypeProto
STRUCT_TYPE_GRID_SPAN: StructTypeProto
STRUCT_TYPE_INSETS: StructTypeProto
STRUCT_TYPE_CORNERS: StructTypeProto
STRUCT_TYPE_AXIS2: StructTypeProto
STRUCT_TYPE_AXIS3: StructTypeProto
STRUCT_TYPE_COLOR: StructTypeProto
STRUCT_TYPE_FILL: StructTypeProto
STRUCT_TYPE_FONT: StructTypeProto
STRUCT_TYPE_BORDER: StructTypeProto
STRUCT_TYPE_SHADOW: StructTypeProto
STRUCT_TYPE_GRADIENT: StructTypeProto
STRUCT_TYPE_GRADIENT_STOP: StructTypeProto
STRUCT_TYPE_TRANSITION: StructTypeProto
STRUCT_TYPE_EFFECT: StructTypeProto
STRUCT_TYPE_STROKE: StructTypeProto
STRUCT_TYPE_STROKE_CAP: StructTypeProto
STRUCT_TYPE_STROKE_PATH: StructTypeProto
STRUCT_TYPE_STROKE_POINT: StructTypeProto
TENANCY_UNSPECIFIED: TenancyProto
TENANCY_DEDICATED: TenancyProto
TENANCY_SHARED: TenancyProto
TEXT_ALIGN_UNSPECIFIED: TextAlignProto
TEXT_ALIGN_LEFT: TextAlignProto
TEXT_ALIGN_CENTER: TextAlignProto
TEXT_ALIGN_RIGHT: TextAlignProto
TEXT_ALIGN_JUSTIFY: TextAlignProto
TEXT_DECORATION_UNSPECIFIED: TextDecorationProto
TEXT_DECORATION_NONE: TextDecorationProto
TEXT_DECORATION_UNDERLINE: TextDecorationProto
TEXT_DECORATION_STRIKETHROUGH: TextDecorationProto
TEXT_SPAN_TYPE_UNSPECIFIED: TextSpanTypeProto
TEXT_SPAN_TYPE_TEXT: TextSpanTypeProto
TEXT_SPAN_TYPE_HARD_BREAK: TextSpanTypeProto
TEXT_SPAN_TYPE_MENTION: TextSpanTypeProto
TEXT_SPAN_TYPE_LINK: TextSpanTypeProto
TEXT_SPAN_TYPE_CITATION: TextSpanTypeProto
TEXT_SPAN_TYPE_EQUATION: TextSpanTypeProto
TEXT_SPLIT_TYPE_UNSPECIFIED: TextSplitTypeProto
TEXT_SPLIT_TYPE_CHAR: TextSplitTypeProto
TEXT_SPLIT_TYPE_WORD: TextSplitTypeProto
TEXT_SPLIT_TYPE_LINE: TextSplitTypeProto
TEXT_TRANSFORM_UNSPECIFIED: TextTransformProto
TEXT_TRANSFORM_NONE: TextTransformProto
TEXT_TRANSFORM_UPPERCASE: TextTransformProto
TEXT_TRANSFORM_LOWERCASE: TextTransformProto
TEXT_TRANSFORM_CAPITALIZE: TextTransformProto
THREAD_STATUS_UNSPECIFIED: ThreadStatusProto
THREAD_STATUS_OPEN: ThreadStatusProto
THREAD_STATUS_CLOSED: ThreadStatusProto
TIMER_TYPE_UNSPECIFIED: TimerTypeProto
TIMER_TYPE_ONCE: TimerTypeProto
TIMER_TYPE_RECURRING: TimerTypeProto
TOOL_TYPE_UNSPECIFIED: ToolTypeProto
TOOL_TYPE_SELECT: ToolTypeProto
TOOL_TYPE_DRAG: ToolTypeProto
TOOL_TYPE_INSPECT: ToolTypeProto
TOOL_TYPE_ANNOTATE: ToolTypeProto
TRAIT_TYPE_UNSPECIFIED: TraitTypeProto
TRAIT_TYPE_GLOBAL: TraitTypeProto
TRAIT_TYPE_SPATIAL: TraitTypeProto
TRAIT_TYPE_ORDERED: TraitTypeProto
TRAIT_TYPE_ARCHIVABLE: TraitTypeProto
TRAIT_TYPE_DELETABLE: TraitTypeProto
TRAIT_TYPE_CUSTOMIZABLE: TraitTypeProto
TRAIT_TYPE_EXTENSIBLE: TraitTypeProto
TRAIT_TYPE_IRREVERSIBLE: TraitTypeProto
TRAIT_TYPE_TAGGABLE: TraitTypeProto
TRAIT_TYPE_OWNABLE: TraitTypeProto
TRAIT_TYPE_OWNER: TraitTypeProto
TRAIT_TYPE_JOINABLE: TraitTypeProto
TRAIT_TYPE_SUBJECT: TraitTypeProto
TRAIT_TYPE_RUNNABLE: TraitTypeProto
TRAIT_TYPE_SCRIPTABLE: TraitTypeProto
TRAIT_TYPE_SOURCEABLE: TraitTypeProto
TRAIT_TYPE_STARABLE: TraitTypeProto
TRAIT_TYPE_REACTABLE: TraitTypeProto
TRAIT_TYPE_FOLLOWABLE: TraitTypeProto
TRANSITION_TYPE_UNSPECIFIED: TransitionTypeProto
TRANSITION_TYPE_TWEEN: TransitionTypeProto
TRANSITION_TYPE_SPRING: TransitionTypeProto
TRIGGER_TYPE_UNSPECIFIED: TriggerTypeProto
TRIGGER_TYPE_EVENT: TriggerTypeProto
TYPE_CARDINALITY_UNSPECIFIED: TypeCardinalityProto
TYPE_CARDINALITY_SCALAR: TypeCardinalityProto
TYPE_CARDINALITY_LIST: TypeCardinalityProto
TYPE_CARDINALITY_MAP: TypeCardinalityProto
UNIVERSE_CATEGORY_UNSPECIFIED: UniverseCategoryProto
UNIVERSE_CATEGORY_META: UniverseCategoryProto
UNIVERSE_CATEGORY_UNIVERSE: UniverseCategoryProto
UNIVERSE_CATEGORY_SPACE: UniverseCategoryProto
UNIVERSE_CATEGORY_ACCESS: UniverseCategoryProto
UNIVERSE_CATEGORY_DATA: UniverseCategoryProto
UNIVERSE_CATEGORY_LOGIC: UniverseCategoryProto
UNIVERSE_CATEGORY_INTELLIGENCE: UniverseCategoryProto
UNIVERSE_CATEGORY_INFRASTRUCTURE: UniverseCategoryProto
UNIVERSE_CATEGORY_DEPLOYMENT: UniverseCategoryProto
UNIVERSE_CATEGORY_OBSERVABILITY: UniverseCategoryProto
UNIVERSE_CATEGORY_OPTIMIZATION: UniverseCategoryProto
UNIVERSE_CATEGORY_SOCIAL: UniverseCategoryProto
UNIVERSE_CATEGORY_FINANCE: UniverseCategoryProto
UNIVERSE_CATEGORY_SCENE: UniverseCategoryProto
UNIVERSE_CATEGORY_VIEW: UniverseCategoryProto
UNIVERSE_CATEGORY_CANVAS: UniverseCategoryProto
UNIVERSE_CATEGORY_INTERACTION: UniverseCategoryProto
UNIVERSE_CATEGORY_ANIMATION: UniverseCategoryProto
UNIVERSE_CATEGORY_STYLE: UniverseCategoryProto
USER_STATUS_UNSPECIFIED: UserStatusProto
USER_STATUS_CREATING: UserStatusProto
USER_STATUS_ACTIVE: UserStatusProto
VALUE_FACTORY_UNSPECIFIED: ValueFactoryProto
VALUE_FACTORY_UUID: ValueFactoryProto
VALUE_FACTORY_NOW: ValueFactoryProto
VALUE_FACTORY_REGION: ValueFactoryProto
VALUE_FACTORY_SELF: ValueFactoryProto
VARIANT_STATE_TYPE_UNSPECIFIED: VariantStateTypeProto
VARIANT_STATE_TYPE_LOADING: VariantStateTypeProto
VARIANT_STATE_TYPE_ERROR: VariantStateTypeProto
VARIANT_TYPE_UNSPECIFIED: VariantTypeProto
VARIANT_TYPE_DYNAMIC: VariantTypeProto
VARIANT_TYPE_BREAKPOINT: VariantTypeProto
VARIANT_TYPE_PLATFORM: VariantTypeProto
WINDOW_TYPE_UNSPECIFIED: WindowTypeProto
WINDOW_TYPE_BROWSER: WindowTypeProto
WINDOW_TYPE_DESKTOP: WindowTypeProto
WINDOW_TYPE_MOBILE: WindowTypeProto

class ActionProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "source_ptr", "name", "icon", "text", "cardinality")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    CARDINALITY_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    source_ptr: NodeReferenceProto
    name: str
    icon: IconProto
    text: TextProto
    cardinality: MethodCardinalityProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., source_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ..., text: _Optional[_Union[TextProto, _Mapping]] = ..., cardinality: _Optional[_Union[MethodCardinalityProto, str]] = ...) -> None: ...

class ActionDefinitionProto(_message.Message):
    __slots__ = ("metatype", "id", "name", "icon", "description", "properties")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    id: int
    name: str
    icon: IconProto
    description: str
    properties: _containers.RepeatedCompositeFieldContainer[PropertyDefinitionProto]
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., id: _Optional[int] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ..., description: _Optional[str] = ..., properties: _Optional[_Iterable[_Union[PropertyDefinitionProto, _Mapping]]] = ...) -> None: ...

class AgentProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "name", "slug", "cursor_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    CURSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    name: str
    slug: str
    cursor_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., name: _Optional[str] = ..., slug: _Optional[str] = ..., cursor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class AggregationProto(_message.Message):
    __slots__ = ("metatype", "type", "expression")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    EXPRESSION_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: AggregationTypeProto
    expression: ExpressionProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[AggregationTypeProto, str]] = ..., expression: _Optional[_Union[ExpressionProto, _Mapping]] = ...) -> None: ...

class AnnotationShapeProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "base_type", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "script_ptr", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "layout", "direction", "distribute", "align", "gap", "padding", "grid", "grid_span", "aspect_ratio", "is_wrap", "is_visible", "opacity", "fill", "rotation", "skew", "scale", "shadow", "border", "radius", "stroke", "text")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
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
    STROKE_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    script_ptr: NodeReferenceProto
    name: str
    position: PositionProto
    width: DimensionProto
    height: DimensionProto
    min_width: DimensionProto
    min_height: DimensionProto
    max_width: DimensionProto
    max_height: DimensionProto
    layout: LayoutProto
    direction: DirectionProto
    distribute: DistributeProto
    align: AlignProto
    gap: Axis2Proto
    padding: InsetsProto
    grid: GridProto
    grid_span: GridSpanProto
    aspect_ratio: float
    is_wrap: bool
    is_visible: bool
    opacity: float
    fill: FillProto
    rotation: Axis3Proto
    skew: Vector2fProto
    scale: float
    shadow: ShadowProto
    border: BorderProto
    radius: CornersProto
    stroke: StrokeProto
    text: TextProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionProto, _Mapping]] = ..., width: _Optional[_Union[DimensionProto, _Mapping]] = ..., height: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., layout: _Optional[_Union[LayoutProto, str]] = ..., direction: _Optional[_Union[DirectionProto, str]] = ..., distribute: _Optional[_Union[DistributeProto, str]] = ..., align: _Optional[_Union[AlignProto, str]] = ..., gap: _Optional[_Union[Axis2Proto, _Mapping]] = ..., padding: _Optional[_Union[InsetsProto, _Mapping]] = ..., grid: _Optional[_Union[GridProto, _Mapping]] = ..., grid_span: _Optional[_Union[GridSpanProto, _Mapping]] = ..., aspect_ratio: _Optional[float] = ..., is_wrap: bool = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., fill: _Optional[_Union[FillProto, _Mapping]] = ..., rotation: _Optional[_Union[Axis3Proto, _Mapping]] = ..., skew: _Optional[_Union[Vector2fProto, _Mapping]] = ..., scale: _Optional[float] = ..., shadow: _Optional[_Union[ShadowProto, _Mapping]] = ..., border: _Optional[_Union[BorderProto, _Mapping]] = ..., radius: _Optional[_Union[CornersProto, _Mapping]] = ..., stroke: _Optional[_Union[StrokeProto, _Mapping]] = ..., text: _Optional[_Union[TextProto, _Mapping]] = ...) -> None: ...

class ArrowProto(_message.Message):
    __slots__ = ("metatype", "start_type", "start", "end_type", "end")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    START_TYPE_FIELD_NUMBER: _ClassVar[int]
    START_FIELD_NUMBER: _ClassVar[int]
    END_TYPE_FIELD_NUMBER: _ClassVar[int]
    END_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    start_type: ArrowHeadTypeProto
    start: Vector2fProto
    end_type: ArrowHeadTypeProto
    end: Vector2fProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., start_type: _Optional[_Union[ArrowHeadTypeProto, str]] = ..., start: _Optional[_Union[Vector2fProto, _Mapping]] = ..., end_type: _Optional[_Union[ArrowHeadTypeProto, str]] = ..., end: _Optional[_Union[Vector2fProto, _Mapping]] = ...) -> None: ...

class ArrowShapeProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "base_type", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "script_ptr", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "layout", "direction", "distribute", "align", "gap", "padding", "grid", "grid_span", "aspect_ratio", "is_wrap", "is_visible", "opacity", "fill", "rotation", "skew", "scale", "shadow", "border", "radius", "stroke", "start_type", "start", "end_type", "end")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
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
    STROKE_FIELD_NUMBER: _ClassVar[int]
    START_TYPE_FIELD_NUMBER: _ClassVar[int]
    START_FIELD_NUMBER: _ClassVar[int]
    END_TYPE_FIELD_NUMBER: _ClassVar[int]
    END_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    script_ptr: NodeReferenceProto
    name: str
    position: PositionProto
    width: DimensionProto
    height: DimensionProto
    min_width: DimensionProto
    min_height: DimensionProto
    max_width: DimensionProto
    max_height: DimensionProto
    layout: LayoutProto
    direction: DirectionProto
    distribute: DistributeProto
    align: AlignProto
    gap: Axis2Proto
    padding: InsetsProto
    grid: GridProto
    grid_span: GridSpanProto
    aspect_ratio: float
    is_wrap: bool
    is_visible: bool
    opacity: float
    fill: FillProto
    rotation: Axis3Proto
    skew: Vector2fProto
    scale: float
    shadow: ShadowProto
    border: BorderProto
    radius: CornersProto
    stroke: StrokeProto
    start_type: ArrowHeadTypeProto
    start: Vector2fProto
    end_type: ArrowHeadTypeProto
    end: Vector2fProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionProto, _Mapping]] = ..., width: _Optional[_Union[DimensionProto, _Mapping]] = ..., height: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., layout: _Optional[_Union[LayoutProto, str]] = ..., direction: _Optional[_Union[DirectionProto, str]] = ..., distribute: _Optional[_Union[DistributeProto, str]] = ..., align: _Optional[_Union[AlignProto, str]] = ..., gap: _Optional[_Union[Axis2Proto, _Mapping]] = ..., padding: _Optional[_Union[InsetsProto, _Mapping]] = ..., grid: _Optional[_Union[GridProto, _Mapping]] = ..., grid_span: _Optional[_Union[GridSpanProto, _Mapping]] = ..., aspect_ratio: _Optional[float] = ..., is_wrap: bool = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., fill: _Optional[_Union[FillProto, _Mapping]] = ..., rotation: _Optional[_Union[Axis3Proto, _Mapping]] = ..., skew: _Optional[_Union[Vector2fProto, _Mapping]] = ..., scale: _Optional[float] = ..., shadow: _Optional[_Union[ShadowProto, _Mapping]] = ..., border: _Optional[_Union[BorderProto, _Mapping]] = ..., radius: _Optional[_Union[CornersProto, _Mapping]] = ..., stroke: _Optional[_Union[StrokeProto, _Mapping]] = ..., start_type: _Optional[_Union[ArrowHeadTypeProto, str]] = ..., start: _Optional[_Union[Vector2fProto, _Mapping]] = ..., end_type: _Optional[_Union[ArrowHeadTypeProto, str]] = ..., end: _Optional[_Union[Vector2fProto, _Mapping]] = ...) -> None: ...

class Axis2Proto(_message.Message):
    __slots__ = ("metatype", "base", "x", "y")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    BASE_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    base: float
    x: float
    y: float
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., base: _Optional[float] = ..., x: _Optional[float] = ..., y: _Optional[float] = ...) -> None: ...

class Axis3Proto(_message.Message):
    __slots__ = ("metatype", "base", "x", "y", "z")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    BASE_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    Z_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    base: float
    x: float
    y: float
    z: float
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., base: _Optional[float] = ..., x: _Optional[float] = ..., y: _Optional[float] = ..., z: _Optional[float] = ...) -> None: ...

class BorderProto(_message.Message):
    __slots__ = ("metatype", "type", "color", "width", "style_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    STYLE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: BorderTypeProto
    color: ColorProto
    width: InsetsProto
    style_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[BorderTypeProto, str]] = ..., color: _Optional[_Union[ColorProto, _Mapping]] = ..., width: _Optional[_Union[InsetsProto, _Mapping]] = ..., style_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class BorderStyleProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "order_key", "type", "name", "color", "width", "style_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    STYLE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    order_key: str
    type: BorderTypeProto
    name: str
    color: ColorProto
    width: InsetsProto
    style_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[BorderTypeProto, str]] = ..., name: _Optional[str] = ..., color: _Optional[_Union[ColorProto, _Mapping]] = ..., width: _Optional[_Union[InsetsProto, _Mapping]] = ..., style_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class BranchProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "owned_by_ptr", "name", "icon", "head_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    HEAD_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    owned_by_ptr: NodeReferenceProto
    name: str
    icon: IconProto
    head_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ..., head_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class BuiltinDefinitionProto(_message.Message):
    __slots__ = ("metatype", "id", "name", "icon", "description")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    id: int
    name: str
    icon: IconProto
    description: str
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., id: _Optional[int] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ..., description: _Optional[str] = ...) -> None: ...

class CanvasProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "base_type", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "script_ptr", "type", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "layout", "direction", "distribute", "align", "gap", "padding", "grid", "grid_span", "aspect_ratio", "is_wrap", "is_visible", "opacity", "fill", "rotation", "skew", "scale", "shadow", "border", "radius")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
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
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    script_ptr: NodeReferenceProto
    type: CanvasTypeProto
    name: str
    position: PositionProto
    width: DimensionProto
    height: DimensionProto
    min_width: DimensionProto
    min_height: DimensionProto
    max_width: DimensionProto
    max_height: DimensionProto
    layout: LayoutProto
    direction: DirectionProto
    distribute: DistributeProto
    align: AlignProto
    gap: Axis2Proto
    padding: InsetsProto
    grid: GridProto
    grid_span: GridSpanProto
    aspect_ratio: float
    is_wrap: bool
    is_visible: bool
    opacity: float
    fill: FillProto
    rotation: Axis3Proto
    skew: Vector2fProto
    scale: float
    shadow: ShadowProto
    border: BorderProto
    radius: CornersProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., type: _Optional[_Union[CanvasTypeProto, str]] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionProto, _Mapping]] = ..., width: _Optional[_Union[DimensionProto, _Mapping]] = ..., height: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., layout: _Optional[_Union[LayoutProto, str]] = ..., direction: _Optional[_Union[DirectionProto, str]] = ..., distribute: _Optional[_Union[DistributeProto, str]] = ..., align: _Optional[_Union[AlignProto, str]] = ..., gap: _Optional[_Union[Axis2Proto, _Mapping]] = ..., padding: _Optional[_Union[InsetsProto, _Mapping]] = ..., grid: _Optional[_Union[GridProto, _Mapping]] = ..., grid_span: _Optional[_Union[GridSpanProto, _Mapping]] = ..., aspect_ratio: _Optional[float] = ..., is_wrap: bool = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., fill: _Optional[_Union[FillProto, _Mapping]] = ..., rotation: _Optional[_Union[Axis3Proto, _Mapping]] = ..., skew: _Optional[_Union[Vector2fProto, _Mapping]] = ..., scale: _Optional[float] = ..., shadow: _Optional[_Union[ShadowProto, _Mapping]] = ..., border: _Optional[_Union[BorderProto, _Mapping]] = ..., radius: _Optional[_Union[CornersProto, _Mapping]] = ...) -> None: ...

class ChangeProto(_message.Message):
    __slots__ = ("metatype", "id", "name", "created_at", "created_by_ptr", "origin", "debounce", "edits")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ORIGIN_FIELD_NUMBER: _ClassVar[int]
    DEBOUNCE_FIELD_NUMBER: _ClassVar[int]
    EDITS_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    id: str
    name: str
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    origin: OriginProto
    debounce: ChangeDebounceProto
    edits: _containers.RepeatedCompositeFieldContainer[EditProto]
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., id: _Optional[str] = ..., name: _Optional[str] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., origin: _Optional[_Union[OriginProto, _Mapping]] = ..., debounce: _Optional[_Union[ChangeDebounceProto, str]] = ..., edits: _Optional[_Iterable[_Union[EditProto, _Mapping]]] = ...) -> None: ...

class ChangeEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "name", "origin", "debounce", "edits_ids", "nodes_ids")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ORIGIN_FIELD_NUMBER: _ClassVar[int]
    DEBOUNCE_FIELD_NUMBER: _ClassVar[int]
    EDITS_IDS_FIELD_NUMBER: _ClassVar[int]
    NODES_IDS_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    name: str
    origin: OriginProto
    debounce: ChangeDebounceProto
    edits_ids: _containers.RepeatedScalarFieldContainer[str]
    nodes_ids: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., origin: _Optional[_Union[OriginProto, _Mapping]] = ..., debounce: _Optional[_Union[ChangeDebounceProto, str]] = ..., edits_ids: _Optional[_Iterable[str]] = ..., nodes_ids: _Optional[_Iterable[str]] = ...) -> None: ...

class ChangeResultProto(_message.Message):
    __slots__ = ("metatype", "id", "created_at", "debounce", "status", "edits", "cascaded_edits")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    DEBOUNCE_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    EDITS_FIELD_NUMBER: _ClassVar[int]
    CASCADED_EDITS_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    id: str
    created_at: _timestamp_pb2.Timestamp
    debounce: ChangeDebounceProto
    status: ChangeStatusProto
    edits: _containers.RepeatedCompositeFieldContainer[EditProto]
    cascaded_edits: _containers.RepeatedCompositeFieldContainer[EditProto]
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., id: _Optional[str] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., debounce: _Optional[_Union[ChangeDebounceProto, str]] = ..., status: _Optional[_Union[ChangeStatusProto, str]] = ..., edits: _Optional[_Iterable[_Union[EditProto, _Mapping]]] = ..., cascaded_edits: _Optional[_Iterable[_Union[EditProto, _Mapping]]] = ...) -> None: ...

class ClickEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "position", "pressure", "shift_key", "alt_key", "ctrl_key", "meta_key", "accel_key", "button")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    PRESSURE_FIELD_NUMBER: _ClassVar[int]
    SHIFT_KEY_FIELD_NUMBER: _ClassVar[int]
    ALT_KEY_FIELD_NUMBER: _ClassVar[int]
    CTRL_KEY_FIELD_NUMBER: _ClassVar[int]
    META_KEY_FIELD_NUMBER: _ClassVar[int]
    ACCEL_KEY_FIELD_NUMBER: _ClassVar[int]
    BUTTON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    position: Vector2fProto
    pressure: float
    shift_key: bool
    alt_key: bool
    ctrl_key: bool
    meta_key: bool
    accel_key: bool
    button: MouseButtonProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., position: _Optional[_Union[Vector2fProto, _Mapping]] = ..., pressure: _Optional[float] = ..., shift_key: bool = ..., alt_key: bool = ..., ctrl_key: bool = ..., meta_key: bool = ..., accel_key: bool = ..., button: _Optional[_Union[MouseButtonProto, str]] = ...) -> None: ...

class ClientProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "browser_version", "type", "name", "machine_ptr", "user_ptr", "access_token", "seen_at", "logged_in_at", "cursor_ptr", "device_type", "device_name", "operating_system", "browser_name")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    BROWSER_VERSION_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    MACHINE_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_PTR_FIELD_NUMBER: _ClassVar[int]
    ACCESS_TOKEN_FIELD_NUMBER: _ClassVar[int]
    SEEN_AT_FIELD_NUMBER: _ClassVar[int]
    LOGGED_IN_AT_FIELD_NUMBER: _ClassVar[int]
    CURSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    DEVICE_TYPE_FIELD_NUMBER: _ClassVar[int]
    DEVICE_NAME_FIELD_NUMBER: _ClassVar[int]
    OPERATING_SYSTEM_FIELD_NUMBER: _ClassVar[int]
    BROWSER_NAME_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    browser_version: str
    type: ClientTypeProto
    name: str
    machine_ptr: NodeReferenceProto
    user_ptr: NodeReferenceProto
    access_token: str
    seen_at: _timestamp_pb2.Timestamp
    logged_in_at: _timestamp_pb2.Timestamp
    cursor_ptr: NodeReferenceProto
    device_type: str
    device_name: str
    operating_system: str
    browser_name: str
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., browser_version: _Optional[str] = ..., type: _Optional[_Union[ClientTypeProto, str]] = ..., name: _Optional[str] = ..., machine_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., user_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., access_token: _Optional[str] = ..., seen_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., logged_in_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., cursor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., device_type: _Optional[str] = ..., device_name: _Optional[str] = ..., operating_system: _Optional[str] = ..., browser_name: _Optional[str] = ...) -> None: ...

class ClipboardEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class CollectionConstraintProto(_message.Message):
    __slots__ = ("metatype", "min_length", "max_length")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    MIN_LENGTH_FIELD_NUMBER: _ClassVar[int]
    MAX_LENGTH_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    min_length: int
    max_length: int
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., min_length: _Optional[int] = ..., max_length: _Optional[int] = ...) -> None: ...

class ColorProto(_message.Message):
    __slots__ = ("metatype", "type", "style_ptr", "hue", "shade", "intent", "x", "y", "z", "alpha")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    STYLE_PTR_FIELD_NUMBER: _ClassVar[int]
    HUE_FIELD_NUMBER: _ClassVar[int]
    SHADE_FIELD_NUMBER: _ClassVar[int]
    INTENT_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    Z_FIELD_NUMBER: _ClassVar[int]
    ALPHA_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: ColorTypeProto
    style_ptr: NodeReferenceProto
    hue: ColorHueProto
    shade: ColorShadeProto
    intent: ColorIntentProto
    x: float
    y: float
    z: float
    alpha: float
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[ColorTypeProto, str]] = ..., style_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., hue: _Optional[_Union[ColorHueProto, str]] = ..., shade: _Optional[_Union[ColorShadeProto, str]] = ..., intent: _Optional[_Union[ColorIntentProto, str]] = ..., x: _Optional[float] = ..., y: _Optional[float] = ..., z: _Optional[float] = ..., alpha: _Optional[float] = ...) -> None: ...

class ColorStyleProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "order_key", "type", "name", "hue", "shade", "intent", "x", "y", "z", "alpha", "dark")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    HUE_FIELD_NUMBER: _ClassVar[int]
    SHADE_FIELD_NUMBER: _ClassVar[int]
    INTENT_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    Z_FIELD_NUMBER: _ClassVar[int]
    ALPHA_FIELD_NUMBER: _ClassVar[int]
    DARK_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    order_key: str
    type: ColorTypeProto
    name: str
    hue: ColorHueProto
    shade: ColorShadeProto
    intent: ColorIntentProto
    x: float
    y: float
    z: float
    alpha: float
    dark: ColorProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[ColorTypeProto, str]] = ..., name: _Optional[str] = ..., hue: _Optional[_Union[ColorHueProto, str]] = ..., shade: _Optional[_Union[ColorShadeProto, str]] = ..., intent: _Optional[_Union[ColorIntentProto, str]] = ..., x: _Optional[float] = ..., y: _Optional[float] = ..., z: _Optional[float] = ..., alpha: _Optional[float] = ..., dark: _Optional[_Union[ColorProto, _Mapping]] = ...) -> None: ...

class ConditionProto(_message.Message):
    __slots__ = ("metatype", "type", "left", "right")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    LEFT_FIELD_NUMBER: _ClassVar[int]
    RIGHT_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: ConditionalTypeProto
    left: ExpressionProto
    right: ExpressionProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[ConditionalTypeProto, str]] = ..., left: _Optional[_Union[ExpressionProto, _Mapping]] = ..., right: _Optional[_Union[ExpressionProto, _Mapping]] = ...) -> None: ...

class ConstantDefinitionProto(_message.Message):
    __slots__ = ("metatype", "name", "description", "value", "is_deferred")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    IS_DEFERRED_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    name: str
    description: str
    value: ValueProto
    is_deferred: bool
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., name: _Optional[str] = ..., description: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ..., is_deferred: bool = ...) -> None: ...

class ContainerViewProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "base_type", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "script_ptr", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "layout", "direction", "distribute", "align", "gap", "padding", "grid", "grid_span", "aspect_ratio", "is_wrap", "is_visible", "opacity", "fill", "rotation", "skew", "scale", "shadow", "border", "radius")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
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
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    script_ptr: NodeReferenceProto
    name: str
    position: PositionProto
    width: DimensionProto
    height: DimensionProto
    min_width: DimensionProto
    min_height: DimensionProto
    max_width: DimensionProto
    max_height: DimensionProto
    layout: LayoutProto
    direction: DirectionProto
    distribute: DistributeProto
    align: AlignProto
    gap: Axis2Proto
    padding: InsetsProto
    grid: GridProto
    grid_span: GridSpanProto
    aspect_ratio: float
    is_wrap: bool
    is_visible: bool
    opacity: float
    fill: FillProto
    rotation: Axis3Proto
    skew: Vector2fProto
    scale: float
    shadow: ShadowProto
    border: BorderProto
    radius: CornersProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionProto, _Mapping]] = ..., width: _Optional[_Union[DimensionProto, _Mapping]] = ..., height: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., layout: _Optional[_Union[LayoutProto, str]] = ..., direction: _Optional[_Union[DirectionProto, str]] = ..., distribute: _Optional[_Union[DistributeProto, str]] = ..., align: _Optional[_Union[AlignProto, str]] = ..., gap: _Optional[_Union[Axis2Proto, _Mapping]] = ..., padding: _Optional[_Union[InsetsProto, _Mapping]] = ..., grid: _Optional[_Union[GridProto, _Mapping]] = ..., grid_span: _Optional[_Union[GridSpanProto, _Mapping]] = ..., aspect_ratio: _Optional[float] = ..., is_wrap: bool = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., fill: _Optional[_Union[FillProto, _Mapping]] = ..., rotation: _Optional[_Union[Axis3Proto, _Mapping]] = ..., skew: _Optional[_Union[Vector2fProto, _Mapping]] = ..., scale: _Optional[float] = ..., shadow: _Optional[_Union[ShadowProto, _Mapping]] = ..., border: _Optional[_Union[BorderProto, _Mapping]] = ..., radius: _Optional[_Union[CornersProto, _Mapping]] = ...) -> None: ...

class ContentViewProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "base_type", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "script_ptr", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "align", "is_visible", "opacity")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
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
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    script_ptr: NodeReferenceProto
    name: str
    position: PositionProto
    width: DimensionProto
    height: DimensionProto
    min_width: DimensionProto
    min_height: DimensionProto
    max_width: DimensionProto
    max_height: DimensionProto
    align: AlignProto
    is_visible: bool
    opacity: float
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionProto, _Mapping]] = ..., width: _Optional[_Union[DimensionProto, _Mapping]] = ..., height: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., align: _Optional[_Union[AlignProto, str]] = ..., is_visible: bool = ..., opacity: _Optional[float] = ...) -> None: ...

class CopyEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class CornersProto(_message.Message):
    __slots__ = ("metatype", "base", "top_left", "top_right", "bottom_left", "bottom_right")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    BASE_FIELD_NUMBER: _ClassVar[int]
    TOP_LEFT_FIELD_NUMBER: _ClassVar[int]
    TOP_RIGHT_FIELD_NUMBER: _ClassVar[int]
    BOTTOM_LEFT_FIELD_NUMBER: _ClassVar[int]
    BOTTOM_RIGHT_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    base: int
    top_left: int
    top_right: int
    bottom_left: int
    bottom_right: int
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., base: _Optional[int] = ..., top_left: _Optional[int] = ..., top_right: _Optional[int] = ..., bottom_left: _Optional[int] = ..., bottom_right: _Optional[int] = ...) -> None: ...

class CounterMeasurementEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class CounterMetricProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "order_key", "source_ptr", "name", "icon")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    order_key: str
    source_ptr: NodeReferenceProto
    name: str
    icon: IconProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., order_key: _Optional[str] = ..., source_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ...) -> None: ...

class CursorProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "owned_by_ptr", "status", "active_at")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_AT_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    owned_by_ptr: NodeReferenceProto
    status: CursorStatusProto
    active_at: _timestamp_pb2.Timestamp
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., status: _Optional[_Union[CursorStatusProto, str]] = ..., active_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...) -> None: ...

class CustomEntityDefinitionProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "owned_by_ptr", "base_type", "base_traits", "is_abstract", "prototype_ptr", "source_ptr", "script_ptr", "name", "icon")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    BASE_TRAITS_FIELD_NUMBER: _ClassVar[int]
    IS_ABSTRACT_FIELD_NUMBER: _ClassVar[int]
    PROTOTYPE_PTR_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    owned_by_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    base_traits: _containers.RepeatedCompositeFieldContainer[NodeDefinitionReferenceProto]
    is_abstract: bool
    prototype_ptr: NodeReferenceProto
    source_ptr: NodeReferenceProto
    script_ptr: NodeReferenceProto
    name: str
    icon: IconProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., base_traits: _Optional[_Iterable[_Union[NodeDefinitionReferenceProto, _Mapping]]] = ..., is_abstract: bool = ..., prototype_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., source_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ...) -> None: ...

class CustomEnumDefinitionProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "source_ptr", "name", "icon")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    source_ptr: NodeReferenceProto
    name: str
    icon: IconProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., source_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ...) -> None: ...

class CustomEventDefinitionProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "custom_values", "order_key", "base_type", "base_traits", "is_abstract", "source_ptr", "name", "icon")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    BASE_TRAITS_FIELD_NUMBER: _ClassVar[int]
    IS_ABSTRACT_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    base_type: NodeDefinitionReferenceProto
    base_traits: _containers.RepeatedCompositeFieldContainer[NodeDefinitionReferenceProto]
    is_abstract: bool
    source_ptr: NodeReferenceProto
    name: str
    icon: IconProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., base_traits: _Optional[_Iterable[_Union[NodeDefinitionReferenceProto, _Mapping]]] = ..., is_abstract: bool = ..., source_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ...) -> None: ...

class CustomOptionProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "order_key", "source_ptr", "name", "icon", "group_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    GROUP_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    order_key: str
    source_ptr: NodeReferenceProto
    name: str
    icon: IconProto
    group_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., order_key: _Optional[str] = ..., source_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ..., group_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class CustomOptionGroupProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "order_key", "source_ptr", "name", "icon")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    order_key: str
    source_ptr: NodeReferenceProto
    name: str
    icon: IconProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., order_key: _Optional[str] = ..., source_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ...) -> None: ...

class CustomPropertyProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "order_key", "source_ptr", "type", "name", "icon", "group_ptr", "cardinality", "scalar_type", "primitive_type", "enum_type", "node_type", "struct_type", "definition_ptr", "key_type", "value", "value_factory", "collection_constraint", "string_constraint", "number_constraint", "node_constraint", "edge_type", "cascade", "is_required", "is_unique", "is_computed", "is_readonly")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    GROUP_PTR_FIELD_NUMBER: _ClassVar[int]
    CARDINALITY_FIELD_NUMBER: _ClassVar[int]
    SCALAR_TYPE_FIELD_NUMBER: _ClassVar[int]
    PRIMITIVE_TYPE_FIELD_NUMBER: _ClassVar[int]
    ENUM_TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    STRUCT_TYPE_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    KEY_TYPE_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    VALUE_FACTORY_FIELD_NUMBER: _ClassVar[int]
    COLLECTION_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    STRING_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    NUMBER_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    NODE_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    EDGE_TYPE_FIELD_NUMBER: _ClassVar[int]
    CASCADE_FIELD_NUMBER: _ClassVar[int]
    IS_REQUIRED_FIELD_NUMBER: _ClassVar[int]
    IS_UNIQUE_FIELD_NUMBER: _ClassVar[int]
    IS_COMPUTED_FIELD_NUMBER: _ClassVar[int]
    IS_READONLY_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    order_key: str
    source_ptr: NodeReferenceProto
    type: PropertyTypeProto
    name: str
    icon: IconProto
    group_ptr: NodeReferenceProto
    cardinality: TypeCardinalityProto
    scalar_type: ScalarTypeProto
    primitive_type: PrimitiveTypeProto
    enum_type: EnumTypeProto
    node_type: NodeTypeProto
    struct_type: StructTypeProto
    definition_ptr: NodeReferenceProto
    key_type: TypeProto
    value: ValueProto
    value_factory: ValueFactoryProto
    collection_constraint: CollectionConstraintProto
    string_constraint: StringConstraintProto
    number_constraint: NumberConstraintProto
    node_constraint: NodeConstraintProto
    edge_type: EdgeTypeProto
    cascade: CascadeActionProto
    is_required: bool
    is_unique: bool
    is_computed: bool
    is_readonly: bool
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., order_key: _Optional[str] = ..., source_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., type: _Optional[_Union[PropertyTypeProto, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ..., group_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., cardinality: _Optional[_Union[TypeCardinalityProto, str]] = ..., scalar_type: _Optional[_Union[ScalarTypeProto, str]] = ..., primitive_type: _Optional[_Union[PrimitiveTypeProto, str]] = ..., enum_type: _Optional[_Union[EnumTypeProto, str]] = ..., node_type: _Optional[_Union[NodeTypeProto, str]] = ..., struct_type: _Optional[_Union[StructTypeProto, str]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., key_type: _Optional[_Union[TypeProto, _Mapping]] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ..., value_factory: _Optional[_Union[ValueFactoryProto, str]] = ..., collection_constraint: _Optional[_Union[CollectionConstraintProto, _Mapping]] = ..., string_constraint: _Optional[_Union[StringConstraintProto, _Mapping]] = ..., number_constraint: _Optional[_Union[NumberConstraintProto, _Mapping]] = ..., node_constraint: _Optional[_Union[NodeConstraintProto, _Mapping]] = ..., edge_type: _Optional[_Union[EdgeTypeProto, str]] = ..., cascade: _Optional[_Union[CascadeActionProto, str]] = ..., is_required: bool = ..., is_unique: bool = ..., is_computed: bool = ..., is_readonly: bool = ...) -> None: ...

class CustomPropertyGroupProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "order_key", "source_ptr", "name", "icon")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    order_key: str
    source_ptr: NodeReferenceProto
    name: str
    icon: IconProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., order_key: _Optional[str] = ..., source_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ...) -> None: ...

class CustomStructProto(_message.Message):
    __slots__ = ("metatype", "definition_ptr", "custom_values")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    definition_ptr: NodeReferenceProto
    custom_values: _containers.MessageMap[str, ValueProto]
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ...) -> None: ...

class CustomStructDefinitionProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "prototype", "base_type", "is_frozen", "source_ptr", "name", "icon")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    PROTOTYPE_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    IS_FROZEN_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    prototype: CustomStructProto
    base_type: StructDefinitionReferenceProto
    is_frozen: bool
    source_ptr: NodeReferenceProto
    name: str
    icon: IconProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., prototype: _Optional[_Union[CustomStructProto, _Mapping]] = ..., base_type: _Optional[_Union[StructDefinitionReferenceProto, _Mapping]] = ..., is_frozen: bool = ..., source_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ...) -> None: ...

class CustomTraitDefinitionProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "base_type", "base_traits", "is_abstract", "source_ptr", "script_ptr", "name", "icon")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    BASE_TRAITS_FIELD_NUMBER: _ClassVar[int]
    IS_ABSTRACT_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    base_type: NodeDefinitionReferenceProto
    base_traits: _containers.RepeatedCompositeFieldContainer[NodeDefinitionReferenceProto]
    is_abstract: bool
    source_ptr: NodeReferenceProto
    script_ptr: NodeReferenceProto
    name: str
    icon: IconProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., base_traits: _Optional[_Iterable[_Union[NodeDefinitionReferenceProto, _Mapping]]] = ..., is_abstract: bool = ..., source_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ...) -> None: ...

class CutEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class DatabaseProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "base_type", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "script_ptr", "status", "type", "name", "icon", "region", "galaxy_name", "external_name", "custom_schema_name", "tenancy", "connection_url")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    GALAXY_NAME_FIELD_NUMBER: _ClassVar[int]
    EXTERNAL_NAME_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_SCHEMA_NAME_FIELD_NUMBER: _ClassVar[int]
    TENANCY_FIELD_NUMBER: _ClassVar[int]
    CONNECTION_URL_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    script_ptr: NodeReferenceProto
    status: ResourceStatusProto
    type: DatabaseTypeProto
    name: str
    icon: IconProto
    region: RegionProto
    galaxy_name: str
    external_name: str
    custom_schema_name: str
    tenancy: TenancyProto
    connection_url: str
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., status: _Optional[_Union[ResourceStatusProto, str]] = ..., type: _Optional[_Union[DatabaseTypeProto, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ..., region: _Optional[_Union[RegionProto, str]] = ..., galaxy_name: _Optional[str] = ..., external_name: _Optional[str] = ..., custom_schema_name: _Optional[str] = ..., tenancy: _Optional[_Union[TenancyProto, str]] = ..., connection_url: _Optional[str] = ...) -> None: ...

class DatabaseInfoProto(_message.Message):
    __slots__ = ("metatype", "type", "region", "galaxy_name", "external_name", "custom_schema_name", "tenancy", "connection_url")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    GALAXY_NAME_FIELD_NUMBER: _ClassVar[int]
    EXTERNAL_NAME_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_SCHEMA_NAME_FIELD_NUMBER: _ClassVar[int]
    TENANCY_FIELD_NUMBER: _ClassVar[int]
    CONNECTION_URL_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: DatabaseTypeProto
    region: RegionProto
    galaxy_name: str
    external_name: str
    custom_schema_name: str
    tenancy: TenancyProto
    connection_url: str
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[DatabaseTypeProto, str]] = ..., region: _Optional[_Union[RegionProto, str]] = ..., galaxy_name: _Optional[str] = ..., external_name: _Optional[str] = ..., custom_schema_name: _Optional[str] = ..., tenancy: _Optional[_Union[TenancyProto, str]] = ..., connection_url: _Optional[str] = ...) -> None: ...

class DimensionProto(_message.Message):
    __slots__ = ("metatype", "type", "unit", "value")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    UNIT_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: DimensionTypeProto
    unit: LengthUnitProto
    value: float
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[DimensionTypeProto, str]] = ..., unit: _Optional[_Union[LengthUnitProto, str]] = ..., value: _Optional[float] = ...) -> None: ...

class DoubleClickEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "position", "pressure", "shift_key", "alt_key", "ctrl_key", "meta_key", "accel_key", "button")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    PRESSURE_FIELD_NUMBER: _ClassVar[int]
    SHIFT_KEY_FIELD_NUMBER: _ClassVar[int]
    ALT_KEY_FIELD_NUMBER: _ClassVar[int]
    CTRL_KEY_FIELD_NUMBER: _ClassVar[int]
    META_KEY_FIELD_NUMBER: _ClassVar[int]
    ACCEL_KEY_FIELD_NUMBER: _ClassVar[int]
    BUTTON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    position: Vector2fProto
    pressure: float
    shift_key: bool
    alt_key: bool
    ctrl_key: bool
    meta_key: bool
    accel_key: bool
    button: MouseButtonProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., position: _Optional[_Union[Vector2fProto, _Mapping]] = ..., pressure: _Optional[float] = ..., shift_key: bool = ..., alt_key: bool = ..., ctrl_key: bool = ..., meta_key: bool = ..., accel_key: bool = ..., button: _Optional[_Union[MouseButtonProto, str]] = ...) -> None: ...

class DragEndEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "position")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    position: Vector2fProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., position: _Optional[_Union[Vector2fProto, _Mapping]] = ...) -> None: ...

class DragEnterEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "position")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    position: Vector2fProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., position: _Optional[_Union[Vector2fProto, _Mapping]] = ...) -> None: ...

class DragEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "position")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    position: Vector2fProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., position: _Optional[_Union[Vector2fProto, _Mapping]] = ...) -> None: ...

class DragLeaveEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "position")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    position: Vector2fProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., position: _Optional[_Union[Vector2fProto, _Mapping]] = ...) -> None: ...

class DragOverEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "position")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    position: Vector2fProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., position: _Optional[_Union[Vector2fProto, _Mapping]] = ...) -> None: ...

class DragStartEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "position")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    position: Vector2fProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., position: _Optional[_Union[Vector2fProto, _Mapping]] = ...) -> None: ...

class DropEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "position")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    position: Vector2fProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., position: _Optional[_Union[Vector2fProto, _Mapping]] = ...) -> None: ...

class EditProto(_message.Message):
    __slots__ = ("metatype", "id", "type", "operation", "node_ptr", "attribute", "key", "value", "undo", "snapshot_ptr", "ancestors")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    OPERATION_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    ATTRIBUTE_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    UNDO_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    ANCESTORS_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    id: str
    type: EditTypeProto
    operation: EditOperationProto
    node_ptr: NodeReferenceProto
    attribute: PropertyReferenceProto
    key: ValueProto
    value: ValueProto
    undo: EditProto
    snapshot_ptr: NodeReferenceProto
    ancestors: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., id: _Optional[str] = ..., type: _Optional[_Union[EditTypeProto, str]] = ..., operation: _Optional[_Union[EditOperationProto, str]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., attribute: _Optional[_Union[PropertyReferenceProto, _Mapping]] = ..., key: _Optional[_Union[ValueProto, _Mapping]] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ..., undo: _Optional[_Union[EditProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., ancestors: _Optional[_Iterable[str]] = ...) -> None: ...

class EditEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "type", "node_ptr", "operation", "attribute", "key", "key_unpacked", "value", "undo", "ancestors_ids")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    OPERATION_FIELD_NUMBER: _ClassVar[int]
    ATTRIBUTE_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    KEY_UNPACKED_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    UNDO_FIELD_NUMBER: _ClassVar[int]
    ANCESTORS_IDS_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    type: EditTypeProto
    node_ptr: NodeReferenceProto
    operation: EditOperationProto
    attribute: PropertyReferenceProto
    key: ValueProto
    key_unpacked: _struct_pb2.Value
    value: ValueProto
    undo: EditProto
    ancestors_ids: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., type: _Optional[_Union[EditTypeProto, str]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., operation: _Optional[_Union[EditOperationProto, str]] = ..., attribute: _Optional[_Union[PropertyReferenceProto, _Mapping]] = ..., key: _Optional[_Union[ValueProto, _Mapping]] = ..., key_unpacked: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ..., undo: _Optional[_Union[EditProto, _Mapping]] = ..., ancestors_ids: _Optional[_Iterable[str]] = ...) -> None: ...

class EffectProto(_message.Message):
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
    metatype: StructTypeProto
    type: EffectTypeProto
    style_ptr: NodeReferenceProto
    opacity: float
    offset: Vector2fProto
    scale: float
    rotate: Axis3Proto
    skew: Vector2fProto
    perspective: float
    delay: _duration_pb2.Duration
    duration: float
    threshold: float
    once: bool
    repeat: RepeatTypeProto
    split: TextSplitTypeProto
    offscreen: OffscreenBehaviorProto
    transition: TransitionProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[EffectTypeProto, str]] = ..., style_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., opacity: _Optional[float] = ..., offset: _Optional[_Union[Vector2fProto, _Mapping]] = ..., scale: _Optional[float] = ..., rotate: _Optional[_Union[Axis3Proto, _Mapping]] = ..., skew: _Optional[_Union[Vector2fProto, _Mapping]] = ..., perspective: _Optional[float] = ..., delay: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., duration: _Optional[float] = ..., threshold: _Optional[float] = ..., once: bool = ..., repeat: _Optional[_Union[RepeatTypeProto, str]] = ..., split: _Optional[_Union[TextSplitTypeProto, str]] = ..., offscreen: _Optional[_Union[OffscreenBehaviorProto, str]] = ..., transition: _Optional[_Union[TransitionProto, _Mapping]] = ...) -> None: ...

class EffectStyleProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "order_key", "type", "name", "opacity", "offset", "scale", "rotate", "skew", "perspective", "delay", "duration", "threshold", "once", "repeat", "split", "offscreen", "transition")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
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
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    order_key: str
    type: EffectTypeProto
    name: str
    opacity: float
    offset: Vector2fProto
    scale: float
    rotate: Axis3Proto
    skew: Vector2fProto
    perspective: float
    delay: _duration_pb2.Duration
    duration: float
    threshold: float
    once: bool
    repeat: RepeatTypeProto
    split: TextSplitTypeProto
    offscreen: OffscreenBehaviorProto
    transition: TransitionProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[EffectTypeProto, str]] = ..., name: _Optional[str] = ..., opacity: _Optional[float] = ..., offset: _Optional[_Union[Vector2fProto, _Mapping]] = ..., scale: _Optional[float] = ..., rotate: _Optional[_Union[Axis3Proto, _Mapping]] = ..., skew: _Optional[_Union[Vector2fProto, _Mapping]] = ..., perspective: _Optional[float] = ..., delay: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., duration: _Optional[float] = ..., threshold: _Optional[float] = ..., once: bool = ..., repeat: _Optional[_Union[RepeatTypeProto, str]] = ..., split: _Optional[_Union[TextSplitTypeProto, str]] = ..., offscreen: _Optional[_Union[OffscreenBehaviorProto, str]] = ..., transition: _Optional[_Union[TransitionProto, _Mapping]] = ...) -> None: ...

class EntitlementProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "type", "expires_at", "target_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    EXPIRES_AT_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    type: EntitlementTypeProto
    expires_at: _timestamp_pb2.Timestamp
    target_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., type: _Optional[_Union[EntitlementTypeProto, str]] = ..., expires_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class EntitlementEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "target_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    target_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class EntitlementExpiredEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "target_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    target_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class EntitlementGrantedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "target_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    target_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class EntitlementRequestedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "target_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    target_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class EntitlementRevokedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "target_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    target_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class EntityProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class EnumDefinitionProto(_message.Message):
    __slots__ = ("metatype", "id", "type", "name", "icon", "description", "options")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    OPTIONS_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    id: int
    type: EnumTypeProto
    name: str
    icon: IconProto
    description: str
    options: _containers.RepeatedCompositeFieldContainer[OptionDefinitionProto]
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., id: _Optional[int] = ..., type: _Optional[_Union[EnumTypeProto, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ..., description: _Optional[str] = ..., options: _Optional[_Iterable[_Union[OptionDefinitionProto, _Mapping]]] = ...) -> None: ...

class EnvironmentProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "name", "icon")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    name: str
    icon: IconProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ...) -> None: ...

class EventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class EventCursorProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "owned_by_ptr", "status", "active_at")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_AT_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    owned_by_ptr: NodeReferenceProto
    status: CursorStatusProto
    active_at: _timestamp_pb2.Timestamp
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., status: _Optional[_Union[CursorStatusProto, str]] = ..., active_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...) -> None: ...

class ExpressionProto(_message.Message):
    __slots__ = ("metatype", "type", "literal", "attribute", "condition", "function", "aggregation")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    LITERAL_FIELD_NUMBER: _ClassVar[int]
    ATTRIBUTE_FIELD_NUMBER: _ClassVar[int]
    CONDITION_FIELD_NUMBER: _ClassVar[int]
    FUNCTION_FIELD_NUMBER: _ClassVar[int]
    AGGREGATION_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: ExpressionTypeProto
    literal: ValueProto
    attribute: PropertyReferenceProto
    condition: ConditionProto
    function: FunctionProto
    aggregation: AggregationProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[ExpressionTypeProto, str]] = ..., literal: _Optional[_Union[ValueProto, _Mapping]] = ..., attribute: _Optional[_Union[PropertyReferenceProto, _Mapping]] = ..., condition: _Optional[_Union[ConditionProto, _Mapping]] = ..., function: _Optional[_Union[FunctionProto, _Mapping]] = ..., aggregation: _Optional[_Union[AggregationProto, _Mapping]] = ...) -> None: ...

class FileProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "base_type", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "script_ptr", "status", "type", "name", "source", "mime_type", "format", "size", "sha256", "width", "height", "aspect_ratio", "codec", "duration", "url", "content_url", "thumbnail_url", "favicon_url", "thumbnail_width", "thumbnail_height", "content")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
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
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    script_ptr: NodeReferenceProto
    status: ResourceStatusProto
    type: FileTypeProto
    name: str
    source: FileSourceProto
    mime_type: str
    format: FileFormatProto
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
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., status: _Optional[_Union[ResourceStatusProto, str]] = ..., type: _Optional[_Union[FileTypeProto, str]] = ..., name: _Optional[str] = ..., source: _Optional[_Union[FileSourceProto, str]] = ..., mime_type: _Optional[str] = ..., format: _Optional[_Union[FileFormatProto, str]] = ..., size: _Optional[int] = ..., sha256: _Optional[str] = ..., width: _Optional[int] = ..., height: _Optional[int] = ..., aspect_ratio: _Optional[float] = ..., codec: _Optional[str] = ..., duration: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., url: _Optional[str] = ..., content_url: _Optional[str] = ..., thumbnail_url: _Optional[str] = ..., favicon_url: _Optional[str] = ..., thumbnail_width: _Optional[int] = ..., thumbnail_height: _Optional[int] = ..., content: _Optional[bytes] = ...) -> None: ...

class FillProto(_message.Message):
    __slots__ = ("metatype", "type", "style_ptr", "color", "gradient", "image_ptr", "position", "size")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    STYLE_PTR_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    GRADIENT_FIELD_NUMBER: _ClassVar[int]
    IMAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    SIZE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: FillTypeProto
    style_ptr: NodeReferenceProto
    color: ColorProto
    gradient: GradientProto
    image_ptr: NodeReferenceProto
    position: FillPositionProto
    size: FillSizeProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[FillTypeProto, str]] = ..., style_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., color: _Optional[_Union[ColorProto, _Mapping]] = ..., gradient: _Optional[_Union[GradientProto, _Mapping]] = ..., image_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., position: _Optional[_Union[FillPositionProto, str]] = ..., size: _Optional[_Union[FillSizeProto, str]] = ...) -> None: ...

class FillStyleProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "order_key", "type", "name", "color", "gradient", "image_ptr", "position", "size")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    GRADIENT_FIELD_NUMBER: _ClassVar[int]
    IMAGE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    SIZE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    order_key: str
    type: FillTypeProto
    name: str
    color: ColorProto
    gradient: GradientProto
    image_ptr: NodeReferenceProto
    position: FillPositionProto
    size: FillSizeProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[FillTypeProto, str]] = ..., name: _Optional[str] = ..., color: _Optional[_Union[ColorProto, _Mapping]] = ..., gradient: _Optional[_Union[GradientProto, _Mapping]] = ..., image_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., position: _Optional[_Union[FillPositionProto, str]] = ..., size: _Optional[_Union[FillSizeProto, str]] = ...) -> None: ...

class FocusEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class FocusInEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class FocusOutEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class FolderProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "order_key", "owned_by_ptr", "type", "name", "icon", "slug", "main_scene_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    MAIN_SCENE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    order_key: str
    owned_by_ptr: NodeReferenceProto
    type: FolderTypeProto
    name: str
    icon: IconProto
    slug: str
    main_scene_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., order_key: _Optional[str] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., type: _Optional[_Union[FolderTypeProto, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ..., slug: _Optional[str] = ..., main_scene_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class FollowProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "owned_by_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    owned_by_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class FontProto(_message.Message):
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
    metatype: StructTypeProto
    type: FontTypeProto
    style_ptr: NodeReferenceProto
    weight: FontWeightProto
    color: FillProto
    size: FontSizeProto
    align: TextAlignProto
    line_height: LengthProto
    letter_spacing: LengthProto
    decoration: TextDecorationProto
    transform: TextTransformProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[FontTypeProto, str]] = ..., style_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., weight: _Optional[_Union[FontWeightProto, str]] = ..., color: _Optional[_Union[FillProto, _Mapping]] = ..., size: _Optional[_Union[FontSizeProto, str]] = ..., align: _Optional[_Union[TextAlignProto, str]] = ..., line_height: _Optional[_Union[LengthProto, _Mapping]] = ..., letter_spacing: _Optional[_Union[LengthProto, _Mapping]] = ..., decoration: _Optional[_Union[TextDecorationProto, str]] = ..., transform: _Optional[_Union[TextTransformProto, str]] = ...) -> None: ...

class FontStyleProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "order_key", "type", "name", "weight", "color", "size", "align", "line_height", "letter_spacing", "decoration", "transform")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    WEIGHT_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    SIZE_FIELD_NUMBER: _ClassVar[int]
    ALIGN_FIELD_NUMBER: _ClassVar[int]
    LINE_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    LETTER_SPACING_FIELD_NUMBER: _ClassVar[int]
    DECORATION_FIELD_NUMBER: _ClassVar[int]
    TRANSFORM_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    order_key: str
    type: FontTypeProto
    name: str
    weight: FontWeightProto
    color: FillProto
    size: FontSizeProto
    align: TextAlignProto
    line_height: LengthProto
    letter_spacing: LengthProto
    decoration: TextDecorationProto
    transform: TextTransformProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[FontTypeProto, str]] = ..., name: _Optional[str] = ..., weight: _Optional[_Union[FontWeightProto, str]] = ..., color: _Optional[_Union[FillProto, _Mapping]] = ..., size: _Optional[_Union[FontSizeProto, str]] = ..., align: _Optional[_Union[TextAlignProto, str]] = ..., line_height: _Optional[_Union[LengthProto, _Mapping]] = ..., letter_spacing: _Optional[_Union[LengthProto, _Mapping]] = ..., decoration: _Optional[_Union[TextDecorationProto, str]] = ..., transform: _Optional[_Union[TextTransformProto, str]] = ...) -> None: ...

class FrameViewProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "base_type", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "script_ptr", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "layout", "direction", "distribute", "align", "gap", "padding", "grid", "grid_span", "aspect_ratio", "is_wrap", "is_visible", "opacity", "fill", "rotation", "skew", "scale", "shadow", "border", "radius")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
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
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    script_ptr: NodeReferenceProto
    name: str
    position: PositionProto
    width: DimensionProto
    height: DimensionProto
    min_width: DimensionProto
    min_height: DimensionProto
    max_width: DimensionProto
    max_height: DimensionProto
    layout: LayoutProto
    direction: DirectionProto
    distribute: DistributeProto
    align: AlignProto
    gap: Axis2Proto
    padding: InsetsProto
    grid: GridProto
    grid_span: GridSpanProto
    aspect_ratio: float
    is_wrap: bool
    is_visible: bool
    opacity: float
    fill: FillProto
    rotation: Axis3Proto
    skew: Vector2fProto
    scale: float
    shadow: ShadowProto
    border: BorderProto
    radius: CornersProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionProto, _Mapping]] = ..., width: _Optional[_Union[DimensionProto, _Mapping]] = ..., height: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., layout: _Optional[_Union[LayoutProto, str]] = ..., direction: _Optional[_Union[DirectionProto, str]] = ..., distribute: _Optional[_Union[DistributeProto, str]] = ..., align: _Optional[_Union[AlignProto, str]] = ..., gap: _Optional[_Union[Axis2Proto, _Mapping]] = ..., padding: _Optional[_Union[InsetsProto, _Mapping]] = ..., grid: _Optional[_Union[GridProto, _Mapping]] = ..., grid_span: _Optional[_Union[GridSpanProto, _Mapping]] = ..., aspect_ratio: _Optional[float] = ..., is_wrap: bool = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., fill: _Optional[_Union[FillProto, _Mapping]] = ..., rotation: _Optional[_Union[Axis3Proto, _Mapping]] = ..., skew: _Optional[_Union[Vector2fProto, _Mapping]] = ..., scale: _Optional[float] = ..., shadow: _Optional[_Union[ShadowProto, _Mapping]] = ..., border: _Optional[_Union[BorderProto, _Mapping]] = ..., radius: _Optional[_Union[CornersProto, _Mapping]] = ...) -> None: ...

class FriendshipProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "user_a_ptr", "user_b_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_A_PTR_FIELD_NUMBER: _ClassVar[int]
    USER_B_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    user_a_ptr: NodeReferenceProto
    user_b_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., user_a_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., user_b_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class FriendshipInviteProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "owned_by_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    owned_by_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class FriendshipInviteAcceptedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class FriendshipInviteEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class FriendshipInviteRejectedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class FriendshipInviteRescindedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class FriendshipInviteSentEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class FunctionProto(_message.Message):
    __slots__ = ("metatype", "type", "left", "right")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    LEFT_FIELD_NUMBER: _ClassVar[int]
    RIGHT_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: FunctionTypeProto
    left: ExpressionProto
    right: ExpressionProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[FunctionTypeProto, str]] = ..., left: _Optional[_Union[ExpressionProto, _Mapping]] = ..., right: _Optional[_Union[ExpressionProto, _Mapping]] = ...) -> None: ...

class GalaxyInfoProto(_message.Message):
    __slots__ = ("metatype", "region", "name", "host")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    HOST_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    region: RegionProto
    name: str
    host: str
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., region: _Optional[_Union[RegionProto, str]] = ..., name: _Optional[str] = ..., host: _Optional[str] = ...) -> None: ...

class GaugeMeasurementEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class GaugeMetricProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "order_key", "source_ptr", "name", "icon")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    order_key: str
    source_ptr: NodeReferenceProto
    name: str
    icon: IconProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., order_key: _Optional[str] = ..., source_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ...) -> None: ...

class GradientProto(_message.Message):
    __slots__ = ("metatype", "type", "style_ptr", "angle", "stops", "center_anchor")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    STYLE_PTR_FIELD_NUMBER: _ClassVar[int]
    ANGLE_FIELD_NUMBER: _ClassVar[int]
    STOPS_FIELD_NUMBER: _ClassVar[int]
    CENTER_ANCHOR_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: GradientTypeProto
    style_ptr: NodeReferenceProto
    angle: float
    stops: _containers.RepeatedCompositeFieldContainer[GradientStopProto]
    center_anchor: Axis2Proto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[GradientTypeProto, str]] = ..., style_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., angle: _Optional[float] = ..., stops: _Optional[_Iterable[_Union[GradientStopProto, _Mapping]]] = ..., center_anchor: _Optional[_Union[Axis2Proto, _Mapping]] = ...) -> None: ...

class GradientStopProto(_message.Message):
    __slots__ = ("metatype", "color", "position")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    color: ColorProto
    position: float
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., color: _Optional[_Union[ColorProto, _Mapping]] = ..., position: _Optional[float] = ...) -> None: ...

class GradientStyleProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "order_key", "type", "name", "angle", "stops", "center_anchor", "dark")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ANGLE_FIELD_NUMBER: _ClassVar[int]
    STOPS_FIELD_NUMBER: _ClassVar[int]
    CENTER_ANCHOR_FIELD_NUMBER: _ClassVar[int]
    DARK_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    order_key: str
    type: GradientTypeProto
    name: str
    angle: float
    stops: _containers.RepeatedCompositeFieldContainer[GradientStopProto]
    center_anchor: Axis2Proto
    dark: GradientProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[GradientTypeProto, str]] = ..., name: _Optional[str] = ..., angle: _Optional[float] = ..., stops: _Optional[_Iterable[_Union[GradientStopProto, _Mapping]]] = ..., center_anchor: _Optional[_Union[Axis2Proto, _Mapping]] = ..., dark: _Optional[_Union[GradientProto, _Mapping]] = ...) -> None: ...

class GridProto(_message.Message):
    __slots__ = ("metatype", "columns", "rows", "column_width", "column_min_width", "row_height")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    COLUMNS_FIELD_NUMBER: _ClassVar[int]
    ROWS_FIELD_NUMBER: _ClassVar[int]
    COLUMN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    COLUMN_MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    ROW_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    columns: int
    rows: int
    column_width: DimensionProto
    column_min_width: DimensionProto
    row_height: DimensionProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., columns: _Optional[int] = ..., rows: _Optional[int] = ..., column_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., column_min_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., row_height: _Optional[_Union[DimensionProto, _Mapping]] = ...) -> None: ...

class GridSpanProto(_message.Message):
    __slots__ = ("metatype", "columns", "rows")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    COLUMNS_FIELD_NUMBER: _ClassVar[int]
    ROWS_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    columns: int
    rows: int
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., columns: _Optional[int] = ..., rows: _Optional[int] = ...) -> None: ...

class HandleProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "slug")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    slug: str
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., slug: _Optional[str] = ...) -> None: ...

class HistogramProto(_message.Message):
    __slots__ = ("metatype", "buckets", "counts")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    BUCKETS_FIELD_NUMBER: _ClassVar[int]
    COUNTS_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    buckets: _containers.RepeatedCompositeFieldContainer[ValueProto]
    counts: _containers.RepeatedScalarFieldContainer[int]
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., buckets: _Optional[_Iterable[_Union[ValueProto, _Mapping]]] = ..., counts: _Optional[_Iterable[int]] = ...) -> None: ...

class HistogramMeasurementEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class HistogramMetricProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "order_key", "source_ptr", "name", "icon")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    order_key: str
    source_ptr: NodeReferenceProto
    name: str
    icon: IconProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., order_key: _Optional[str] = ..., source_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ...) -> None: ...

class IconProto(_message.Message):
    __slots__ = ("metatype", "type", "emoji", "fa_name", "vsc_name", "file_ptr", "file_url", "color")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    EMOJI_FIELD_NUMBER: _ClassVar[int]
    FA_NAME_FIELD_NUMBER: _ClassVar[int]
    VSC_NAME_FIELD_NUMBER: _ClassVar[int]
    FILE_PTR_FIELD_NUMBER: _ClassVar[int]
    FILE_URL_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: IconTypeProto
    emoji: str
    fa_name: str
    vsc_name: str
    file_ptr: NodeReferenceProto
    file_url: str
    color: ColorProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[IconTypeProto, str]] = ..., emoji: _Optional[str] = ..., fa_name: _Optional[str] = ..., vsc_name: _Optional[str] = ..., file_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., file_url: _Optional[str] = ..., color: _Optional[_Union[ColorProto, _Mapping]] = ...) -> None: ...

class InputEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class InputViewProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "base_type", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "script_ptr", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "is_visible", "opacity")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
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
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    script_ptr: NodeReferenceProto
    name: str
    position: PositionProto
    width: DimensionProto
    height: DimensionProto
    min_width: DimensionProto
    min_height: DimensionProto
    max_width: DimensionProto
    max_height: DimensionProto
    is_visible: bool
    opacity: float
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionProto, _Mapping]] = ..., width: _Optional[_Union[DimensionProto, _Mapping]] = ..., height: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., is_visible: bool = ..., opacity: _Optional[float] = ...) -> None: ...

class InsetsProto(_message.Message):
    __slots__ = ("metatype", "base", "top", "left", "right", "bottom")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    BASE_FIELD_NUMBER: _ClassVar[int]
    TOP_FIELD_NUMBER: _ClassVar[int]
    LEFT_FIELD_NUMBER: _ClassVar[int]
    RIGHT_FIELD_NUMBER: _ClassVar[int]
    BOTTOM_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    base: int
    top: int
    left: int
    right: int
    bottom: int
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., base: _Optional[int] = ..., top: _Optional[int] = ..., left: _Optional[int] = ..., right: _Optional[int] = ..., bottom: _Optional[int] = ...) -> None: ...

class InternalViewProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "base_type", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "script_ptr", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    script_ptr: NodeReferenceProto
    name: str
    position: PositionProto
    width: DimensionProto
    height: DimensionProto
    min_width: DimensionProto
    min_height: DimensionProto
    max_width: DimensionProto
    max_height: DimensionProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionProto, _Mapping]] = ..., width: _Optional[_Union[DimensionProto, _Mapping]] = ..., height: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_height: _Optional[_Union[DimensionProto, _Mapping]] = ...) -> None: ...

class InviteProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "owned_by_ptr", "member_ptr", "role_ptr", "role_type")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MEMBER_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLE_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLE_TYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    owned_by_ptr: NodeReferenceProto
    member_ptr: NodeReferenceProto
    role_ptr: NodeReferenceProto
    role_type: RoleTypeProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., member_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., role_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., role_type: _Optional[_Union[RoleTypeProto, str]] = ...) -> None: ...

class InviteAcceptedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "joinable_ptr", "member_ptr", "role_ptr", "role_type")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    JOINABLE_PTR_FIELD_NUMBER: _ClassVar[int]
    MEMBER_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLE_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLE_TYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    joinable_ptr: NodeReferenceProto
    member_ptr: NodeReferenceProto
    role_ptr: NodeReferenceProto
    role_type: RoleTypeProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., joinable_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., member_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., role_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., role_type: _Optional[_Union[RoleTypeProto, str]] = ...) -> None: ...

class InviteEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "joinable_ptr", "member_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    JOINABLE_PTR_FIELD_NUMBER: _ClassVar[int]
    MEMBER_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    joinable_ptr: NodeReferenceProto
    member_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., joinable_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., member_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class InviteRejectedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "joinable_ptr", "member_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    JOINABLE_PTR_FIELD_NUMBER: _ClassVar[int]
    MEMBER_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    joinable_ptr: NodeReferenceProto
    member_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., joinable_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., member_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class InviteRescindedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "joinable_ptr", "member_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    JOINABLE_PTR_FIELD_NUMBER: _ClassVar[int]
    MEMBER_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    joinable_ptr: NodeReferenceProto
    member_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., joinable_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., member_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class InviteSentEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "joinable_ptr", "member_ptr", "role_ptr", "role_type")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    JOINABLE_PTR_FIELD_NUMBER: _ClassVar[int]
    MEMBER_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLE_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLE_TYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    joinable_ptr: NodeReferenceProto
    member_ptr: NodeReferenceProto
    role_ptr: NodeReferenceProto
    role_type: RoleTypeProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., joinable_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., member_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., role_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., role_type: _Optional[_Union[RoleTypeProto, str]] = ...) -> None: ...

class JoinProto(_message.Message):
    __slots__ = ("metatype", "type", "definition", "recursive", "depth", "on")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_FIELD_NUMBER: _ClassVar[int]
    RECURSIVE_FIELD_NUMBER: _ClassVar[int]
    DEPTH_FIELD_NUMBER: _ClassVar[int]
    ON_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: JoinTypeProto
    definition: NodeDefinitionReferenceProto
    recursive: bool
    depth: int
    on: ConditionProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[JoinTypeProto, str]] = ..., definition: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., recursive: bool = ..., depth: _Optional[int] = ..., on: _Optional[_Union[ConditionProto, _Mapping]] = ...) -> None: ...

class KeyDownEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "key", "code", "repeat", "shift_key", "alt_key", "ctrl_key", "meta_key")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    CODE_FIELD_NUMBER: _ClassVar[int]
    REPEAT_FIELD_NUMBER: _ClassVar[int]
    SHIFT_KEY_FIELD_NUMBER: _ClassVar[int]
    ALT_KEY_FIELD_NUMBER: _ClassVar[int]
    CTRL_KEY_FIELD_NUMBER: _ClassVar[int]
    META_KEY_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    key: str
    code: str
    repeat: bool
    shift_key: bool
    alt_key: bool
    ctrl_key: bool
    meta_key: bool
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., key: _Optional[str] = ..., code: _Optional[str] = ..., repeat: bool = ..., shift_key: bool = ..., alt_key: bool = ..., ctrl_key: bool = ..., meta_key: bool = ...) -> None: ...

class KeyPressEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "key", "code", "repeat", "shift_key", "alt_key", "ctrl_key", "meta_key")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    CODE_FIELD_NUMBER: _ClassVar[int]
    REPEAT_FIELD_NUMBER: _ClassVar[int]
    SHIFT_KEY_FIELD_NUMBER: _ClassVar[int]
    ALT_KEY_FIELD_NUMBER: _ClassVar[int]
    CTRL_KEY_FIELD_NUMBER: _ClassVar[int]
    META_KEY_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    key: str
    code: str
    repeat: bool
    shift_key: bool
    alt_key: bool
    ctrl_key: bool
    meta_key: bool
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., key: _Optional[str] = ..., code: _Optional[str] = ..., repeat: bool = ..., shift_key: bool = ..., alt_key: bool = ..., ctrl_key: bool = ..., meta_key: bool = ...) -> None: ...

class KeyUpEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "key", "code", "repeat", "shift_key", "alt_key", "ctrl_key", "meta_key")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    CODE_FIELD_NUMBER: _ClassVar[int]
    REPEAT_FIELD_NUMBER: _ClassVar[int]
    SHIFT_KEY_FIELD_NUMBER: _ClassVar[int]
    ALT_KEY_FIELD_NUMBER: _ClassVar[int]
    CTRL_KEY_FIELD_NUMBER: _ClassVar[int]
    META_KEY_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    key: str
    code: str
    repeat: bool
    shift_key: bool
    alt_key: bool
    ctrl_key: bool
    meta_key: bool
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., key: _Optional[str] = ..., code: _Optional[str] = ..., repeat: bool = ..., shift_key: bool = ..., alt_key: bool = ..., ctrl_key: bool = ..., meta_key: bool = ...) -> None: ...

class KeyboardEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "key", "code", "repeat", "shift_key", "alt_key", "ctrl_key", "meta_key")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    CODE_FIELD_NUMBER: _ClassVar[int]
    REPEAT_FIELD_NUMBER: _ClassVar[int]
    SHIFT_KEY_FIELD_NUMBER: _ClassVar[int]
    ALT_KEY_FIELD_NUMBER: _ClassVar[int]
    CTRL_KEY_FIELD_NUMBER: _ClassVar[int]
    META_KEY_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    key: str
    code: str
    repeat: bool
    shift_key: bool
    alt_key: bool
    ctrl_key: bool
    meta_key: bool
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., key: _Optional[str] = ..., code: _Optional[str] = ..., repeat: bool = ..., shift_key: bool = ..., alt_key: bool = ..., ctrl_key: bool = ..., meta_key: bool = ...) -> None: ...

class LabelViewProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "base_type", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "script_ptr", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "layout", "direction", "distribute", "align", "gap", "padding", "grid", "grid_span", "aspect_ratio", "is_wrap", "is_visible", "opacity", "fill", "rotation", "skew", "scale", "shadow", "border", "radius")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
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
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    script_ptr: NodeReferenceProto
    name: str
    position: PositionProto
    width: DimensionProto
    height: DimensionProto
    min_width: DimensionProto
    min_height: DimensionProto
    max_width: DimensionProto
    max_height: DimensionProto
    layout: LayoutProto
    direction: DirectionProto
    distribute: DistributeProto
    align: AlignProto
    gap: Axis2Proto
    padding: InsetsProto
    grid: GridProto
    grid_span: GridSpanProto
    aspect_ratio: float
    is_wrap: bool
    is_visible: bool
    opacity: float
    fill: FillProto
    rotation: Axis3Proto
    skew: Vector2fProto
    scale: float
    shadow: ShadowProto
    border: BorderProto
    radius: CornersProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionProto, _Mapping]] = ..., width: _Optional[_Union[DimensionProto, _Mapping]] = ..., height: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., layout: _Optional[_Union[LayoutProto, str]] = ..., direction: _Optional[_Union[DirectionProto, str]] = ..., distribute: _Optional[_Union[DistributeProto, str]] = ..., align: _Optional[_Union[AlignProto, str]] = ..., gap: _Optional[_Union[Axis2Proto, _Mapping]] = ..., padding: _Optional[_Union[InsetsProto, _Mapping]] = ..., grid: _Optional[_Union[GridProto, _Mapping]] = ..., grid_span: _Optional[_Union[GridSpanProto, _Mapping]] = ..., aspect_ratio: _Optional[float] = ..., is_wrap: bool = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., fill: _Optional[_Union[FillProto, _Mapping]] = ..., rotation: _Optional[_Union[Axis3Proto, _Mapping]] = ..., skew: _Optional[_Union[Vector2fProto, _Mapping]] = ..., scale: _Optional[float] = ..., shadow: _Optional[_Union[ShadowProto, _Mapping]] = ..., border: _Optional[_Union[BorderProto, _Mapping]] = ..., radius: _Optional[_Union[CornersProto, _Mapping]] = ...) -> None: ...

class LayerProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "base_type", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "owned_by_ptr", "script_ptr", "type", "name", "icon", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "layout", "direction", "distribute", "align", "gap", "padding", "grid", "grid_span", "aspect_ratio", "is_wrap", "is_visible", "opacity", "fill", "rotation", "skew", "scale", "shadow", "border", "radius")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
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
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    owned_by_ptr: NodeReferenceProto
    script_ptr: NodeReferenceProto
    type: LayerTypeProto
    name: str
    icon: IconProto
    position: PositionProto
    width: DimensionProto
    height: DimensionProto
    min_width: DimensionProto
    min_height: DimensionProto
    max_width: DimensionProto
    max_height: DimensionProto
    layout: LayoutProto
    direction: DirectionProto
    distribute: DistributeProto
    align: AlignProto
    gap: Axis2Proto
    padding: InsetsProto
    grid: GridProto
    grid_span: GridSpanProto
    aspect_ratio: float
    is_wrap: bool
    is_visible: bool
    opacity: float
    fill: FillProto
    rotation: Axis3Proto
    skew: Vector2fProto
    scale: float
    shadow: ShadowProto
    border: BorderProto
    radius: CornersProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., type: _Optional[_Union[LayerTypeProto, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ..., position: _Optional[_Union[PositionProto, _Mapping]] = ..., width: _Optional[_Union[DimensionProto, _Mapping]] = ..., height: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., layout: _Optional[_Union[LayoutProto, str]] = ..., direction: _Optional[_Union[DirectionProto, str]] = ..., distribute: _Optional[_Union[DistributeProto, str]] = ..., align: _Optional[_Union[AlignProto, str]] = ..., gap: _Optional[_Union[Axis2Proto, _Mapping]] = ..., padding: _Optional[_Union[InsetsProto, _Mapping]] = ..., grid: _Optional[_Union[GridProto, _Mapping]] = ..., grid_span: _Optional[_Union[GridSpanProto, _Mapping]] = ..., aspect_ratio: _Optional[float] = ..., is_wrap: bool = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., fill: _Optional[_Union[FillProto, _Mapping]] = ..., rotation: _Optional[_Union[Axis3Proto, _Mapping]] = ..., skew: _Optional[_Union[Vector2fProto, _Mapping]] = ..., scale: _Optional[float] = ..., shadow: _Optional[_Union[ShadowProto, _Mapping]] = ..., border: _Optional[_Union[BorderProto, _Mapping]] = ..., radius: _Optional[_Union[CornersProto, _Mapping]] = ...) -> None: ...

class LeftClickEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "position", "pressure", "shift_key", "alt_key", "ctrl_key", "meta_key", "accel_key", "button")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    PRESSURE_FIELD_NUMBER: _ClassVar[int]
    SHIFT_KEY_FIELD_NUMBER: _ClassVar[int]
    ALT_KEY_FIELD_NUMBER: _ClassVar[int]
    CTRL_KEY_FIELD_NUMBER: _ClassVar[int]
    META_KEY_FIELD_NUMBER: _ClassVar[int]
    ACCEL_KEY_FIELD_NUMBER: _ClassVar[int]
    BUTTON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    position: Vector2fProto
    pressure: float
    shift_key: bool
    alt_key: bool
    ctrl_key: bool
    meta_key: bool
    accel_key: bool
    button: MouseButtonProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., position: _Optional[_Union[Vector2fProto, _Mapping]] = ..., pressure: _Optional[float] = ..., shift_key: bool = ..., alt_key: bool = ..., ctrl_key: bool = ..., meta_key: bool = ..., accel_key: bool = ..., button: _Optional[_Union[MouseButtonProto, str]] = ...) -> None: ...

class LengthProto(_message.Message):
    __slots__ = ("metatype", "unit", "value")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    UNIT_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    unit: LengthUnitProto
    value: float
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., unit: _Optional[_Union[LengthUnitProto, str]] = ..., value: _Optional[float] = ...) -> None: ...

class LineProto(_message.Message):
    __slots__ = ("metatype", "stroke", "points")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    STROKE_FIELD_NUMBER: _ClassVar[int]
    POINTS_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    stroke: StrokeProto
    points: _containers.RepeatedCompositeFieldContainer[Vector2fProto]
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., stroke: _Optional[_Union[StrokeProto, _Mapping]] = ..., points: _Optional[_Iterable[_Union[Vector2fProto, _Mapping]]] = ...) -> None: ...

class LineShapeProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "base_type", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "script_ptr", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "layout", "direction", "distribute", "align", "gap", "padding", "grid", "grid_span", "aspect_ratio", "is_wrap", "is_visible", "opacity", "fill", "rotation", "skew", "scale", "shadow", "border", "radius", "stroke", "points")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
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
    STROKE_FIELD_NUMBER: _ClassVar[int]
    POINTS_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    script_ptr: NodeReferenceProto
    name: str
    position: PositionProto
    width: DimensionProto
    height: DimensionProto
    min_width: DimensionProto
    min_height: DimensionProto
    max_width: DimensionProto
    max_height: DimensionProto
    layout: LayoutProto
    direction: DirectionProto
    distribute: DistributeProto
    align: AlignProto
    gap: Axis2Proto
    padding: InsetsProto
    grid: GridProto
    grid_span: GridSpanProto
    aspect_ratio: float
    is_wrap: bool
    is_visible: bool
    opacity: float
    fill: FillProto
    rotation: Axis3Proto
    skew: Vector2fProto
    scale: float
    shadow: ShadowProto
    border: BorderProto
    radius: CornersProto
    stroke: StrokeProto
    points: _containers.RepeatedCompositeFieldContainer[Vector2fProto]
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionProto, _Mapping]] = ..., width: _Optional[_Union[DimensionProto, _Mapping]] = ..., height: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., layout: _Optional[_Union[LayoutProto, str]] = ..., direction: _Optional[_Union[DirectionProto, str]] = ..., distribute: _Optional[_Union[DistributeProto, str]] = ..., align: _Optional[_Union[AlignProto, str]] = ..., gap: _Optional[_Union[Axis2Proto, _Mapping]] = ..., padding: _Optional[_Union[InsetsProto, _Mapping]] = ..., grid: _Optional[_Union[GridProto, _Mapping]] = ..., grid_span: _Optional[_Union[GridSpanProto, _Mapping]] = ..., aspect_ratio: _Optional[float] = ..., is_wrap: bool = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., fill: _Optional[_Union[FillProto, _Mapping]] = ..., rotation: _Optional[_Union[Axis3Proto, _Mapping]] = ..., skew: _Optional[_Union[Vector2fProto, _Mapping]] = ..., scale: _Optional[float] = ..., shadow: _Optional[_Union[ShadowProto, _Mapping]] = ..., border: _Optional[_Union[BorderProto, _Mapping]] = ..., radius: _Optional[_Union[CornersProto, _Mapping]] = ..., stroke: _Optional[_Union[StrokeProto, _Mapping]] = ..., points: _Optional[_Iterable[_Union[Vector2fProto, _Mapping]]] = ...) -> None: ...

class LogEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "content", "attributes", "level")
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
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    CONTENT_FIELD_NUMBER: _ClassVar[int]
    ATTRIBUTES_FIELD_NUMBER: _ClassVar[int]
    LEVEL_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    content: str
    attributes: _containers.MessageMap[str, _struct_pb2.Value]
    level: LogLevelProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., content: _Optional[str] = ..., attributes: _Optional[_Mapping[str, _struct_pb2.Value]] = ..., level: _Optional[_Union[LogLevelProto, str]] = ...) -> None: ...

class MachineProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "base_type", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "script_ptr", "status", "type", "version", "external_name", "external_id", "image_id", "grpc_url", "vnc_url", "client_ptr", "cpu", "ram", "width", "height", "is_headless")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
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
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    script_ptr: NodeReferenceProto
    status: ResourceStatusProto
    type: MachineTypeProto
    version: str
    external_name: str
    external_id: str
    image_id: str
    grpc_url: str
    vnc_url: str
    client_ptr: NodeReferenceProto
    cpu: float
    ram: float
    width: int
    height: int
    is_headless: bool
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., status: _Optional[_Union[ResourceStatusProto, str]] = ..., type: _Optional[_Union[MachineTypeProto, str]] = ..., version: _Optional[str] = ..., external_name: _Optional[str] = ..., external_id: _Optional[str] = ..., image_id: _Optional[str] = ..., grpc_url: _Optional[str] = ..., vnc_url: _Optional[str] = ..., client_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., cpu: _Optional[float] = ..., ram: _Optional[float] = ..., width: _Optional[int] = ..., height: _Optional[int] = ..., is_headless: bool = ...) -> None: ...

class MeasurementEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class MembershipProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "owned_by_ptr", "member_ptr", "role_ptr", "role_type")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    MEMBER_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLE_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLE_TYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    owned_by_ptr: NodeReferenceProto
    member_ptr: NodeReferenceProto
    role_ptr: NodeReferenceProto
    role_type: RoleTypeProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., member_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., role_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., role_type: _Optional[_Union[RoleTypeProto, str]] = ...) -> None: ...

class MembershipEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "joinable_ptr", "member_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    JOINABLE_PTR_FIELD_NUMBER: _ClassVar[int]
    MEMBER_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    joinable_ptr: NodeReferenceProto
    member_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., joinable_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., member_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class MembershipJoinedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "joinable_ptr", "member_ptr", "role_ptr", "role_type")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    JOINABLE_PTR_FIELD_NUMBER: _ClassVar[int]
    MEMBER_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLE_PTR_FIELD_NUMBER: _ClassVar[int]
    ROLE_TYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    joinable_ptr: NodeReferenceProto
    member_ptr: NodeReferenceProto
    role_ptr: NodeReferenceProto
    role_type: RoleTypeProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., joinable_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., member_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., role_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., role_type: _Optional[_Union[RoleTypeProto, str]] = ...) -> None: ...

class MembershipLeftEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "joinable_ptr", "member_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    JOINABLE_PTR_FIELD_NUMBER: _ClassVar[int]
    MEMBER_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    joinable_ptr: NodeReferenceProto
    member_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., joinable_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., member_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class MessageProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "owned_by_ptr", "thread_ptr", "edited_at", "reply_to_ptr", "forwarded_from_ptr", "text", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    THREAD_PTR_FIELD_NUMBER: _ClassVar[int]
    EDITED_AT_FIELD_NUMBER: _ClassVar[int]
    REPLY_TO_PTR_FIELD_NUMBER: _ClassVar[int]
    FORWARDED_FROM_PTR_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    owned_by_ptr: NodeReferenceProto
    thread_ptr: NodeReferenceProto
    edited_at: _timestamp_pb2.Timestamp
    reply_to_ptr: NodeReferenceProto
    forwarded_from_ptr: NodeReferenceProto
    text: TextProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., thread_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., edited_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., reply_to_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., forwarded_from_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., text: _Optional[_Union[TextProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class MethodProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "source_ptr", "name", "icon", "text", "cardinality")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    CARDINALITY_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    source_ptr: NodeReferenceProto
    name: str
    icon: IconProto
    text: TextProto
    cardinality: MethodCardinalityProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., source_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ..., text: _Optional[_Union[TextProto, _Mapping]] = ..., cardinality: _Optional[_Union[MethodCardinalityProto, str]] = ...) -> None: ...

class MethodDefinitionProto(_message.Message):
    __slots__ = ("metatype", "id", "name", "icon", "description", "properties")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    id: int
    name: str
    icon: IconProto
    description: str
    properties: _containers.RepeatedCompositeFieldContainer[PropertyDefinitionProto]
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., id: _Optional[int] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ..., description: _Optional[str] = ..., properties: _Optional[_Iterable[_Union[PropertyDefinitionProto, _Mapping]]] = ...) -> None: ...

class MetricProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "order_key", "source_ptr", "name", "icon")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    order_key: str
    source_ptr: NodeReferenceProto
    name: str
    icon: IconProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., order_key: _Optional[str] = ..., source_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ...) -> None: ...

class MiddleClickEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "position", "pressure", "shift_key", "alt_key", "ctrl_key", "meta_key", "accel_key", "button")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    PRESSURE_FIELD_NUMBER: _ClassVar[int]
    SHIFT_KEY_FIELD_NUMBER: _ClassVar[int]
    ALT_KEY_FIELD_NUMBER: _ClassVar[int]
    CTRL_KEY_FIELD_NUMBER: _ClassVar[int]
    META_KEY_FIELD_NUMBER: _ClassVar[int]
    ACCEL_KEY_FIELD_NUMBER: _ClassVar[int]
    BUTTON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    position: Vector2fProto
    pressure: float
    shift_key: bool
    alt_key: bool
    ctrl_key: bool
    meta_key: bool
    accel_key: bool
    button: MouseButtonProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., position: _Optional[_Union[Vector2fProto, _Mapping]] = ..., pressure: _Optional[float] = ..., shift_key: bool = ..., alt_key: bool = ..., ctrl_key: bool = ..., meta_key: bool = ..., accel_key: bool = ..., button: _Optional[_Union[MouseButtonProto, str]] = ...) -> None: ...

class MouseEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "position", "pressure", "shift_key", "alt_key", "ctrl_key", "meta_key", "accel_key", "button")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    PRESSURE_FIELD_NUMBER: _ClassVar[int]
    SHIFT_KEY_FIELD_NUMBER: _ClassVar[int]
    ALT_KEY_FIELD_NUMBER: _ClassVar[int]
    CTRL_KEY_FIELD_NUMBER: _ClassVar[int]
    META_KEY_FIELD_NUMBER: _ClassVar[int]
    ACCEL_KEY_FIELD_NUMBER: _ClassVar[int]
    BUTTON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    position: Vector2fProto
    pressure: float
    shift_key: bool
    alt_key: bool
    ctrl_key: bool
    meta_key: bool
    accel_key: bool
    button: MouseButtonProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., position: _Optional[_Union[Vector2fProto, _Mapping]] = ..., pressure: _Optional[float] = ..., shift_key: bool = ..., alt_key: bool = ..., ctrl_key: bool = ..., meta_key: bool = ..., accel_key: bool = ..., button: _Optional[_Union[MouseButtonProto, str]] = ...) -> None: ...

class NodeProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class NodeConstraintProto(_message.Message):
    __slots__ = ("metatype", "node_types", "node_traits")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPES_FIELD_NUMBER: _ClassVar[int]
    NODE_TRAITS_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    node_types: _containers.RepeatedScalarFieldContainer[NodeTypeProto]
    node_traits: _containers.RepeatedScalarFieldContainer[TraitTypeProto]
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., node_types: _Optional[_Iterable[_Union[NodeTypeProto, str]]] = ..., node_traits: _Optional[_Iterable[_Union[TraitTypeProto, str]]] = ...) -> None: ...

class NodeDefinitionProto(_message.Message):
    __slots__ = ("metatype", "id", "type", "name", "icon", "description", "primary_store_types", "properties", "groups", "is_global", "is_spatial", "is_abstract", "is_extensible", "is_frozen", "base_type", "extended_by", "inherits", "inherited_by", "base_traits", "traits", "root_type", "parent_types", "child_types", "ancestor_types", "descendant_types", "event_types", "base_event_types")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    PRIMARY_STORE_TYPES_FIELD_NUMBER: _ClassVar[int]
    PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    GROUPS_FIELD_NUMBER: _ClassVar[int]
    IS_GLOBAL_FIELD_NUMBER: _ClassVar[int]
    IS_SPATIAL_FIELD_NUMBER: _ClassVar[int]
    IS_ABSTRACT_FIELD_NUMBER: _ClassVar[int]
    IS_EXTENSIBLE_FIELD_NUMBER: _ClassVar[int]
    IS_FROZEN_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    EXTENDED_BY_FIELD_NUMBER: _ClassVar[int]
    INHERITS_FIELD_NUMBER: _ClassVar[int]
    INHERITED_BY_FIELD_NUMBER: _ClassVar[int]
    BASE_TRAITS_FIELD_NUMBER: _ClassVar[int]
    TRAITS_FIELD_NUMBER: _ClassVar[int]
    ROOT_TYPE_FIELD_NUMBER: _ClassVar[int]
    PARENT_TYPES_FIELD_NUMBER: _ClassVar[int]
    CHILD_TYPES_FIELD_NUMBER: _ClassVar[int]
    ANCESTOR_TYPES_FIELD_NUMBER: _ClassVar[int]
    DESCENDANT_TYPES_FIELD_NUMBER: _ClassVar[int]
    EVENT_TYPES_FIELD_NUMBER: _ClassVar[int]
    BASE_EVENT_TYPES_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    id: int
    type: NodeTypeProto
    name: str
    icon: IconProto
    description: str
    primary_store_types: _containers.RepeatedScalarFieldContainer[StoreTypeProto]
    properties: _containers.RepeatedCompositeFieldContainer[PropertyDefinitionProto]
    groups: _containers.RepeatedCompositeFieldContainer[PropertyGroupDefinitionProto]
    is_global: bool
    is_spatial: bool
    is_abstract: bool
    is_extensible: bool
    is_frozen: bool
    base_type: NodeTypeProto
    extended_by: _containers.RepeatedScalarFieldContainer[NodeTypeProto]
    inherits: _containers.RepeatedScalarFieldContainer[NodeTypeProto]
    inherited_by: _containers.RepeatedScalarFieldContainer[NodeTypeProto]
    base_traits: _containers.RepeatedScalarFieldContainer[TraitTypeProto]
    traits: _containers.RepeatedScalarFieldContainer[TraitTypeProto]
    root_type: NodeTypeProto
    parent_types: _containers.RepeatedScalarFieldContainer[NodeTypeProto]
    child_types: _containers.RepeatedScalarFieldContainer[NodeTypeProto]
    ancestor_types: _containers.RepeatedScalarFieldContainer[NodeTypeProto]
    descendant_types: _containers.RepeatedScalarFieldContainer[NodeTypeProto]
    event_types: _containers.RepeatedScalarFieldContainer[NodeTypeProto]
    base_event_types: _containers.RepeatedScalarFieldContainer[NodeTypeProto]
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., id: _Optional[int] = ..., type: _Optional[_Union[NodeTypeProto, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ..., description: _Optional[str] = ..., primary_store_types: _Optional[_Iterable[_Union[StoreTypeProto, str]]] = ..., properties: _Optional[_Iterable[_Union[PropertyDefinitionProto, _Mapping]]] = ..., groups: _Optional[_Iterable[_Union[PropertyGroupDefinitionProto, _Mapping]]] = ..., is_global: bool = ..., is_spatial: bool = ..., is_abstract: bool = ..., is_extensible: bool = ..., is_frozen: bool = ..., base_type: _Optional[_Union[NodeTypeProto, str]] = ..., extended_by: _Optional[_Iterable[_Union[NodeTypeProto, str]]] = ..., inherits: _Optional[_Iterable[_Union[NodeTypeProto, str]]] = ..., inherited_by: _Optional[_Iterable[_Union[NodeTypeProto, str]]] = ..., base_traits: _Optional[_Iterable[_Union[TraitTypeProto, str]]] = ..., traits: _Optional[_Iterable[_Union[TraitTypeProto, str]]] = ..., root_type: _Optional[_Union[NodeTypeProto, str]] = ..., parent_types: _Optional[_Iterable[_Union[NodeTypeProto, str]]] = ..., child_types: _Optional[_Iterable[_Union[NodeTypeProto, str]]] = ..., ancestor_types: _Optional[_Iterable[_Union[NodeTypeProto, str]]] = ..., descendant_types: _Optional[_Iterable[_Union[NodeTypeProto, str]]] = ..., event_types: _Optional[_Iterable[_Union[NodeTypeProto, str]]] = ..., base_event_types: _Optional[_Iterable[_Union[NodeTypeProto, str]]] = ...) -> None: ...

class NodeDefinitionReferenceProto(_message.Message):
    __slots__ = ("metatype", "type", "node_type", "definition_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: NodeDefinitionTypeProto
    node_type: NodeTypeProto
    definition_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[NodeDefinitionTypeProto, str]] = ..., node_type: _Optional[_Union[NodeTypeProto, str]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class NodeReferenceProto(_message.Message):
    __slots__ = ("metatype", "type", "id", "definition_id", "snapshot_id", "space_id", "store_type")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_ID_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_ID_FIELD_NUMBER: _ClassVar[int]
    SPACE_ID_FIELD_NUMBER: _ClassVar[int]
    STORE_TYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: NodeTypeProto
    id: str
    definition_id: str
    snapshot_id: str
    space_id: str
    store_type: StoreTypeProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., definition_id: _Optional[str] = ..., snapshot_id: _Optional[str] = ..., space_id: _Optional[str] = ..., store_type: _Optional[_Union[StoreTypeProto, str]] = ...) -> None: ...

class NotificationProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "owned_by_ptr", "status", "title", "text")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    TITLE_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    owned_by_ptr: NodeReferenceProto
    status: NotificationStatusProto
    title: str
    text: TextProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., status: _Optional[_Union[NotificationStatusProto, str]] = ..., title: _Optional[str] = ..., text: _Optional[_Union[TextProto, _Mapping]] = ...) -> None: ...

class NotificationDismissedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class NotificationEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class NotificationExpiredEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class NotificationReadEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class NotificationRescindedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class NotificationSentEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class NumberConstraintProto(_message.Message):
    __slots__ = ("metatype", "format", "min_value", "max_value", "step_value", "precision", "scale")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    FORMAT_FIELD_NUMBER: _ClassVar[int]
    MIN_VALUE_FIELD_NUMBER: _ClassVar[int]
    MAX_VALUE_FIELD_NUMBER: _ClassVar[int]
    STEP_VALUE_FIELD_NUMBER: _ClassVar[int]
    PRECISION_FIELD_NUMBER: _ClassVar[int]
    SCALE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    format: NumberFormatProto
    min_value: float
    max_value: float
    step_value: float
    precision: int
    scale: int
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., format: _Optional[_Union[NumberFormatProto, str]] = ..., min_value: _Optional[float] = ..., max_value: _Optional[float] = ..., step_value: _Optional[float] = ..., precision: _Optional[int] = ..., scale: _Optional[int] = ...) -> None: ...

class NumberInputViewProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "base_type", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "script_ptr", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "is_visible", "opacity", "value", "placeholder")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
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
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    script_ptr: NodeReferenceProto
    name: str
    position: PositionProto
    width: DimensionProto
    height: DimensionProto
    min_width: DimensionProto
    min_height: DimensionProto
    max_width: DimensionProto
    max_height: DimensionProto
    is_visible: bool
    opacity: float
    value: float
    placeholder: str
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionProto, _Mapping]] = ..., width: _Optional[_Union[DimensionProto, _Mapping]] = ..., height: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., value: _Optional[float] = ..., placeholder: _Optional[str] = ...) -> None: ...

class ObjectDefinitionReferenceProto(_message.Message):
    __slots__ = ("metatype", "type", "node_type", "trait_type", "struct_type", "definition_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    TRAIT_TYPE_FIELD_NUMBER: _ClassVar[int]
    STRUCT_TYPE_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: ObjectDefinitionTypeProto
    node_type: NodeTypeProto
    trait_type: TraitTypeProto
    struct_type: StructTypeProto
    definition_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[ObjectDefinitionTypeProto, str]] = ..., node_type: _Optional[_Union[NodeTypeProto, str]] = ..., trait_type: _Optional[_Union[TraitTypeProto, str]] = ..., struct_type: _Optional[_Union[StructTypeProto, str]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class OptionDefinitionProto(_message.Message):
    __slots__ = ("metatype", "id", "type", "name", "icon", "description", "group_id")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    GROUP_ID_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    id: int
    type: EnumTypeProto
    name: str
    icon: IconProto
    description: str
    group_id: int
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., id: _Optional[int] = ..., type: _Optional[_Union[EnumTypeProto, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ..., description: _Optional[str] = ..., group_id: _Optional[int] = ...) -> None: ...

class OptionGroupDefinitionProto(_message.Message):
    __slots__ = ("metatype", "id", "name", "icon", "description")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    id: int
    name: str
    icon: IconProto
    description: str
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., id: _Optional[int] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ..., description: _Optional[str] = ...) -> None: ...

class OrganizationProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "slug", "status", "space_ptr", "handle_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    HANDLE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    slug: str
    status: OrganizationStatusProto
    space_ptr: NodeReferenceProto
    handle_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., slug: _Optional[str] = ..., status: _Optional[_Union[OrganizationStatusProto, str]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., handle_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class OriginProto(_message.Message):
    __slots__ = ("metatype", "type", "id", "ck", "nonce")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CK_FIELD_NUMBER: _ClassVar[int]
    NONCE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: ClientTypeProto
    id: str
    ck: str
    nonce: str
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[ClientTypeProto, str]] = ..., id: _Optional[str] = ..., ck: _Optional[str] = ..., nonce: _Optional[str] = ...) -> None: ...

class PaletteProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "order_key", "name", "icon")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    order_key: str
    name: str
    icon: IconProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ...) -> None: ...

class PasteEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class PermissionProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "type", "name", "icon")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    type: PermissionTypeProto
    name: str
    icon: IconProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., type: _Optional[_Union[PermissionTypeProto, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ...) -> None: ...

class PermissionDefinitionProto(_message.Message):
    __slots__ = ("metatype", "id", "type", "name", "icon", "description", "node_type")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    id: int
    type: EnumTypeProto
    name: str
    icon: IconProto
    description: str
    node_type: NodeTypeProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., id: _Optional[int] = ..., type: _Optional[_Union[EnumTypeProto, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ..., description: _Optional[str] = ..., node_type: _Optional[_Union[NodeTypeProto, str]] = ...) -> None: ...

class PointerDownEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "position", "pressure", "shift_key", "alt_key", "ctrl_key", "meta_key", "accel_key")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    PRESSURE_FIELD_NUMBER: _ClassVar[int]
    SHIFT_KEY_FIELD_NUMBER: _ClassVar[int]
    ALT_KEY_FIELD_NUMBER: _ClassVar[int]
    CTRL_KEY_FIELD_NUMBER: _ClassVar[int]
    META_KEY_FIELD_NUMBER: _ClassVar[int]
    ACCEL_KEY_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    position: Vector2fProto
    pressure: float
    shift_key: bool
    alt_key: bool
    ctrl_key: bool
    meta_key: bool
    accel_key: bool
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., position: _Optional[_Union[Vector2fProto, _Mapping]] = ..., pressure: _Optional[float] = ..., shift_key: bool = ..., alt_key: bool = ..., ctrl_key: bool = ..., meta_key: bool = ..., accel_key: bool = ...) -> None: ...

class PointerEnterEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "position", "pressure", "shift_key", "alt_key", "ctrl_key", "meta_key", "accel_key")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    PRESSURE_FIELD_NUMBER: _ClassVar[int]
    SHIFT_KEY_FIELD_NUMBER: _ClassVar[int]
    ALT_KEY_FIELD_NUMBER: _ClassVar[int]
    CTRL_KEY_FIELD_NUMBER: _ClassVar[int]
    META_KEY_FIELD_NUMBER: _ClassVar[int]
    ACCEL_KEY_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    position: Vector2fProto
    pressure: float
    shift_key: bool
    alt_key: bool
    ctrl_key: bool
    meta_key: bool
    accel_key: bool
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., position: _Optional[_Union[Vector2fProto, _Mapping]] = ..., pressure: _Optional[float] = ..., shift_key: bool = ..., alt_key: bool = ..., ctrl_key: bool = ..., meta_key: bool = ..., accel_key: bool = ...) -> None: ...

class PointerEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "position", "pressure", "shift_key", "alt_key", "ctrl_key", "meta_key", "accel_key")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    PRESSURE_FIELD_NUMBER: _ClassVar[int]
    SHIFT_KEY_FIELD_NUMBER: _ClassVar[int]
    ALT_KEY_FIELD_NUMBER: _ClassVar[int]
    CTRL_KEY_FIELD_NUMBER: _ClassVar[int]
    META_KEY_FIELD_NUMBER: _ClassVar[int]
    ACCEL_KEY_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    position: Vector2fProto
    pressure: float
    shift_key: bool
    alt_key: bool
    ctrl_key: bool
    meta_key: bool
    accel_key: bool
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., position: _Optional[_Union[Vector2fProto, _Mapping]] = ..., pressure: _Optional[float] = ..., shift_key: bool = ..., alt_key: bool = ..., ctrl_key: bool = ..., meta_key: bool = ..., accel_key: bool = ...) -> None: ...

class PointerLeaveEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "position", "pressure", "shift_key", "alt_key", "ctrl_key", "meta_key", "accel_key")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    PRESSURE_FIELD_NUMBER: _ClassVar[int]
    SHIFT_KEY_FIELD_NUMBER: _ClassVar[int]
    ALT_KEY_FIELD_NUMBER: _ClassVar[int]
    CTRL_KEY_FIELD_NUMBER: _ClassVar[int]
    META_KEY_FIELD_NUMBER: _ClassVar[int]
    ACCEL_KEY_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    position: Vector2fProto
    pressure: float
    shift_key: bool
    alt_key: bool
    ctrl_key: bool
    meta_key: bool
    accel_key: bool
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., position: _Optional[_Union[Vector2fProto, _Mapping]] = ..., pressure: _Optional[float] = ..., shift_key: bool = ..., alt_key: bool = ..., ctrl_key: bool = ..., meta_key: bool = ..., accel_key: bool = ...) -> None: ...

class PointerLongPressEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "position", "pressure", "shift_key", "alt_key", "ctrl_key", "meta_key", "accel_key")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    PRESSURE_FIELD_NUMBER: _ClassVar[int]
    SHIFT_KEY_FIELD_NUMBER: _ClassVar[int]
    ALT_KEY_FIELD_NUMBER: _ClassVar[int]
    CTRL_KEY_FIELD_NUMBER: _ClassVar[int]
    META_KEY_FIELD_NUMBER: _ClassVar[int]
    ACCEL_KEY_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    position: Vector2fProto
    pressure: float
    shift_key: bool
    alt_key: bool
    ctrl_key: bool
    meta_key: bool
    accel_key: bool
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., position: _Optional[_Union[Vector2fProto, _Mapping]] = ..., pressure: _Optional[float] = ..., shift_key: bool = ..., alt_key: bool = ..., ctrl_key: bool = ..., meta_key: bool = ..., accel_key: bool = ...) -> None: ...

class PointerMoveEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "position", "pressure", "shift_key", "alt_key", "ctrl_key", "meta_key", "accel_key")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    PRESSURE_FIELD_NUMBER: _ClassVar[int]
    SHIFT_KEY_FIELD_NUMBER: _ClassVar[int]
    ALT_KEY_FIELD_NUMBER: _ClassVar[int]
    CTRL_KEY_FIELD_NUMBER: _ClassVar[int]
    META_KEY_FIELD_NUMBER: _ClassVar[int]
    ACCEL_KEY_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    position: Vector2fProto
    pressure: float
    shift_key: bool
    alt_key: bool
    ctrl_key: bool
    meta_key: bool
    accel_key: bool
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., position: _Optional[_Union[Vector2fProto, _Mapping]] = ..., pressure: _Optional[float] = ..., shift_key: bool = ..., alt_key: bool = ..., ctrl_key: bool = ..., meta_key: bool = ..., accel_key: bool = ...) -> None: ...

class PointerOverEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "position", "pressure", "shift_key", "alt_key", "ctrl_key", "meta_key", "accel_key")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    PRESSURE_FIELD_NUMBER: _ClassVar[int]
    SHIFT_KEY_FIELD_NUMBER: _ClassVar[int]
    ALT_KEY_FIELD_NUMBER: _ClassVar[int]
    CTRL_KEY_FIELD_NUMBER: _ClassVar[int]
    META_KEY_FIELD_NUMBER: _ClassVar[int]
    ACCEL_KEY_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    position: Vector2fProto
    pressure: float
    shift_key: bool
    alt_key: bool
    ctrl_key: bool
    meta_key: bool
    accel_key: bool
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., position: _Optional[_Union[Vector2fProto, _Mapping]] = ..., pressure: _Optional[float] = ..., shift_key: bool = ..., alt_key: bool = ..., ctrl_key: bool = ..., meta_key: bool = ..., accel_key: bool = ...) -> None: ...

class PointerUpEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "position", "pressure", "shift_key", "alt_key", "ctrl_key", "meta_key", "accel_key")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    PRESSURE_FIELD_NUMBER: _ClassVar[int]
    SHIFT_KEY_FIELD_NUMBER: _ClassVar[int]
    ALT_KEY_FIELD_NUMBER: _ClassVar[int]
    CTRL_KEY_FIELD_NUMBER: _ClassVar[int]
    META_KEY_FIELD_NUMBER: _ClassVar[int]
    ACCEL_KEY_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    position: Vector2fProto
    pressure: float
    shift_key: bool
    alt_key: bool
    ctrl_key: bool
    meta_key: bool
    accel_key: bool
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., position: _Optional[_Union[Vector2fProto, _Mapping]] = ..., pressure: _Optional[float] = ..., shift_key: bool = ..., alt_key: bool = ..., ctrl_key: bool = ..., meta_key: bool = ..., accel_key: bool = ...) -> None: ...

class PositionProto(_message.Message):
    __slots__ = ("metatype", "type", "top", "left", "width", "height")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    TOP_FIELD_NUMBER: _ClassVar[int]
    LEFT_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: PositionTypeProto
    top: LengthProto
    left: LengthProto
    width: LengthProto
    height: LengthProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[PositionTypeProto, str]] = ..., top: _Optional[_Union[LengthProto, _Mapping]] = ..., left: _Optional[_Union[LengthProto, _Mapping]] = ..., width: _Optional[_Union[LengthProto, _Mapping]] = ..., height: _Optional[_Union[LengthProto, _Mapping]] = ...) -> None: ...

class PropertyDefinitionProto(_message.Message):
    __slots__ = ("metatype", "id", "type", "name", "icon", "description", "object", "original_object", "group_id", "cardinality", "scalar_type", "primitive_type", "enum_type", "node_type", "struct_type", "key_type", "value", "value_factory", "collection_constraint", "string_constraint", "number_constraint", "node_constraint", "node_is_extensible", "node_is_heterogenous", "node_is_spatial", "edge_type", "cascade", "is_required", "is_unique", "is_readonly", "is_wired", "is_stored", "is_repr", "is_hash", "is_eq", "is_managed")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    OBJECT_FIELD_NUMBER: _ClassVar[int]
    ORIGINAL_OBJECT_FIELD_NUMBER: _ClassVar[int]
    GROUP_ID_FIELD_NUMBER: _ClassVar[int]
    CARDINALITY_FIELD_NUMBER: _ClassVar[int]
    SCALAR_TYPE_FIELD_NUMBER: _ClassVar[int]
    PRIMITIVE_TYPE_FIELD_NUMBER: _ClassVar[int]
    ENUM_TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    STRUCT_TYPE_FIELD_NUMBER: _ClassVar[int]
    KEY_TYPE_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    VALUE_FACTORY_FIELD_NUMBER: _ClassVar[int]
    COLLECTION_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    STRING_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    NUMBER_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    NODE_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    NODE_IS_EXTENSIBLE_FIELD_NUMBER: _ClassVar[int]
    NODE_IS_HETEROGENOUS_FIELD_NUMBER: _ClassVar[int]
    NODE_IS_SPATIAL_FIELD_NUMBER: _ClassVar[int]
    EDGE_TYPE_FIELD_NUMBER: _ClassVar[int]
    CASCADE_FIELD_NUMBER: _ClassVar[int]
    IS_REQUIRED_FIELD_NUMBER: _ClassVar[int]
    IS_UNIQUE_FIELD_NUMBER: _ClassVar[int]
    IS_READONLY_FIELD_NUMBER: _ClassVar[int]
    IS_WIRED_FIELD_NUMBER: _ClassVar[int]
    IS_STORED_FIELD_NUMBER: _ClassVar[int]
    IS_REPR_FIELD_NUMBER: _ClassVar[int]
    IS_HASH_FIELD_NUMBER: _ClassVar[int]
    IS_EQ_FIELD_NUMBER: _ClassVar[int]
    IS_MANAGED_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    id: int
    type: PropertyTypeProto
    name: str
    icon: IconProto
    description: str
    object: ObjectDefinitionReferenceProto
    original_object: ObjectDefinitionReferenceProto
    group_id: int
    cardinality: TypeCardinalityProto
    scalar_type: ScalarTypeProto
    primitive_type: PrimitiveTypeProto
    enum_type: EnumTypeProto
    node_type: NodeTypeProto
    struct_type: StructTypeProto
    key_type: TypeProto
    value: ValueProto
    value_factory: ValueFactoryProto
    collection_constraint: CollectionConstraintProto
    string_constraint: StringConstraintProto
    number_constraint: NumberConstraintProto
    node_constraint: NodeConstraintProto
    node_is_extensible: bool
    node_is_heterogenous: bool
    node_is_spatial: bool
    edge_type: EdgeTypeProto
    cascade: CascadeActionProto
    is_required: bool
    is_unique: bool
    is_readonly: bool
    is_wired: bool
    is_stored: bool
    is_repr: bool
    is_hash: bool
    is_eq: bool
    is_managed: bool
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., id: _Optional[int] = ..., type: _Optional[_Union[PropertyTypeProto, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ..., description: _Optional[str] = ..., object: _Optional[_Union[ObjectDefinitionReferenceProto, _Mapping]] = ..., original_object: _Optional[_Union[ObjectDefinitionReferenceProto, _Mapping]] = ..., group_id: _Optional[int] = ..., cardinality: _Optional[_Union[TypeCardinalityProto, str]] = ..., scalar_type: _Optional[_Union[ScalarTypeProto, str]] = ..., primitive_type: _Optional[_Union[PrimitiveTypeProto, str]] = ..., enum_type: _Optional[_Union[EnumTypeProto, str]] = ..., node_type: _Optional[_Union[NodeTypeProto, str]] = ..., struct_type: _Optional[_Union[StructTypeProto, str]] = ..., key_type: _Optional[_Union[TypeProto, _Mapping]] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ..., value_factory: _Optional[_Union[ValueFactoryProto, str]] = ..., collection_constraint: _Optional[_Union[CollectionConstraintProto, _Mapping]] = ..., string_constraint: _Optional[_Union[StringConstraintProto, _Mapping]] = ..., number_constraint: _Optional[_Union[NumberConstraintProto, _Mapping]] = ..., node_constraint: _Optional[_Union[NodeConstraintProto, _Mapping]] = ..., node_is_extensible: bool = ..., node_is_heterogenous: bool = ..., node_is_spatial: bool = ..., edge_type: _Optional[_Union[EdgeTypeProto, str]] = ..., cascade: _Optional[_Union[CascadeActionProto, str]] = ..., is_required: bool = ..., is_unique: bool = ..., is_readonly: bool = ..., is_wired: bool = ..., is_stored: bool = ..., is_repr: bool = ..., is_hash: bool = ..., is_eq: bool = ..., is_managed: bool = ...) -> None: ...

class PropertyGroupDefinitionProto(_message.Message):
    __slots__ = ("metatype", "id", "name", "icon", "description")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    id: int
    name: str
    icon: IconProto
    description: str
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., id: _Optional[int] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ..., description: _Optional[str] = ...) -> None: ...

class PropertyReferenceProto(_message.Message):
    __slots__ = ("metatype", "type", "node_type", "trait_type", "struct_type", "id", "custom_property_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    TRAIT_TYPE_FIELD_NUMBER: _ClassVar[int]
    STRUCT_TYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_PROPERTY_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: PropertyReferenceTypeProto
    node_type: NodeTypeProto
    trait_type: TraitTypeProto
    struct_type: StructTypeProto
    id: int
    custom_property_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[PropertyReferenceTypeProto, str]] = ..., node_type: _Optional[_Union[NodeTypeProto, str]] = ..., trait_type: _Optional[_Union[TraitTypeProto, str]] = ..., struct_type: _Optional[_Union[StructTypeProto, str]] = ..., id: _Optional[int] = ..., custom_property_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class QueryProto(_message.Message):
    __slots__ = ("metatype", "id", "type", "name", "definition", "subqueries", "join", "select", "where", "having", "group_by", "aggregation", "sort", "limit", "offset", "snapshot_ptr", "snapshot_path")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_FIELD_NUMBER: _ClassVar[int]
    SUBQUERIES_FIELD_NUMBER: _ClassVar[int]
    JOIN_FIELD_NUMBER: _ClassVar[int]
    SELECT_FIELD_NUMBER: _ClassVar[int]
    WHERE_FIELD_NUMBER: _ClassVar[int]
    HAVING_FIELD_NUMBER: _ClassVar[int]
    GROUP_BY_FIELD_NUMBER: _ClassVar[int]
    AGGREGATION_FIELD_NUMBER: _ClassVar[int]
    SORT_FIELD_NUMBER: _ClassVar[int]
    LIMIT_FIELD_NUMBER: _ClassVar[int]
    OFFSET_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PATH_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    id: str
    type: QueryTypeProto
    name: str
    definition: NodeDefinitionReferenceProto
    subqueries: _containers.RepeatedCompositeFieldContainer[QueryProto]
    join: JoinProto
    select: SelectProto
    where: ConditionProto
    having: ConditionProto
    group_by: _containers.RepeatedCompositeFieldContainer[ExpressionProto]
    aggregation: AggregationProto
    sort: _containers.RepeatedCompositeFieldContainer[SortProto]
    limit: int
    offset: int
    snapshot_ptr: NodeReferenceProto
    snapshot_path: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., id: _Optional[str] = ..., type: _Optional[_Union[QueryTypeProto, str]] = ..., name: _Optional[str] = ..., definition: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., subqueries: _Optional[_Iterable[_Union[QueryProto, _Mapping]]] = ..., join: _Optional[_Union[JoinProto, _Mapping]] = ..., select: _Optional[_Union[SelectProto, _Mapping]] = ..., where: _Optional[_Union[ConditionProto, _Mapping]] = ..., having: _Optional[_Union[ConditionProto, _Mapping]] = ..., group_by: _Optional[_Iterable[_Union[ExpressionProto, _Mapping]]] = ..., aggregation: _Optional[_Union[AggregationProto, _Mapping]] = ..., sort: _Optional[_Iterable[_Union[SortProto, _Mapping]]] = ..., limit: _Optional[int] = ..., offset: _Optional[int] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_path: _Optional[_Iterable[str]] = ...) -> None: ...

class QueryEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "type", "node_ptr", "name", "definition", "join", "select", "subqueries", "where", "having", "group_by", "aggregation", "sort", "limit", "offset")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_FIELD_NUMBER: _ClassVar[int]
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
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    type: QueryTypeProto
    node_ptr: NodeReferenceProto
    name: str
    definition: NodeDefinitionReferenceProto
    join: JoinProto
    select: SelectProto
    subqueries: _containers.RepeatedCompositeFieldContainer[QueryProto]
    where: ConditionProto
    having: ConditionProto
    group_by: _containers.RepeatedCompositeFieldContainer[ExpressionProto]
    aggregation: AggregationProto
    sort: _containers.RepeatedCompositeFieldContainer[SortProto]
    limit: int
    offset: int
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., type: _Optional[_Union[QueryTypeProto, str]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., definition: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., join: _Optional[_Union[JoinProto, _Mapping]] = ..., select: _Optional[_Union[SelectProto, _Mapping]] = ..., subqueries: _Optional[_Iterable[_Union[QueryProto, _Mapping]]] = ..., where: _Optional[_Union[ConditionProto, _Mapping]] = ..., having: _Optional[_Union[ConditionProto, _Mapping]] = ..., group_by: _Optional[_Iterable[_Union[ExpressionProto, _Mapping]]] = ..., aggregation: _Optional[_Union[AggregationProto, _Mapping]] = ..., sort: _Optional[_Iterable[_Union[SortProto, _Mapping]]] = ..., limit: _Optional[int] = ..., offset: _Optional[int] = ...) -> None: ...

class QueryResultProto(_message.Message):
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
    metatype: StructTypeProto
    id: str
    type: QueryTypeProto
    groups: _containers.RepeatedCompositeFieldContainer[QueryResultGroupProto]
    subresults: _containers.RepeatedCompositeFieldContainer[QueryResultProto]
    nodes: _containers.RepeatedCompositeFieldContainer[ValueProto]
    count: int
    exists: bool
    scalar: ValueProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., id: _Optional[str] = ..., type: _Optional[_Union[QueryTypeProto, str]] = ..., groups: _Optional[_Iterable[_Union[QueryResultGroupProto, _Mapping]]] = ..., subresults: _Optional[_Iterable[_Union[QueryResultProto, _Mapping]]] = ..., nodes: _Optional[_Iterable[_Union[ValueProto, _Mapping]]] = ..., count: _Optional[int] = ..., exists: bool = ..., scalar: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...

class QueryResultGroupProto(_message.Message):
    __slots__ = ("metatype", "type", "discriminator", "nodes", "count", "exists", "scalar")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    DISCRIMINATOR_FIELD_NUMBER: _ClassVar[int]
    NODES_FIELD_NUMBER: _ClassVar[int]
    COUNT_FIELD_NUMBER: _ClassVar[int]
    EXISTS_FIELD_NUMBER: _ClassVar[int]
    SCALAR_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: QueryTypeProto
    discriminator: ValueProto
    nodes: _containers.RepeatedCompositeFieldContainer[ValueProto]
    count: int
    exists: bool
    scalar: ValueProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[QueryTypeProto, str]] = ..., discriminator: _Optional[_Union[ValueProto, _Mapping]] = ..., nodes: _Optional[_Iterable[_Union[ValueProto, _Mapping]]] = ..., count: _Optional[int] = ..., exists: bool = ..., scalar: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...

class QueryUpdateProto(_message.Message):
    __slots__ = ("metatype", "type", "result")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    RESULT_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: QueryUpdateTypeProto
    result: QueryResultProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[QueryUpdateTypeProto, str]] = ..., result: _Optional[_Union[QueryResultProto, _Mapping]] = ...) -> None: ...

class ReactionProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "owned_by_ptr", "content")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    CONTENT_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    owned_by_ptr: NodeReferenceProto
    content: str
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., content: _Optional[str] = ...) -> None: ...

class RecordProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "base_type", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "custom_values", "owned_by_ptr", "script_ptr")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    owned_by_ptr: NodeReferenceProto
    script_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class ResourceProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "definition_ptr", "base_type", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "script_ptr", "status")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    script_ptr: NodeReferenceProto
    status: ResourceStatusProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., status: _Optional[_Union[ResourceStatusProto, str]] = ...) -> None: ...

class RightClickEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "position", "pressure", "shift_key", "alt_key", "ctrl_key", "meta_key", "accel_key", "button")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    PRESSURE_FIELD_NUMBER: _ClassVar[int]
    SHIFT_KEY_FIELD_NUMBER: _ClassVar[int]
    ALT_KEY_FIELD_NUMBER: _ClassVar[int]
    CTRL_KEY_FIELD_NUMBER: _ClassVar[int]
    META_KEY_FIELD_NUMBER: _ClassVar[int]
    ACCEL_KEY_FIELD_NUMBER: _ClassVar[int]
    BUTTON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    position: Vector2fProto
    pressure: float
    shift_key: bool
    alt_key: bool
    ctrl_key: bool
    meta_key: bool
    accel_key: bool
    button: MouseButtonProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., position: _Optional[_Union[Vector2fProto, _Mapping]] = ..., pressure: _Optional[float] = ..., shift_key: bool = ..., alt_key: bool = ..., ctrl_key: bool = ..., meta_key: bool = ..., accel_key: bool = ..., button: _Optional[_Union[MouseButtonProto, str]] = ...) -> None: ...

class RoleProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "order_key", "type", "name", "icon")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    order_key: str
    type: RoleTypeProto
    name: str
    icon: IconProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[RoleTypeProto, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ...) -> None: ...

class RoleAssignedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "subject_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    SUBJECT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    subject_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., subject_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class RoleEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "subject_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    SUBJECT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    subject_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., subject_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class RoleUnassignedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "subject_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    SUBJECT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    subject_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., subject_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class RouteProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "order_key", "owned_by_ptr", "name", "scene_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SCENE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    order_key: str
    owned_by_ptr: NodeReferenceProto
    name: str
    scene_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., order_key: _Optional[str] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., scene_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class RunProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "custom_values", "target_ptr", "status", "duration", "scheduled_at", "started_at", "seen_at", "interrupted_at", "terminated_at")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    SCHEDULED_AT_FIELD_NUMBER: _ClassVar[int]
    STARTED_AT_FIELD_NUMBER: _ClassVar[int]
    SEEN_AT_FIELD_NUMBER: _ClassVar[int]
    INTERRUPTED_AT_FIELD_NUMBER: _ClassVar[int]
    TERMINATED_AT_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    custom_values: _containers.MessageMap[str, ValueProto]
    target_ptr: NodeReferenceProto
    status: RunStatusProto
    duration: _duration_pb2.Duration
    scheduled_at: _timestamp_pb2.Timestamp
    started_at: _timestamp_pb2.Timestamp
    seen_at: _timestamp_pb2.Timestamp
    interrupted_at: _timestamp_pb2.Timestamp
    terminated_at: _timestamp_pb2.Timestamp
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., target_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., status: _Optional[_Union[RunStatusProto, str]] = ..., duration: _Optional[_Union[_duration_pb2.Duration, _Mapping]] = ..., scheduled_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., started_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., seen_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., interrupted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., terminated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...) -> None: ...

class RunCompletedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "target_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    target_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class RunEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "target_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    target_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class RunFailedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "target_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    target_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class RunPauseRequestedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "target_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    target_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class RunPausedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "target_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    target_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class RunResumeRequestedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "target_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    target_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class RunResumedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "target_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    target_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class RunStartedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "target_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    target_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class RunStopRequestedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "target_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    target_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class SanctionProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "type", "expires_at", "target_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    EXPIRES_AT_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    type: SanctionTypeProto
    expires_at: _timestamp_pb2.Timestamp
    target_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., type: _Optional[_Union[SanctionTypeProto, str]] = ..., expires_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class SanctionEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "target_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    target_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class SanctionExpiredEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "target_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    target_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class SanctionGrantedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "target_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    target_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class SanctionRequestedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "target_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    target_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class SanctionRevokedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "target_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    target_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class SceneProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "base_type", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "owned_by_ptr", "script_ptr", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "layout", "direction", "distribute", "align", "gap", "padding", "grid", "grid_span", "aspect_ratio", "is_wrap", "is_visible", "opacity", "fill", "rotation", "skew", "scale", "shadow", "border", "radius", "root_view_ptr")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
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
    ROOT_VIEW_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    owned_by_ptr: NodeReferenceProto
    script_ptr: NodeReferenceProto
    name: str
    position: PositionProto
    width: DimensionProto
    height: DimensionProto
    min_width: DimensionProto
    min_height: DimensionProto
    max_width: DimensionProto
    max_height: DimensionProto
    layout: LayoutProto
    direction: DirectionProto
    distribute: DistributeProto
    align: AlignProto
    gap: Axis2Proto
    padding: InsetsProto
    grid: GridProto
    grid_span: GridSpanProto
    aspect_ratio: float
    is_wrap: bool
    is_visible: bool
    opacity: float
    fill: FillProto
    rotation: Axis3Proto
    skew: Vector2fProto
    scale: float
    shadow: ShadowProto
    border: BorderProto
    radius: CornersProto
    root_view_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionProto, _Mapping]] = ..., width: _Optional[_Union[DimensionProto, _Mapping]] = ..., height: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., layout: _Optional[_Union[LayoutProto, str]] = ..., direction: _Optional[_Union[DirectionProto, str]] = ..., distribute: _Optional[_Union[DistributeProto, str]] = ..., align: _Optional[_Union[AlignProto, str]] = ..., gap: _Optional[_Union[Axis2Proto, _Mapping]] = ..., padding: _Optional[_Union[InsetsProto, _Mapping]] = ..., grid: _Optional[_Union[GridProto, _Mapping]] = ..., grid_span: _Optional[_Union[GridSpanProto, _Mapping]] = ..., aspect_ratio: _Optional[float] = ..., is_wrap: bool = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., fill: _Optional[_Union[FillProto, _Mapping]] = ..., rotation: _Optional[_Union[Axis3Proto, _Mapping]] = ..., skew: _Optional[_Union[Vector2fProto, _Mapping]] = ..., scale: _Optional[float] = ..., shadow: _Optional[_Union[ShadowProto, _Mapping]] = ..., border: _Optional[_Union[BorderProto, _Mapping]] = ..., radius: _Optional[_Union[CornersProto, _Mapping]] = ..., root_view_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class SceneEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class ScheduleProto(_message.Message):
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
    metatype: StructTypeProto
    frequency: ScheduleFrequencyProto
    interval: int
    start: _timestamp_pb2.Timestamp
    end: _timestamp_pb2.Timestamp
    count: int
    week_start: DayOfWeekProto
    by_set_pos: _containers.RepeatedScalarFieldContainer[int]
    by_month: _containers.RepeatedScalarFieldContainer[MonthProto]
    by_month_day: _containers.RepeatedScalarFieldContainer[int]
    by_year_day: _containers.RepeatedScalarFieldContainer[int]
    by_easter: _containers.RepeatedScalarFieldContainer[int]
    by_week_no: _containers.RepeatedScalarFieldContainer[int]
    by_week_day: _containers.RepeatedScalarFieldContainer[DayOfWeekProto]
    by_hour: _containers.RepeatedScalarFieldContainer[int]
    by_minute: _containers.RepeatedScalarFieldContainer[int]
    by_second: _containers.RepeatedScalarFieldContainer[int]
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., frequency: _Optional[_Union[ScheduleFrequencyProto, str]] = ..., interval: _Optional[int] = ..., start: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., end: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., count: _Optional[int] = ..., week_start: _Optional[_Union[DayOfWeekProto, str]] = ..., by_set_pos: _Optional[_Iterable[int]] = ..., by_month: _Optional[_Iterable[_Union[MonthProto, str]]] = ..., by_month_day: _Optional[_Iterable[int]] = ..., by_year_day: _Optional[_Iterable[int]] = ..., by_easter: _Optional[_Iterable[int]] = ..., by_week_no: _Optional[_Iterable[int]] = ..., by_week_day: _Optional[_Iterable[_Union[DayOfWeekProto, str]]] = ..., by_hour: _Optional[_Iterable[int]] = ..., by_minute: _Optional[_Iterable[int]] = ..., by_second: _Optional[_Iterable[int]] = ...) -> None: ...

class ScreenCursorProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "owned_by_ptr", "status", "active_at", "position")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_AT_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    owned_by_ptr: NodeReferenceProto
    status: CursorStatusProto
    active_at: _timestamp_pb2.Timestamp
    position: Vector2iProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., status: _Optional[_Union[CursorStatusProto, str]] = ..., active_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., position: _Optional[_Union[Vector2iProto, _Mapping]] = ...) -> None: ...

class ScriptProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "name", "code")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    CODE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    name: str
    code: str
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., code: _Optional[str] = ...) -> None: ...

class SelectProto(_message.Message):
    __slots__ = ("metatype", "attributes")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ATTRIBUTES_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    attributes: _containers.RepeatedCompositeFieldContainer[PropertyReferenceProto]
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., attributes: _Optional[_Iterable[_Union[PropertyReferenceProto, _Mapping]]] = ...) -> None: ...

class SelectionProto(_message.Message):
    __slots__ = ("metatype",)
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ...) -> None: ...

class ServiceProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "base_type", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "owned_by_ptr", "source_ptr", "script_ptr", "name", "icon")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    SOURCE_PTR_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    owned_by_ptr: NodeReferenceProto
    source_ptr: NodeReferenceProto
    script_ptr: NodeReferenceProto
    name: str
    icon: IconProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., source_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ...) -> None: ...

class ShadowProto(_message.Message):
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
    metatype: StructTypeProto
    type: ShadowTypeProto
    style_ptr: NodeReferenceProto
    color: ColorProto
    position: ShadowPositionProto
    offset: Axis2Proto
    blur: int
    spread: int
    diffusion: float
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[ShadowTypeProto, str]] = ..., style_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., color: _Optional[_Union[ColorProto, _Mapping]] = ..., position: _Optional[_Union[ShadowPositionProto, str]] = ..., offset: _Optional[_Union[Axis2Proto, _Mapping]] = ..., blur: _Optional[int] = ..., spread: _Optional[int] = ..., diffusion: _Optional[float] = ...) -> None: ...

class ShadowStyleProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "order_key", "type", "name", "color", "position", "offset", "blur", "spread", "diffusion")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    OFFSET_FIELD_NUMBER: _ClassVar[int]
    BLUR_FIELD_NUMBER: _ClassVar[int]
    SPREAD_FIELD_NUMBER: _ClassVar[int]
    DIFFUSION_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    order_key: str
    type: ShadowTypeProto
    name: str
    color: ColorProto
    position: ShadowPositionProto
    offset: Axis2Proto
    blur: int
    spread: int
    diffusion: float
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[ShadowTypeProto, str]] = ..., name: _Optional[str] = ..., color: _Optional[_Union[ColorProto, _Mapping]] = ..., position: _Optional[_Union[ShadowPositionProto, str]] = ..., offset: _Optional[_Union[Axis2Proto, _Mapping]] = ..., blur: _Optional[int] = ..., spread: _Optional[int] = ..., diffusion: _Optional[float] = ...) -> None: ...

class ShapeProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "base_type", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "script_ptr", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "layout", "direction", "distribute", "align", "gap", "padding", "grid", "grid_span", "aspect_ratio", "is_wrap", "is_visible", "opacity", "fill", "rotation", "skew", "scale", "shadow", "border", "radius", "stroke")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
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
    STROKE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    script_ptr: NodeReferenceProto
    name: str
    position: PositionProto
    width: DimensionProto
    height: DimensionProto
    min_width: DimensionProto
    min_height: DimensionProto
    max_width: DimensionProto
    max_height: DimensionProto
    layout: LayoutProto
    direction: DirectionProto
    distribute: DistributeProto
    align: AlignProto
    gap: Axis2Proto
    padding: InsetsProto
    grid: GridProto
    grid_span: GridSpanProto
    aspect_ratio: float
    is_wrap: bool
    is_visible: bool
    opacity: float
    fill: FillProto
    rotation: Axis3Proto
    skew: Vector2fProto
    scale: float
    shadow: ShadowProto
    border: BorderProto
    radius: CornersProto
    stroke: StrokeProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionProto, _Mapping]] = ..., width: _Optional[_Union[DimensionProto, _Mapping]] = ..., height: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., layout: _Optional[_Union[LayoutProto, str]] = ..., direction: _Optional[_Union[DirectionProto, str]] = ..., distribute: _Optional[_Union[DistributeProto, str]] = ..., align: _Optional[_Union[AlignProto, str]] = ..., gap: _Optional[_Union[Axis2Proto, _Mapping]] = ..., padding: _Optional[_Union[InsetsProto, _Mapping]] = ..., grid: _Optional[_Union[GridProto, _Mapping]] = ..., grid_span: _Optional[_Union[GridSpanProto, _Mapping]] = ..., aspect_ratio: _Optional[float] = ..., is_wrap: bool = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., fill: _Optional[_Union[FillProto, _Mapping]] = ..., rotation: _Optional[_Union[Axis3Proto, _Mapping]] = ..., skew: _Optional[_Union[Vector2fProto, _Mapping]] = ..., scale: _Optional[float] = ..., shadow: _Optional[_Union[ShadowProto, _Mapping]] = ..., border: _Optional[_Union[BorderProto, _Mapping]] = ..., radius: _Optional[_Union[CornersProto, _Mapping]] = ..., stroke: _Optional[_Union[StrokeProto, _Mapping]] = ...) -> None: ...

class SignalProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "base_type", "snapshot_ptr", "created_at", "created_by_ptr", "custom_values", "script_ptr", "node_ptr")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    custom_values: _containers.MessageMap[str, ValueProto]
    script_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class SliderInputViewProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "base_type", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "script_ptr", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "is_visible", "opacity", "value", "min_value", "max_value", "step")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
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
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    script_ptr: NodeReferenceProto
    name: str
    position: PositionProto
    width: DimensionProto
    height: DimensionProto
    min_width: DimensionProto
    min_height: DimensionProto
    max_width: DimensionProto
    max_height: DimensionProto
    is_visible: bool
    opacity: float
    value: float
    min_value: float
    max_value: float
    step: float
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionProto, _Mapping]] = ..., width: _Optional[_Union[DimensionProto, _Mapping]] = ..., height: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., value: _Optional[float] = ..., min_value: _Optional[float] = ..., max_value: _Optional[float] = ..., step: _Optional[float] = ...) -> None: ...

class SnapshotProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "archived_at", "deleted_at", "owned_by_ptr", "type", "name", "status")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    ARCHIVED_AT_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    archived_at: _timestamp_pb2.Timestamp
    deleted_at: _timestamp_pb2.Timestamp
    owned_by_ptr: NodeReferenceProto
    type: SnapshotTypeProto
    name: str
    status: SnapshotStatusProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., archived_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., type: _Optional[_Union[SnapshotTypeProto, str]] = ..., name: _Optional[str] = ..., status: _Optional[_Union[SnapshotStatusProto, str]] = ...) -> None: ...

class SortProto(_message.Message):
    __slots__ = ("metatype", "type", "by", "mode")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    BY_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: SortTypeProto
    by: ExpressionProto
    mode: SortModeProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[SortTypeProto, str]] = ..., by: _Optional[_Union[ExpressionProto, _Mapping]] = ..., mode: _Optional[_Union[SortModeProto, str]] = ...) -> None: ...

class SpaceProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "owned_by_ptr", "name", "slug", "status", "handle_ptr", "system_folder_ptr", "home_folder_ptr", "region", "galaxy_name", "database_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    HANDLE_PTR_FIELD_NUMBER: _ClassVar[int]
    SYSTEM_FOLDER_PTR_FIELD_NUMBER: _ClassVar[int]
    HOME_FOLDER_PTR_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    GALAXY_NAME_FIELD_NUMBER: _ClassVar[int]
    DATABASE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    owned_by_ptr: NodeReferenceProto
    name: str
    slug: str
    status: SpaceStatusProto
    handle_ptr: NodeReferenceProto
    system_folder_ptr: NodeReferenceProto
    home_folder_ptr: NodeReferenceProto
    region: RegionProto
    galaxy_name: str
    database_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., slug: _Optional[str] = ..., status: _Optional[_Union[SpaceStatusProto, str]] = ..., handle_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., system_folder_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., home_folder_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., region: _Optional[_Union[RegionProto, str]] = ..., galaxy_name: _Optional[str] = ..., database_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class SpanEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class SplitViewProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "base_type", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "script_ptr", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "layout", "direction", "distribute", "align", "gap", "padding", "grid", "grid_span", "aspect_ratio", "is_wrap", "is_visible", "opacity", "fill", "rotation", "skew", "scale", "shadow", "border", "radius")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
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
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    script_ptr: NodeReferenceProto
    name: str
    position: PositionProto
    width: DimensionProto
    height: DimensionProto
    min_width: DimensionProto
    min_height: DimensionProto
    max_width: DimensionProto
    max_height: DimensionProto
    layout: LayoutProto
    direction: DirectionProto
    distribute: DistributeProto
    align: AlignProto
    gap: Axis2Proto
    padding: InsetsProto
    grid: GridProto
    grid_span: GridSpanProto
    aspect_ratio: float
    is_wrap: bool
    is_visible: bool
    opacity: float
    fill: FillProto
    rotation: Axis3Proto
    skew: Vector2fProto
    scale: float
    shadow: ShadowProto
    border: BorderProto
    radius: CornersProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionProto, _Mapping]] = ..., width: _Optional[_Union[DimensionProto, _Mapping]] = ..., height: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., layout: _Optional[_Union[LayoutProto, str]] = ..., direction: _Optional[_Union[DirectionProto, str]] = ..., distribute: _Optional[_Union[DistributeProto, str]] = ..., align: _Optional[_Union[AlignProto, str]] = ..., gap: _Optional[_Union[Axis2Proto, _Mapping]] = ..., padding: _Optional[_Union[InsetsProto, _Mapping]] = ..., grid: _Optional[_Union[GridProto, _Mapping]] = ..., grid_span: _Optional[_Union[GridSpanProto, _Mapping]] = ..., aspect_ratio: _Optional[float] = ..., is_wrap: bool = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., fill: _Optional[_Union[FillProto, _Mapping]] = ..., rotation: _Optional[_Union[Axis3Proto, _Mapping]] = ..., skew: _Optional[_Union[Vector2fProto, _Mapping]] = ..., scale: _Optional[float] = ..., shadow: _Optional[_Union[ShadowProto, _Mapping]] = ..., border: _Optional[_Union[BorderProto, _Mapping]] = ..., radius: _Optional[_Union[CornersProto, _Mapping]] = ...) -> None: ...

class StarProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "owned_by_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    owned_by_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class StringConstraintProto(_message.Message):
    __slots__ = ("metatype", "format", "regex", "starts_with", "ends_with")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    FORMAT_FIELD_NUMBER: _ClassVar[int]
    REGEX_FIELD_NUMBER: _ClassVar[int]
    STARTS_WITH_FIELD_NUMBER: _ClassVar[int]
    ENDS_WITH_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    format: StringFormatProto
    regex: str
    starts_with: str
    ends_with: str
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., format: _Optional[_Union[StringFormatProto, str]] = ..., regex: _Optional[str] = ..., starts_with: _Optional[str] = ..., ends_with: _Optional[str] = ...) -> None: ...

class StrokeProto(_message.Message):
    __slots__ = ("metatype", "type", "size", "thinning", "smoothing", "streamline", "easing", "color", "start", "end")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    SIZE_FIELD_NUMBER: _ClassVar[int]
    THINNING_FIELD_NUMBER: _ClassVar[int]
    SMOOTHING_FIELD_NUMBER: _ClassVar[int]
    STREAMLINE_FIELD_NUMBER: _ClassVar[int]
    EASING_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    START_FIELD_NUMBER: _ClassVar[int]
    END_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: StrokeTypeProto
    size: int
    thinning: float
    smoothing: float
    streamline: float
    easing: EasingProto
    color: ColorProto
    start: StrokeCapProto
    end: StrokeCapProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[StrokeTypeProto, str]] = ..., size: _Optional[int] = ..., thinning: _Optional[float] = ..., smoothing: _Optional[float] = ..., streamline: _Optional[float] = ..., easing: _Optional[_Union[EasingProto, str]] = ..., color: _Optional[_Union[ColorProto, _Mapping]] = ..., start: _Optional[_Union[StrokeCapProto, _Mapping]] = ..., end: _Optional[_Union[StrokeCapProto, _Mapping]] = ...) -> None: ...

class StrokeCapProto(_message.Message):
    __slots__ = ("metatype", "cap", "taper", "easing")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    CAP_FIELD_NUMBER: _ClassVar[int]
    TAPER_FIELD_NUMBER: _ClassVar[int]
    EASING_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    cap: bool
    taper: bool
    easing: EasingProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., cap: bool = ..., taper: bool = ..., easing: _Optional[_Union[EasingProto, str]] = ...) -> None: ...

class StrokePathProto(_message.Message):
    __slots__ = ("metatype", "points")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    POINTS_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    points: _containers.RepeatedCompositeFieldContainer[StrokePointProto]
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., points: _Optional[_Iterable[_Union[StrokePointProto, _Mapping]]] = ...) -> None: ...

class StrokePointProto(_message.Message):
    __slots__ = ("metatype", "point", "original_point", "pressure", "direction", "distance", "running_length", "radius")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    POINT_FIELD_NUMBER: _ClassVar[int]
    ORIGINAL_POINT_FIELD_NUMBER: _ClassVar[int]
    PRESSURE_FIELD_NUMBER: _ClassVar[int]
    DIRECTION_FIELD_NUMBER: _ClassVar[int]
    DISTANCE_FIELD_NUMBER: _ClassVar[int]
    RUNNING_LENGTH_FIELD_NUMBER: _ClassVar[int]
    RADIUS_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    point: Vector2fProto
    original_point: Vector2fProto
    pressure: float
    direction: Vector2fProto
    distance: float
    running_length: float
    radius: float
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., point: _Optional[_Union[Vector2fProto, _Mapping]] = ..., original_point: _Optional[_Union[Vector2fProto, _Mapping]] = ..., pressure: _Optional[float] = ..., direction: _Optional[_Union[Vector2fProto, _Mapping]] = ..., distance: _Optional[float] = ..., running_length: _Optional[float] = ..., radius: _Optional[float] = ...) -> None: ...

class StrokeStyleProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "order_key", "type", "name", "size", "thinning", "smoothing", "streamline", "easing", "start", "end")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SIZE_FIELD_NUMBER: _ClassVar[int]
    THINNING_FIELD_NUMBER: _ClassVar[int]
    SMOOTHING_FIELD_NUMBER: _ClassVar[int]
    STREAMLINE_FIELD_NUMBER: _ClassVar[int]
    EASING_FIELD_NUMBER: _ClassVar[int]
    START_FIELD_NUMBER: _ClassVar[int]
    END_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    order_key: str
    type: StrokeTypeProto
    name: str
    size: int
    thinning: float
    smoothing: float
    streamline: float
    easing: EasingProto
    start: StrokeCapProto
    end: StrokeCapProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[StrokeTypeProto, str]] = ..., name: _Optional[str] = ..., size: _Optional[int] = ..., thinning: _Optional[float] = ..., smoothing: _Optional[float] = ..., streamline: _Optional[float] = ..., easing: _Optional[_Union[EasingProto, str]] = ..., start: _Optional[_Union[StrokeCapProto, _Mapping]] = ..., end: _Optional[_Union[StrokeCapProto, _Mapping]] = ...) -> None: ...

class StructProto(_message.Message):
    __slots__ = ("metatype",)
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ...) -> None: ...

class StructDefinitionProto(_message.Message):
    __slots__ = ("metatype", "id", "type", "name", "icon", "description", "properties", "groups", "is_frozen", "is_abstract", "is_extensible", "base_type", "extended_by", "inherits", "inherited_by")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    GROUPS_FIELD_NUMBER: _ClassVar[int]
    IS_FROZEN_FIELD_NUMBER: _ClassVar[int]
    IS_ABSTRACT_FIELD_NUMBER: _ClassVar[int]
    IS_EXTENSIBLE_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    EXTENDED_BY_FIELD_NUMBER: _ClassVar[int]
    INHERITS_FIELD_NUMBER: _ClassVar[int]
    INHERITED_BY_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    id: int
    type: StructTypeProto
    name: str
    icon: IconProto
    description: str
    properties: _containers.RepeatedCompositeFieldContainer[PropertyDefinitionProto]
    groups: _containers.RepeatedCompositeFieldContainer[PropertyGroupDefinitionProto]
    is_frozen: bool
    is_abstract: bool
    is_extensible: bool
    base_type: StructTypeProto
    extended_by: _containers.RepeatedScalarFieldContainer[StructTypeProto]
    inherits: _containers.RepeatedScalarFieldContainer[StructTypeProto]
    inherited_by: _containers.RepeatedScalarFieldContainer[StructTypeProto]
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., id: _Optional[int] = ..., type: _Optional[_Union[StructTypeProto, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ..., description: _Optional[str] = ..., properties: _Optional[_Iterable[_Union[PropertyDefinitionProto, _Mapping]]] = ..., groups: _Optional[_Iterable[_Union[PropertyGroupDefinitionProto, _Mapping]]] = ..., is_frozen: bool = ..., is_abstract: bool = ..., is_extensible: bool = ..., base_type: _Optional[_Union[StructTypeProto, str]] = ..., extended_by: _Optional[_Iterable[_Union[StructTypeProto, str]]] = ..., inherits: _Optional[_Iterable[_Union[StructTypeProto, str]]] = ..., inherited_by: _Optional[_Iterable[_Union[StructTypeProto, str]]] = ...) -> None: ...

class StructDefinitionReferenceProto(_message.Message):
    __slots__ = ("metatype", "type", "struct_type", "definition_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    STRUCT_TYPE_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: StructDefinitionTypeProto
    struct_type: StructTypeProto
    definition_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[StructDefinitionTypeProto, str]] = ..., struct_type: _Optional[_Union[StructTypeProto, str]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class StyleProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "order_key", "name")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    order_key: str
    name: str
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ...) -> None: ...

class TagProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "order_key", "name", "icon")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    order_key: str
    name: str
    icon: IconProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ...) -> None: ...

class TaggingProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "order_key", "tag_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TAG_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    order_key: str
    tag_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., order_key: _Optional[str] = ..., tag_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class TeamProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "name", "slug")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    name: str
    slug: str
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., slug: _Optional[str] = ...) -> None: ...

class TextProto(_message.Message):
    __slots__ = ("metatype", "spans", "is_bold", "is_italic", "is_strikethrough", "is_underline", "is_code")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    SPANS_FIELD_NUMBER: _ClassVar[int]
    IS_BOLD_FIELD_NUMBER: _ClassVar[int]
    IS_ITALIC_FIELD_NUMBER: _ClassVar[int]
    IS_STRIKETHROUGH_FIELD_NUMBER: _ClassVar[int]
    IS_UNDERLINE_FIELD_NUMBER: _ClassVar[int]
    IS_CODE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    spans: _containers.RepeatedCompositeFieldContainer[TextSpanProto]
    is_bold: bool
    is_italic: bool
    is_strikethrough: bool
    is_underline: bool
    is_code: bool
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., spans: _Optional[_Iterable[_Union[TextSpanProto, _Mapping]]] = ..., is_bold: bool = ..., is_italic: bool = ..., is_strikethrough: bool = ..., is_underline: bool = ..., is_code: bool = ...) -> None: ...

class TextSpanProto(_message.Message):
    __slots__ = ("metatype", "type", "content", "node_ptr", "url", "is_bold", "is_italic", "is_strikethrough", "is_underline", "is_code")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    CONTENT_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    URL_FIELD_NUMBER: _ClassVar[int]
    IS_BOLD_FIELD_NUMBER: _ClassVar[int]
    IS_ITALIC_FIELD_NUMBER: _ClassVar[int]
    IS_STRIKETHROUGH_FIELD_NUMBER: _ClassVar[int]
    IS_UNDERLINE_FIELD_NUMBER: _ClassVar[int]
    IS_CODE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: TextSpanTypeProto
    content: str
    node_ptr: NodeReferenceProto
    url: str
    is_bold: bool
    is_italic: bool
    is_strikethrough: bool
    is_underline: bool
    is_code: bool
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[TextSpanTypeProto, str]] = ..., content: _Optional[str] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., url: _Optional[str] = ..., is_bold: bool = ..., is_italic: bool = ..., is_strikethrough: bool = ..., is_underline: bool = ..., is_code: bool = ...) -> None: ...

class TextViewProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "base_type", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "script_ptr", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height", "align", "is_visible", "opacity", "font", "color", "text")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
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
    FONT_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    script_ptr: NodeReferenceProto
    name: str
    position: PositionProto
    width: DimensionProto
    height: DimensionProto
    min_width: DimensionProto
    min_height: DimensionProto
    max_width: DimensionProto
    max_height: DimensionProto
    align: AlignProto
    is_visible: bool
    opacity: float
    font: FontProto
    color: FillProto
    text: TextProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionProto, _Mapping]] = ..., width: _Optional[_Union[DimensionProto, _Mapping]] = ..., height: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., align: _Optional[_Union[AlignProto, str]] = ..., is_visible: bool = ..., opacity: _Optional[float] = ..., font: _Optional[_Union[FontProto, _Mapping]] = ..., color: _Optional[_Union[FillProto, _Mapping]] = ..., text: _Optional[_Union[TextProto, _Mapping]] = ...) -> None: ...

class ThemeProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "order_key", "name")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    order_key: str
    name: str
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., order_key: _Optional[str] = ..., name: _Optional[str] = ...) -> None: ...

class ThreadProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "owned_by_ptr", "name")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    owned_by_ptr: NodeReferenceProto
    name: str
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ...) -> None: ...

class ThreadCursorProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "owned_by_ptr", "status", "active_at")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    ACTIVE_AT_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    owned_by_ptr: NodeReferenceProto
    status: CursorStatusProto
    active_at: _timestamp_pb2.Timestamp
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., status: _Optional[_Union[CursorStatusProto, str]] = ..., active_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...) -> None: ...

class TimerProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "type", "name", "schedule")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SCHEDULE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    type: TimerTypeProto
    name: str
    schedule: ScheduleProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., type: _Optional[_Union[TimerTypeProto, str]] = ..., name: _Optional[str] = ..., schedule: _Optional[_Union[ScheduleProto, _Mapping]] = ...) -> None: ...

class TimerCancelledEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class TimerCompletedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class TimerEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class TimerStartedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class TraitDefinitionProto(_message.Message):
    __slots__ = ("metatype", "id", "type", "name", "icon", "description", "properties", "groups", "alias", "is_extensible", "traits", "base_traits", "event_types", "base_event_types")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    DESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    PROPERTIES_FIELD_NUMBER: _ClassVar[int]
    GROUPS_FIELD_NUMBER: _ClassVar[int]
    ALIAS_FIELD_NUMBER: _ClassVar[int]
    IS_EXTENSIBLE_FIELD_NUMBER: _ClassVar[int]
    TRAITS_FIELD_NUMBER: _ClassVar[int]
    BASE_TRAITS_FIELD_NUMBER: _ClassVar[int]
    EVENT_TYPES_FIELD_NUMBER: _ClassVar[int]
    BASE_EVENT_TYPES_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    id: int
    type: TraitTypeProto
    name: str
    icon: IconProto
    description: str
    properties: _containers.RepeatedCompositeFieldContainer[PropertyDefinitionProto]
    groups: _containers.RepeatedCompositeFieldContainer[PropertyGroupDefinitionProto]
    alias: str
    is_extensible: bool
    traits: _containers.RepeatedScalarFieldContainer[TraitTypeProto]
    base_traits: _containers.RepeatedScalarFieldContainer[TraitTypeProto]
    event_types: _containers.RepeatedScalarFieldContainer[NodeTypeProto]
    base_event_types: _containers.RepeatedScalarFieldContainer[NodeTypeProto]
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., id: _Optional[int] = ..., type: _Optional[_Union[TraitTypeProto, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ..., description: _Optional[str] = ..., properties: _Optional[_Iterable[_Union[PropertyDefinitionProto, _Mapping]]] = ..., groups: _Optional[_Iterable[_Union[PropertyGroupDefinitionProto, _Mapping]]] = ..., alias: _Optional[str] = ..., is_extensible: bool = ..., traits: _Optional[_Iterable[_Union[TraitTypeProto, str]]] = ..., base_traits: _Optional[_Iterable[_Union[TraitTypeProto, str]]] = ..., event_types: _Optional[_Iterable[_Union[NodeTypeProto, str]]] = ..., base_event_types: _Optional[_Iterable[_Union[NodeTypeProto, str]]] = ...) -> None: ...

class TransitionProto(_message.Message):
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
    metatype: StructTypeProto
    type: TransitionTypeProto
    style_ptr: NodeReferenceProto
    delay: float
    duration: float
    ease: _containers.RepeatedScalarFieldContainer[float]
    stiffness: float
    damping: float
    mass: float
    bounce: float
    spring_type: SpringTypeProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[TransitionTypeProto, str]] = ..., style_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., delay: _Optional[float] = ..., duration: _Optional[float] = ..., ease: _Optional[_Iterable[float]] = ..., stiffness: _Optional[float] = ..., damping: _Optional[float] = ..., mass: _Optional[float] = ..., bounce: _Optional[float] = ..., spring_type: _Optional[_Union[SpringTypeProto, str]] = ...) -> None: ...

class TransitionStyleProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "order_key", "type", "name", "delay", "duration", "ease", "stiffness", "damping", "mass", "bounce", "spring_type")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    DELAY_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    EASE_FIELD_NUMBER: _ClassVar[int]
    STIFFNESS_FIELD_NUMBER: _ClassVar[int]
    DAMPING_FIELD_NUMBER: _ClassVar[int]
    MASS_FIELD_NUMBER: _ClassVar[int]
    BOUNCE_FIELD_NUMBER: _ClassVar[int]
    SPRING_TYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    order_key: str
    type: TransitionTypeProto
    name: str
    delay: float
    duration: float
    ease: _containers.RepeatedScalarFieldContainer[float]
    stiffness: float
    damping: float
    mass: float
    bounce: float
    spring_type: SpringTypeProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., order_key: _Optional[str] = ..., type: _Optional[_Union[TransitionTypeProto, str]] = ..., name: _Optional[str] = ..., delay: _Optional[float] = ..., duration: _Optional[float] = ..., ease: _Optional[_Iterable[float]] = ..., stiffness: _Optional[float] = ..., damping: _Optional[float] = ..., mass: _Optional[float] = ..., bounce: _Optional[float] = ..., spring_type: _Optional[_Union[SpringTypeProto, str]] = ...) -> None: ...

class TriggerProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "name", "icon", "event", "where", "target_ptr", "arguments")
    class ArgumentsEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    EVENT_FIELD_NUMBER: _ClassVar[int]
    WHERE_FIELD_NUMBER: _ClassVar[int]
    TARGET_PTR_FIELD_NUMBER: _ClassVar[int]
    ARGUMENTS_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    name: str
    icon: IconProto
    event: NodeDefinitionReferenceProto
    where: ConditionProto
    target_ptr: NodeReferenceProto
    arguments: _containers.MessageMap[str, ValueProto]
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ..., event: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., where: _Optional[_Union[ConditionProto, _Mapping]] = ..., target_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., arguments: _Optional[_Mapping[str, ValueProto]] = ...) -> None: ...

class TriggerEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class TypeProto(_message.Message):
    __slots__ = ("metatype", "cardinality", "scalar_type", "primitive_type", "enum_type", "node_type", "struct_type", "definition_ptr", "key_type", "is_required", "value", "value_factory", "collection_constraint", "string_constraint", "number_constraint", "node_constraint")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    CARDINALITY_FIELD_NUMBER: _ClassVar[int]
    SCALAR_TYPE_FIELD_NUMBER: _ClassVar[int]
    PRIMITIVE_TYPE_FIELD_NUMBER: _ClassVar[int]
    ENUM_TYPE_FIELD_NUMBER: _ClassVar[int]
    NODE_TYPE_FIELD_NUMBER: _ClassVar[int]
    STRUCT_TYPE_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    KEY_TYPE_FIELD_NUMBER: _ClassVar[int]
    IS_REQUIRED_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    VALUE_FACTORY_FIELD_NUMBER: _ClassVar[int]
    COLLECTION_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    STRING_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    NUMBER_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    NODE_CONSTRAINT_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    cardinality: TypeCardinalityProto
    scalar_type: ScalarTypeProto
    primitive_type: PrimitiveTypeProto
    enum_type: EnumTypeProto
    node_type: NodeTypeProto
    struct_type: StructTypeProto
    definition_ptr: NodeReferenceProto
    key_type: TypeProto
    is_required: bool
    value: ValueProto
    value_factory: ValueFactoryProto
    collection_constraint: CollectionConstraintProto
    string_constraint: StringConstraintProto
    number_constraint: NumberConstraintProto
    node_constraint: NodeConstraintProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., cardinality: _Optional[_Union[TypeCardinalityProto, str]] = ..., scalar_type: _Optional[_Union[ScalarTypeProto, str]] = ..., primitive_type: _Optional[_Union[PrimitiveTypeProto, str]] = ..., enum_type: _Optional[_Union[EnumTypeProto, str]] = ..., node_type: _Optional[_Union[NodeTypeProto, str]] = ..., struct_type: _Optional[_Union[StructTypeProto, str]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., key_type: _Optional[_Union[TypeProto, _Mapping]] = ..., is_required: bool = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ..., value_factory: _Optional[_Union[ValueFactoryProto, str]] = ..., collection_constraint: _Optional[_Union[CollectionConstraintProto, _Mapping]] = ..., string_constraint: _Optional[_Union[StringConstraintProto, _Mapping]] = ..., number_constraint: _Optional[_Union[NumberConstraintProto, _Mapping]] = ..., node_constraint: _Optional[_Union[NodeConstraintProto, _Mapping]] = ...) -> None: ...

class UniverseProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class UserProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "custom_values", "name", "slug", "status", "last_logged_in_at", "is_staff", "space_ptr", "handle_ptr", "cursor_ptr", "email", "password_salt", "password_hash")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SLUG_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    LAST_LOGGED_IN_AT_FIELD_NUMBER: _ClassVar[int]
    IS_STAFF_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    HANDLE_PTR_FIELD_NUMBER: _ClassVar[int]
    CURSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    EMAIL_FIELD_NUMBER: _ClassVar[int]
    PASSWORD_SALT_FIELD_NUMBER: _ClassVar[int]
    PASSWORD_HASH_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    custom_values: _containers.MessageMap[str, ValueProto]
    name: str
    slug: str
    status: UserStatusProto
    last_logged_in_at: _timestamp_pb2.Timestamp
    is_staff: bool
    space_ptr: NodeReferenceProto
    handle_ptr: NodeReferenceProto
    cursor_ptr: NodeReferenceProto
    email: str
    password_salt: bytes
    password_hash: bytes
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., name: _Optional[str] = ..., slug: _Optional[str] = ..., status: _Optional[_Union[UserStatusProto, str]] = ..., last_logged_in_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., is_staff: bool = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., handle_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., cursor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., email: _Optional[str] = ..., password_salt: _Optional[bytes] = ..., password_hash: _Optional[bytes] = ...) -> None: ...

class ValueProto(_message.Message):
    __slots__ = ("metatype", "type", "value")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    type: TypeProto
    value: _struct_pb2.Value
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., type: _Optional[_Union[TypeProto, _Mapping]] = ..., value: _Optional[_Union[_struct_pb2.Value, _Mapping]] = ...) -> None: ...

class VariantProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "owned_by_ptr", "type", "name", "icon", "max_width", "max_height", "min_width", "min_height")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    owned_by_ptr: NodeReferenceProto
    type: VariantTypeProto
    name: str
    icon: IconProto
    max_width: LengthProto
    max_height: LengthProto
    min_width: LengthProto
    min_height: LengthProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., type: _Optional[_Union[VariantTypeProto, str]] = ..., name: _Optional[str] = ..., icon: _Optional[_Union[IconProto, _Mapping]] = ..., max_width: _Optional[_Union[LengthProto, _Mapping]] = ..., max_height: _Optional[_Union[LengthProto, _Mapping]] = ..., min_width: _Optional[_Union[LengthProto, _Mapping]] = ..., min_height: _Optional[_Union[LengthProto, _Mapping]] = ...) -> None: ...

class VectorProto(_message.Message):
    __slots__ = ("metatype",)
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ...) -> None: ...

class Vector2fProto(_message.Message):
    __slots__ = ("metatype", "x", "y")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    x: float
    y: float
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., x: _Optional[float] = ..., y: _Optional[float] = ...) -> None: ...

class Vector2iProto(_message.Message):
    __slots__ = ("metatype", "x", "y")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    x: int
    y: int
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., x: _Optional[int] = ..., y: _Optional[int] = ...) -> None: ...

class Vector3fProto(_message.Message):
    __slots__ = ("metatype", "x", "y", "z")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    Z_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    x: float
    y: float
    z: float
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., x: _Optional[float] = ..., y: _Optional[float] = ..., z: _Optional[float] = ...) -> None: ...

class Vector3iProto(_message.Message):
    __slots__ = ("metatype", "x", "y", "z")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    Z_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    x: int
    y: int
    z: int
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., x: _Optional[int] = ..., y: _Optional[int] = ..., z: _Optional[int] = ...) -> None: ...

class Vector4fProto(_message.Message):
    __slots__ = ("metatype", "x", "y", "z", "w")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    Z_FIELD_NUMBER: _ClassVar[int]
    W_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    x: float
    y: float
    z: float
    w: float
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., x: _Optional[float] = ..., y: _Optional[float] = ..., z: _Optional[float] = ..., w: _Optional[float] = ...) -> None: ...

class Vector4iProto(_message.Message):
    __slots__ = ("metatype", "x", "y", "z", "w")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    Z_FIELD_NUMBER: _ClassVar[int]
    W_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    x: int
    y: int
    z: int
    w: int
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ..., x: _Optional[int] = ..., y: _Optional[int] = ..., z: _Optional[int] = ..., w: _Optional[int] = ...) -> None: ...

class VectorfProto(_message.Message):
    __slots__ = ("metatype",)
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ...) -> None: ...

class VectoriProto(_message.Message):
    __slots__ = ("metatype",)
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    metatype: StructTypeProto
    def __init__(self, metatype: _Optional[_Union[StructTypeProto, str]] = ...) -> None: ...

class ViewProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "definition_ptr", "base_type", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "custom_values", "order_key", "script_ptr", "name", "position", "width", "height", "min_width", "min_height", "max_width", "max_height")
    class CustomValuesEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: ValueProto
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[ValueProto, _Mapping]] = ...) -> None: ...
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    DEFINITION_PTR_FIELD_NUMBER: _ClassVar[int]
    BASE_TYPE_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_VALUES_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_PTR_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MIN_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MIN_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    MAX_WIDTH_FIELD_NUMBER: _ClassVar[int]
    MAX_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    definition_ptr: NodeReferenceProto
    base_type: NodeDefinitionReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    custom_values: _containers.MessageMap[str, ValueProto]
    order_key: str
    script_ptr: NodeReferenceProto
    name: str
    position: PositionProto
    width: DimensionProto
    height: DimensionProto
    min_width: DimensionProto
    min_height: DimensionProto
    max_width: DimensionProto
    max_height: DimensionProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., definition_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., base_type: _Optional[_Union[NodeDefinitionReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., custom_values: _Optional[_Mapping[str, ValueProto]] = ..., order_key: _Optional[str] = ..., script_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., name: _Optional[str] = ..., position: _Optional[_Union[PositionProto, _Mapping]] = ..., width: _Optional[_Union[DimensionProto, _Mapping]] = ..., height: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., min_height: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_width: _Optional[_Union[DimensionProto, _Mapping]] = ..., max_height: _Optional[_Union[DimensionProto, _Mapping]] = ...) -> None: ...

class ViewEnteredEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class ViewEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class ViewExitedEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ...) -> None: ...

class WheelEventProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "snapshot_ptr", "created_at", "created_by_ptr", "node_ptr", "position", "pressure", "shift_key", "alt_key", "ctrl_key", "meta_key", "accel_key", "button", "delta")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    NODE_PTR_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    PRESSURE_FIELD_NUMBER: _ClassVar[int]
    SHIFT_KEY_FIELD_NUMBER: _ClassVar[int]
    ALT_KEY_FIELD_NUMBER: _ClassVar[int]
    CTRL_KEY_FIELD_NUMBER: _ClassVar[int]
    META_KEY_FIELD_NUMBER: _ClassVar[int]
    ACCEL_KEY_FIELD_NUMBER: _ClassVar[int]
    BUTTON_FIELD_NUMBER: _ClassVar[int]
    DELTA_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    snapshot_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    node_ptr: NodeReferenceProto
    position: Vector2fProto
    pressure: float
    shift_key: bool
    alt_key: bool
    ctrl_key: bool
    meta_key: bool
    accel_key: bool
    button: MouseButtonProto
    delta: Vector2fProto
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., node_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., position: _Optional[_Union[Vector2fProto, _Mapping]] = ..., pressure: _Optional[float] = ..., shift_key: bool = ..., alt_key: bool = ..., ctrl_key: bool = ..., meta_key: bool = ..., accel_key: bool = ..., button: _Optional[_Union[MouseButtonProto, str]] = ..., delta: _Optional[_Union[Vector2fProto, _Mapping]] = ...) -> None: ...

class WindowProto(_message.Message):
    __slots__ = ("metatype", "id", "parent_ptr", "space_ptr", "materialization", "snapshot_ptr", "predecessor_ptr", "template_ptr", "instance_root_ptr", "created_at", "created_by_ptr", "updated_at", "updated_by_ptr", "deleted_at", "order_key", "owned_by_ptr", "type", "name")
    METATYPE_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    PARENT_PTR_FIELD_NUMBER: _ClassVar[int]
    SPACE_PTR_FIELD_NUMBER: _ClassVar[int]
    MATERIALIZATION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_PTR_FIELD_NUMBER: _ClassVar[int]
    PREDECESSOR_PTR_FIELD_NUMBER: _ClassVar[int]
    TEMPLATE_PTR_FIELD_NUMBER: _ClassVar[int]
    INSTANCE_ROOT_PTR_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    CREATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    DELETED_AT_FIELD_NUMBER: _ClassVar[int]
    ORDER_KEY_FIELD_NUMBER: _ClassVar[int]
    OWNED_BY_PTR_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    metatype: NodeTypeProto
    id: str
    parent_ptr: NodeReferenceProto
    space_ptr: NodeReferenceProto
    materialization: MaterializationProto
    snapshot_ptr: NodeReferenceProto
    predecessor_ptr: NodeReferenceProto
    template_ptr: NodeReferenceProto
    instance_root_ptr: NodeReferenceProto
    created_at: _timestamp_pb2.Timestamp
    created_by_ptr: NodeReferenceProto
    updated_at: _timestamp_pb2.Timestamp
    updated_by_ptr: NodeReferenceProto
    deleted_at: _timestamp_pb2.Timestamp
    order_key: str
    owned_by_ptr: NodeReferenceProto
    type: WindowTypeProto
    name: str
    def __init__(self, metatype: _Optional[_Union[NodeTypeProto, str]] = ..., id: _Optional[str] = ..., parent_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., space_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., materialization: _Optional[_Union[MaterializationProto, str]] = ..., snapshot_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., predecessor_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., template_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., instance_root_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., created_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., created_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., updated_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., updated_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., deleted_at: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., order_key: _Optional[str] = ..., owned_by_ptr: _Optional[_Union[NodeReferenceProto, _Mapping]] = ..., type: _Optional[_Union[WindowTypeProto, str]] = ..., name: _Optional[str] = ...) -> None: ...

class SomeNodeProto(_message.Message):
    __slots__ = ("custom_entity_definition", "custom_trait_definition", "snapshot", "custom_event_definition", "edit_event", "change_event", "query_event", "custom_enum_definition", "custom_option", "custom_option_group", "custom_property", "custom_property_group", "custom_struct_definition", "agent", "entitlement_requested_event", "entitlement_granted_event", "entitlement_revoked_event", "entitlement_expired_event", "entitlement", "invite_sent_event", "invite_rescinded_event", "invite_accepted_event", "invite_rejected_event", "invite", "membership_joined_event", "membership_left_event", "membership", "permission", "role_assigned_event", "role_unassigned_event", "role", "sanction_requested_event", "sanction_granted_event", "sanction_revoked_event", "sanction_expired_event", "sanction", "view_entered_event", "view_exited_event", "frame_view", "internal_view", "label_view", "number_input_view", "slider_input_view", "split_view", "text_view", "annotation_shape", "arrow_shape", "canvas", "line_shape", "file", "environment", "log_event", "run_started_event", "run_pause_requested_event", "run_paused_event", "run_resume_requested_event", "run_resumed_event", "run_stop_requested_event", "run_failed_event", "run_completed_event", "span_event", "database", "machine", "copy_event", "cut_event", "paste_event", "drag_start_event", "drag_end_event", "drag_over_event", "drag_enter_event", "drag_leave_event", "drop_event", "focus_in_event", "focus_out_event", "key_down_event", "key_up_event", "key_press_event", "pointer_down_event", "pointer_up_event", "pointer_move_event", "pointer_enter_event", "pointer_over_event", "pointer_leave_event", "pointer_long_press_event", "left_click_event", "right_click_event", "middle_click_event", "double_click_event", "wheel_event", "method", "action", "event_cursor", "screen_cursor", "thread_cursor", "route", "script", "service", "timer_started_event", "timer_completed_event", "timer_cancelled_event", "timer", "trigger", "gauge_metric", "gauge_measurement_event", "counter_metric", "counter_measurement_event", "histogram_metric", "histogram_measurement_event", "layer", "scene", "variant", "window", "follow", "message", "notification_sent_event", "notification_rescinded_event", "notification_read_event", "notification_dismissed_event", "notification_expired_event", "notification", "reaction", "star", "thread", "branch", "folder", "tag", "tagging", "color_style", "border_style", "transition_style", "effect_style", "gradient_style", "fill_style", "font_style", "palette", "shadow_style", "stroke_style", "theme", "client", "friendship", "friendship_invite_sent_event", "friendship_invite_rescinded_event", "friendship_invite_accepted_event", "friendship_invite_rejected_event", "friendship_invite", "handle", "organization", "space", "team", "user")
    CUSTOM_ENTITY_DEFINITION_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_TRAIT_DEFINITION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_EVENT_DEFINITION_FIELD_NUMBER: _ClassVar[int]
    EDIT_EVENT_FIELD_NUMBER: _ClassVar[int]
    CHANGE_EVENT_FIELD_NUMBER: _ClassVar[int]
    QUERY_EVENT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_ENUM_DEFINITION_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_OPTION_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_OPTION_GROUP_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_PROPERTY_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_PROPERTY_GROUP_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_STRUCT_DEFINITION_FIELD_NUMBER: _ClassVar[int]
    AGENT_FIELD_NUMBER: _ClassVar[int]
    ENTITLEMENT_REQUESTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    ENTITLEMENT_GRANTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    ENTITLEMENT_REVOKED_EVENT_FIELD_NUMBER: _ClassVar[int]
    ENTITLEMENT_EXPIRED_EVENT_FIELD_NUMBER: _ClassVar[int]
    ENTITLEMENT_FIELD_NUMBER: _ClassVar[int]
    INVITE_SENT_EVENT_FIELD_NUMBER: _ClassVar[int]
    INVITE_RESCINDED_EVENT_FIELD_NUMBER: _ClassVar[int]
    INVITE_ACCEPTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    INVITE_REJECTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    INVITE_FIELD_NUMBER: _ClassVar[int]
    MEMBERSHIP_JOINED_EVENT_FIELD_NUMBER: _ClassVar[int]
    MEMBERSHIP_LEFT_EVENT_FIELD_NUMBER: _ClassVar[int]
    MEMBERSHIP_FIELD_NUMBER: _ClassVar[int]
    PERMISSION_FIELD_NUMBER: _ClassVar[int]
    ROLE_ASSIGNED_EVENT_FIELD_NUMBER: _ClassVar[int]
    ROLE_UNASSIGNED_EVENT_FIELD_NUMBER: _ClassVar[int]
    ROLE_FIELD_NUMBER: _ClassVar[int]
    SANCTION_REQUESTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    SANCTION_GRANTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    SANCTION_REVOKED_EVENT_FIELD_NUMBER: _ClassVar[int]
    SANCTION_EXPIRED_EVENT_FIELD_NUMBER: _ClassVar[int]
    SANCTION_FIELD_NUMBER: _ClassVar[int]
    VIEW_ENTERED_EVENT_FIELD_NUMBER: _ClassVar[int]
    VIEW_EXITED_EVENT_FIELD_NUMBER: _ClassVar[int]
    FRAME_VIEW_FIELD_NUMBER: _ClassVar[int]
    INTERNAL_VIEW_FIELD_NUMBER: _ClassVar[int]
    LABEL_VIEW_FIELD_NUMBER: _ClassVar[int]
    NUMBER_INPUT_VIEW_FIELD_NUMBER: _ClassVar[int]
    SLIDER_INPUT_VIEW_FIELD_NUMBER: _ClassVar[int]
    SPLIT_VIEW_FIELD_NUMBER: _ClassVar[int]
    TEXT_VIEW_FIELD_NUMBER: _ClassVar[int]
    ANNOTATION_SHAPE_FIELD_NUMBER: _ClassVar[int]
    ARROW_SHAPE_FIELD_NUMBER: _ClassVar[int]
    CANVAS_FIELD_NUMBER: _ClassVar[int]
    LINE_SHAPE_FIELD_NUMBER: _ClassVar[int]
    FILE_FIELD_NUMBER: _ClassVar[int]
    ENVIRONMENT_FIELD_NUMBER: _ClassVar[int]
    LOG_EVENT_FIELD_NUMBER: _ClassVar[int]
    RUN_STARTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    RUN_PAUSE_REQUESTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    RUN_PAUSED_EVENT_FIELD_NUMBER: _ClassVar[int]
    RUN_RESUME_REQUESTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    RUN_RESUMED_EVENT_FIELD_NUMBER: _ClassVar[int]
    RUN_STOP_REQUESTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    RUN_FAILED_EVENT_FIELD_NUMBER: _ClassVar[int]
    RUN_COMPLETED_EVENT_FIELD_NUMBER: _ClassVar[int]
    SPAN_EVENT_FIELD_NUMBER: _ClassVar[int]
    DATABASE_FIELD_NUMBER: _ClassVar[int]
    MACHINE_FIELD_NUMBER: _ClassVar[int]
    COPY_EVENT_FIELD_NUMBER: _ClassVar[int]
    CUT_EVENT_FIELD_NUMBER: _ClassVar[int]
    PASTE_EVENT_FIELD_NUMBER: _ClassVar[int]
    DRAG_START_EVENT_FIELD_NUMBER: _ClassVar[int]
    DRAG_END_EVENT_FIELD_NUMBER: _ClassVar[int]
    DRAG_OVER_EVENT_FIELD_NUMBER: _ClassVar[int]
    DRAG_ENTER_EVENT_FIELD_NUMBER: _ClassVar[int]
    DRAG_LEAVE_EVENT_FIELD_NUMBER: _ClassVar[int]
    DROP_EVENT_FIELD_NUMBER: _ClassVar[int]
    FOCUS_IN_EVENT_FIELD_NUMBER: _ClassVar[int]
    FOCUS_OUT_EVENT_FIELD_NUMBER: _ClassVar[int]
    KEY_DOWN_EVENT_FIELD_NUMBER: _ClassVar[int]
    KEY_UP_EVENT_FIELD_NUMBER: _ClassVar[int]
    KEY_PRESS_EVENT_FIELD_NUMBER: _ClassVar[int]
    POINTER_DOWN_EVENT_FIELD_NUMBER: _ClassVar[int]
    POINTER_UP_EVENT_FIELD_NUMBER: _ClassVar[int]
    POINTER_MOVE_EVENT_FIELD_NUMBER: _ClassVar[int]
    POINTER_ENTER_EVENT_FIELD_NUMBER: _ClassVar[int]
    POINTER_OVER_EVENT_FIELD_NUMBER: _ClassVar[int]
    POINTER_LEAVE_EVENT_FIELD_NUMBER: _ClassVar[int]
    POINTER_LONG_PRESS_EVENT_FIELD_NUMBER: _ClassVar[int]
    LEFT_CLICK_EVENT_FIELD_NUMBER: _ClassVar[int]
    RIGHT_CLICK_EVENT_FIELD_NUMBER: _ClassVar[int]
    MIDDLE_CLICK_EVENT_FIELD_NUMBER: _ClassVar[int]
    DOUBLE_CLICK_EVENT_FIELD_NUMBER: _ClassVar[int]
    WHEEL_EVENT_FIELD_NUMBER: _ClassVar[int]
    METHOD_FIELD_NUMBER: _ClassVar[int]
    ACTION_FIELD_NUMBER: _ClassVar[int]
    EVENT_CURSOR_FIELD_NUMBER: _ClassVar[int]
    SCREEN_CURSOR_FIELD_NUMBER: _ClassVar[int]
    THREAD_CURSOR_FIELD_NUMBER: _ClassVar[int]
    ROUTE_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_FIELD_NUMBER: _ClassVar[int]
    SERVICE_FIELD_NUMBER: _ClassVar[int]
    TIMER_STARTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    TIMER_COMPLETED_EVENT_FIELD_NUMBER: _ClassVar[int]
    TIMER_CANCELLED_EVENT_FIELD_NUMBER: _ClassVar[int]
    TIMER_FIELD_NUMBER: _ClassVar[int]
    TRIGGER_FIELD_NUMBER: _ClassVar[int]
    GAUGE_METRIC_FIELD_NUMBER: _ClassVar[int]
    GAUGE_MEASUREMENT_EVENT_FIELD_NUMBER: _ClassVar[int]
    COUNTER_METRIC_FIELD_NUMBER: _ClassVar[int]
    COUNTER_MEASUREMENT_EVENT_FIELD_NUMBER: _ClassVar[int]
    HISTOGRAM_METRIC_FIELD_NUMBER: _ClassVar[int]
    HISTOGRAM_MEASUREMENT_EVENT_FIELD_NUMBER: _ClassVar[int]
    LAYER_FIELD_NUMBER: _ClassVar[int]
    SCENE_FIELD_NUMBER: _ClassVar[int]
    VARIANT_FIELD_NUMBER: _ClassVar[int]
    WINDOW_FIELD_NUMBER: _ClassVar[int]
    FOLLOW_FIELD_NUMBER: _ClassVar[int]
    MESSAGE_FIELD_NUMBER: _ClassVar[int]
    NOTIFICATION_SENT_EVENT_FIELD_NUMBER: _ClassVar[int]
    NOTIFICATION_RESCINDED_EVENT_FIELD_NUMBER: _ClassVar[int]
    NOTIFICATION_READ_EVENT_FIELD_NUMBER: _ClassVar[int]
    NOTIFICATION_DISMISSED_EVENT_FIELD_NUMBER: _ClassVar[int]
    NOTIFICATION_EXPIRED_EVENT_FIELD_NUMBER: _ClassVar[int]
    NOTIFICATION_FIELD_NUMBER: _ClassVar[int]
    REACTION_FIELD_NUMBER: _ClassVar[int]
    STAR_FIELD_NUMBER: _ClassVar[int]
    THREAD_FIELD_NUMBER: _ClassVar[int]
    BRANCH_FIELD_NUMBER: _ClassVar[int]
    FOLDER_FIELD_NUMBER: _ClassVar[int]
    TAG_FIELD_NUMBER: _ClassVar[int]
    TAGGING_FIELD_NUMBER: _ClassVar[int]
    COLOR_STYLE_FIELD_NUMBER: _ClassVar[int]
    BORDER_STYLE_FIELD_NUMBER: _ClassVar[int]
    TRANSITION_STYLE_FIELD_NUMBER: _ClassVar[int]
    EFFECT_STYLE_FIELD_NUMBER: _ClassVar[int]
    GRADIENT_STYLE_FIELD_NUMBER: _ClassVar[int]
    FILL_STYLE_FIELD_NUMBER: _ClassVar[int]
    FONT_STYLE_FIELD_NUMBER: _ClassVar[int]
    PALETTE_FIELD_NUMBER: _ClassVar[int]
    SHADOW_STYLE_FIELD_NUMBER: _ClassVar[int]
    STROKE_STYLE_FIELD_NUMBER: _ClassVar[int]
    THEME_FIELD_NUMBER: _ClassVar[int]
    CLIENT_FIELD_NUMBER: _ClassVar[int]
    FRIENDSHIP_FIELD_NUMBER: _ClassVar[int]
    FRIENDSHIP_INVITE_SENT_EVENT_FIELD_NUMBER: _ClassVar[int]
    FRIENDSHIP_INVITE_RESCINDED_EVENT_FIELD_NUMBER: _ClassVar[int]
    FRIENDSHIP_INVITE_ACCEPTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    FRIENDSHIP_INVITE_REJECTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    FRIENDSHIP_INVITE_FIELD_NUMBER: _ClassVar[int]
    HANDLE_FIELD_NUMBER: _ClassVar[int]
    ORGANIZATION_FIELD_NUMBER: _ClassVar[int]
    SPACE_FIELD_NUMBER: _ClassVar[int]
    TEAM_FIELD_NUMBER: _ClassVar[int]
    USER_FIELD_NUMBER: _ClassVar[int]
    custom_entity_definition: CustomEntityDefinitionProto
    custom_trait_definition: CustomTraitDefinitionProto
    snapshot: SnapshotProto
    custom_event_definition: CustomEventDefinitionProto
    edit_event: EditEventProto
    change_event: ChangeEventProto
    query_event: QueryEventProto
    custom_enum_definition: CustomEnumDefinitionProto
    custom_option: CustomOptionProto
    custom_option_group: CustomOptionGroupProto
    custom_property: CustomPropertyProto
    custom_property_group: CustomPropertyGroupProto
    custom_struct_definition: CustomStructDefinitionProto
    agent: AgentProto
    entitlement_requested_event: EntitlementRequestedEventProto
    entitlement_granted_event: EntitlementGrantedEventProto
    entitlement_revoked_event: EntitlementRevokedEventProto
    entitlement_expired_event: EntitlementExpiredEventProto
    entitlement: EntitlementProto
    invite_sent_event: InviteSentEventProto
    invite_rescinded_event: InviteRescindedEventProto
    invite_accepted_event: InviteAcceptedEventProto
    invite_rejected_event: InviteRejectedEventProto
    invite: InviteProto
    membership_joined_event: MembershipJoinedEventProto
    membership_left_event: MembershipLeftEventProto
    membership: MembershipProto
    permission: PermissionProto
    role_assigned_event: RoleAssignedEventProto
    role_unassigned_event: RoleUnassignedEventProto
    role: RoleProto
    sanction_requested_event: SanctionRequestedEventProto
    sanction_granted_event: SanctionGrantedEventProto
    sanction_revoked_event: SanctionRevokedEventProto
    sanction_expired_event: SanctionExpiredEventProto
    sanction: SanctionProto
    view_entered_event: ViewEnteredEventProto
    view_exited_event: ViewExitedEventProto
    frame_view: FrameViewProto
    internal_view: InternalViewProto
    label_view: LabelViewProto
    number_input_view: NumberInputViewProto
    slider_input_view: SliderInputViewProto
    split_view: SplitViewProto
    text_view: TextViewProto
    annotation_shape: AnnotationShapeProto
    arrow_shape: ArrowShapeProto
    canvas: CanvasProto
    line_shape: LineShapeProto
    file: FileProto
    environment: EnvironmentProto
    log_event: LogEventProto
    run_started_event: RunStartedEventProto
    run_pause_requested_event: RunPauseRequestedEventProto
    run_paused_event: RunPausedEventProto
    run_resume_requested_event: RunResumeRequestedEventProto
    run_resumed_event: RunResumedEventProto
    run_stop_requested_event: RunStopRequestedEventProto
    run_failed_event: RunFailedEventProto
    run_completed_event: RunCompletedEventProto
    span_event: SpanEventProto
    database: DatabaseProto
    machine: MachineProto
    copy_event: CopyEventProto
    cut_event: CutEventProto
    paste_event: PasteEventProto
    drag_start_event: DragStartEventProto
    drag_end_event: DragEndEventProto
    drag_over_event: DragOverEventProto
    drag_enter_event: DragEnterEventProto
    drag_leave_event: DragLeaveEventProto
    drop_event: DropEventProto
    focus_in_event: FocusInEventProto
    focus_out_event: FocusOutEventProto
    key_down_event: KeyDownEventProto
    key_up_event: KeyUpEventProto
    key_press_event: KeyPressEventProto
    pointer_down_event: PointerDownEventProto
    pointer_up_event: PointerUpEventProto
    pointer_move_event: PointerMoveEventProto
    pointer_enter_event: PointerEnterEventProto
    pointer_over_event: PointerOverEventProto
    pointer_leave_event: PointerLeaveEventProto
    pointer_long_press_event: PointerLongPressEventProto
    left_click_event: LeftClickEventProto
    right_click_event: RightClickEventProto
    middle_click_event: MiddleClickEventProto
    double_click_event: DoubleClickEventProto
    wheel_event: WheelEventProto
    method: MethodProto
    action: ActionProto
    event_cursor: EventCursorProto
    screen_cursor: ScreenCursorProto
    thread_cursor: ThreadCursorProto
    route: RouteProto
    script: ScriptProto
    service: ServiceProto
    timer_started_event: TimerStartedEventProto
    timer_completed_event: TimerCompletedEventProto
    timer_cancelled_event: TimerCancelledEventProto
    timer: TimerProto
    trigger: TriggerProto
    gauge_metric: GaugeMetricProto
    gauge_measurement_event: GaugeMeasurementEventProto
    counter_metric: CounterMetricProto
    counter_measurement_event: CounterMeasurementEventProto
    histogram_metric: HistogramMetricProto
    histogram_measurement_event: HistogramMeasurementEventProto
    layer: LayerProto
    scene: SceneProto
    variant: VariantProto
    window: WindowProto
    follow: FollowProto
    message: MessageProto
    notification_sent_event: NotificationSentEventProto
    notification_rescinded_event: NotificationRescindedEventProto
    notification_read_event: NotificationReadEventProto
    notification_dismissed_event: NotificationDismissedEventProto
    notification_expired_event: NotificationExpiredEventProto
    notification: NotificationProto
    reaction: ReactionProto
    star: StarProto
    thread: ThreadProto
    branch: BranchProto
    folder: FolderProto
    tag: TagProto
    tagging: TaggingProto
    color_style: ColorStyleProto
    border_style: BorderStyleProto
    transition_style: TransitionStyleProto
    effect_style: EffectStyleProto
    gradient_style: GradientStyleProto
    fill_style: FillStyleProto
    font_style: FontStyleProto
    palette: PaletteProto
    shadow_style: ShadowStyleProto
    stroke_style: StrokeStyleProto
    theme: ThemeProto
    client: ClientProto
    friendship: FriendshipProto
    friendship_invite_sent_event: FriendshipInviteSentEventProto
    friendship_invite_rescinded_event: FriendshipInviteRescindedEventProto
    friendship_invite_accepted_event: FriendshipInviteAcceptedEventProto
    friendship_invite_rejected_event: FriendshipInviteRejectedEventProto
    friendship_invite: FriendshipInviteProto
    handle: HandleProto
    organization: OrganizationProto
    space: SpaceProto
    team: TeamProto
    user: UserProto
    def __init__(self, custom_entity_definition: _Optional[_Union[CustomEntityDefinitionProto, _Mapping]] = ..., custom_trait_definition: _Optional[_Union[CustomTraitDefinitionProto, _Mapping]] = ..., snapshot: _Optional[_Union[SnapshotProto, _Mapping]] = ..., custom_event_definition: _Optional[_Union[CustomEventDefinitionProto, _Mapping]] = ..., edit_event: _Optional[_Union[EditEventProto, _Mapping]] = ..., change_event: _Optional[_Union[ChangeEventProto, _Mapping]] = ..., query_event: _Optional[_Union[QueryEventProto, _Mapping]] = ..., custom_enum_definition: _Optional[_Union[CustomEnumDefinitionProto, _Mapping]] = ..., custom_option: _Optional[_Union[CustomOptionProto, _Mapping]] = ..., custom_option_group: _Optional[_Union[CustomOptionGroupProto, _Mapping]] = ..., custom_property: _Optional[_Union[CustomPropertyProto, _Mapping]] = ..., custom_property_group: _Optional[_Union[CustomPropertyGroupProto, _Mapping]] = ..., custom_struct_definition: _Optional[_Union[CustomStructDefinitionProto, _Mapping]] = ..., agent: _Optional[_Union[AgentProto, _Mapping]] = ..., entitlement_requested_event: _Optional[_Union[EntitlementRequestedEventProto, _Mapping]] = ..., entitlement_granted_event: _Optional[_Union[EntitlementGrantedEventProto, _Mapping]] = ..., entitlement_revoked_event: _Optional[_Union[EntitlementRevokedEventProto, _Mapping]] = ..., entitlement_expired_event: _Optional[_Union[EntitlementExpiredEventProto, _Mapping]] = ..., entitlement: _Optional[_Union[EntitlementProto, _Mapping]] = ..., invite_sent_event: _Optional[_Union[InviteSentEventProto, _Mapping]] = ..., invite_rescinded_event: _Optional[_Union[InviteRescindedEventProto, _Mapping]] = ..., invite_accepted_event: _Optional[_Union[InviteAcceptedEventProto, _Mapping]] = ..., invite_rejected_event: _Optional[_Union[InviteRejectedEventProto, _Mapping]] = ..., invite: _Optional[_Union[InviteProto, _Mapping]] = ..., membership_joined_event: _Optional[_Union[MembershipJoinedEventProto, _Mapping]] = ..., membership_left_event: _Optional[_Union[MembershipLeftEventProto, _Mapping]] = ..., membership: _Optional[_Union[MembershipProto, _Mapping]] = ..., permission: _Optional[_Union[PermissionProto, _Mapping]] = ..., role_assigned_event: _Optional[_Union[RoleAssignedEventProto, _Mapping]] = ..., role_unassigned_event: _Optional[_Union[RoleUnassignedEventProto, _Mapping]] = ..., role: _Optional[_Union[RoleProto, _Mapping]] = ..., sanction_requested_event: _Optional[_Union[SanctionRequestedEventProto, _Mapping]] = ..., sanction_granted_event: _Optional[_Union[SanctionGrantedEventProto, _Mapping]] = ..., sanction_revoked_event: _Optional[_Union[SanctionRevokedEventProto, _Mapping]] = ..., sanction_expired_event: _Optional[_Union[SanctionExpiredEventProto, _Mapping]] = ..., sanction: _Optional[_Union[SanctionProto, _Mapping]] = ..., view_entered_event: _Optional[_Union[ViewEnteredEventProto, _Mapping]] = ..., view_exited_event: _Optional[_Union[ViewExitedEventProto, _Mapping]] = ..., frame_view: _Optional[_Union[FrameViewProto, _Mapping]] = ..., internal_view: _Optional[_Union[InternalViewProto, _Mapping]] = ..., label_view: _Optional[_Union[LabelViewProto, _Mapping]] = ..., number_input_view: _Optional[_Union[NumberInputViewProto, _Mapping]] = ..., slider_input_view: _Optional[_Union[SliderInputViewProto, _Mapping]] = ..., split_view: _Optional[_Union[SplitViewProto, _Mapping]] = ..., text_view: _Optional[_Union[TextViewProto, _Mapping]] = ..., annotation_shape: _Optional[_Union[AnnotationShapeProto, _Mapping]] = ..., arrow_shape: _Optional[_Union[ArrowShapeProto, _Mapping]] = ..., canvas: _Optional[_Union[CanvasProto, _Mapping]] = ..., line_shape: _Optional[_Union[LineShapeProto, _Mapping]] = ..., file: _Optional[_Union[FileProto, _Mapping]] = ..., environment: _Optional[_Union[EnvironmentProto, _Mapping]] = ..., log_event: _Optional[_Union[LogEventProto, _Mapping]] = ..., run_started_event: _Optional[_Union[RunStartedEventProto, _Mapping]] = ..., run_pause_requested_event: _Optional[_Union[RunPauseRequestedEventProto, _Mapping]] = ..., run_paused_event: _Optional[_Union[RunPausedEventProto, _Mapping]] = ..., run_resume_requested_event: _Optional[_Union[RunResumeRequestedEventProto, _Mapping]] = ..., run_resumed_event: _Optional[_Union[RunResumedEventProto, _Mapping]] = ..., run_stop_requested_event: _Optional[_Union[RunStopRequestedEventProto, _Mapping]] = ..., run_failed_event: _Optional[_Union[RunFailedEventProto, _Mapping]] = ..., run_completed_event: _Optional[_Union[RunCompletedEventProto, _Mapping]] = ..., span_event: _Optional[_Union[SpanEventProto, _Mapping]] = ..., database: _Optional[_Union[DatabaseProto, _Mapping]] = ..., machine: _Optional[_Union[MachineProto, _Mapping]] = ..., copy_event: _Optional[_Union[CopyEventProto, _Mapping]] = ..., cut_event: _Optional[_Union[CutEventProto, _Mapping]] = ..., paste_event: _Optional[_Union[PasteEventProto, _Mapping]] = ..., drag_start_event: _Optional[_Union[DragStartEventProto, _Mapping]] = ..., drag_end_event: _Optional[_Union[DragEndEventProto, _Mapping]] = ..., drag_over_event: _Optional[_Union[DragOverEventProto, _Mapping]] = ..., drag_enter_event: _Optional[_Union[DragEnterEventProto, _Mapping]] = ..., drag_leave_event: _Optional[_Union[DragLeaveEventProto, _Mapping]] = ..., drop_event: _Optional[_Union[DropEventProto, _Mapping]] = ..., focus_in_event: _Optional[_Union[FocusInEventProto, _Mapping]] = ..., focus_out_event: _Optional[_Union[FocusOutEventProto, _Mapping]] = ..., key_down_event: _Optional[_Union[KeyDownEventProto, _Mapping]] = ..., key_up_event: _Optional[_Union[KeyUpEventProto, _Mapping]] = ..., key_press_event: _Optional[_Union[KeyPressEventProto, _Mapping]] = ..., pointer_down_event: _Optional[_Union[PointerDownEventProto, _Mapping]] = ..., pointer_up_event: _Optional[_Union[PointerUpEventProto, _Mapping]] = ..., pointer_move_event: _Optional[_Union[PointerMoveEventProto, _Mapping]] = ..., pointer_enter_event: _Optional[_Union[PointerEnterEventProto, _Mapping]] = ..., pointer_over_event: _Optional[_Union[PointerOverEventProto, _Mapping]] = ..., pointer_leave_event: _Optional[_Union[PointerLeaveEventProto, _Mapping]] = ..., pointer_long_press_event: _Optional[_Union[PointerLongPressEventProto, _Mapping]] = ..., left_click_event: _Optional[_Union[LeftClickEventProto, _Mapping]] = ..., right_click_event: _Optional[_Union[RightClickEventProto, _Mapping]] = ..., middle_click_event: _Optional[_Union[MiddleClickEventProto, _Mapping]] = ..., double_click_event: _Optional[_Union[DoubleClickEventProto, _Mapping]] = ..., wheel_event: _Optional[_Union[WheelEventProto, _Mapping]] = ..., method: _Optional[_Union[MethodProto, _Mapping]] = ..., action: _Optional[_Union[ActionProto, _Mapping]] = ..., event_cursor: _Optional[_Union[EventCursorProto, _Mapping]] = ..., screen_cursor: _Optional[_Union[ScreenCursorProto, _Mapping]] = ..., thread_cursor: _Optional[_Union[ThreadCursorProto, _Mapping]] = ..., route: _Optional[_Union[RouteProto, _Mapping]] = ..., script: _Optional[_Union[ScriptProto, _Mapping]] = ..., service: _Optional[_Union[ServiceProto, _Mapping]] = ..., timer_started_event: _Optional[_Union[TimerStartedEventProto, _Mapping]] = ..., timer_completed_event: _Optional[_Union[TimerCompletedEventProto, _Mapping]] = ..., timer_cancelled_event: _Optional[_Union[TimerCancelledEventProto, _Mapping]] = ..., timer: _Optional[_Union[TimerProto, _Mapping]] = ..., trigger: _Optional[_Union[TriggerProto, _Mapping]] = ..., gauge_metric: _Optional[_Union[GaugeMetricProto, _Mapping]] = ..., gauge_measurement_event: _Optional[_Union[GaugeMeasurementEventProto, _Mapping]] = ..., counter_metric: _Optional[_Union[CounterMetricProto, _Mapping]] = ..., counter_measurement_event: _Optional[_Union[CounterMeasurementEventProto, _Mapping]] = ..., histogram_metric: _Optional[_Union[HistogramMetricProto, _Mapping]] = ..., histogram_measurement_event: _Optional[_Union[HistogramMeasurementEventProto, _Mapping]] = ..., layer: _Optional[_Union[LayerProto, _Mapping]] = ..., scene: _Optional[_Union[SceneProto, _Mapping]] = ..., variant: _Optional[_Union[VariantProto, _Mapping]] = ..., window: _Optional[_Union[WindowProto, _Mapping]] = ..., follow: _Optional[_Union[FollowProto, _Mapping]] = ..., message: _Optional[_Union[MessageProto, _Mapping]] = ..., notification_sent_event: _Optional[_Union[NotificationSentEventProto, _Mapping]] = ..., notification_rescinded_event: _Optional[_Union[NotificationRescindedEventProto, _Mapping]] = ..., notification_read_event: _Optional[_Union[NotificationReadEventProto, _Mapping]] = ..., notification_dismissed_event: _Optional[_Union[NotificationDismissedEventProto, _Mapping]] = ..., notification_expired_event: _Optional[_Union[NotificationExpiredEventProto, _Mapping]] = ..., notification: _Optional[_Union[NotificationProto, _Mapping]] = ..., reaction: _Optional[_Union[ReactionProto, _Mapping]] = ..., star: _Optional[_Union[StarProto, _Mapping]] = ..., thread: _Optional[_Union[ThreadProto, _Mapping]] = ..., branch: _Optional[_Union[BranchProto, _Mapping]] = ..., folder: _Optional[_Union[FolderProto, _Mapping]] = ..., tag: _Optional[_Union[TagProto, _Mapping]] = ..., tagging: _Optional[_Union[TaggingProto, _Mapping]] = ..., color_style: _Optional[_Union[ColorStyleProto, _Mapping]] = ..., border_style: _Optional[_Union[BorderStyleProto, _Mapping]] = ..., transition_style: _Optional[_Union[TransitionStyleProto, _Mapping]] = ..., effect_style: _Optional[_Union[EffectStyleProto, _Mapping]] = ..., gradient_style: _Optional[_Union[GradientStyleProto, _Mapping]] = ..., fill_style: _Optional[_Union[FillStyleProto, _Mapping]] = ..., font_style: _Optional[_Union[FontStyleProto, _Mapping]] = ..., palette: _Optional[_Union[PaletteProto, _Mapping]] = ..., shadow_style: _Optional[_Union[ShadowStyleProto, _Mapping]] = ..., stroke_style: _Optional[_Union[StrokeStyleProto, _Mapping]] = ..., theme: _Optional[_Union[ThemeProto, _Mapping]] = ..., client: _Optional[_Union[ClientProto, _Mapping]] = ..., friendship: _Optional[_Union[FriendshipProto, _Mapping]] = ..., friendship_invite_sent_event: _Optional[_Union[FriendshipInviteSentEventProto, _Mapping]] = ..., friendship_invite_rescinded_event: _Optional[_Union[FriendshipInviteRescindedEventProto, _Mapping]] = ..., friendship_invite_accepted_event: _Optional[_Union[FriendshipInviteAcceptedEventProto, _Mapping]] = ..., friendship_invite_rejected_event: _Optional[_Union[FriendshipInviteRejectedEventProto, _Mapping]] = ..., friendship_invite: _Optional[_Union[FriendshipInviteProto, _Mapping]] = ..., handle: _Optional[_Union[HandleProto, _Mapping]] = ..., organization: _Optional[_Union[OrganizationProto, _Mapping]] = ..., space: _Optional[_Union[SpaceProto, _Mapping]] = ..., team: _Optional[_Union[TeamProto, _Mapping]] = ..., user: _Optional[_Union[UserProto, _Mapping]] = ...) -> None: ...

class SomeEntityProto(_message.Message):
    __slots__ = ("custom_entity_definition", "custom_trait_definition", "snapshot", "custom_event_definition", "custom_enum_definition", "custom_option", "custom_option_group", "custom_property", "custom_property_group", "custom_struct_definition", "agent", "entitlement", "invite", "membership", "permission", "role", "sanction", "frame_view", "internal_view", "label_view", "number_input_view", "slider_input_view", "split_view", "text_view", "annotation_shape", "arrow_shape", "canvas", "line_shape", "file", "environment", "database", "machine", "method", "action", "event_cursor", "screen_cursor", "thread_cursor", "route", "script", "service", "timer", "trigger", "gauge_metric", "counter_metric", "histogram_metric", "layer", "scene", "variant", "window", "follow", "message", "notification", "reaction", "star", "thread", "branch", "folder", "tag", "tagging", "color_style", "border_style", "transition_style", "effect_style", "gradient_style", "fill_style", "font_style", "palette", "shadow_style", "stroke_style", "theme", "client", "friendship", "friendship_invite", "handle", "organization", "space", "team", "user")
    CUSTOM_ENTITY_DEFINITION_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_TRAIT_DEFINITION_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_EVENT_DEFINITION_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_ENUM_DEFINITION_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_OPTION_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_OPTION_GROUP_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_PROPERTY_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_PROPERTY_GROUP_FIELD_NUMBER: _ClassVar[int]
    CUSTOM_STRUCT_DEFINITION_FIELD_NUMBER: _ClassVar[int]
    AGENT_FIELD_NUMBER: _ClassVar[int]
    ENTITLEMENT_FIELD_NUMBER: _ClassVar[int]
    INVITE_FIELD_NUMBER: _ClassVar[int]
    MEMBERSHIP_FIELD_NUMBER: _ClassVar[int]
    PERMISSION_FIELD_NUMBER: _ClassVar[int]
    ROLE_FIELD_NUMBER: _ClassVar[int]
    SANCTION_FIELD_NUMBER: _ClassVar[int]
    FRAME_VIEW_FIELD_NUMBER: _ClassVar[int]
    INTERNAL_VIEW_FIELD_NUMBER: _ClassVar[int]
    LABEL_VIEW_FIELD_NUMBER: _ClassVar[int]
    NUMBER_INPUT_VIEW_FIELD_NUMBER: _ClassVar[int]
    SLIDER_INPUT_VIEW_FIELD_NUMBER: _ClassVar[int]
    SPLIT_VIEW_FIELD_NUMBER: _ClassVar[int]
    TEXT_VIEW_FIELD_NUMBER: _ClassVar[int]
    ANNOTATION_SHAPE_FIELD_NUMBER: _ClassVar[int]
    ARROW_SHAPE_FIELD_NUMBER: _ClassVar[int]
    CANVAS_FIELD_NUMBER: _ClassVar[int]
    LINE_SHAPE_FIELD_NUMBER: _ClassVar[int]
    FILE_FIELD_NUMBER: _ClassVar[int]
    ENVIRONMENT_FIELD_NUMBER: _ClassVar[int]
    DATABASE_FIELD_NUMBER: _ClassVar[int]
    MACHINE_FIELD_NUMBER: _ClassVar[int]
    METHOD_FIELD_NUMBER: _ClassVar[int]
    ACTION_FIELD_NUMBER: _ClassVar[int]
    EVENT_CURSOR_FIELD_NUMBER: _ClassVar[int]
    SCREEN_CURSOR_FIELD_NUMBER: _ClassVar[int]
    THREAD_CURSOR_FIELD_NUMBER: _ClassVar[int]
    ROUTE_FIELD_NUMBER: _ClassVar[int]
    SCRIPT_FIELD_NUMBER: _ClassVar[int]
    SERVICE_FIELD_NUMBER: _ClassVar[int]
    TIMER_FIELD_NUMBER: _ClassVar[int]
    TRIGGER_FIELD_NUMBER: _ClassVar[int]
    GAUGE_METRIC_FIELD_NUMBER: _ClassVar[int]
    COUNTER_METRIC_FIELD_NUMBER: _ClassVar[int]
    HISTOGRAM_METRIC_FIELD_NUMBER: _ClassVar[int]
    LAYER_FIELD_NUMBER: _ClassVar[int]
    SCENE_FIELD_NUMBER: _ClassVar[int]
    VARIANT_FIELD_NUMBER: _ClassVar[int]
    WINDOW_FIELD_NUMBER: _ClassVar[int]
    FOLLOW_FIELD_NUMBER: _ClassVar[int]
    MESSAGE_FIELD_NUMBER: _ClassVar[int]
    NOTIFICATION_FIELD_NUMBER: _ClassVar[int]
    REACTION_FIELD_NUMBER: _ClassVar[int]
    STAR_FIELD_NUMBER: _ClassVar[int]
    THREAD_FIELD_NUMBER: _ClassVar[int]
    BRANCH_FIELD_NUMBER: _ClassVar[int]
    FOLDER_FIELD_NUMBER: _ClassVar[int]
    TAG_FIELD_NUMBER: _ClassVar[int]
    TAGGING_FIELD_NUMBER: _ClassVar[int]
    COLOR_STYLE_FIELD_NUMBER: _ClassVar[int]
    BORDER_STYLE_FIELD_NUMBER: _ClassVar[int]
    TRANSITION_STYLE_FIELD_NUMBER: _ClassVar[int]
    EFFECT_STYLE_FIELD_NUMBER: _ClassVar[int]
    GRADIENT_STYLE_FIELD_NUMBER: _ClassVar[int]
    FILL_STYLE_FIELD_NUMBER: _ClassVar[int]
    FONT_STYLE_FIELD_NUMBER: _ClassVar[int]
    PALETTE_FIELD_NUMBER: _ClassVar[int]
    SHADOW_STYLE_FIELD_NUMBER: _ClassVar[int]
    STROKE_STYLE_FIELD_NUMBER: _ClassVar[int]
    THEME_FIELD_NUMBER: _ClassVar[int]
    CLIENT_FIELD_NUMBER: _ClassVar[int]
    FRIENDSHIP_FIELD_NUMBER: _ClassVar[int]
    FRIENDSHIP_INVITE_FIELD_NUMBER: _ClassVar[int]
    HANDLE_FIELD_NUMBER: _ClassVar[int]
    ORGANIZATION_FIELD_NUMBER: _ClassVar[int]
    SPACE_FIELD_NUMBER: _ClassVar[int]
    TEAM_FIELD_NUMBER: _ClassVar[int]
    USER_FIELD_NUMBER: _ClassVar[int]
    custom_entity_definition: CustomEntityDefinitionProto
    custom_trait_definition: CustomTraitDefinitionProto
    snapshot: SnapshotProto
    custom_event_definition: CustomEventDefinitionProto
    custom_enum_definition: CustomEnumDefinitionProto
    custom_option: CustomOptionProto
    custom_option_group: CustomOptionGroupProto
    custom_property: CustomPropertyProto
    custom_property_group: CustomPropertyGroupProto
    custom_struct_definition: CustomStructDefinitionProto
    agent: AgentProto
    entitlement: EntitlementProto
    invite: InviteProto
    membership: MembershipProto
    permission: PermissionProto
    role: RoleProto
    sanction: SanctionProto
    frame_view: FrameViewProto
    internal_view: InternalViewProto
    label_view: LabelViewProto
    number_input_view: NumberInputViewProto
    slider_input_view: SliderInputViewProto
    split_view: SplitViewProto
    text_view: TextViewProto
    annotation_shape: AnnotationShapeProto
    arrow_shape: ArrowShapeProto
    canvas: CanvasProto
    line_shape: LineShapeProto
    file: FileProto
    environment: EnvironmentProto
    database: DatabaseProto
    machine: MachineProto
    method: MethodProto
    action: ActionProto
    event_cursor: EventCursorProto
    screen_cursor: ScreenCursorProto
    thread_cursor: ThreadCursorProto
    route: RouteProto
    script: ScriptProto
    service: ServiceProto
    timer: TimerProto
    trigger: TriggerProto
    gauge_metric: GaugeMetricProto
    counter_metric: CounterMetricProto
    histogram_metric: HistogramMetricProto
    layer: LayerProto
    scene: SceneProto
    variant: VariantProto
    window: WindowProto
    follow: FollowProto
    message: MessageProto
    notification: NotificationProto
    reaction: ReactionProto
    star: StarProto
    thread: ThreadProto
    branch: BranchProto
    folder: FolderProto
    tag: TagProto
    tagging: TaggingProto
    color_style: ColorStyleProto
    border_style: BorderStyleProto
    transition_style: TransitionStyleProto
    effect_style: EffectStyleProto
    gradient_style: GradientStyleProto
    fill_style: FillStyleProto
    font_style: FontStyleProto
    palette: PaletteProto
    shadow_style: ShadowStyleProto
    stroke_style: StrokeStyleProto
    theme: ThemeProto
    client: ClientProto
    friendship: FriendshipProto
    friendship_invite: FriendshipInviteProto
    handle: HandleProto
    organization: OrganizationProto
    space: SpaceProto
    team: TeamProto
    user: UserProto
    def __init__(self, custom_entity_definition: _Optional[_Union[CustomEntityDefinitionProto, _Mapping]] = ..., custom_trait_definition: _Optional[_Union[CustomTraitDefinitionProto, _Mapping]] = ..., snapshot: _Optional[_Union[SnapshotProto, _Mapping]] = ..., custom_event_definition: _Optional[_Union[CustomEventDefinitionProto, _Mapping]] = ..., custom_enum_definition: _Optional[_Union[CustomEnumDefinitionProto, _Mapping]] = ..., custom_option: _Optional[_Union[CustomOptionProto, _Mapping]] = ..., custom_option_group: _Optional[_Union[CustomOptionGroupProto, _Mapping]] = ..., custom_property: _Optional[_Union[CustomPropertyProto, _Mapping]] = ..., custom_property_group: _Optional[_Union[CustomPropertyGroupProto, _Mapping]] = ..., custom_struct_definition: _Optional[_Union[CustomStructDefinitionProto, _Mapping]] = ..., agent: _Optional[_Union[AgentProto, _Mapping]] = ..., entitlement: _Optional[_Union[EntitlementProto, _Mapping]] = ..., invite: _Optional[_Union[InviteProto, _Mapping]] = ..., membership: _Optional[_Union[MembershipProto, _Mapping]] = ..., permission: _Optional[_Union[PermissionProto, _Mapping]] = ..., role: _Optional[_Union[RoleProto, _Mapping]] = ..., sanction: _Optional[_Union[SanctionProto, _Mapping]] = ..., frame_view: _Optional[_Union[FrameViewProto, _Mapping]] = ..., internal_view: _Optional[_Union[InternalViewProto, _Mapping]] = ..., label_view: _Optional[_Union[LabelViewProto, _Mapping]] = ..., number_input_view: _Optional[_Union[NumberInputViewProto, _Mapping]] = ..., slider_input_view: _Optional[_Union[SliderInputViewProto, _Mapping]] = ..., split_view: _Optional[_Union[SplitViewProto, _Mapping]] = ..., text_view: _Optional[_Union[TextViewProto, _Mapping]] = ..., annotation_shape: _Optional[_Union[AnnotationShapeProto, _Mapping]] = ..., arrow_shape: _Optional[_Union[ArrowShapeProto, _Mapping]] = ..., canvas: _Optional[_Union[CanvasProto, _Mapping]] = ..., line_shape: _Optional[_Union[LineShapeProto, _Mapping]] = ..., file: _Optional[_Union[FileProto, _Mapping]] = ..., environment: _Optional[_Union[EnvironmentProto, _Mapping]] = ..., database: _Optional[_Union[DatabaseProto, _Mapping]] = ..., machine: _Optional[_Union[MachineProto, _Mapping]] = ..., method: _Optional[_Union[MethodProto, _Mapping]] = ..., action: _Optional[_Union[ActionProto, _Mapping]] = ..., event_cursor: _Optional[_Union[EventCursorProto, _Mapping]] = ..., screen_cursor: _Optional[_Union[ScreenCursorProto, _Mapping]] = ..., thread_cursor: _Optional[_Union[ThreadCursorProto, _Mapping]] = ..., route: _Optional[_Union[RouteProto, _Mapping]] = ..., script: _Optional[_Union[ScriptProto, _Mapping]] = ..., service: _Optional[_Union[ServiceProto, _Mapping]] = ..., timer: _Optional[_Union[TimerProto, _Mapping]] = ..., trigger: _Optional[_Union[TriggerProto, _Mapping]] = ..., gauge_metric: _Optional[_Union[GaugeMetricProto, _Mapping]] = ..., counter_metric: _Optional[_Union[CounterMetricProto, _Mapping]] = ..., histogram_metric: _Optional[_Union[HistogramMetricProto, _Mapping]] = ..., layer: _Optional[_Union[LayerProto, _Mapping]] = ..., scene: _Optional[_Union[SceneProto, _Mapping]] = ..., variant: _Optional[_Union[VariantProto, _Mapping]] = ..., window: _Optional[_Union[WindowProto, _Mapping]] = ..., follow: _Optional[_Union[FollowProto, _Mapping]] = ..., message: _Optional[_Union[MessageProto, _Mapping]] = ..., notification: _Optional[_Union[NotificationProto, _Mapping]] = ..., reaction: _Optional[_Union[ReactionProto, _Mapping]] = ..., star: _Optional[_Union[StarProto, _Mapping]] = ..., thread: _Optional[_Union[ThreadProto, _Mapping]] = ..., branch: _Optional[_Union[BranchProto, _Mapping]] = ..., folder: _Optional[_Union[FolderProto, _Mapping]] = ..., tag: _Optional[_Union[TagProto, _Mapping]] = ..., tagging: _Optional[_Union[TaggingProto, _Mapping]] = ..., color_style: _Optional[_Union[ColorStyleProto, _Mapping]] = ..., border_style: _Optional[_Union[BorderStyleProto, _Mapping]] = ..., transition_style: _Optional[_Union[TransitionStyleProto, _Mapping]] = ..., effect_style: _Optional[_Union[EffectStyleProto, _Mapping]] = ..., gradient_style: _Optional[_Union[GradientStyleProto, _Mapping]] = ..., fill_style: _Optional[_Union[FillStyleProto, _Mapping]] = ..., font_style: _Optional[_Union[FontStyleProto, _Mapping]] = ..., palette: _Optional[_Union[PaletteProto, _Mapping]] = ..., shadow_style: _Optional[_Union[ShadowStyleProto, _Mapping]] = ..., stroke_style: _Optional[_Union[StrokeStyleProto, _Mapping]] = ..., theme: _Optional[_Union[ThemeProto, _Mapping]] = ..., client: _Optional[_Union[ClientProto, _Mapping]] = ..., friendship: _Optional[_Union[FriendshipProto, _Mapping]] = ..., friendship_invite: _Optional[_Union[FriendshipInviteProto, _Mapping]] = ..., handle: _Optional[_Union[HandleProto, _Mapping]] = ..., organization: _Optional[_Union[OrganizationProto, _Mapping]] = ..., space: _Optional[_Union[SpaceProto, _Mapping]] = ..., team: _Optional[_Union[TeamProto, _Mapping]] = ..., user: _Optional[_Union[UserProto, _Mapping]] = ...) -> None: ...

class SomeEventProto(_message.Message):
    __slots__ = ("edit_event", "change_event", "query_event", "entitlement_requested_event", "entitlement_granted_event", "entitlement_revoked_event", "entitlement_expired_event", "invite_sent_event", "invite_rescinded_event", "invite_accepted_event", "invite_rejected_event", "membership_joined_event", "membership_left_event", "role_assigned_event", "role_unassigned_event", "sanction_requested_event", "sanction_granted_event", "sanction_revoked_event", "sanction_expired_event", "view_entered_event", "view_exited_event", "log_event", "run_started_event", "run_pause_requested_event", "run_paused_event", "run_resume_requested_event", "run_resumed_event", "run_stop_requested_event", "run_failed_event", "run_completed_event", "span_event", "copy_event", "cut_event", "paste_event", "drag_start_event", "drag_end_event", "drag_over_event", "drag_enter_event", "drag_leave_event", "drop_event", "focus_in_event", "focus_out_event", "key_down_event", "key_up_event", "key_press_event", "pointer_down_event", "pointer_up_event", "pointer_move_event", "pointer_enter_event", "pointer_over_event", "pointer_leave_event", "pointer_long_press_event", "left_click_event", "right_click_event", "middle_click_event", "double_click_event", "wheel_event", "timer_started_event", "timer_completed_event", "timer_cancelled_event", "gauge_measurement_event", "counter_measurement_event", "histogram_measurement_event", "notification_sent_event", "notification_rescinded_event", "notification_read_event", "notification_dismissed_event", "notification_expired_event", "friendship_invite_sent_event", "friendship_invite_rescinded_event", "friendship_invite_accepted_event", "friendship_invite_rejected_event")
    EDIT_EVENT_FIELD_NUMBER: _ClassVar[int]
    CHANGE_EVENT_FIELD_NUMBER: _ClassVar[int]
    QUERY_EVENT_FIELD_NUMBER: _ClassVar[int]
    ENTITLEMENT_REQUESTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    ENTITLEMENT_GRANTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    ENTITLEMENT_REVOKED_EVENT_FIELD_NUMBER: _ClassVar[int]
    ENTITLEMENT_EXPIRED_EVENT_FIELD_NUMBER: _ClassVar[int]
    INVITE_SENT_EVENT_FIELD_NUMBER: _ClassVar[int]
    INVITE_RESCINDED_EVENT_FIELD_NUMBER: _ClassVar[int]
    INVITE_ACCEPTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    INVITE_REJECTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    MEMBERSHIP_JOINED_EVENT_FIELD_NUMBER: _ClassVar[int]
    MEMBERSHIP_LEFT_EVENT_FIELD_NUMBER: _ClassVar[int]
    ROLE_ASSIGNED_EVENT_FIELD_NUMBER: _ClassVar[int]
    ROLE_UNASSIGNED_EVENT_FIELD_NUMBER: _ClassVar[int]
    SANCTION_REQUESTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    SANCTION_GRANTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    SANCTION_REVOKED_EVENT_FIELD_NUMBER: _ClassVar[int]
    SANCTION_EXPIRED_EVENT_FIELD_NUMBER: _ClassVar[int]
    VIEW_ENTERED_EVENT_FIELD_NUMBER: _ClassVar[int]
    VIEW_EXITED_EVENT_FIELD_NUMBER: _ClassVar[int]
    LOG_EVENT_FIELD_NUMBER: _ClassVar[int]
    RUN_STARTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    RUN_PAUSE_REQUESTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    RUN_PAUSED_EVENT_FIELD_NUMBER: _ClassVar[int]
    RUN_RESUME_REQUESTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    RUN_RESUMED_EVENT_FIELD_NUMBER: _ClassVar[int]
    RUN_STOP_REQUESTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    RUN_FAILED_EVENT_FIELD_NUMBER: _ClassVar[int]
    RUN_COMPLETED_EVENT_FIELD_NUMBER: _ClassVar[int]
    SPAN_EVENT_FIELD_NUMBER: _ClassVar[int]
    COPY_EVENT_FIELD_NUMBER: _ClassVar[int]
    CUT_EVENT_FIELD_NUMBER: _ClassVar[int]
    PASTE_EVENT_FIELD_NUMBER: _ClassVar[int]
    DRAG_START_EVENT_FIELD_NUMBER: _ClassVar[int]
    DRAG_END_EVENT_FIELD_NUMBER: _ClassVar[int]
    DRAG_OVER_EVENT_FIELD_NUMBER: _ClassVar[int]
    DRAG_ENTER_EVENT_FIELD_NUMBER: _ClassVar[int]
    DRAG_LEAVE_EVENT_FIELD_NUMBER: _ClassVar[int]
    DROP_EVENT_FIELD_NUMBER: _ClassVar[int]
    FOCUS_IN_EVENT_FIELD_NUMBER: _ClassVar[int]
    FOCUS_OUT_EVENT_FIELD_NUMBER: _ClassVar[int]
    KEY_DOWN_EVENT_FIELD_NUMBER: _ClassVar[int]
    KEY_UP_EVENT_FIELD_NUMBER: _ClassVar[int]
    KEY_PRESS_EVENT_FIELD_NUMBER: _ClassVar[int]
    POINTER_DOWN_EVENT_FIELD_NUMBER: _ClassVar[int]
    POINTER_UP_EVENT_FIELD_NUMBER: _ClassVar[int]
    POINTER_MOVE_EVENT_FIELD_NUMBER: _ClassVar[int]
    POINTER_ENTER_EVENT_FIELD_NUMBER: _ClassVar[int]
    POINTER_OVER_EVENT_FIELD_NUMBER: _ClassVar[int]
    POINTER_LEAVE_EVENT_FIELD_NUMBER: _ClassVar[int]
    POINTER_LONG_PRESS_EVENT_FIELD_NUMBER: _ClassVar[int]
    LEFT_CLICK_EVENT_FIELD_NUMBER: _ClassVar[int]
    RIGHT_CLICK_EVENT_FIELD_NUMBER: _ClassVar[int]
    MIDDLE_CLICK_EVENT_FIELD_NUMBER: _ClassVar[int]
    DOUBLE_CLICK_EVENT_FIELD_NUMBER: _ClassVar[int]
    WHEEL_EVENT_FIELD_NUMBER: _ClassVar[int]
    TIMER_STARTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    TIMER_COMPLETED_EVENT_FIELD_NUMBER: _ClassVar[int]
    TIMER_CANCELLED_EVENT_FIELD_NUMBER: _ClassVar[int]
    GAUGE_MEASUREMENT_EVENT_FIELD_NUMBER: _ClassVar[int]
    COUNTER_MEASUREMENT_EVENT_FIELD_NUMBER: _ClassVar[int]
    HISTOGRAM_MEASUREMENT_EVENT_FIELD_NUMBER: _ClassVar[int]
    NOTIFICATION_SENT_EVENT_FIELD_NUMBER: _ClassVar[int]
    NOTIFICATION_RESCINDED_EVENT_FIELD_NUMBER: _ClassVar[int]
    NOTIFICATION_READ_EVENT_FIELD_NUMBER: _ClassVar[int]
    NOTIFICATION_DISMISSED_EVENT_FIELD_NUMBER: _ClassVar[int]
    NOTIFICATION_EXPIRED_EVENT_FIELD_NUMBER: _ClassVar[int]
    FRIENDSHIP_INVITE_SENT_EVENT_FIELD_NUMBER: _ClassVar[int]
    FRIENDSHIP_INVITE_RESCINDED_EVENT_FIELD_NUMBER: _ClassVar[int]
    FRIENDSHIP_INVITE_ACCEPTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    FRIENDSHIP_INVITE_REJECTED_EVENT_FIELD_NUMBER: _ClassVar[int]
    edit_event: EditEventProto
    change_event: ChangeEventProto
    query_event: QueryEventProto
    entitlement_requested_event: EntitlementRequestedEventProto
    entitlement_granted_event: EntitlementGrantedEventProto
    entitlement_revoked_event: EntitlementRevokedEventProto
    entitlement_expired_event: EntitlementExpiredEventProto
    invite_sent_event: InviteSentEventProto
    invite_rescinded_event: InviteRescindedEventProto
    invite_accepted_event: InviteAcceptedEventProto
    invite_rejected_event: InviteRejectedEventProto
    membership_joined_event: MembershipJoinedEventProto
    membership_left_event: MembershipLeftEventProto
    role_assigned_event: RoleAssignedEventProto
    role_unassigned_event: RoleUnassignedEventProto
    sanction_requested_event: SanctionRequestedEventProto
    sanction_granted_event: SanctionGrantedEventProto
    sanction_revoked_event: SanctionRevokedEventProto
    sanction_expired_event: SanctionExpiredEventProto
    view_entered_event: ViewEnteredEventProto
    view_exited_event: ViewExitedEventProto
    log_event: LogEventProto
    run_started_event: RunStartedEventProto
    run_pause_requested_event: RunPauseRequestedEventProto
    run_paused_event: RunPausedEventProto
    run_resume_requested_event: RunResumeRequestedEventProto
    run_resumed_event: RunResumedEventProto
    run_stop_requested_event: RunStopRequestedEventProto
    run_failed_event: RunFailedEventProto
    run_completed_event: RunCompletedEventProto
    span_event: SpanEventProto
    copy_event: CopyEventProto
    cut_event: CutEventProto
    paste_event: PasteEventProto
    drag_start_event: DragStartEventProto
    drag_end_event: DragEndEventProto
    drag_over_event: DragOverEventProto
    drag_enter_event: DragEnterEventProto
    drag_leave_event: DragLeaveEventProto
    drop_event: DropEventProto
    focus_in_event: FocusInEventProto
    focus_out_event: FocusOutEventProto
    key_down_event: KeyDownEventProto
    key_up_event: KeyUpEventProto
    key_press_event: KeyPressEventProto
    pointer_down_event: PointerDownEventProto
    pointer_up_event: PointerUpEventProto
    pointer_move_event: PointerMoveEventProto
    pointer_enter_event: PointerEnterEventProto
    pointer_over_event: PointerOverEventProto
    pointer_leave_event: PointerLeaveEventProto
    pointer_long_press_event: PointerLongPressEventProto
    left_click_event: LeftClickEventProto
    right_click_event: RightClickEventProto
    middle_click_event: MiddleClickEventProto
    double_click_event: DoubleClickEventProto
    wheel_event: WheelEventProto
    timer_started_event: TimerStartedEventProto
    timer_completed_event: TimerCompletedEventProto
    timer_cancelled_event: TimerCancelledEventProto
    gauge_measurement_event: GaugeMeasurementEventProto
    counter_measurement_event: CounterMeasurementEventProto
    histogram_measurement_event: HistogramMeasurementEventProto
    notification_sent_event: NotificationSentEventProto
    notification_rescinded_event: NotificationRescindedEventProto
    notification_read_event: NotificationReadEventProto
    notification_dismissed_event: NotificationDismissedEventProto
    notification_expired_event: NotificationExpiredEventProto
    friendship_invite_sent_event: FriendshipInviteSentEventProto
    friendship_invite_rescinded_event: FriendshipInviteRescindedEventProto
    friendship_invite_accepted_event: FriendshipInviteAcceptedEventProto
    friendship_invite_rejected_event: FriendshipInviteRejectedEventProto
    def __init__(self, edit_event: _Optional[_Union[EditEventProto, _Mapping]] = ..., change_event: _Optional[_Union[ChangeEventProto, _Mapping]] = ..., query_event: _Optional[_Union[QueryEventProto, _Mapping]] = ..., entitlement_requested_event: _Optional[_Union[EntitlementRequestedEventProto, _Mapping]] = ..., entitlement_granted_event: _Optional[_Union[EntitlementGrantedEventProto, _Mapping]] = ..., entitlement_revoked_event: _Optional[_Union[EntitlementRevokedEventProto, _Mapping]] = ..., entitlement_expired_event: _Optional[_Union[EntitlementExpiredEventProto, _Mapping]] = ..., invite_sent_event: _Optional[_Union[InviteSentEventProto, _Mapping]] = ..., invite_rescinded_event: _Optional[_Union[InviteRescindedEventProto, _Mapping]] = ..., invite_accepted_event: _Optional[_Union[InviteAcceptedEventProto, _Mapping]] = ..., invite_rejected_event: _Optional[_Union[InviteRejectedEventProto, _Mapping]] = ..., membership_joined_event: _Optional[_Union[MembershipJoinedEventProto, _Mapping]] = ..., membership_left_event: _Optional[_Union[MembershipLeftEventProto, _Mapping]] = ..., role_assigned_event: _Optional[_Union[RoleAssignedEventProto, _Mapping]] = ..., role_unassigned_event: _Optional[_Union[RoleUnassignedEventProto, _Mapping]] = ..., sanction_requested_event: _Optional[_Union[SanctionRequestedEventProto, _Mapping]] = ..., sanction_granted_event: _Optional[_Union[SanctionGrantedEventProto, _Mapping]] = ..., sanction_revoked_event: _Optional[_Union[SanctionRevokedEventProto, _Mapping]] = ..., sanction_expired_event: _Optional[_Union[SanctionExpiredEventProto, _Mapping]] = ..., view_entered_event: _Optional[_Union[ViewEnteredEventProto, _Mapping]] = ..., view_exited_event: _Optional[_Union[ViewExitedEventProto, _Mapping]] = ..., log_event: _Optional[_Union[LogEventProto, _Mapping]] = ..., run_started_event: _Optional[_Union[RunStartedEventProto, _Mapping]] = ..., run_pause_requested_event: _Optional[_Union[RunPauseRequestedEventProto, _Mapping]] = ..., run_paused_event: _Optional[_Union[RunPausedEventProto, _Mapping]] = ..., run_resume_requested_event: _Optional[_Union[RunResumeRequestedEventProto, _Mapping]] = ..., run_resumed_event: _Optional[_Union[RunResumedEventProto, _Mapping]] = ..., run_stop_requested_event: _Optional[_Union[RunStopRequestedEventProto, _Mapping]] = ..., run_failed_event: _Optional[_Union[RunFailedEventProto, _Mapping]] = ..., run_completed_event: _Optional[_Union[RunCompletedEventProto, _Mapping]] = ..., span_event: _Optional[_Union[SpanEventProto, _Mapping]] = ..., copy_event: _Optional[_Union[CopyEventProto, _Mapping]] = ..., cut_event: _Optional[_Union[CutEventProto, _Mapping]] = ..., paste_event: _Optional[_Union[PasteEventProto, _Mapping]] = ..., drag_start_event: _Optional[_Union[DragStartEventProto, _Mapping]] = ..., drag_end_event: _Optional[_Union[DragEndEventProto, _Mapping]] = ..., drag_over_event: _Optional[_Union[DragOverEventProto, _Mapping]] = ..., drag_enter_event: _Optional[_Union[DragEnterEventProto, _Mapping]] = ..., drag_leave_event: _Optional[_Union[DragLeaveEventProto, _Mapping]] = ..., drop_event: _Optional[_Union[DropEventProto, _Mapping]] = ..., focus_in_event: _Optional[_Union[FocusInEventProto, _Mapping]] = ..., focus_out_event: _Optional[_Union[FocusOutEventProto, _Mapping]] = ..., key_down_event: _Optional[_Union[KeyDownEventProto, _Mapping]] = ..., key_up_event: _Optional[_Union[KeyUpEventProto, _Mapping]] = ..., key_press_event: _Optional[_Union[KeyPressEventProto, _Mapping]] = ..., pointer_down_event: _Optional[_Union[PointerDownEventProto, _Mapping]] = ..., pointer_up_event: _Optional[_Union[PointerUpEventProto, _Mapping]] = ..., pointer_move_event: _Optional[_Union[PointerMoveEventProto, _Mapping]] = ..., pointer_enter_event: _Optional[_Union[PointerEnterEventProto, _Mapping]] = ..., pointer_over_event: _Optional[_Union[PointerOverEventProto, _Mapping]] = ..., pointer_leave_event: _Optional[_Union[PointerLeaveEventProto, _Mapping]] = ..., pointer_long_press_event: _Optional[_Union[PointerLongPressEventProto, _Mapping]] = ..., left_click_event: _Optional[_Union[LeftClickEventProto, _Mapping]] = ..., right_click_event: _Optional[_Union[RightClickEventProto, _Mapping]] = ..., middle_click_event: _Optional[_Union[MiddleClickEventProto, _Mapping]] = ..., double_click_event: _Optional[_Union[DoubleClickEventProto, _Mapping]] = ..., wheel_event: _Optional[_Union[WheelEventProto, _Mapping]] = ..., timer_started_event: _Optional[_Union[TimerStartedEventProto, _Mapping]] = ..., timer_completed_event: _Optional[_Union[TimerCompletedEventProto, _Mapping]] = ..., timer_cancelled_event: _Optional[_Union[TimerCancelledEventProto, _Mapping]] = ..., gauge_measurement_event: _Optional[_Union[GaugeMeasurementEventProto, _Mapping]] = ..., counter_measurement_event: _Optional[_Union[CounterMeasurementEventProto, _Mapping]] = ..., histogram_measurement_event: _Optional[_Union[HistogramMeasurementEventProto, _Mapping]] = ..., notification_sent_event: _Optional[_Union[NotificationSentEventProto, _Mapping]] = ..., notification_rescinded_event: _Optional[_Union[NotificationRescindedEventProto, _Mapping]] = ..., notification_read_event: _Optional[_Union[NotificationReadEventProto, _Mapping]] = ..., notification_dismissed_event: _Optional[_Union[NotificationDismissedEventProto, _Mapping]] = ..., notification_expired_event: _Optional[_Union[NotificationExpiredEventProto, _Mapping]] = ..., friendship_invite_sent_event: _Optional[_Union[FriendshipInviteSentEventProto, _Mapping]] = ..., friendship_invite_rescinded_event: _Optional[_Union[FriendshipInviteRescindedEventProto, _Mapping]] = ..., friendship_invite_accepted_event: _Optional[_Union[FriendshipInviteAcceptedEventProto, _Mapping]] = ..., friendship_invite_rejected_event: _Optional[_Union[FriendshipInviteRejectedEventProto, _Mapping]] = ...) -> None: ...
