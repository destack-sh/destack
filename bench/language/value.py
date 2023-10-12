from copy import deepcopy
from typing import TYPE_CHECKING, Any, Collection, Optional

from bench.language.const import TypeFlag
from bench.language.module import NS, Node, node_component, nproperty
from bench.language.validation import ValidationHandler
from bench.utils.proxy import proxy_value

if TYPE_CHECKING:
    from bench.language import HasFields, NodeVisitor, Session


@node_component
class HasValue(Node):
    value: Any | None = nproperty(default_factory=dict, copy=deepcopy)

    @property
    def _type_of_value(self) -> Optional["HasFields"]:
        return self  # assume this is a statement with fields

    def _validate_inner(self, properties: Collection[str], on_issue: "ValidationHandler") -> None:
        # type may not be ready if not attached (e.g. Record in a Database)
        if "value" in properties and self._type_of_value is not None:
            from bench.language.typing import check_type

            try:
                is_array = bool(self._type_of_value.flags & TypeFlag.IsArray)
                get_k = lambda f: f.py_ident if self._status == NS.Tracked else f._typed_key  # noqa
                check_type(
                    self.value or {}, self._type_of_value, get_k=get_k, ignore_array=is_array
                )
            except TypeError as e:
                on_issue(self, f"invalid value: {e}", ["value"])

    def _visit_inner(self, visitor: "NodeVisitor") -> None:
        pass  # TODO @Broken: visit referenced nodes :NodesAsValues

    def _activate_inner(self, session: "Session") -> None:
        if self.value is None:
            return
        from bench.language.typing import TypedDict, unpack_value

        if not self._type_of_value and not self.attached:
            return  # ignore for e.g. new Records that don't have a parent type from DB yet

        def _onwrite_value(key: str) -> None:
            from bench.language.typing import check_type

            is_array = bool(self._type_of_value.flags & TypeFlag.IsArray)
            check_type(self.value, self._type_of_value, ignore_array=is_array)
            if self.attached:
                self.session.tracer.node_update(self, ["value"])

        assert self._type_of_value is not None, f"missing type for {self!r}"
        value = unpack_value(
            self.value, self._type_of_value, ignore_array=True, ignore_outer_map=True
        )
        value = TypedDict(value, self._type_of_value)
        value = proxy_value(value, onread=lambda *args: None, onwrite=_onwrite_value)
        self._set_untracked("value", value)

    def _deactivate_inner(self) -> None:
        self._set_untracked("value", self._raw_value())

    def _raw_value(self, _force: bool = False) -> dict | None:
        """The raw/stripped value with field keys."""
        from bench.language.typing import pack_value

        if self.value is None or self._status != NS.Tracked and not _force:
            return self.value
        assert self._type_of_value is not None, f"missing type for {self!r}"
        return pack_value(
            self.value,
            self._type_of_value,
            ignore_array=True,
            ignore_outer_map=True,
            none_if_invalid=True,
        )

    def _raw_named_value(self):
        """The raw/stripped value with field names."""
        from bench.language.typing import map_value

        assert self._type_of_value is not None, f"missing type for {self!r}"
        return map_value(
            self._raw_value(),
            self._type_of_value,
            ignore_array=True,
            ignore_outer_map=True,
            none_if_invalid=True,
            map_k=lambda f: (f._typed_key, f.py_ident),
        )
