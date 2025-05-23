from bench.language.core import (
    BuiltinEnum,
    EnumType,
    IsProvisionable,
    Node,
    NodeType,
    enum_,
    node_,
    p_system,
    property_,
)
from bench.pb2.lang_pb2 import ScalerData

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.SCALER_TYPE)
class ScalerType(BuiltinEnum):
    # NOTE: should match respective NodeType
    COMPUTER = 2100


@enum_(EnumType.SCALER_STRATEGY)
class ScalerStrategy(BuiltinEnum):
    MANUAL = 1
    AUTO = 2


@node_(NodeType.SCALER)
class Scaler(IsProvisionable, Node[ScalerData]):
    """
    A Scaler automatically scales another provisionable Resource.
    """

    type: ScalerType = p_system(30)

    # content
    strategy: ScalerStrategy = property_(60, default=ScalerStrategy.AUTO)
    target_count: int = property_(61, default=0)
    min_count: int = property_(62, default=0)
    max_count: int = property_(63, default=16)
    name_template: str | None = property_(65)
