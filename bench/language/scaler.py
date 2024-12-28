from bench.language.bench import StaticResource
from bench.language.const import EnumType, NodeType, enum_
from bench.language.node import node_, node_subtype_
from bench.language.property import p_regular
from bench.language.validation import constraint
from bench.proto.wire.lang_pb2 import ScalerData
from bench.utils.func import IdEnum


@enum_(EnumType.SCALER_STRATEGY)
class ScalerStrategy(IdEnum):
    MANUAL = 1
    AUTO = 2


@node_(NodeType.SCALER)
class Scaler(StaticResource[ScalerData]):
    """
    A Scaler is a static Resource that automatically scales a dynamic Resource.
    """

    type: NodeType = p_regular(30)

    # content
    strategy: ScalerStrategy = p_regular(50)
    target_count: int = p_regular(51, default=0, constraint=constraint(min_value=0))
    min_count: int = p_regular(52, default=0, constraint=constraint(min_value=0, max_value=16))
    max_count: int = p_regular(53, default=16, constraint=constraint(min_value=0, max_value=64))
    min_ready_count: int = p_regular(54, default=0, constraint=constraint(min_value=0))

    # flags
    is_active: bool = p_regular(80, default=True)


@node_subtype_(NodeType.MACHINE)
class MachineScaler(Scaler):
    pass


@node_subtype_(NodeType.BROWSER)
class BrowserScaler(Scaler):
    pass
