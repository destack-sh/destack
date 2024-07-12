from dataclasses import dataclass
from typing import TYPE_CHECKING, cast
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.const import TypeKind
from bench.language.node import BuiltinObject, Node
from bench.language.value import SomeValue, ValueObject

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True)
class ProjectionOptions:
    max_depth: int = 5


@dataclass(slots=True)
class Projection:
    """
    A projection into the Bench graph.
    Should be a proper Struct at some point (so we can inspect it in the editor).
    """

    nodes_by_id: dict[UUID, Node]

    def add_node(self, node: Node) -> bool:
        if node.id in self.nodes_by_id:
            return False
        else:
            self.nodes_by_id[node.id] = node
            return True


def project_builtin_object(obj: BuiltinObject, projection: Projection) -> list[Node]:
    for prop in obj.__node_reference_properties__.values():
        pass


def project_value_scalar(value: SomeValue, projection: Projection) -> list[Node]: ...


def project_value_object(obj: ValueObject, projection: Projection) -> list[Node]:
    _value = obj._value
    assert _value is not None, f"{obj!r} has no value"

    for field in obj._type._base_fields:
        field_type = field._to_resolved()
        field_value = cast(SomeValue, _value.get(field.storage_key))
        if field_value is None:
            continue
        elif field_type.kind == TypeKind.OBJECT:
            if field_type.is_list:
                for item in cast(list[ValueObject], field_value):
                    project_value_object(item, projection)
            else:
                assert isinstance(field_value, ValueObject), f"{field_value!r} is not an object"
                project_value_object(field_value, projection)
        elif field_type.kind == TypeKind.NODE or field_type.kind == TypeKind.BASED_NODE:
            if field_type.is_list:
                for item in cast(list[Node], field_value):
                    project_builtin_object(cast(BuiltinObject, item), projection)
            else:
                assert isinstance(field_value, BuiltinObject), f"{field_value!r} is not a node"
                project_builtin_object(cast(BuiltinObject, field_value), projection)
        elif field_type.kind == TypeKind.STRUCT:
            project_builtin_object(cast(BuiltinObject, field_value), projection)


@tracer.start_as_current_span("projection.project")
def project(*objs: BuiltinObject | ValueObject | None) -> Projection:
    projection: Projection = Projection(nodes_by_id={})

    return projection
