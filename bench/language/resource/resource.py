import abc
from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional, Self, TypeVar
from uuid import UUID

from bench.language.core import (
    NAME_CONSTRAINT,
    OWNER_TYPES,
    REGION,
    TITLE_CONSTRAINT,
    BenchNode,
    BuiltinEnum,
    EnumType,
    HasTrace,
    Node,
    NodeType,
    Owner,
    Region,
    StructType,
    active_session,
    bittuple,
    enum_,
    node_component_,
    p_internal,
    p_node_parent,
    p_regular,
    p_system,
)
from bench.pb2 import AnyNodeData

if TYPE_CHECKING:
    from bench.language import (
        Bench,
        NodeReference,
        Region,
        Scaler,
        Text,
    )

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.RESOURCE_STATUS)
class ResourceStatus(BuiltinEnum):
    """Generalized status of a Resource in its lifecycle."""

    # definition
    DECLARED = 1
    # extant
    UP = 10
    SLEEPING = 11
    DOWN = 15
    DEGRADED = 16
    # terminal
    DECOMMISSIONED = 30

    @property
    def is_extant(self) -> bool:
        """Whether this resouce does/should exist."""
        return 10 <= self.value <= 20


@enum_(EnumType.RESOURCE_OCCUPANCY)
class ResourceOccupancy(BuiltinEnum):
    """The occupancy of a resource (i.e. whether/how it's being used)."""

    AVAILABLE = 1
    RESERVED = 2
    OCCUPIED = 3
    DIRTY = 20


EXTANT_RESOURCE_STATUSES = bittuple(*(s for s in ResourceStatus if 10 <= s.value <= 20))

NodeDataT = TypeVar("NodeDataT", bound=AnyNodeData)


@node_component_()
class Resource[NodeDataT: AnyNodeData](BenchNode[NodeDataT], HasTrace, abc.ABC):
    """
    A Resource in a Bench.
    Resources generally work on the 'desired state' principle.
    Where applicable, the target state is stored in target_* properties.
    """

    parent: Optional["Bench"] = p_node_parent(4, NodeType.BENCH, is_system=True)
    # ... space for type/name/...
    status: ResourceStatus = p_system(33, default=ResourceStatus.DECLARED, default_sql=None)
    text: Optional["Text"] = p_regular(34, default=None, struct=StructType.TEXT)
    region: Region = p_system(35, default=REGION, default_sql=None)

    # target status
    activated_at: Optional[datetime] = p_internal(40, default=None)
    deactivated_at: Optional[datetime] = p_internal(41, default=None)
    reset_at: Optional[datetime] = p_internal(42, default=None)
    suspended_at: Optional[datetime] = p_internal(43, default=None)
    decommissioned_at: Optional[datetime] = p_internal(44, default=None)
    # current status
    active_at: Optional[datetime] = p_system(45, default=None)

    def __content_str__(self):
        return Node.__default_content_str__(self)

    def _get_target_diff(self, *keys: str) -> dict[str, Any]:
        """
        Checks whether specific properties <key> differ from their target_<key> values.
        Returns the target values of differing properties.
        """
        target_diff: dict[str, Any] = {}
        for key in keys:
            prop = self.__properties__.get(key)
            assert prop is not None, f"no property '{key}' in {self.__class__.__name__}"
            current_value = getattr(self, key)
            target_value = getattr(self, f"target_{key}")
            if current_value != target_value:
                target_diff[key] = target_value
        return target_diff

    @property
    def target_status(self) -> ResourceStatus:
        """The implied target status of this Resource."""
        if self.decommissioned_at is not None:
            return ResourceStatus.DECOMMISSIONED
        elif self.suspended_at is not None and not (
            self.activated_at is not None and self.activated_at > self.suspended_at
        ):
            return ResourceStatus.SLEEPING
        elif self.deactivated_at is not None and not (
            self.activated_at is not None and self.activated_at > self.deactivated_at
        ):
            return ResourceStatus.DOWN
        else:
            return ResourceStatus.UP

    def provision(self) -> None:
        """Provision this Resource."""
        assert not self.status.is_extant, f"{self!r} already exists"
        self.activated_at = self.active_session._oracle.utc()

    def decommission(self) -> None:
        """Decommission this Resource."""
        assert self.status.is_extant, f"{self!r} does not exist"
        self.decommissioned_at = self.active_session._oracle.utc()

    async def wait_until_status(self, status: ResourceStatus):
        """Wait until this Resource reaches the given status."""
        await self.wait_until(lambda r: r.status == status)

    async def wait_until_ready(self) -> None:
        """Wait until this Resource is ready."""
        await self.wait_until(lambda r: r.status == ResourceStatus.UP)


@node_component_()
class StaticResource[NodeDataT: AnyNodeData](Resource[NodeDataT]):
    """
    A 'static' Resource in a Bench.
    """

    name: str = p_regular(32, constraint=NAME_CONSTRAINT)

    @classmethod
    def new(cls, name: str, **kwargs: Any) -> Self:
        """Creates a new Resource of this type. Defaults to current Bench"""

        # parent
        if "parent" not in kwargs:
            if NodeType.BENCH not in cls.__parent_types__:
                raise ValueError(f"cannot create {cls!r} without parent")
            bench = active_session().bench
            assert bench is not None, "no active Bench"
            kwargs["parent"] = bench

        resource = cls(name=name, **kwargs)
        return resource


@node_component_()
class DynamicResource[NodeDataT: AnyNodeData](Resource[NodeDataT]):
    """
    A 'dynamic' Resource in a Bench.
    """

    name: str = p_regular(32, constraint=TITLE_CONSTRAINT)

    occupancy: ResourceOccupancy = p_system(
        36, default=ResourceOccupancy.RESERVED, default_sql=None
    )
    owned_by: Optional[Owner] = p_system(37, require=False, array=False, references=OWNER_TYPES)
    scaler: Optional["Scaler"] = p_system(
        38, require=False, array=False, references=NodeType.SCALER
    )
    if TYPE_CHECKING:
        scaler_id: Optional[UUID] = None
        scaler_ptr: Optional[NodeReference] = None

    @classmethod
    def new(cls, *, name: str | None = None, **kwargs: Any) -> Self:
        """Creates a new Resource of this type. Defaults to current Bench"""
        session = active_session()

        # parent
        if "parent" not in kwargs:
            if NodeType.BENCH not in cls.__parent_types__:
                raise ValueError(f"cannot create {cls!r} without parent")
            bench = session.bench
            assert bench is not None, "no active Bench"
            kwargs["parent"] = bench

        # title
        if name is None:
            # fabricate title
            now = session._oracle.utc()
            name = cls.metatype.bench_name + now.strftime("%Y-%m-%d %H:%M:%S")

        resource = cls(name=name, **kwargs)
        return resource
