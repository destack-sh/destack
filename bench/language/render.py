import base64
from dataclasses import dataclass
from datetime import datetime, timedelta
from typing import TYPE_CHECKING, cast

import structlog
from opentelemetry import trace

from bench.language.const import EnumType, PrimitiveType, TypeKind
from bench.language.node import BuiltinObject, Node, Struct
from bench.language.setup import ENUM_CLASS_BY_TYPE
from bench.language.value import ScalarValue, SomeValue, ValueObject

if TYPE_CHECKING:
    from bench.language.field import TypeInfoBase

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True)
class RenderOptions:
    scope: Node


class Renderer:
    def __init__(self, options: RenderOptions):
        self._options = options

    @property
    def scope(self) -> Node:
        return self._options.scope

    def _render_value_scalar(self, value: "ScalarValue", typ: "TypeInfoBase") -> str:
        """Renders single scalar value into Bench python."""
        if typ.kind == TypeKind.PRIMITIVE:
            if typ.primitive_type == PrimitiveType.BYTES:
                value_b64 = base64.b64encode(cast(bytes, value)).decode("utf-8")
                return f"base64.b64decode({value_b64!r})"
            elif typ.primitive_type == PrimitiveType.DATETIME:
                value_iso = cast(datetime, value).isoformat()
                return f"datetime.fromisoformat({value_iso!r})"
            elif typ.primitive_type == PrimitiveType.INTERVAL:
                return f"timedelta(seconds={cast(timedelta, value).total_seconds()})"
            else:
                return repr(value)
        elif typ.kind == TypeKind.NODE or typ.kind == TypeKind.BASED_NODE:
            assert isinstance(value, Node), f"{value!r} is not a node (expected {typ!r})"
            # nocheckin: render/alias node reference properly
            ident = value.py_ident
            assert ident is not None, f"{value!r} has no identifier"
            return ident
        elif typ.kind == TypeKind.ENUM:
            enum_cls = ENUM_CLASS_BY_TYPE[cast(EnumType, typ.bench_type)]
            value = enum_cls(value)
            return f"{enum_cls.__name__}.{value.name}"
        elif typ.kind == TypeKind.STRUCT:
            return self._render_builtin_object(cast(Struct, value))
        else:
            raise RuntimeError(f"unexpected type {typ!r}")

    def _render_value_object_scalar(self, value: "ValueObject", typ: "TypeInfoBase") -> str:
        """Renders single Object into Bench python (recursively)."""
        assert typ.base_type is not None, f"{value!r} has no base type"
        repr_by_name: dict[str, str] = {}
        for field in typ._base_fields:
            field_type = field._to_resolved()
            field_value = cast(SomeValue, getattr(value, field.name, None))
            field_value_repr = self._render_value(field_value, field_type)
            repr_by_name[field.name] = field_value_repr
        return f"{typ.base_type.name}({', '.join(f'{k}={v}' for k, v in repr_by_name.items())})"

    def _render_value(self, value: "SomeValue | None", typ: "TypeInfoBase") -> str:
        """Renders a value into Bench python (recursively)."""
        from bench.language.value import ValueObject

        if value is None:
            return "None"
        typ = typ._to_resolved()
        assert typ.kind != TypeKind.ALIAS, f"unresolved type {typ!r}"
        if typ.kind == TypeKind.OBJECT:
            # nested object
            if not typ.is_list:
                assert type(value) is ValueObject, f"{value!r} is not an object (expected {typ!r})"
                return self._render_value_object_scalar(value, typ)
            else:
                assert isinstance(value, list), f"{value!r} is not a list (expected {typ!r})"
                return f"[{', '.join(self._render_value_object_scalar(cast(ValueObject, v), typ) for v in value)}]"
        else:
            # scalar
            if not typ.is_list:
                return self._render_value_scalar(cast("ScalarValue", value), typ)
            else:
                assert isinstance(value, list), f"{value!r} is not a list (expected {typ!r})"
                return f"[{', '.join(self._render_value_scalar(v, typ) for v in value)}]"

    def _render_builtin_object(self, value: BuiltinObject) -> str:
        """Renders the given Node/Struct to Bench python."""
        raise NotImplementedError

    def render(self, *values: BuiltinObject | ValueObject | None) -> str:
        """Renders the given objects to Bench python and prettifies it."""
        raise NotImplementedError


@tracer.start_as_current_span("renderer.render")
def render(*objs: BuiltinObject | ValueObject | None, scope: Node) -> str:
    """Renders the given object to Bench python and prettifies it."""
    renderer = Renderer(RenderOptions(scope=scope))
    return renderer.render(*objs)
