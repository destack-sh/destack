from bench.language.core import (
    BuiltinObjectMutable,
    IsInBench,
    IsResource,
    Node,
    NodeType,
    Region,
    StructMutable,
    StructType,
    node_,
    object_,
    property_,
    struct_,
)
from bench.pb2 import CellData

# pyright: reportIncompatibleVariableOverride=false


@object_()
class CellBase(BuiltinObjectMutable):
    """A Cell is a logical grouping of hosts."""

    # infra
    region: Region = property_(50, can_write="system", is_repr=True)
    cell_name: str = property_(51, can_write="system", is_repr=True)
    host: str = property_(52, can_write="system", is_repr=True)


@struct_(StructType.CELL_INFO)
class CellInfo(CellBase, StructMutable):
    pass


@node_(NodeType.CELL)
class Cell(IsResource, IsInBench, CellBase, Node[CellData]):
    pass
