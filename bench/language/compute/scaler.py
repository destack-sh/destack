from bench.language.core import (
    BuiltinEnum,
    EnumType,
    NodeType,
    Resource,
    constraint,
    enum_,
    node_,
    p_regular,
    p_system,
    subnode_,
)
from bench.pb2.lang_pb2 import ScalerData

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.SCALER_TYPE)
class ScalerType(BuiltinEnum):
    # NOTE: should match respective NodeType
    MACHINE = 2100
    BROWSER = 2150


@enum_(EnumType.SCALER_STRATEGY)
class ScalerStrategy(BuiltinEnum):
    MANUAL = 1
    AUTO = 2


@node_(NodeType.SCALER, has_subtypes=True)
class Scaler(Resource[ScalerData]):
    """
    A Scaler is a static Resource that automatically scales a dynamic Resource.
    """

    type: ScalerType = p_system(30)

    # content
    strategy: ScalerStrategy = p_regular(60, default=ScalerStrategy.AUTO, default_sql=None)
    target_count: int = p_regular(61, default=0, constraint=constraint(min_value=0))
    min_count: int = p_regular(62, default=0, constraint=constraint(min_value=0, max_value=16))
    max_count: int = p_regular(63, default=16, constraint=constraint(min_value=0, max_value=64))
    min_ready_count: int = p_regular(64, default=0, constraint=constraint(min_value=0))

    # flags
    is_active: bool = p_regular(75, default=True)


@subnode_(NodeType.MACHINE)
class MachineScaler(Scaler):
    pass


@subnode_(NodeType.BROWSER)
class BrowserScaler(Scaler):
    pass
