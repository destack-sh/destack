from dataclasses import field
from typing import TYPE_CHECKING

from bench.language import HasFields
from bench.language.module import ModuleNode, ModuleVisitor, node
from bench.utils.proxy import proxy_value

if TYPE_CHECKING:
    from bench.language.session import Scope, Session


@node
class HasValue(HasFields, ModuleNode):
    value: dict | None = field(default_factory=dict)
    _value_unpacked: bool = False

    def _clear(self):
        pass

    def _interp(self, scope: "Scope") -> None:
        pass

    def _index(self) -> None:
        pass

    def _visit(self, visitor: "ModuleVisitor") -> None:
        pass

    def _onwrite_value(self, key: str) -> None:
        self.session.tracer.value_update(self, key)

    def _activate_in(self, session: "Session") -> None:
        from bench.language.mapping import unpack_value

        if self._value_unpacked:
            self.value = self._raw_value()
            self._value_unpacked = False
        # proxy
        value = unpack_value(self.value or {}, self, ignore_array=True, ignore_outer_map=True)
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
            return pack_value(self.value, self, ignore_array=True, ignore_outer_map=True)

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
