from typing import TYPE_CHECKING, Optional
from uuid import UUID

from bench.language.core import (
    NAME_CONSTRAINT,
    REGION,
    SLUG_CONSTRAINT,
    BenchNode,
    BuiltinEnum,
    EnumType,
    LocalNodeList,
    NodeType,
    Region,
    StructType,
    enum_,
    node_,
    p_kernel,
    p_node_children,
    p_node_parent,
    p_regular,
    p_system,
)
from bench.language.core.node import IsOwnable
from bench.pb2 import BenchData
from bench.utils.func import generate_encryption_key

if TYPE_CHECKING:
    from bench.language import (
        Handle,
        Icon,
        NodeReference,
        Package,
        Region,
        Store,
        Text,
    )

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.BENCH_STATUS)
class BenchStatus(BuiltinEnum):
    """The status of a Bench"""

    RESERVED = 20  # not yet initialized
    ACTIVATED = 50


@node_(NodeType.BENCH, roots=())
class Bench(IsOwnable, BenchNode[BenchData]):
    """
    A Bench is an AI-native operating system for higher order software.
    """

    parent: None = p_node_parent(4)
    main_handle: Optional["Handle"] = p_system(
        31, require=False, array=False, references=NodeType.HANDLE, fk=True
    )  # not actually optional but Handle.parent = Bench
    handles: LocalNodeList["Handle"] = p_node_children(NodeType.HANDLE)
    slug: str = p_system(32, unique=True, constraint=SLUG_CONSTRAINT)  # must match main handle
    name: str = p_regular(33, constraint=NAME_CONSTRAINT)
    text: Optional["Text"] = p_regular(
        34, default=None, require=False, array=False, struct=StructType.TEXT
    )
    icon: Optional["Icon"] = p_regular(35, default=None, struct=StructType.ICON)
    region: "Region" = p_system(37, require=True, default=REGION, default_sql=None)
    encryption_key: str = p_kernel(
        38,
        require=True,
        encrypt=True,
        defer=True,
        sensitive=True,
        default_factory=lambda: generate_encryption_key(32),
    )

    # status
    status: BenchStatus = p_system(40, default=BenchStatus.RESERVED)

    # content
    main_store: Optional["Store"] = p_system(
        50, require=False, array=False, references=NodeType.STORE, fk=True, same_bench=True
    )
    main_package: Optional["Package"] = p_regular(
        51,
        require=False,
        array=False,
        references=NodeType.PACKAGE,
        fk=True,
        same_bench=True,
    )
    if TYPE_CHECKING:
        main_store_id: Optional[UUID] = None
        main_store_ptr: Optional[NodeReference] = None
        main_package_id: Optional[UUID] = None
        main_package_ptr: Optional[NodeReference] = None

    packages: LocalNodeList["Package"] = p_node_children(NodeType.PACKAGE)

    @property
    def is_attached(self) -> bool:
        return True
