from typing import TYPE_CHECKING, Self, final

from ..builtin import (
    MethodType,
    Object,
    StructType,
    declare_property,
    declare_struct,
)
from .function import FunctionDefinition

if TYPE_CHECKING:
    from destack.core import MethodDeclaration

    from .property import PropertyDefinition


type_ = type


@declare_struct(
    StructType.METHOD_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class MethodDefinition(FunctionDefinition):
    """Definition of a builtin Method."""

    type: MethodType = declare_property(100)

    # content
    input_properties: list["PropertyDefinition"] = declare_property(121)
    output_property: "PropertyDefinition | None" = declare_property(122)

    @classmethod
    def from_declaration(
        cls, object_cls: type_["Object"], declaration: "MethodDeclaration"
    ) -> "Self":
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
            input_properties=[
                PropertyDefinition.from_declaration(prop) for prop in declaration.input_properties
            ],
            output_property=PropertyDefinition.from_declaration(declaration.output_property)
            if declaration.output_property
            else None,
        )
