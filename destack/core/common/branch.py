from typing import (
    TYPE_CHECKING,
    final,
)

from ..builtin import (
    Entity,
    EnumType,
    NodeType,
    OptionEnum,
    TraitType,
    declare_entity,
    declare_enum,
    declare_option,
    declare_property,
)

if TYPE_CHECKING:
    pass


@declare_enum(EnumType.BRANCH_TYPE)
class BranchType(OptionEnum):
    """The type of a Branch."""

    PARTIAL = declare_option(2)
    FULL = declare_option(10)
    ROOT = declare_option(11)


@declare_entity(
    NodeType.BRANCH,
    is_final=True,
    traits=(TraitType.OWNABLE,),
)
@final
class Branch(Entity):
    """
    A Branch is a version of a Snapshot.
    """

    type: BranchType = declare_property(100)
