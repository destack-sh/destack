from typing import TYPE_CHECKING, Any, Optional

import structlog

from bench.language.connection import (
    AggregateResult,
    FetchOptions,
    FetchResult,
    PostgresConnection,
    PostgresEngine,
)
from bench.language.const import NodeType
from bench.language.node import HasBaseNode, IdentityNode, node
from bench.language.property import (
    p_node_parent,
    p_secret_value_packed,
    p_value_packed,
    p_value_runtime,
)
from bench.language.query import QueryBuilder
from bench.language.value import HasValues
from bench.proto.wire import AnyNodeData, NodeReferenceData, RecordData
from bench.utils.func import describe_type

if TYPE_CHECKING:
    from bench.language import Block, TypeInfo

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@node(NodeType.RECORD, passthrough="value", stored_custom=True, local=True)
class Record(IdentityNode[RecordData], HasBaseNode, HasValues):
    """A record in a database. The containing table is usually a real Postgres table."""

    # :RecordSchema
    parent: "Block" = p_node_parent(4, NodeType.BLOCK)
    value_packed: Any = p_value_packed(30)
    secret_value_packed = p_secret_value_packed(31)
    value: Any = p_value_runtime(30, 31, type=4)

    # could also have Record.secret_value_packed as in Block (no materialization needed?)

    def __content_str__(self):
        return f"{describe_type(self.value) or '<empty>'}"

    @property
    def base(self) -> "Block":
        return self.parent

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return data.parent_ptr

    @property
    def _type(self) -> "TypeInfo":
        return getattr(self.parent, "as_type")

    @property
    def keys(self):
        return self.value.keys

    def __contains__(self, item: str):
        return item in self.value

    def __setitem__(self, key, value):
        self.value[key] = value


class RecordPostgresEngine(PostgresEngine[Record, RecordData]):
    pass


class RecordConnection(PostgresConnection[Record, RecordData]):
    async def fetch(
        self, query: "QueryBuilder[Record, RecordData]", options: FetchOptions
    ) -> FetchResult:
        raise NotImplementedError

    async def aggregate(self, query: "QueryBuilder[Record, RecordData]") -> AggregateResult:
        raise NotImplementedError
