import abc
from typing import TYPE_CHECKING, Any, Optional, cast

import structlog

from bench.language.const import NodeType
from bench.language.node import BasedNode, Node, NodeList, NRel, _Passthrough, node, node_component
from bench.language.notice import NoticeHandler
from bench.language.property import (
    Property,
    p_node_child,
    p_node_parent,
    p_runtime,
    p_secret_value_packed,
    p_value_packed,
    p_value_runtime,
)
from bench.language.query import (
    AggregateResult,
    FetchOptions,
    FetchResult,
    PostgresConnection,
    PostgresEngine,
    QueryBuilder,
)
from bench.language.value import HasValues
from bench.proto.wire import AnyNodeData, NodeReferenceData, RecordData
from bench.sql.core import RECORD_EPHEMERAL_TABLE, Table
from bench.utils.func import describe_type

if TYPE_CHECKING:
    from bench.language import Block, Query, TypeInfo

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@node(
    NodeType.RECORD,
    passthrough=(("value", _Passthrough.Full),),
    stored_custom=True,
    index_in_search=True,
    local=True,
)
class Record(BasedNode[RecordData], HasValues):
    """A record in a database. The containing table is usually a real Postgres table."""

    # :RecordSchema
    parent: "Block" = p_node_parent(4, NodeType.BLOCK)
    value_packed: Any = p_value_packed(30)
    secret_value_packed = p_secret_value_packed(31)
    value = p_value_runtime(30, 31, type=4)

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


class RecordList(NodeList[Record], QueryBuilder[Record, RecordData], abc.ABC):  # type: ignore
    """A NodeList for remote records."""

    def __init__(self, parent: "Block", property: Property):
        NodeList[Record].__init__(self, parent, property)
        QueryBuilder.__init__(self, node_type=NodeType.RECORD, base=parent)


@node_component()
class HasDatabase(Node):
    queries: NodeList["Query"] = p_node_child(
        NodeType.QUERY, NRel.NAMED | NRel.SCOPED | NRel.ORDERED
    )
    records: RecordList = p_node_child(NodeType.RECORD, NRel.STORED_CUSTOM, list=RecordList)
    _table: Optional[Table] = p_runtime(default=None)

    def _clear_inner(self, scope: Optional["Node"] = None) -> None:
        self._table = None

    def _interp_inner(self, scope: Optional["Node"], on_notice: "NoticeHandler") -> None:
        from bench.sql.engine import map_database_to_pg_table

        if self.ephemeral:
            self._table = RECORD_EPHEMERAL_TABLE
        else:
            self._table = map_database_to_pg_table(cast("Block", self))

    @property
    def is_materialized(self):
        # should this database be a real database table
        return True

    @property
    def ephemeral(self) -> bool:
        return not self.is_materialized
