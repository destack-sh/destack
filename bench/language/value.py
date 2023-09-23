from typing import TYPE_CHECKING, Any, Collection

from bench.language.module import ModuleNode, NodeVisitor, node_component, nproperty, nruntime
from bench.utils.proxy import proxy_value

if TYPE_CHECKING:
    from bench.language import HasFields, Scope, Session


@node_component
class HasValue(ModuleNode):
    value: Any | None = nproperty(default=None)
    _value_unpacked: bool = nruntime(default=False)

    @property
    def _type_of_value(self) -> "HasFields":
        return self  # assume this is a HasFields

    def _clear(self):
        pass

    def _interp(self, scope: "Scope") -> None:
        pass  # TODO @Interp: interp value in HasValue

    def _index(self) -> None:
        pass

    def _visit(self, visitor: "NodeVisitor") -> None:
        pass

    def _validate(self, properties: Collection[str]):
        if "value" in properties:
            from bench.language.mapping import check_type

            check_type(self.value, self._type_of_value)

    def _onwrite_value(self, key: str) -> None:
        self.session.tracer.value_update(self, key)

    def _activate_in(self, session: "Session") -> None:
        from bench.language.mapping import unpack_value

        if self._value_unpacked:
            self.value = self._raw_value()
            self._value_unpacked = False
        # proxy
        value = unpack_value(
            self.value or {}, self._type_of_value, ignore_array=True, ignore_outer_map=True
        )
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
