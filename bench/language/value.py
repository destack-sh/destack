from copy import deepcopy
from typing import TYPE_CHECKING, Any, Collection, Optional

from bench.language.node import NS, UNSET, Node, node_component, struct_property
from bench.language.text import Text
from bench.language.validation import ValidationHandler
from bench.sql.core import ColumnType
from bench.utils.proxy import proxy_value, unproxy_value

if TYPE_CHECKING:
    from bench.language import HasFields, NodeVisitor, Session


@node_component
class HasValue(Node):
    value: Any | None = struct_property(
        UNSET, default=None, copy=deepcopy, column_type=ColumnType.JSON
    )

    @property
    def _type_of_value(self) -> Optional["HasFields"]:
        return self  # assume this is a statement with fields

    def _validate_inner(self, properties: Collection[str], on_invalid: "ValidationHandler") -> None:
        # type may not be ready if not attached (e.g. Record in a Database)
        if "value" in properties and self._type_of_value is not None:
            from bench.language.packer import check_type

            try:
                get_k = lambda f: f.py_ident if self._status == NS.ACTIVE else f._typed_key  # noqa
                check_type(self.value or {}, self._type_of_value, get_k=get_k)
            except TypeError as e:
                on_invalid(self, f"invalid value: {e}", ["value"])

    def _visit_inner(self, visitor: "NodeVisitor") -> None:
        if self.value:  # :VisitValue
            from bench.language.packer import walk_value

            for n in walk_value(self.value, self._type_of_value):  # :VisitValue
                if isinstance(n, Node):
                    visitor.visit_reference(n)
                elif isinstance(n, Text):
                    for mention in n.mentions:
                        if isinstance(mention.reference, Node):
                            visitor.visit_reference(mention.reference)

    def _attached_inner(self) -> None:
        # pack this value if it couldn't be packed in deactivate/detach
        #  (e.g. the type wasn't available on instantiation)
        assert self._type_of_value is not None, f"missing type for {self!r}"
        is_packed = (
            self._type_of_value.resolved_fields
            and self.value
            and any(self.value.get(k.py_ident) for k in self._type_of_value.resolved_fields)
        )
        if is_packed and self.module:
            from bench.language.packer import check_type, pack_value

            check_type(self.value, self._type_of_value)
            value = pack_value(
                self.value, self._type_of_value, ignore_outer=True, none_if_invalid=True
            )
            self._set_untracked("value", value)

    def _activate_inner(self, session: "Session") -> None:
        if self.value is None:
            return
        from bench.language.packer import TypedDict, unpack_value

        if not self.attached and not self._type_of_value:
            # don't activate new nodes that don't have a type from yet (e.g., Records)
            return

        def _onwrite_value(key: str) -> None:
            from bench.language.packer import check_type

            check_type(self.value, self._type_of_value)
            if self.attached:
                self.session.update(self, ["value"])

        assert self._type_of_value is not None, f"missing type for {self!r}"
        if self.attached:
            value = unpack_value(
                self.value, self._type_of_value, session=self._session, ignore_outer=True
            )
        else:
            value = self.value  # don't unpack if not attached (user sets 'unpacked' values)
        value = TypedDict(value, self._type_of_value)
        value = proxy_value(value, onread=lambda *args: None, onwrite=_onwrite_value)
        self._set_untracked("value", value)

    def _deactivate_inner(self) -> None:
        self._set_untracked("value", self._raw_value())

    def _raw_value(self, _force: bool = False) -> dict | None:
        """The raw/stripped value with field keys."""
        from bench.language.packer import pack_value

        if self.value is None:
            return None
        if not _force and (self._status != NS.ACTIVE or not self.attached):
            return unproxy_value(self.value)
        assert self._type_of_value is not None, f"missing type for {self!r}"
        return pack_value(self.value, self._type_of_value, ignore_outer=True, none_if_invalid=True)
