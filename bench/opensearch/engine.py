import base64
import json
from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Any, Collection, Iterable, Optional
from uuid import UUID

import psycopg
import structlog
from psycopg import sql

from bench.language import C, ConditionalOp, Field, Package, Property, SortMode, SortOp
from bench.language.const import BenchType, BlockType, EditType, NodeType, StoreEngineType
from bench.language.database import HasDatabase
from bench.language.expression import METATYPE_KEY, Expression, ExpressionOps, S
from bench.language.node import BENCH_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE, Node, Struct
from bench.language.query import StoreEngineError, StoreEngineIncapableError
from bench.opensearch import core as os
from bench.opensearch.client import get_os_errors, os_client
from bench.proto import wire, wiring
from bench.proto.wire import EditData
from bench.sql.core import PrimitiveType

logger = structlog.get_logger(__name__)

MAXIMUM_NESTING_DEPTH = 3

#
# Mapping schemas
#


OS_FIELD_TYPE_BY_PRIMITIVE_TYPE: dict[PrimitiveType, os.FieldType] = {
    PrimitiveType.STRING: os.FieldType.TEXT,
    PrimitiveType.BOOLEAN: os.FieldType.BOOLEAN,
    PrimitiveType.INT32: os.FieldType.INTEGER,
    PrimitiveType.INT64: os.FieldType.LONG,
    PrimitiveType.FLOAT32: os.FieldType.FLOAT,
    PrimitiveType.DATETIME: os.FieldType.DATE,
    PrimitiveType.INTERVAL: os.FieldType.LONG,
    PrimitiveType.JSON: os.FieldType.FLAT_OBJECT,
    PrimitiveType.VECTOR: os.FieldType.KNN_VECTOR,
    PrimitiveType.UUID: os.FieldType.KEYWORD,
    PrimitiveType.BYTES: os.FieldType.BINARY,
}


def map_field_to_os_field(field: Field) -> os.Field:
    """Maps the dynamic Field to an indexed OS Field."""
    raise NotImplementedError


async def update_local_os_schema(package: Package, dynamic: str = "strict") -> None:
    """
    Updates *all* OpenSearch field mappings for a package
    TODO :Performance: update OS schema more efficiently on type changes
    """
    value_mappings: dict[str, os.Field] = {}
    inputs_mappings: dict[str, os.Field] = {}
    outputs_mappings: dict[str, os.Field] = {}

    # add dynamic user mappings
    for node in package._graph.iter_descendants(package, NodeType.BLOCK, recursive=True):
        if node.type == BlockType.DATABASE:
            # all fields go into Record.value
            for field in node.fields:
                value_mappings[field.storage_key] = map_field_to_os_field(field)

    # actually update mappings
    mappings = {}
    for key, sub_mappings in (
        ("value", value_mappings),
        ("inputs", inputs_mappings),
        ("outputs", outputs_mappings),
    ):
        sub_mappings = {k: v.to_dict() for (k, v) in sub_mappings.items() if v is not None}
        mappings[key] = {"type": "object", "dynamic": dynamic, "properties": sub_mappings}

    await os_client.indices.put_mapping(index=package.os_name, body={"properties": mappings})
    logger.info(
        "os.update_mappings.done",
        package=package,
        value_mappings=len(value_mappings),
        inputs_mappings=len(inputs_mappings),
        outputs_mappings=len(outputs_mappings),
    )


#
# Mapping queries
#


@dataclass
class CompilationContext:
    root_limit: Optional[int]


class Compiler(ABC):
    @abstractmethod
    def compile(self, ctx: CompilationContext, obj: Any) -> dict[str, Any]:
        raise NotImplementedError


_COMPILERS: dict[type, Compiler] = {}


def compiler(dsl_type: type):
    def wrapper(cls: type):
        if dsl_type in _COMPILERS:
            raise RuntimeError(
                f"compiler for {dsl_type} already registered: {_COMPILERS[dsl_type]}"
            )
        _COMPILERS[dsl_type] = cls()
        return cls

    return wrapper


OS_CONDITIONAL_OP_BY_BENCH = {
    ConditionalOp.NOT: "must_not",
    ConditionalOp.AND: "must",
    ConditionalOp.OR: "should",
    ConditionalOp.GREATER_THAN: "gt",
    ConditionalOp.GREATER_THAN_OR_EQUALS: "gte",
    ConditionalOp.LESS_THAN: "lt",
    ConditionalOp.LESS_THAN_OR_EQUALS: "lte",
}


def _compile_expression_ref(expr: Expression) -> str:
    if expr.property is not None:
        return expr.property.name
    if expr.field is not None:
        assert (
            expr.field._introspected_from is None
        ), f"cannot use introspected: {expr!r}->{expr.field!r}"
        return expr.field._source_key
    else:
        raise TypeError(f"unexpected field ref: {expr!r}")


def compile_os_conditional(ctx: CompilationContext, cond: Expression) -> dict[str, Any]:
    key = _compile_expression_ref(cond) if cond.target else None
    if cond.op == ConditionalOp.TRUE:
        return {"match_all": {}}
    elif cond.op == ConditionalOp.FALSE:
        return {"match_none": {}}
    elif cond.op in ExpressionOps.COND_LOGICAL:
        clauses = [compile_os_conditional(ctx, c) for c in cond.clauses]
        return {"bool": {OS_CONDITIONAL_OP_BY_BENCH[cond.op]: clauses}}
    elif cond.op in (ConditionalOp.EQUALS, ConditionalOp.IN):
        if isinstance(cond.value, list):
            return {"terms": {key: cond.value}}
        else:
            return {"term": {key: cond.value}}
    elif cond.op in (ConditionalOp.NOT_EQUALS, ConditionalOp.NOT_IN):
        return {"bool": {"must_not": {"term": {key: cond.value}}}}
    elif cond.op in (
        ConditionalOp.GREATER_THAN,
        ConditionalOp.GREATER_THAN_OR_EQUALS,
        ConditionalOp.LESS_THAN,
        ConditionalOp.LESS_THAN_OR_EQUALS,
    ):
        return {"range": {key: {OS_CONDITIONAL_OP_BY_BENCH[cond.op]: cond.value}}}
    elif cond.op == ConditionalOp.MATCHES:
        return {"match": {key: cond.value}}
    elif cond.op == ConditionalOp.STARTS_WITH:
        return {"match_phrase_prefix": {key: cond.value.lower()}}
    elif cond.op == ConditionalOp.NEAR:
        # TODO :Performance :Robustness: tune knn k relative to database and query limit
        return {"knn": {key: {"vector": cond.value, "k": ctx.root_limit * 3}}}
    elif cond.op == ConditionalOp.EXISTS:
        return {"exists": {"field": key}}
    elif cond.op == ConditionalOp.NOT_EXISTS:
        return {"bool": {"must_not": {"exists": {"field": key}}}}
    raise StoreEngineIncapableError(StoreEngineType.OPENSEARCH, cond, "unsupported conditional")


OS_SORT_ORDER_BY_BENCH = {
    SortOp.ASCENDING: "asc",
    SortOp.DESCENDING: "desc",
}
OS_SORT_MODE_BY_BENCH = {
    SortMode.MIN: "min",
    SortMode.MAX: "max",
    SortMode.AVERAGE: "avg",
    SortMode.MEDIAN: "median",
    SortMode.SUM: "sum",
}


def compile_os_sort(ctx: CompilationContext, sort: Expression) -> dict[str, Any]:
    props = {"order": OS_SORT_ORDER_BY_BENCH[sort.op]}
    if sort.sort_mode:
        props["mode"] = OS_SORT_MODE_BY_BENCH[sort.sort_mode]
    if sort.property is not None:
        return {sort.property.resolved_name: props}
    else:
        return {sort.field._source_key: props}


@dataclass(frozen=True)
class OsSearch:
    """Compiled search query for OS."""

    metatype: BenchType
    limit: int | None = None
    skip: int | None = None
    count: bool = True
    after: Optional[str] = None
    sort: Optional[list[dict]] = None
    query: Optional[dict] = None

    def to_dict(self) -> dict[str, Any]:
        search = {
            "query": self.query,
            "sort": self.sort,
            "track_total_hits": self.count,
        }
        if self.limit:
            search["size"] = self.limit
        if self.after:
            # cursor is base64 encoded json of search after if it exists,
            # otherwise just from for relevance-scored search (opaque to client)
            after = json.loads(base64.b64decode(self.after).decode())
            if isinstance(after, list):
                search["search_after"] = after
            elif isinstance(after, int):
                search["from"] = after
            else:
                raise ValueError("invalid cursor")
        elif self.skip is not None:
            search["from"] = self.skip
        return search


@dataclass(frozen=True)
class OsSearchResult:
    total: Optional[int]
    results: list[dict[str, Any]]
    cursors: list[str]
    start_cursor: Optional[str]

    def as_records(self) -> list[wire.AnyNodeData]:
        records_data: list[wire.AnyNodeData] = []
        for result in self.results:
            records_data.append(unpack_struct(result["_source"]))
        return records_data


def os_compile_search(
    metatype: BenchType,
    query: Optional[Expression] = None,
    sort: Optional[list[Expression]] = None,
    limit: int | None = None,
    skip: int | None = None,
    count: bool = True,
    after: Optional[str] = None,
):
    if after and skip is not None:
        raise ValueError("cannot specify both after and skip")
    combined_query = C(
        ConditionalOp.AND,
        clauses=[C(ConditionalOp.EQUALS, field_key=METATYPE_KEY, value=metatype.name)],
    )
    if query is not None:
        combined_query &= query
    # add id to sort as tiebreaker if not already present
    if sort and not any(s.field_key == "_id" for s in sort):
        sort = sort + [S(SortOp.ASCENDING, field_key="_id")]
    sort = sort or [S(SortOp.ASCENDING, field_key="_id")]
    ctx = CompilationContext(root_limit=limit)
    compiled_query = compile_os_conditional(ctx, combined_query)
    compiled_sort = [compile_os_sort(ctx, s) for s in sort]
    search = OsSearch(
        type=type,
        limit=limit,
        count=count,
        after=after,
        skip=skip,
        sort=compiled_sort,
        query=compiled_query,
    )
    return search


def _wrap_os_error(
    e: Exception, expr: Expression | list[Expression]
) -> StoreEngineError | Exception:
    return StoreEngineError(StoreEngineType.OPENSEARCH, expr, reason=str(e))


async def os_search(
    os_name: str,
    metatype: BenchType,
    filter: Optional[Expression] = None,
    sort: Optional[list[Expression]] = None,
    limit: int | None = None,
    skip: int | None = None,
    count: bool = True,
    after: Optional[str] = None,
) -> OsSearchResult:
    """Executes a search query against OpenSearch."""
    search = os_compile_search(metatype, filter, sort, limit, skip, count, after)
    logger.debug("os.search", os_name=os_name, search=search)
    try:
        os_results = await os_client.search(index=os_name, body=search.to_dict())
    except Exception as e:
        raise _wrap_os_error(e, [filter, sort]) from e
    total = os_results["hits"]["total"]["value"] if search.count else None
    results = os_results["hits"]["hits"]
    cursors = [encode_os_cursor(r, search.after, i) for i, r in enumerate(results)]
    return OsSearchResult(total=total, results=results, cursors=cursors, start_cursor=after)


def encode_os_cursor(record: dict[str, Any], after: Optional[str], i: int) -> str:
    # for relevance-scored search, cursor is from offset, so add i to it
    # otherwise, cursor is search_after, so encode 'sort' from record
    if "sort" in record:
        return base64.b64encode(json.dumps(record["sort"]).encode()).decode("utf-8")
    else:
        after = json.loads(base64.b64decode(after).decode()) if after else 0
        if not isinstance(after, int):
            raise ValueError("invalid cursor")
        return base64.b64encode(json.dumps(after + i).encode()).decode("utf-8")


#
# Mapping nodes to documents
#


NAME_FIELD = os.Field(
    os.FT.TEXT,
    fields={
        os.SubfieldType.starts_with: os.Field(os.FieldType.SEARCH_AS_YOU_TYPE),
        os.SubfieldType.key: os.Field(os.FieldType.KEYWORD),
    },
)
HTML_FIELD = os.Field(os.FieldType.TEXT, analyzer=os.Analyzer.HTML)
TEXT_FIELD = HTML_FIELD


def _is_property_indexed_in_search(prop: Property) -> bool:
    return prop.is_stored and not prop.is_encrypted and not prop.is_deferred


def map_struct_type_to_os_document(
    struct: type[Struct], seen_types: tuple[type[Struct], ...] = ()
) -> os.Document:
    fields: dict[str, os.Field] = {}

    for prop in struct.__stored_properties__.values():
        if not _is_property_indexed_in_search(prop):
            continue
        if prop.is_enum:
            field = os.Field(os.FieldType.KEYWORD)
        elif prop.is_struct:
            if prop.reference_struct in seen_types:
                continue  # no recursive indexing
            struct_cls = STRUCT_CLASS_BY_TYPE[prop.reference_struct]
            mapped = map_struct_type_to_os_document(
                struct_cls, seen_types + (prop.reference_struct,)
            )
            field = os.Field(os.FieldType.OBJECT, properties=mapped.fields)
        elif prop.primitive_type == PrimitiveType.JSON:
            field = os.Field(os.FieldType.OBJECT, dynamic="strict")
        else:
            field_type = OS_FIELD_TYPE_BY_PRIMITIVE_TYPE.get(prop.primitive_type)
            if field_type is None:
                raise ValueError(f"unsupported column type in {prop!r}: {prop.primitive_type}")
            field = os.Field(field_type)
            if field.type == os.FieldType.DATE:
                field.ignore_malformed = True  # allow 'resetting' dates to null
        fields[prop.name] = field
    if issubclass(struct, Node) and "name" not in fields:
        fields["name"] = NAME_FIELD
    return os.Document(fields=fields)


LOCAL_DOCUMENTS: tuple[os.Document, ...] = tuple(
    map_struct_type_to_os_document(struct)
    for struct in BENCH_CLASS_BY_TYPE.values()
    if struct.__is_indexed_in_search__
)


def _pack_struct_prop(prop: Property, value: Any, ignore_array: bool) -> Any:
    if value is None:
        return None
    elif prop.is_list and not ignore_array:
        return [_pack_struct_prop(prop, v, ignore_array=True) for v in value]
    elif prop.is_struct:
        return pack_struct(value)
    elif prop.primitive_type == PrimitiveType.JSON:
        return wiring.pack_json_value(value)
    elif prop.is_enum:
        return wiring.pack_enum(value)
    else:
        return value


def _unpack_struct_prop(prop: Property, value: Any, ignore_array: bool) -> Any:
    if value is None:
        return None
    elif prop.is_list and not ignore_array:
        return [_unpack_struct_prop(prop, v, ignore_array=True) for v in value]
    elif prop.is_struct:
        return unpack_struct(value)
    elif prop.primitive_type == PrimitiveType.JSON:
        return wiring.unpack_json_value(value)
    elif prop.is_enum:
        return wiring.unpack_enum(prop.enum_cls, value)
    else:
        return value


def pack_struct(node: wire.AnyNodeData | wire.AnyStructData) -> dict:
    metatype = wiring.unpack_enum(BenchType, node.metatype)
    bench_cls = BENCH_CLASS_BY_TYPE[metatype]
    document: dict[str, Any] = {METATYPE_KEY: metatype.name}
    for prop in bench_cls.__stored_properties__.values():
        if not _is_property_indexed_in_search(prop):
            continue
        value = getattr(node, prop.name)
        document[prop.name] = _pack_struct_prop(prop, value, ignore_array=False)
    return document


def unpack_struct(source: dict) -> wire.AnyNodeData | wire.AnyStructData:
    metatype = BenchType(source[METATYPE_KEY])
    bench_cls = BENCH_CLASS_BY_TYPE[metatype]
    proto_cls = wiring.PROTO_CLASS_BY_TYPE[metatype]
    proto_kwargs = {}
    for prop in bench_cls.__stored_properties__.values():
        if not _is_property_indexed_in_search(prop):
            continue
        value = source.get(prop.name)
        proto_kwargs[prop.name] = _unpack_struct_prop(prop, value, ignore_array=False)
    return proto_cls(**proto_kwargs)


async def sync_pg_databases_to_os(
    package: Package,
    pg_cursor: psycopg.AsyncCursor,
    record_ids_by_db: list[tuple["HasDatabase", set[UUID] | None]],
) -> None:
    """Synchronizes local PG databases to OpenSearch. Mirror only the given record ids if given."""
    from bench.sql.engine import (
        PostgresConditionalOp,
        SqlComparison,
        pg_select,
        pg_unpack_record_data_row,
    )

    log = logger.bind(package=package, databases=[r[0] for r in record_ids_by_db])
    log.debug("os.sync_pg_databases_to_os")
    os_name = package.os_name
    ops: list[dict[str, Any]] = []

    async def _flush():
        if ops:
            log.debug("os.sync_pg_databases_to_os.flush", ops=len(ops))
            await os_client.bulk(body=ops)
            ops.clear()

    for database, record_ids in record_ids_by_db:
        logger.debug("os.sync_pg_databases_to_os.db", database=database, records=len(record_ids))
        if record_ids is None:
            # update entire table if record_ids is None
            records_data = await pg_select(cur=pg_cursor, table=database._table)
            records_data = [pg_unpack_record_data_row(database, row) for row in records_data]
            # delete table by query
            await os_client.delete_by_query(
                index=os_name, body={"query": {"term": {"block_key": database.dynamic_key}}}
            )
        else:
            if not record_ids:
                continue
            # otherwise update only the given record ids
            where = SqlComparison(
                sql.Identifier("id"), PostgresConditionalOp.EQ, sql.SQL("ANY(%(updated_ids)s)")
            )
            records_data = await pg_select(
                cur=pg_cursor,
                table=database._table,
                where=where,
                params={"updated_ids": list(record_ids)},
            )
            records_data = [pg_unpack_record_data_row(database, row) for row in records_data]
            records_by_id = {r.id: r for r in records_data}
            missing_ids = record_ids - records_by_id.keys()
            # delete missing ids
            for deleted_record_id in missing_ids:
                ops.append({"delete": {"_index": os_name, "_id": str(deleted_record_id)}})

        # upsert records
        for record_data in records_data:
            ops.append({"index": {"_index": os_name, "_id": str(record_data.id)}})
            ops.append(pack_struct(record_data))

    await _flush()
    log.debug("os.sync_pg_databases_to_os.done")


DEFAULT_FIELDS = {
    METATYPE_KEY: os.METATYPE_FIELD,
}
GLOBAL_INDEX_SHARDS = 5
GLOBAL_INDEX_REPLICAS = 1
BENCH_INDEX_SHARDS = 1
BENCH_INDEX_REPLICAS = 0
BENCH_MAPPING_TOTAL_FIELDS_LIMIT = 10000  # TODO :Performance: reconsider OS mapping limit


def _collect_fields(docs: Iterable[os.Document]) -> dict[str, os.Field]:
    fields = {}
    for doc in docs:
        for field_name, field in doc.fields.items():
            existing_field = fields.get(field_name)
            if existing_field is not None and existing_field != field:
                raise ValueError(
                    f"field {field_name} defined twice with different types: {existing_field} != {field}"
                )
            fields[field_name] = field
    return fields


async def _create_os_index(
    index_name: str,
    *,
    shards: int,
    replicas: int,
    documents: Collection[os.Document],
    upsert: bool = False,
) -> None:
    fields = {**DEFAULT_FIELDS, **_collect_fields(documents)}
    mappings = {field_name: field.to_dict() for field_name, field in fields.items()}
    tokenizers = {
        tokenizer.value: definition for tokenizer, definition in os.CUSTOM_TOKENIZERS.items()
    }
    analyzers = {analyzer.value: definition for analyzer, definition in os.CUSTOM_ANALYZERS.items()}

    log = logger.bind(
        index_name=index_name,
        shards=shards,
        replicas=replicas,
        fields=list(fields.keys()),
    )
    log.info("os.create_index")
    result = await os_client.indices.create(
        index=index_name,
        body={
            "settings": {
                "index": {"number_of_shards": shards, "number_of_replicas": replicas, "knn": True},
                "analysis": {"tokenizer": tokenizers, "analyzer": analyzers},
                "mapping": {"total_fields": {"limit": BENCH_MAPPING_TOTAL_FIELDS_LIMIT}},
            },
            "mappings": {"dynamic": "strict", "properties": mappings},
        },
        ignore=400 if upsert else 0,
    )
    # upsert if index already exists
    if result.get("acknowledged") is not True:
        if not upsert:
            raise IndexError(f"failed to create index {index_name}: {result}")
        logger.info("os.create_index.upsert", index_name=index_name)
        # close index
        await os_client.indices.close(index=index_name)
        # update mutable settings
        await os_client.indices.put_settings(
            index=index_name,
            body={
                "analysis": {"tokenizer": tokenizers, "analyzer": analyzers},
                "mapping": {"total_fields": {"limit": BENCH_MAPPING_TOTAL_FIELDS_LIMIT}},
            },
        )
        await os_client.indices.put_mapping(
            index=index_name, body={"dynamic": "strict", "properties": mappings}
        )
        # reopen index
        await os_client.indices.open(index=index_name)
    log.info("os.create_index.done")


async def create_global_os_index(upsert: bool = False) -> None:
    logger.info("os.create_global_index")
    await _create_os_index(
        os.GLOBAL_INDEX_NAME,
        shards=GLOBAL_INDEX_SHARDS,
        replicas=GLOBAL_INDEX_REPLICAS,
        documents=[],  # nothing yet
        upsert=upsert,
    )


async def create_local_os_index(
    *, os_name: str, os_username: str, os_password: str, is_public: bool, upsert: bool
) -> None:
    """
    Creates the OpenSearch index and corresponding roles/user for a bench.
    """
    log = logger.bind(os_name=os_name)
    log.info("os.create_local_index")
    # create index
    await _create_os_index(
        index_name=os_name,
        shards=BENCH_INDEX_SHARDS,
        replicas=BENCH_INDEX_REPLICAS,
        documents=LOCAL_DOCUMENTS,
        upsert=upsert,
    )

    # create write access role for bench owner
    log.info("os.create_owner_role")
    owner_role_name = f"{os_name}-rw"
    rep = await os_client.security.get_role(role=owner_role_name, ignore=404)
    owner_role = rep.get(owner_role_name)
    if owner_role is not None:
        await os_client.security.delete_role(role=owner_role_name)
    rep = await os_client.security.create_role(
        role=owner_role_name,
        body={
            "cluster_permissions": [
                # this is required for all bulk indexing
                #  (the actual permission is checked per index@)
                "indices:data/write/bulk",
            ],
            "index_permissions": [
                {
                    "index_patterns": [os_name],
                    "fls": [],
                    "masked_fields": [],
                    "allowed_actions": ["*"],
                }
            ],
        },
    )
    if rep.get("error"):
        raise RuntimeError(f"failed to create role {owner_role_name}: {rep['error']}")
    log.info("os.create_owner_role.done")

    # create user with those roles
    log.info("os.create_user")
    rep = await os_client.security.get_user(username=os_username, ignore=404)
    user = rep.get(os_username)
    if user is not None:
        await os_client.security.delete_user(username=os_username)
    rep = await os_client.security.create_user(
        username=os_username,
        body={
            "password": os_password,
            "opendistro_security_roles": [owner_role_name],
        },
    )
    if rep.get("error"):
        raise RuntimeError(f"failed to create user {os_username}: {rep['error']}")
    log.info("os.create_user.done", username=os_username)


async def os_write_edits(module: Package, edits: list[EditData], *, refresh: bool = False) -> None:
    """
    Writes/mirrors any relevant edit to OpenSearch.
    All regular DB edit come this way.
    """
    log = logger.bind(module=module, edits=edits)
    log.debug("os.write_edits")
    if not edits:
        if refresh:
            # just refresh the index
            await os_client.indices.refresh(index=module.os_name)
        return  # nothing to do

    # mut state
    ops: list[dict] = []

    async def _flush():
        if ops:
            log.debug("os.write_edits.flush", operations=len(ops))
            ret = await os_client.bulk(ops, refresh="" if refresh else False)
            if ret.get("errors"):
                raise RuntimeError(f"failed to write edit to OpenSearch: {get_os_errors(ret)}")

        ops.clear()

    for edit in edits:
        metatype = wiring.unpack_enum(BenchType, edit.node.metatype)
        node_cls = BENCH_CLASS_BY_TYPE[metatype]
        index = module.os_name if node_cls.__is_local__ else os.GLOBAL_INDEX_NAME
        if not node_cls.__is_indexed_in_search__:
            continue  # ignore
        elif edit.type.type in (
            EditType.CREATE,
            EditType.UPDATE,
            EditType.MOVE,
            EditType.SOFT_DELETE,
            EditType.RESTORE,
        ):
            ops.append({"index": {"_index": index, "_id": str(edit.node.id)}})
            ops.append(pack_struct(wiring.unwrap_some_node(edit.node)))
        elif edit.type.type == EditType.DELETE:
            ops.append({"delete": {"_index": index, "_id": str(edit.node.id)}})
        else:
            raise ValueError(f"unexpected edit type: {edit!r}")

    await _flush()  # flush all remaining edits


async def os_write_records(module: Package, records: list[wire.RecordData]) -> None:
    if not records:
        return
    ops: list[dict] = []
    for record_data in records:
        ops.append({"index": {"_index": module.os_name, "_id": str(record_data.id)}})
        ops.append(pack_struct(record_data))
    logger.debug("os.write_records", operations=len(ops))
    ret = await os_client.bulk(ops)
    if ret.get("errors"):
        raise RuntimeError(f"failed to write records to OpenSearch: {get_os_errors(ret)}")
