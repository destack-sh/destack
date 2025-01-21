import abc
from typing import (
    TYPE_CHECKING,
    ClassVar,
    Self,
    Sequence,
    Union,
    cast,
    dataclass_transform,
    final,
)

import structlog
from opentelemetry import trace

from bench.pb2 import AnyStructData
from bench.utils.env import IS_DEV

from .const import StructType
from .object import BuiltinObject, object_
from .property import _PROPERTY_SPECIFIERS, Property, p_runtime, p_struct_parent

if TYPE_CHECKING:
    from bench.language import CustomObject, Field, Node

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def struct_[_ObjectT: BuiltinObject](struct_type: StructType):
    """Register a class as a concrete struct for the given struct type."""

    def decorate(cls: type[_ObjectT]) -> type[_ObjectT]:
        cls = object_(struct_type=struct_type, is_final=True, is_struct=True)(cls)
        if IS_DEV and cls.__name__ != "Struct" and cls.__name__ != "Struct":
            if not issubclass(cls, (Struct, Struct)):
                raise ValueError(f"{cls} is not a struct")

        return cast(type[_ObjectT], cls)

    return decorate


StructParent = Union["Struct", "Node", "CustomObject"]
StructParentKey = Union["Property", "Field"]


@object_()
class Struct[StructDataT: AnyStructData](BuiltinObject[StructDataT], abc.ABC):
    """A Struct is an ordered collection of Properties."""

    metatype: ClassVar[StructType]  # type: ignore

    __is_struct__: ClassVar[bool] = True

    parent: StructParent | None = p_struct_parent(3)
    parent_key: StructParentKey | None = p_runtime(default=None)

    def __content_str__(self) -> str:
        # default __content_str__ for Structs with all set properties
        value_strs = []
        for prop in self.__declared_properties__.values():
            prop_value = getattr(self, prop.name)
            if prop_value is not None and not (isinstance(prop_value, Sequence) and not prop_value):
                if prop.is_enum:
                    if prop.is_list:
                        prop_value_str = "|".join(p.bench_name for p in prop_value)
                    else:
                        prop_value_str = prop_value.bench_name  # type: ignore
                elif prop.reference_struct:
                    if prop.is_list:
                        prop_value_str = f"{prop.reference_struct.bench_name}[{len(prop_value)}]"
                    else:
                        prop_value_str = f"<{prop.reference_struct.bench_name} ...>"
                else:
                    prop_value_str = repr(prop_value)
                value_strs.append(f"{prop.name}={prop_value_str}")
        return ", ".join(value_strs)

    @final
    def __repr__(self):
        content_str = str(self)
        if content_str:
            return f"<{self.__class__.__name__} {content_str}>"
        else:
            return f"<{self.__class__.__name__}>"

    def replace(self, **kwargs) -> Self:
        """Replaces specific properties in this struct (in a copy)."""
        copy = self.clone()
        for prop_name, prop_value in kwargs.items():
            setattr(copy, prop_name, prop_value)
        return copy

    def override(self, override: "Self | None" = None, copy: bool = True, **kwargs) -> Self:
        """Overrides this Struct with set properties from another struct (in a copy)."""
        if override is None and len(kwargs) == 0:
            return self
        clone = self.clone() if copy else self
        if override is not None:
            for prop in self.__declared_properties__.values():
                override_value = getattr(override, prop.name)
                if clone.is_set(prop, override_value):
                    setattr(clone, prop.name, override_value)
        for key, value in kwargs.items():
            setattr(clone, key, value)
        return clone

    def set_default(self, override: "Self", copy: bool = True, **kwargs) -> Self:
        """Sets default values from another Struct. Like override but only sets if unset."""
        clone = self.clone() if copy else self
        for prop in override.__declared_properties__.values():
            override_value = getattr(override, prop.name)
            if clone.is_set(prop, override_value) and not self.is_set(prop):
                setattr(clone, prop.name, override_value)
        return clone

    def _move_to(
        self,
        parent: StructParent,
        parent_key: StructParentKey,
    ) -> Self:
        """Move or copy this Struct into given parent/prop."""
        assert self.__is_struct__, f"cannot copy non-struct {self!r}"  # this is overriden by Node
        if self.parent is None:  # detached
            self._do_set("parent", parent, track=False)
            self._do_set("parent_key", parent_key, track=False)
            return self
        else:
            copy = self._copy_to(parent, parent_key)
            return copy

    def _copy_to(self, parent: StructParent, parent_key: StructParentKey) -> Self:
        """Create a copy of this Struct for the given parent/prop."""
        kwargs = {p.name: getattr(self, p.name) for p in self.__wired_properties__.values()}
        kwargs["parent"] = parent
        kwargs["parent_key"] = parent_key
        copy = self.__class__(**kwargs)
        return copy
