from typing import TYPE_CHECKING, Any, Collection

from bench.language.const import TypeFlag
from bench.language.module import NS, ModuleNode, node_component, nproperty
from bench.language.validation import ValidationError, ValidationHandler
from bench.utils.proxy import proxy_value

if TYPE_CHECKING:
    from bench.language import HasFields, Session


@node_component
class HasValue(ModuleNode):
    value: Any | None = nproperty(default=None)

    @property
    def _type_of_value(self) -> "HasFields":
        return self  # assume this is a statement with fields

    def _validate_inner(self, properties: Collection[str], on_issue: "ValidationHandler") -> None:
        if "value" in properties:
            from bench.language.mapping import check_type

            try:
                is_array = bool(self._type_of_value.flags & TypeFlag.IsArray)
                check_type(self.value, self._type_of_value, ignore_array=is_array)
            except TypeError as e:
                on_issue(ValidationError(self, ["value"], str(e)))

    def _activate_inner(self, session: "Session") -> None:
        if self.value is None:
            return
        from bench.language.mapping import unpack_value

        def _onwrite_value(key: str) -> None:
            from bench.language.mapping import check_type

            is_array = bool(self._type_of_value.flags & TypeFlag.IsArray)
            check_type(self.value, self._type_of_value, ignore_array=is_array)
            self.session.tracer.node_update(self, ["value"])

        value = unpack_value(
            self.value, self._type_of_value, ignore_array=True, ignore_outer_map=True
        )
        value = proxy_value(value, onread=lambda *args: None, onwrite=_onwrite_value)
        self._set_untracked("value", value)

    def _deactivate_inner(self) -> None:
        self._set_untracked("value", self._raw_value())

    def _raw_value(self) -> dict | None:
        """The raw/stripped value with field keys."""
        from bench.language.mapping import pack_value

        if self.value is None or self._status != NS.Tracked:
            return self.value
        return pack_value(
            self.value,
            self._type_of_value,
            ignore_array=True,
            ignore_outer_map=True,
            none_if_invalid=True,
        )

    def _raw_named_value(self):
        """The raw/stripped value with field names."""
        from bench.language.mapping import map_value

        return map_value(
            self._raw_value(),
            self,
            ignore_array=True,
            ignore_outer_map=True,
            none_if_invalid=True,
            map_k=lambda f: (f.typed_key, f.py_ident),
        )
