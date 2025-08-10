from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, NamedTuple

if TYPE_CHECKING:
    from destack import NodeDefinition, StructDefinition, Type


class ObjectSize(NamedTuple):
    """The estimated size of an Object definition in bytes."""

    min_size: int
    max_size: int | None  # unset means unbounded


class ObjectSizer(ABC):
    """Estimate the actual size of an Object definition in bytes."""

    @abstractmethod
    def size_object(self, object: "StructDefinition | NodeDefinition") -> ObjectSize:
        """Get the estimated size of an object in bytes."""
        raise NotImplementedError

    @abstractmethod
    def size_type(self, type: "Type", include_reference: bool = True) -> ObjectSize:
        """
        Get the estimated size of a Type's value in bytes.
        If include_reference we add the overhead for a reference to this value.
         (In Python for instance every value is an object, so we account for the pointer overhead.)
        """
        raise NotImplementedError

    @abstractmethod
    def size_type_scalar(self, type: "Type") -> ObjectSize:
        """Get the estimated size of a scalar type in bytes."""
        raise NotImplementedError
