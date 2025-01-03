from bench.language.core import (
    EnumType,
    NodeType,
    constraint,
    enum_,
    node_,
    node_subtype_,
    p_regular,
    p_system,
)
from bench.proto.wire.lang_pb2 import ScalerData
from bench.utils.func import IdEnum

from .resource import StaticResource

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.SCALER_TYPE)
class ScalerType(IdEnum):
    # NOTE: should match respective NodeType
    MACHINE = 2100
    BROWSER = 2150


@enum_(EnumType.SCALER_STRATEGY)
class ScalerStrategy(IdEnum):
    MANUAL = 1
    AUTO = 2


@node_(NodeType.SCALER)
class Scaler(StaticResource[ScalerData]):
    """
    A Scaler is a static Resource that automatically scales a dynamic Resource.
    """

    type: ScalerType = p_system(30)

    # content
    strategy: ScalerStrategy = p_regular(50, default=ScalerStrategy.AUTO, default_sql=None)
    target_count: int = p_regular(51, default=0, constraint=constraint(min_value=0))
    min_count: int = p_regular(52, default=0, constraint=constraint(min_value=0, max_value=16))
    max_count: int = p_regular(53, default=16, constraint=constraint(min_value=0, max_value=64))
    min_ready_count: int = p_regular(54, default=0, constraint=constraint(min_value=0))

    # flags
    is_active: bool = p_regular(80, default=True)
    is_main: bool = p_regular(
        81, default=False, description="Whether this is the main Scaler for the target Resource."
    )


@node_subtype_(NodeType.MACHINE)
class MachineScaler(Scaler):
    pass


@node_subtype_(NodeType.BROWSER)
class BrowserScaler(Scaler):
    pass
