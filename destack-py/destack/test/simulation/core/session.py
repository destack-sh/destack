from typing import TYPE_CHECKING

from fastuuid import UUID

from destack.language import Session, Supergraph
from destack.utils.oracle import Oracle

if TYPE_CHECKING:
    from .client import ClientHandle
    from .host import HostHandle
    from .simulation import Simulation


def make_pg_session(
    simulation: "Simulation",
    supergraph: Supergraph | None = None,
) -> "Session":
    """Create a Session to the global and regional Postgres"""
    raise NotImplementedError


async def make_remote_session(
    simulation: "Simulation",
    source_id: str,
    space_id: UUID,
    client: "ClientHandle",
    host: "HostHandle",
    oracle: Oracle,
    supergraph: Supergraph | None = None,
    system: bool = False,
) -> "Session":
    """Create a Session to a remote Destack's Host"""
    raise NotImplementedError
