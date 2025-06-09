from destack.language.core import (
    BuiltinObjectMutable,
    Region,
    StructMutable,
    StructType,
    object_,
    property_,
    struct_,
)

# pyright: reportIncompatibleVariableOverride=false


@object_()
class CellBase(BuiltinObjectMutable):
    """A Cell is a logical grouping of hosts."""

    # infra
    region: Region = property_(50, can_write="system", is_repr=True)
    name: str = property_(51, can_write="system", is_repr=True)
    host: str = property_(52, can_write="system", is_repr=True)


@struct_(StructType.CELL_INFO)
class CellInfo(CellBase, StructMutable):
    pass


# TODO :Infra: map Cells into actual Cell Nodes?
