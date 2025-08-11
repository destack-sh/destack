import dataclasses
from dataclasses import dataclass
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    Literal,
    Optional,
)

from ._const import UNSET
from ._hoisted import (
    PropertyZone,
    ReferenceType,
    ScalarType,
    TypeCardinality,
    ValueFactory,
)
from .declaration import Declaration
from .enum import OptionDeclaration
from .type import NONE_TYPE_DECLARATION, TypeDeclaration, parse_type_declaration
from .universe import (
    NodeType,
    ObjectKind,
    StructType,
)

if TYPE_CHECKING:
    from destack import (
        Condition,
        Object,
        PropertyDefinition,
        PropertyReference,
        Sort,
        Value,
    )

type_ = type


@dataclass(eq=False, slots=True, repr=False)
class PropertyDeclaration(Declaration):
    """
    A system-defined attribute of an Object (Struct or Node).
    PropertyDeclarations are turned into PropertyDefinitions during construction,
     PropertyDeclarations (like their *Declaration brethren) are only for internal use.
    """

    # meta
    zone: PropertyZone = PropertyZone.MEMBER
    id: int = UNSET
    ord: int | None = None
    name: str = UNSET  # name from LHS of assignment
    description: str | None = None
    component: type_["Object"] = UNSET  # builtin object component
    original_component: type_["Object"] = UNSET  # original component (first in chain)
    tags: tuple[str, ...] = ()

    # type
    py_type: Any = type_(None)  # noqa: RUF009
    type: TypeDeclaration = dataclasses.field(default_factory=lambda: NONE_TYPE_DECLARATION)

    # defaults
    default_value: Any | None = None
    default_factory: ValueFactory | None = None
    default_factory_callable: Callable[[], Any] | None = None  # only for runtime properties

    # relationships
    reference_type: ReferenceType | None = None
    reference_as_value: bool = False

    # flags
    is_repr: bool = False  # included in Object.__repr__
    is_hash: bool = True  # included in Object.__hash__
    is_eq: bool = True  # included in Object.equals check
    is_managed: bool = False  # managed internally by the system
    is_static: bool = False  # set automatically at runtime
    is_readonly: bool = False  # can only be set once (at init time)
    is_runtime_only: bool = False  # only set at runtime
    is_interned: bool = False  # should be interned at runtime
    is_boxed: bool = False  # should be boxed at runtime

    _ref: Optional["PropertyReference"] = None
    _definition: Optional["PropertyDefinition"] = None

    def __str__(self):
        if self.component is None:
            return "<detached>"
        return f"{self.component.__name__}.{self.name}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s} ({self.id or '<unset>'})>"

    def __eq__(self, other: Any) -> bool:
        if not isinstance(other, PropertyDeclaration):
            return False
        return self.component == other.component and self.id == other.id

    def clone(self):
        return dataclasses.replace(
            self,
            component=None,
            original_component=self.original_component,
            _ref=None,
            _definition=None,
        )

    def to_ref(self) -> "PropertyReference":
        """A pointer to this Property. `to_ref()` for consistency with `Node.to_ref()`."""

        if self._ref is None:
            from ..common import PropertyReference

            assert self.component is not None, f"{self!r} has no component"
            assert self.id is not None, f"{self!r} has no id"
            metatype = getattr(self.component, "metatype", None)

            if self.component.__declaration__.kind == ObjectKind.NODE:
                ref = PropertyReference(node_type=metatype, id=self.id)
            else:
                ref = PropertyReference(struct_type=metatype, id=self.id)
            self._ref = ref

        return self._ref

    @property
    def code_name(self) -> str:
        return self.name

    @property
    def has_id(self) -> int:
        return self.id is not None and self.id is not UNSET

    @property
    def definition(self) -> "PropertyDefinition":
        if self._definition is None:
            from ..definition import PropertyDefinition

            self._definition = PropertyDefinition.from_declaration(self)
        return self._definition

    def determine(self, object_type: int | None, is_root_node: bool) -> None:
        """Determine type information from annotation, add _ptr property if needed."""
        assert self.py_type is not None, f"no py_type for {self!r}"

        # parse annotation
        try:
            self.type = parse_type_declaration(
                self.py_type,
                reference_type="value" if self.reference_as_value else self.reference_type,
                is_builtin=True,
            )
        except Exception as e:
            raise ValueError(
                f"unexpected type: {self.component.__name__}.{self.name} ({self.py_type})"
            ) from e

        # resolve self type
        if self.type.is_self and object_type is not None:
            assert isinstance(object_type, OptionDeclaration)
            if issubclass(object_type.component, NodeType):
                self.type.node_types = (object_type,)  # type: ignore
            elif issubclass(object_type.component, StructType):
                self.type.struct_type = object_type  # type: ignore
            else:
                raise ValueError(f"unexpected object type for Self: {object_type!r}")

        # check any type
        if self.type.is_any and object_type != StructType.VALUE and not self.is_runtime_only:
            raise ValueError(f"Any is only allowed in Value: {self!r}")

        # parent must be optional
        if self.name == "parent" and self.type.is_required:
            raise ValueError(f"Entity.parent must be optional: {self!r}")
        # root nodes don't have a parent
        if self.name == "parent" and is_root_node:
            self.type.node_types = ()
        # 'type' must be 30
        if (self.name == "type") != (self.id == 100):
            raise ValueError(f"'type' must be 100: {self!r}")

        # default to None if not required and no default
        if not self.type.is_required and self.default_value is UNSET:
            self.default_value = None
        # default to regular node references
        if self.type.scalar_type == ScalarType.NODE_MOMENT and self.reference_type is None:
            self.reference_type = ReferenceType.MOMENT
        # references get a _ptr property (which is wired/stored)
        if (
            self.type.value_type is not None
            and self.type.value_type.scalar_type == ScalarType.NODE_MOMENT
        ):
            # (don't want lists of Node references or Property references in Nodes, it's a mess)
            assert (
                self.type.cardinality == TypeCardinality.SCALAR
                or self.component.__declaration__.kind != ObjectKind.NODE
            ), f"cannot have a list of Node references: {self!r}"

    #
    # Querying
    #

    def eq(self, value: Any) -> "Condition":
        from destack import Condition, ConditionalType, Expression, ExpressionType

        if value is None:
            return self.not_exists()
        return Condition(
            type=ConditionalType.EQUALS,
            left=Expression(type=ExpressionType.ATTRIBUTE, attribute=self.to_ref()),
            right=Expression(type=ExpressionType.LITERAL, literal=value),
        )

    def neq(self, value: Any) -> "Condition":
        from destack import Condition, ConditionalType, Expression, ExpressionType

        if value is None:
            return self.exists()
        return Condition(
            type=ConditionalType.NOT_EQUALS,
            left=Expression(type=ExpressionType.ATTRIBUTE, attribute=self.to_ref()),
            right=Expression(type=ExpressionType.LITERAL, literal=value),
        )

    def gt(self, value: Any) -> "Condition":
        from destack import Condition, ConditionalType, Expression, ExpressionType

        return Condition(
            type=ConditionalType.GREATER_THAN,
            left=Expression(type=ExpressionType.ATTRIBUTE, attribute=self.to_ref()),
            right=Expression(type=ExpressionType.LITERAL, literal=value),
        )

    def gte(self, value: Any) -> "Condition":
        from destack import Condition, ConditionalType, Expression, ExpressionType

        return Condition(
            type=ConditionalType.GREATER_THAN_OR_EQUALS,
            left=Expression(type=ExpressionType.ATTRIBUTE, attribute=self.to_ref()),
            right=Expression(type=ExpressionType.LITERAL, literal=value),
        )

    def lt(self, value: Any) -> "Condition":
        from destack import Condition, ConditionalType, Expression, ExpressionType

        return Condition(
            type=ConditionalType.LESS_THAN,
            left=Expression(type=ExpressionType.ATTRIBUTE, attribute=self.to_ref()),
            right=Expression(type=ExpressionType.LITERAL, literal=value),
        )

    def lte(self, value: Any) -> "Condition":
        from destack import Condition, ConditionalType, Expression, ExpressionType

        return Condition(
            type=ConditionalType.LESS_THAN_OR_EQUALS,
            left=Expression(type=ExpressionType.ATTRIBUTE, attribute=self.to_ref()),
            right=Expression(type=ExpressionType.LITERAL, literal=value),
        )

    def starts_with(self, value: str) -> "Condition":
        from destack import Condition, ConditionalType, Expression, ExpressionType

        return Condition(
            type=ConditionalType.STARTS_WITH,
            left=Expression(type=ExpressionType.ATTRIBUTE, attribute=self.to_ref()),
            right=Expression(type=ExpressionType.LITERAL, literal=Value.of(value)),
        )

    def ends_with(self, value: str) -> "Condition":
        from destack import Condition, ConditionalType, Expression, ExpressionType

        return Condition(
            type=ConditionalType.ENDS_WITH,
            left=Expression(type=ExpressionType.ATTRIBUTE, attribute=self.to_ref()),
            right=Expression(type=ExpressionType.LITERAL, literal=Value.of(value)),
        )

    def in_(self, *values: Any) -> "Condition":
        from destack import Condition, ConditionalType, Expression, ExpressionType

        return Condition(
            type=ConditionalType.IN,
            left=Expression(type=ExpressionType.ATTRIBUTE, attribute=self.to_ref()),
            right=Expression(type=ExpressionType.LITERAL, literal=Value.of(values)),
        )

    def not_in(self, *values: Any) -> "Condition":
        from destack import Condition, ConditionalType, Expression, ExpressionType

        return Condition(
            type=ConditionalType.NOT_IN,
            left=Expression(type=ExpressionType.ATTRIBUTE, attribute=self.to_ref()),
            right=Expression(type=ExpressionType.LITERAL, literal=Value.of(values)),
        )

    def exists(self) -> "Condition":
        from destack import Condition, ConditionalType, Expression, ExpressionType

        return Condition(
            type=ConditionalType.EXISTS,
            left=Expression(type=ExpressionType.ATTRIBUTE, attribute=self.to_ref()),
        )

    def is_not_none(self) -> "Condition":
        from destack import Condition, ConditionalType, Expression, ExpressionType

        return Condition(
            type=ConditionalType.EXISTS,
            left=Expression(type=ExpressionType.ATTRIBUTE, attribute=self.to_ref()),
        )

    def not_exists(self) -> "Condition":
        from destack import Condition, ConditionalType, Expression, ExpressionType

        return Condition(
            type=ConditionalType.NOT_EXISTS,
            left=Expression(type=ExpressionType.ATTRIBUTE, attribute=self.to_ref()),
        )

    def is_none(self) -> "Condition":
        from destack import Condition, ConditionalType, Expression, ExpressionType

        return Condition(
            type=ConditionalType.NOT_EXISTS,
            left=Expression(type=ExpressionType.ATTRIBUTE, attribute=self.to_ref()),
        )

    def asc(self) -> "Sort":
        from destack import Expression, ExpressionType, Sort, SortType

        return Sort(
            type=SortType.ASCENDING,
            by=Expression(type=ExpressionType.ATTRIBUTE, attribute=self.to_ref()),
        )

    def desc(self) -> "Sort":
        from destack import Expression, ExpressionType, Sort, SortType

        return Sort(
            type=SortType.DESCENDING,
            by=Expression(type=ExpressionType.ATTRIBUTE, attribute=self.to_ref()),
        )


def declare_property(
    id: int,
    *,
    description: str | None = None,
    default: Any = UNSET,
    default_factory: ValueFactory | None = None,
    reference_type: ReferenceType | Literal["value"] | None = None,
    is_managed: bool = False,
    is_repr: bool = False,
    is_hash: bool = True,
    is_eq: bool = True,
    is_readonly: bool = False,
    is_interned: bool = False,
    tags: tuple[str, ...] = (),
) -> Any:
    assert id < 256, f"id must be less than 256: {id}"  # for :Encoding
    return PropertyDeclaration(
        id=id,
        description=description,
        default_value=default,
        default_factory=default_factory,
        reference_type=reference_type if reference_type != "value" else None,
        reference_as_value=reference_type == "value",
        is_managed=is_managed,
        is_repr=is_repr,
        is_hash=is_hash,
        is_eq=is_eq,
        is_readonly=is_readonly,
        is_interned=is_interned or reference_type is not None,  # references are always interned
        tags=tags,
    )


def declare_property_runtime(
    id: int,
    *,
    description: str | None = None,
    is_repr: bool = False,
    default: Any = None,
    default_factory: Callable[[], Any] | None = None,
) -> Any:
    """A property that is only used at runtime."""
    assert 400 <= id <= 500, f"runtime property must be between 400 and 500: {id}"
    return PropertyDeclaration(
        id=id,
        description=description,
        default_value=default,
        default_factory_callable=default_factory,
        reference_type=None,
        reference_as_value=True,
        is_managed=True,
        is_runtime_only=True,
        is_repr=is_repr,
        is_hash=False,
        is_eq=False,
    )


_PROPERTY_SPECIFIERS: tuple[Callable, ...] = (
    declare_property,
    declare_property_runtime,
)
