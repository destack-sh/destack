from typing import TYPE_CHECKING, Optional, Self, final

from ..builtin import (
    MethodDeclaration,
    MethodType,
    StructType,
    UInt16,
    declare_property,
    declare_struct,
)
from .function import FunctionDefinition

if TYPE_CHECKING:
    from destack import MethodDeclaration, PropertyDefinition


type_ = type


@declare_struct(
    StructType.METHOD_DEFINITION,
    is_final=True,
)
@final
class MethodDefinition(FunctionDefinition):
    """Definition of a builtin Method."""

    # meta
    type: MethodType = declare_property(100)
    alias_of: Optional[UInt16] = declare_property(120)

    # content
    input_properties: list["PropertyDefinition"] = declare_property(131)
    output_property: Optional["PropertyDefinition"] = declare_property(132)

    @classmethod
    def from_declaration(cls, declaration: "MethodDeclaration") -> "Self":
        from .property import PropertyDefinition

        alias_of = None
        if declaration.alias_of:
            assert declaration.component is not None, (
                f"alias_of requires a component: {declaration!r}"
            )
            method = declaration.component.__dict__.get(declaration.alias_of)
            if not isinstance(method, MethodDeclaration):
                raise ValueError(
                    f"alias_of='{declaration.alias_of}' is not a method in {declaration.component.__name__}"
                )
            alias_of = method.id

        return cls(
            # meta
            id=declaration.id,
            type=declaration.type,
            name=declaration.name,
            description=declaration.description,
            is_async=declaration.is_async,
            is_internal=declaration.is_internal,
            alias_of=alias_of,
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
