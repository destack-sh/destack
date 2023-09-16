from dataclasses import field

from bench.language.module import ModuleNode, node
from bench.utils.proxy import proxy_value


@node
class HasValue(ModuleNode):
    value: dict | None = field(default_factory=dict)

    def _onwrite_value(self, key: str) -> None:
        self.session.tracer.value_update(self, key)

    def _activate_in(self, session: "Session") -> None:
        from bench.language.mapping import unpack_value

        # nocheckin: handle statement type behavior (HasX)
        if self._unpacked:
            self.value = self._raw_value()
            self._unpacked = False
        # proxy
        value = self.value or {}
        value = unpack_value(value, self, ignore_array=True, ignore_outer_map=True)
        self.value = proxy_value(value, onread=lambda *args: None, onwrite=self._onwrite_value)
        self._unpacked = True
        super()._activate_in(session)

    def _deactivate(self) -> None:
        super()._deactivate()
        if self._unpacked:
            self.value = self._raw_value()
            self._unpacked = False

    def _raw_value(self) -> dict:
        """The raw/stripped value with field keys."""
        from bench.language.mapping import pack_value

        if not self._unpacked:
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
