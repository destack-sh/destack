from collections.abc import Sequence
from typing import Self, assert_never, override

import asyncpg
import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench.language import (
    Area,
    Change,
    ChangeResult,
    ChangeStatus,
    CustomNodeDefinition,
    DatabaseInfo,
    Edit,
    EditOperation,
    EditType,
    NodeReference,
    NodeType,
    Query,
    QueryResult,
    RelationReference,
    RelationType,
    Store,
)
from bench.language.registry import NODE_CLASS_BY_TYPE, NODE_TYPES_BY_TRAIT, RELATION_REF_BY_CLASS

from .client import pg_connection
from .core import DatabaseContext, DatabaseTable
from .edit import execute_change
from .map import (
    BENCH_BUILTIN_TABLE_PREFIX,
    BENCH_CUSTOM_TABLE_PREFIX,
    BUILTIN_TABLE_BY_AREA,
    BUILTIN_TABLE_BY_NAME,
)
from .query import execute_query

tracer = trace.get_tracer(__name__)
logger = structlog.get_logger(__name__)


class DatabaseStore(Store):
    """
    A Store backed by a Postgres Databases.
    """

    __supports_edit_types__ = (
        EditType.CREATE,
        EditType.UPSERT,
        EditType.UPDATE,
        EditType.DELETE,
        EditType.MOVE,
        EditType.ARCHIVE,
        EditType.UNARCHIVE,
        EditType.ERASE,
        EditType.RESTORE,
    )
    __supports_operations__ = (
        EditOperation.SET,
        EditOperation.CLEAR,
    )
    __supports_cascade__ = True

    __slots__ = ("context", "database")

    def __init__(self, database: DatabaseInfo, area: Area | None):
        self.database = database
        self.context: DatabaseStoreContext | None = None
        self.area = area

    def __str__(self) -> str:
        return f"database={self.database!r}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self!s}>"

    @tracer.start_as_current_span("database.load")
    async def _load_context(self, conn: asyncpg.Connection) -> "DatabaseStoreContext":
        context = DatabaseStoreContext(self)
        return context

    @override
    @tracer.start_as_current_span("database.query")
    async def query(self, query: Query) -> QueryResult:
        async with pg_connection(self.database) as conn:
            if self.context is None:
                self.context = await self._load_context(conn)
            result = await execute_query(conn, self.context, query)
        logger.debug("database.query", query=query, result=result, span="current")
        return result

    @override
    @tracer.start_as_current_span("database.commit")
    async def commit(self, changes: Sequence[Change]) -> Sequence[ChangeResult]:
        results: list[ChangeResult] = []
        async with pg_connection(self.database) as conn:
            if self.context is None:
                self.context = await self._load_context(conn)

            # each change is its own atomic operation
            for change in changes:
                # duplicate context if we're mutating custom node definitions
                has_custom_edits = any(
                    edit.node_type in (NodeType.CUSTOM_NODE_DEFINITION, NodeType.FIELD)
                    for edit in change.edits
                )
                local_context = self.context.copy() if has_custom_edits else self.context

                try:
                    async with conn.transaction():
                        edits, cascaded_edits = await execute_change(conn, local_context, change)
                        logger.debug(
                            "database.commit.change",
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
                        "database.commit.change.error", change=change, exc_info=e, span="current"
                    )
                    result = ChangeResult(id=change.id, status=ChangeStatus.FAILED)
                results.append(result)
        logger.debug("database.commit", changes=changes, results=results, span="current")
        return results


class DatabaseStoreContext(DatabaseContext):
    __slots__ = ("store", "tables_by_name")

    def __init__(self, store: DatabaseStore):
        self.store = store
        self.tables_by_name: dict[str, DatabaseTable] = {}
        if store.area is None:
            self.tables_by_name.update(BUILTIN_TABLE_BY_NAME)
        else:
            for table in BUILTIN_TABLE_BY_AREA[store.area]:
                self.tables_by_name[table.name] = table
        self.custom_node_definitions: dict[UUID, CustomNodeDefinition] = {}

    def __str__(self) -> str:
        return f"store={self.store!r}, tables={list(self.tables_by_name.keys())}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self!s}>"

    def copy(self) -> Self:
        raise NotImplementedError

    @override
    def apply(self, edits: Sequence[Edit]) -> Sequence[Edit]:
        applied_edits: list[Edit] = []
        for edit in edits:
            if edit.node_type in (NodeType.CUSTOM_NODE_DEFINITION, NodeType.FIELD):
                applied_edits.append(edit)
                raise NotImplementedError(f"apply database context edit: {edit!r}")
        return applied_edits

    @override
    def resolve_relation(self, relation: RelationReference) -> Sequence[RelationReference]:
        if relation.type in (RelationType.BUILTIN_NODE, RelationType.CUSTOM_NODE):
            return (relation,)
        elif relation.type == RelationType.TRAIT:
            assert relation.trait_type is not None, f"no trait_type for {relation!r}"
            node_types = NODE_TYPES_BY_TRAIT.get(relation.trait_type, ())
            return tuple(
                RELATION_REF_BY_CLASS[NODE_CLASS_BY_TYPE[node_type]] for node_type in node_types
            )
        else:
            assert_never(relation.type)

    @override
    def get_relation(self, relation: RelationReference | NodeReference) -> DatabaseTable:
        # map relations to table names
        if isinstance(relation, NodeReference):
            if relation.node_type != NodeType.CUSTOM_NODE_INSTANCE:
                table_name = f"{BENCH_BUILTIN_TABLE_PREFIX}{relation.node_type.name.lower()}"
            else:
                assert relation.definition_id is not None, f"no definition_id for {relation!r}"
                table_name = f"{BENCH_CUSTOM_TABLE_PREFIX}{relation.definition_id}"
        elif isinstance(relation, RelationReference):
            if relation.type == RelationType.BUILTIN_NODE:
                assert relation.node_type is not None, f"no node_type for {relation!r}"
                table_name = f"{BENCH_BUILTIN_TABLE_PREFIX}{relation.node_type.name.lower()}"
            elif relation.type == RelationType.CUSTOM_NODE:
                assert relation.definition_id is not None, f"no definition_id for {relation!r}"
                table_name = f"{BENCH_CUSTOM_TABLE_PREFIX}{relation.definition_id}"
            elif relation.type == RelationType.TRAIT:
                raise RuntimeError(f"cannot get single table for {relation!r}")
            else:
                assert_never(relation.type)
        else:
            assert_never(relation)

        # lookup
        table = self.tables_by_name.get(table_name)
        if table is None:
            raise LookupError(f"no table for {table_name!r} in {self.store!r}")
        return table
