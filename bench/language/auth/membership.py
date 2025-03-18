from typing import TYPE_CHECKING, Union

from bench.language.core import (
    JOINABLE_NODE_TYPES,
    SUBJECT_TYPES,
    BenchNode,
    BuiltinEnum,
    EnumType,
    Joinable,
    NodeType,
    Subject,
    enum_,
    node_,
    p_internal,
    p_node_parent,
    p_regular,
)
from bench.pb2.lang_pb2 import MembershipData

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.MEMBERSHIP_TYPE)
class MembershipType(BuiltinEnum):
    BENCH = 10
    ORGANIZATION = 20
    TEAM = 30
    CHANNEL = 100
    THREAD = 110


@node_(NodeType.MEMBERSHIP, roots=(NodeType.BENCH, NodeType.ORGANIZATION))
class Membership(BenchNode[MembershipData]):
    """
    A Membership to something for someone.
    """

    parent: Union[Joinable, None] = p_node_parent(4, *JOINABLE_NODE_TYPES)
    # meta
    type: MembershipType = p_regular(30, require=True)

    # content
    member: Subject = p_internal(41, require=True, array=False, references=SUBJECT_TYPES.tuple)
