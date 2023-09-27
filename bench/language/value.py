from typing import TYPE_CHECKING, Any, Collection

from bench.language.issue import ValidationError, ValidationHandler
from bench.language.module import ModuleNode, NodeVisitor, node_component, nproperty, nruntime
from bench.utils.proxy import proxy_value

if TYPE_CHECKING:
    from bench.language import HasFields, ScopeNode, Session


@node_component
class HasValue(ModuleNode):
    value: Any | None = nproperty(default=None)
    _value_unpacked: bool = nruntime(default=False)

    @property
    def _type_of_value(self) -> "HasFields":
        return self  # assume this is a HasFields

    def _interp_inner(self, scope: "ScopeNode") -> None:
        pass  # TODO @Interp: interp value in HasValue

    def _visit_inner(self, visitor: "NodeVisitor") -> None:
        pass

    def _validate(self, properties: Collection[str], on_issue: "ValidationHandler") -> None:
        if "value" in properties:
            from bench.language.mapping import check_type

            try:
                check_type(self.value, self._type_of_value)
            except TypeError as e:
                on_issue(ValidationError(self, ["value"], str(e)))

    def _onwrite_value(self, key: str) -> None:
        self.session.tracer.node_update(self, key)

    def _activate_inner(self, session: "Session") -> None:
        from bench.language.mapping import unpack_value

        # should probably move instantiate into interp and only do proxying in activate?
        if self._value_unpacked:
            self.value = self._raw_value()
            self._value_unpacked = False
        # instantiate
        value = unpack_value(
            self.value or {}, self._type_of_value, ignore_array=True, ignore_outer_map=True
        )
        # proxy
        self.value = proxy_value(value, onread=lambda *args: None, onwrite=self._onwrite_value)
        self._value_unpacked = True

    def _deactivate(self) -> None:
        if self._value_unpacked:
            self.value = self._raw_value()
            self._value_unpacked = False

    def _raw_value(self) -> dict:
        """The raw/stripped value with field keys."""
        from bench.language.mapping import pack_value

        if not self._value_unpacked:
            return self.value
        else:
            return pack_value(
                self.value, self._type_of_value, ignore_array=True, ignore_outer_map=True
            )

    def _raw_named_value(self):
        """The raw/stripped value with field names."""
        from bench.language.mapping import map_value

        return map_value(
            self._raw_value(),
            self,
            ignore_array=True,
            ignore_outer_map=True,
            map_k=lambda f: (f.typed_key, f.py_ident),
        )
