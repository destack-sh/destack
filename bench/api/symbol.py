from typing import TYPE_CHECKING, Annotated, Optional

from strawberry import lazy
from strawberry_django_plus import gql
from strawberry_django_plus.gql import auto

from bench import models
from bench.api.misc import SchemaElement

if TYPE_CHECKING:
    from bench.api.project import File, ProjectVersion


@gql.django.type(models.Symbol)
class Symbol(gql.Node):
    project_version: Annotated["ProjectVersion", lazy(".project")]
    name: auto
    type: auto
    type_shortname: auto
    type_name_declaration: auto
    file: Annotated["File", lazy(".project")]
    parent: Optional["Symbol"]
    children: list["Symbol"]
    index: auto
    created_at: auto
    updated_at: auto
    generated: auto
    content: "SymbolContent"
    parameters: list["SymbolParameter"]
    arguments: list["SymbolArgument"]


@gql.django.interface(models.SymbolContent)
class SymbolContent(gql.Node):
    # TODO @Cleanup: add symbol in SymbolContent interface
    #  (should work since it's a 1:1 but strawberry claims it's undefined)
    # symbol: Symbol
    pass


@gql.django.type(models.SymbolParameter)
class SymbolParameter(gql.Node):
    symbol: Symbol
    name: auto
    created_at: auto
    updated_at: auto
    type: auto
    schema: Optional[SchemaElement]


@gql.django.type(models.SymbolArgument)
class SymbolArgument(gql.Node):
    symbol: Symbol
    name: auto
    created_at: auto
    updated_at: auto
    type: auto
    reference: Optional[Symbol]
    value: auto


@gql.django.input(models.Symbol)
class SymbolCreateInput:
    project_version: auto
    name: auto
    type: auto
    file: auto
    parent: auto
    index: auto


@gql.django.partial(models.Symbol)
class SymbolRenameInput(gql.NodeInput):
    name: auto


@gql.type
class SymbolMutation:
    create_symbol: Symbol = gql.django.create_mutation(SymbolCreateInput)
    rename_symbol: Symbol = gql.django.update_mutation(SymbolRenameInput)
