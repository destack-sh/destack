from collections.abc import AsyncIterator, Sequence
from typing import ClassVar, Self, assert_never, override

import asyncpg
import structlog
from opentelemetry import trace

from destack.language import (
    Change,
    ChangeResult,
    ChangeStatus,
    CustomEntityDefinition,
    DatabaseInfo,
    DatabaseType,
    Edit,
    NodeDefinitionReference,
    NodeDefinitionType,
    NodeReference,
    NodeType,
    Query,
    QueryResult,
    QueryUpdate,
    Store,
    StoreImplementation,
    StoreType,
)
from destack.language.registry import (
    NODE_CLASS_BY_TYPE,
    NODE_DEFINITION_REFERENCE_BY_CLASS,
)
from destack.utils.uuid import UUID

from .client import pg_connection
from .core import PostgresContext, PostgresTable
from .edit import execute_change
from .map import DESTACK_BUILTIN_TABLE_PREFIX, get_builtin_schema
from .query import execute_query

tracer = trace.get_tracer(__name__)
logger = structlog.get_logger(__name__)


class PostgresStore(Store):
    """
    A Store backed by a Postgres Database.
    """

    implementation: ClassVar[StoreImplementation | None] = StoreImplementation.POSTGRES

    __slots__ = ("context", "database")

    def __init__(self, database: DatabaseInfo, types: tuple[StoreType, ...]):
        super().__init__(types)
        if database.type != DatabaseType.POSTGRES:
            raise ValueError(f"unexpected {database!r}")
        self.database = database
        self.context: PostgresStoreContext | None = None

    def __str__(self) -> str:
        return f"database={self.database!r}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self!s}>"

    @tracer.start_as_current_span("postgres.load")
    async def _load_context(self, conn: asyncpg.Connection) -> "PostgresStoreContext":
        context = PostgresStoreContext(self)
        return context

    @override
    @tracer.start_as_current_span("postgres.query")
    async def query(self, query: Query) -> QueryResult:
        async with pg_connection(self.database) as conn:
            if self.context is None:
                self.context = await self._load_context(conn)
            result = await execute_query(conn, self.context, query)
        logger.debug("postgres.query", query=query, result=result, span="current")
        return result

    @override
    @tracer.start_as_current_span("postgres.commit")
    async def commit(self, changes: Sequence[Change]) -> Sequence[ChangeResult]:
        results: list[ChangeResult] = []
        async with pg_connection(self.database) as conn:
            if self.context is None:
                self.context = await self._load_context(conn)

            # each change is its own atomic operation
            for change in changes:
                # duplicate context if we're mutating custom node definitions
                has_custom_edits = any(
                    edit.node_ptr.type
                    in (NodeType.CUSTOM_ENTITY_DEFINITION, NodeType.CUSTOM_PROPERTY)
                    for edit in change.edits
                )
                local_context = self.context.copy() if has_custom_edits else self.context

                try:
                    async with conn.transaction():
                        edits, cascaded_edits = await execute_change(conn, local_context, change)
                        logger.trace(
                            "postgres.commit.change",
                            change=change,
                            edits=len(edits),
                            cascaded_edits=len(cascaded_edits),
                            context=local_context,
                            span="current",
                        )
                    # update context if we're mutating custom node definitions
                    result = ChangeResult(
                        id=change.id,
                        status=ChangeStatus.COMPLETED,
                        edits=list(edits),
                        cascaded_edits=list(cascaded_edits),
                    )
                    if has_custom_edits:
                        self.context.apply(edits)
                except Exception as e:
                    logger.error(
                        "postgres.commit.change.error",
                        change=change,
                        exc_info=e,
                        span="current",
                    )
                    result = ChangeResult(id=change.id, status=ChangeStatus.FAILED)
                results.append(result)
        logger.debug("postgres.commit", changes=changes, results=results, span="current")
        return results

    @override
    async def subscribe(self, query: Query) -> AsyncIterator[QueryUpdate]:
        raise NotImplementedError


class PostgresStoreContext(PostgresContext):
    __slots__ = ("store", "tables_by_name")

    def __init__(self, store: PostgresStore):
        self.store = store
        self.tables_by_name: dict[str, PostgresTable] = {}
        for store_type in store.types:
            for table in get_builtin_schema(store_type).tables:
                self.tables_by_name[table.name] = table
        self.custom_node_definitions: dict[UUID, CustomEntityDefinition] = {}

    def __str__(self) -> str:
        return f"store={self.store!r}, tables={list(self.tables_by_name.keys())}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self!s}>"

    def copy(self) -> Self:
        return self  # :PostgresSchemaEdits

    @override
    def apply(self, edits: Sequence[Edit]) -> Sequence[Edit]:
        applied_edits: list[Edit] = []
        for edit in edits:
            if edit.node_ptr.type in (
                NodeType.CUSTOM_ENTITY_DEFINITION,
                NodeType.CUSTOM_PROPERTY,
            ):
                applied_edits.append(edit)
                # :PostgresSchemaEdits
        return applied_edits

    @override
    def resolve(self, definition: NodeDefinitionReference) -> Sequence[NodeDefinitionReference]:
        if definition.type == NodeDefinitionType.BUILTIN:
            assert definition.node_type is not None, f"no node_type for {definition!r}"
            node_cls = NODE_CLASS_BY_TYPE[definition.node_type]
            if not node_cls.__inherited_by__:
                return (definition,)
            subdefinitions: list[NodeDefinitionReference] = []
            if not node_cls.__is_abstract__:
                subdefinitions.append(definition)
            for subnode_type in node_cls.__inherited_by__:
                subnode_cls = NODE_CLASS_BY_TYPE[subnode_type]
                if not subnode_cls.__is_abstract__:
                    subdefinitions.append(NODE_DEFINITION_REFERENCE_BY_CLASS[subnode_cls])
            return tuple(subdefinitions)
        elif definition.type == NodeDefinitionType.CUSTOM:
            raise NotImplementedError(f"cannot resolve {definition!r}")
        else:
            assert_never(definition.type)

    @override
    def get(self, definition: NodeDefinitionReference | NodeReference) -> PostgresTable:
        # map definitions to table names
        if isinstance(definition, NodeReference):
            table_name = f"{DESTACK_BUILTIN_TABLE_PREFIX}{definition.type.name.lower()}"
        elif isinstance(definition, NodeDefinitionReference):
            assert definition.node_type is not None, f"no node_type for {definition!r}"
            table_name = f"{DESTACK_BUILTIN_TABLE_PREFIX}{definition.node_type.name.lower()}"
        else:
            assert_never(definition)

        # lookup
        table = self.tables_by_name.get(table_name)
        if table is None:
            raise LookupError(f"no table for {table_name!r} in {self.store!r}")
        return table
