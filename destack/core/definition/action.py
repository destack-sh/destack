from typing import TYPE_CHECKING, Optional, Self, final

from ..builtin import (
    ActionType,
    StructType,
    declare_property,
    declare_struct,
)
from .function import FunctionDefinition

if TYPE_CHECKING:
    from destack import ActionDeclaration


type_ = type


@declare_struct(
    StructType.ACTION_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class ActionDefinition(FunctionDefinition):
    """Definition of a builtin Action."""

    type: ActionType = declare_property(100)

    # content
    input_message_type: Optional["StructType"] = declare_property(120)
    output_message_type: Optional["StructType"] = declare_property(121)

    @classmethod
    def from_declaration(cls, declaration: "ActionDeclaration") -> "Self":
        return cls(
            # meta
            id=declaration.id,
            type=declaration.type,
            name=declaration.name,
            description=declaration.description,
            is_async=declaration.is_async,
            is_internal=declaration.is_internal,
            # availability
            platforms=list(declaration.platforms),
            languages=list(declaration.languages),
            runtimes=list(declaration.runtimes),
            # content
            input_message_type=declaration.input_message_type,
            output_message_type=declaration.output_message_type,
        )
