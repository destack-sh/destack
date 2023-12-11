import base64
import json
from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Any, NamedTuple, Optional, Union
from uuid import UUID

import psycopg
import structlog
from psycopg import sql

from bench import language as lang
from bench.language import (
    C,
    ConditionalOp,
    HasDatabase,
    HasRun,
    Module,
    QueryEngine,
    SortMode,
    SortOp,
    TypeHint,
    TypeTag,
    wire,
)
from bench.language.const import RUNNABLE_STATEMENT_TYPES, TypeFlag
from bench.language.edit import EditType
from bench.language.expression import (
    TYPE_DISCRIMINATOR_KEY,
    Expression,
    ExpressionOps,
    FieldReference,
    QueryEngineError,
    QueryEngineIncapableError,
    S,
)
from bench.language.field import TYPE_TAG_BY_TYPE_HINT
from bench.search import core as os
from bench.search import mirror
from bench.search.client import os_client, os_client_sync
from bench.search.core import DocumentType, IndexType, SubfieldType

logger = structlog.get_logger(__name__)

MAXIMUM_NESTING_DEPTH = 3


#
# Mapping schemas
#


class FieldMapper:
    """
    Maps a Bench Field to an OpenSearch field (possibly nested).
    Don't bother with lists and optional here.
    """

    def to_os_type(self, type: Union[lang.Field, lang.Statement], depth: int) -> os.Field:
        raise NotImplementedError


TypeSignature = NamedTuple(
    "TypeSignature", [("tag", TypeTag), ("hint", Optional[TypeHint]), ("flags", TypeFlag)]
)

field_mappers: dict[TypeSignature, FieldMapper] = {}


def register_mapper(
    mapper: os.Field | FieldMapper,
    tags: list[TypeTag] = None,
    hints: list[TypeHint] = None,
    flags: TypeFlag = None,
) -> None:
    if not tags and not hints:
        raise ValueError("at least one tag or hint must be specified")
    if isinstance(mapper, os.Field):
        mapper = StaticFieldMapper(mapper)
    tags = tags or []
    hints = hints or []
    flags = flags or TypeFlag.ZERO
    for tag in tags:
        field_mappers[TypeSignature(tag, None, flags)] = mapper
    for hint in hints:
        tag = TYPE_TAG_BY_TYPE_HINT[hint]
        field_mappers[TypeSignature(tag, hint, flags)] = mapper


def get_mapper(type: Union[lang.Field, lang.Statement]) -> FieldMapper:
    if type.tag == TypeTag.TYPE_REFERENCE and isinstance(type.reference, lang.Statement):
        return get_mapper(type.reference)  # skip the reference
    stripped_flags = type.flags & TypeFlag.IS_SECRET
    exact_signature = TypeSignature(type.tag, type.hint, stripped_flags)
    mapping = field_mappers.get(exact_signature)
    if mapping is not None:
        return mapping
    # no exact match, try generic without hint
    stripped_signature = TypeSignature(type.tag, None, stripped_flags)
    mapping = field_mappers.get(stripped_signature)
    if mapping is not None:
        return mapping
    raise LookupError(f"no mapping found for {type}")


@dataclass
class StaticFieldMapper(FieldMapper):
    field: os.Field | os.FT

    def to_os_type(self, type: lang.Field, depth: int) -> os.Field:
        return self.field


class StructFieldMapper(FieldMapper):
    def to_os_type(self, type: lang.Field, depth: int) -> os.Field:
        subfields = {TYPE_DISCRIMINATOR_KEY: os.Field(os.FT.KEYWORD)}
        if isinstance(type.reference, lang.Statement) and type.reference.issues:
            # bail out early if there are issues from a reference
            # (these don't get reported up to every reference but we still can't map it)
            return os.Field(os.FT.OBJECT, properties=subfields)

        for f in type.resolved_fields:
            if f._effective_tag != TypeTag.STRUCT or depth < MAXIMUM_NESTING_DEPTH:
                subfields[f._typed_key] = get_mapper(f).to_os_type(f, depth + 1)
            else:
                # treat as json (but not as flattened yet.. :BadJsonMapping)
                subfields[f._typed_key] = os.Field(os.FT.OBJECT, dynamic=True, enabled=False)
        return os.Field(os.FT.OBJECT, properties=subfields)


class VectorFieldMapper(FieldMapper):
    def to_os_type(self, type: lang.Field, depth: int) -> os.Field:
        # see https://github.com/nmslib/hnswlib/blob/master/ALGO_PARAMS.md#construction-parameters
        # assumes byte-quantized vectors with <= 1024 dimensions
        method = os.KnnMethod(
            name=os.KnnMethodName.HNSW,
            engine=os.KnnEngine.LUCENE,
            space_type=os.KnnSpaceType.L2,
            parameters=os.HnswParameters(ef_construction=128, m=24),
        )
        return os.Field(
            os.FT.KNN_VECTOR, dimension=type.dimensions, method=method, data_type="byte"
        )


# string
register_mapper(
    os.Field(
        os.FT.TEXT,
    ),
    tags=[TypeTag.STRING],
)
register_mapper(
    os.Field(
        os.FT.TEXT,
        # :QuerySubfields
        fields={
            SubfieldType.key: os.Field(os.FT.KEYWORD),
            SubfieldType.starts_with: os.Field(os.FT.SEARCH_AS_YOU_TYPE),
        },
        copy_to="name",  # :RecordNameField
    ),
    hints=[TypeHint.NAME, TypeHint.EMAIL],
)
register_mapper(os.Field(os.FT.KEYWORD), tags=[TypeTag.NODE], hints=[TypeHint.UUID, TypeHint.KEY])
register_mapper(os.Field(os.FT.DATE), hints=[TypeHint.DATE, TypeHint.DATETIME])
# number
register_mapper(os.Field(os.FT.DOUBLE), tags=[TypeTag.NUMBER], hints=[TypeHint.DURATION])
register_mapper(os.Field(os.FT.LONG), hints=[TypeHint.INTEGER])
# boolean
register_mapper(os.Field(os.FT.BOOLEAN), tags=[TypeTag.BOOLEAN])
# vector
register_mapper(VectorFieldMapper(), tags=[TypeTag.VECTOR])
# vector
register_mapper(os.Field(os.FT.FLAT_OBJECT), tags=[TypeTag.JSON])
# blob
register_mapper(
    os.Field(
        os.FT.OBJECT,
        properties={
            **mirror.Blob.__fields__,
            "id": os.Field(os.FT.KEYWORD),
            TYPE_DISCRIMINATOR_KEY: os.Field(os.FT.KEYWORD),
        },
    ),
    tags=[TypeTag.BLOB],
)
# secret
register_mapper(
    os.Field(
        os.FT.OBJECT,
        properties={
            **mirror.Secret.__fields__,
            "id": os.Field(os.FT.KEYWORD),
            TYPE_DISCRIMINATOR_KEY: os.Field(os.FT.KEYWORD),
        },
    ),
    tags=[TypeTag.STRING, TypeTag.NUMBER],
    flags=TypeFlag.IS_SECRET,
)
# struct
register_mapper(StructFieldMapper(), tags=[TypeTag.STRUCT])
# enum
register_mapper(os.Field(os.FT.KEYWORD), tags=[TypeTag.ENUM])


def map_to_os_field(field: lang.Field) -> os.Field:
    os_field = get_mapper(field).to_os_type(field, depth=0)
    if field.flags & TypeFlag.IS_STORE_ONLY:
        os_field.index = False
    return os_field


DOCUMENTS_BY_INDEX = {
    IndexType.GLOBAL: [
        mirror.User,
        mirror.Organization,
        mirror.Project,
        mirror.ProjectVersion,
        mirror.File,
        mirror.Statement,
        mirror.Field,
        mirror.Comment,
    ],
    IndexType.LOCAL: [mirror.Record, mirror.Session, mirror.Run, mirror.LogEntry],
}
SEARCH_SEMANTIC_EDIT_TYPES = {
    EditType.CREATE_FIELD,
    EditType.UPDATE_FIELD,
    EditType.UPDATE_FIELD_TYPE,
    EditType.DELETE_FIELD,
    EditType.TRUNCATE_RESOLVED_FIELDS,
    EditType.CREATE_RESOLVED_FIELD,
}


async def update_os_schema(os_name: str, module: Module, dynamic: str = "strict") -> None:
    """
    Updates *all* OpenSearch field mappings for a module
    TODO @Performance: update OS field mappings more efficiently on field edit
      (especially for library/dependency mappings)
    """
    from bench.language import libs

    value_mappings: dict[str, os.Field] = {}
    inputs_mappings: dict[str, os.Field] = {}
    outputs_mappings: dict[str, os.Field] = {}

    def _map_to_os_field_safe(field: lang.Field) -> os.Field | None:
        try:
            return map_to_os_field(field)
        except LookupError:
            logger.warning("os.update_mappings.field_mapping_failed", field=field)
            return None

    # get library mappings
    for lib in libs.DEFAULT_MODULES.values():
        for node in lib._nodes:
            if HasRun in node._components:
                for field in node.resolved_fields:
                    if field.flags & TypeFlag.IS_OUTPUT:
                        outputs_mappings[field._typed_key] = _map_to_os_field_safe(field)
                    else:
                        inputs_mappings[field._typed_key] = _map_to_os_field_safe(field)
    # ensure library vectors are not indexed (would be pointless waste of resources)
    for field in (*inputs_mappings.values(), *outputs_mappings.values()):
        for f in field.walk():
            if f.type == os.FieldType.KNN_VECTOR:
                f.index = False

    # and 'static' value mappings (hard-coded)
    for value_type in (libs.symbolx_lib.resolve(".reflect.RunMetadata"),):
        for field in value_type.resolved_fields:
            value_mappings[field._typed_key] = _map_to_os_field_safe(field)

    # add dynamic user mappings
    for node in module._nodes:
        if not isinstance(node, lang.Statement):
            continue
        if node.self_errors:
            continue  # ignore symbols with issues
        elif node.type == lang.StatementType.DATABASE:
            # all fields go into Record.value
            for field in node.resolved_fields:
                value_mappings[field._typed_key] = _map_to_os_field_safe(field)
        elif node.type in RUNNABLE_STATEMENT_TYPES:
            # inputs into Execution.inputs, outputs into Execution.outputs
            for field in node.resolved_fields:
                if field.flags & TypeFlag.IS_OUTPUT:
                    outputs_mappings[field._typed_key] = _map_to_os_field_safe(field)
                else:
                    inputs_mappings[field._typed_key] = _map_to_os_field_safe(field)

    # actually update mappings
    mappings = {}
    for key, sub_mappings in (
        ("value", value_mappings),
        ("inputs", inputs_mappings),
        ("outputs", outputs_mappings),
    ):
        sub_mappings = {k: v.to_dict() for (k, v) in sub_mappings.items() if v is not None}
        mappings[key] = {"type": "object", "dynamic": dynamic, "properties": sub_mappings}

    await os_client.indices.put_mapping(index=os_name, body={"properties": mappings})
    logger.info(
        "os.update_mappings.done",
        module=module,
        value_mappings=len(value_mappings),
        inputs_mappings=len(inputs_mappings),
        outputs_mappings=len(outputs_mappings),
    )


#
# Mapping queries
#


_SUPPORTED_SUBFIELDS_BY_TYPE: dict[TypeHint | TypeTag, tuple[SubfieldType, ...]] = {
    # cumulative supported subfields by type
    TypeHint.EMAIL: (SubfieldType.key, SubfieldType.starts_with),
    TypeHint.NAME: (SubfieldType.key, SubfieldType.starts_with),
}


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
}


def _compile_field_key(field: lang.Field | FieldReference) -> str:
    if isinstance(field, lang.Field):
        if field.reflected:
            return field.py_ident
        else:
            return field._source_key
    else:
        return field


def compile_os_conditional(ctx: CompilationContext, cond: Expression) -> dict[str, Any]:
    key = _compile_field_key(cond.field) if cond.field else None
    if cond.op == ConditionalOp.TRUE:
        return {"match_all": {}}
    elif cond.op == ConditionalOp.FALSE:
        return {"match_none": {}}
    elif cond.op in ExpressionOps.COND_LOGICAL:
        clauses = [compile_os_conditional(ctx, c) for c in cond.clauses]
        return {"bool": {OS_CONDITIONAL_OP_BY_BENCH[cond.op]: clauses}}
    elif cond.op == ConditionalOp.EQUALS:
        if isinstance(cond.value, list):
            return {"terms": {key: cond.value}}
        else:
            return {"term": {key: cond.value}}
    elif cond.op == ConditionalOp.NOT_EQUALS:
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
        # TODO @Performance @Robustness: tune knn k relative to database and query limit
        return {"knn": {key: {"vector": cond.value, "k": ctx.root_limit * 3}}}
    elif cond.op == ConditionalOp.EXISTS:
        return {"exists": {"field": key}}
    elif cond.op == ConditionalOp.NOT_EXISTS:
        return {"bool": {"must_not": {"exists": {"field": key}}}}
    raise QueryEngineIncapableError(QueryEngine.OPENSEARCH, cond, "unsupported conditional")


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
    if sort.mode:
        props["mode"] = OS_SORT_MODE_BY_BENCH[sort.mode]
    return {sort.field_key: props}


@dataclass(frozen=True)
class OsSearch:
    """Compiled search query for OS."""

    type: DocumentType
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

    def as_records(self) -> list[wire.RecordData]:
        records_data: list[wire.RecordData] = []
        for result in self.results:
            record_doc = mirror.Record.from_dict(result["_source"], result["_id"])
            record_data = mirror.pack_node_flat(record_doc)
            records_data.append(record_data)
        return records_data


def compile_os_search(
    type: DocumentType,
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
        clauses=[C(ConditionalOp.EQUALS, TYPE_DISCRIMINATOR_KEY, value=type.value)],
    )
    if query is not None:
        combined_query &= query
    # add id to sort as tiebreaker if not already present
    if sort and not any(s.field_key == "_id" for s in sort):
        sort = sort + [S(SortOp.ASCENDING, field="_id")]
    sort = sort or [S(SortOp.ASCENDING, field="_id")]
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
    e: Exception, expr: lang.Expression | list[lang.Expression]
) -> QueryEngineError | Exception:
    return QueryEngineError(QueryEngine.OPENSEARCH, expr, str(e))


async def os_search(
    os_name: str,
    type: "DocumentType",
    filter: Optional[Expression] = None,
    sort: Optional[list[Expression]] = None,
    limit: int | None = None,
    skip: int | None = None,
    count: bool = True,
    after: Optional[str] = None,
) -> OsSearchResult:
    """Executes a search query against OpenSearch."""
    search = compile_os_search(type, filter, sort, limit, skip, count, after)
    logger.debug("os.search", os_name=os_name, search=search)
    try:
        os_results = await os_client.search(index=os_name, body=search.to_dict())
    except Exception as e:
        raise _wrap_os_error(e, [filter, sort]) from e
    total = os_results["hits"]["total"]["value"] if search.count else None
    results = os_results["hits"]["hits"]
    cursors = [encode_os_cursor(r, search.after, i) for i, r in enumerate(results)]
    return OsSearchResult(total=total, results=results, cursors=cursors)


def os_search_sync(
    os_name: str,
    type: "DocumentType",
    filter: Optional[Expression] = None,
    sort: Optional[list[Expression]] = None,
    limit: int | None = None,
    skip: int | None = None,
    count: bool = True,
    after: Optional[str] = None,
) -> OsSearchResult:
    """
    Executes a search query against OpenSearch.
    """
    search = compile_os_search(type, filter, sort, limit, skip, count, after)
    logger.debug("os.search", os_name=os_name, search=search)
    try:
        os_results = os_client_sync.search(index=os_name, body=search.to_dict())
    except Exception as e:
        raise _wrap_os_error(e, [filter, sort]) from e
    total = os_results["hits"]["total"]["value"] if search.count else None
    results = os_results["hits"]["hits"]
    cursors = [encode_os_cursor(r, search.after, i) for i, r in enumerate(results)]
    return OsSearchResult(total=total, results=results, cursors=cursors)


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


async def sync_pg_databases_to_os(
    module: Module,
    pg_cursor: psycopg.AsyncCursor,
    record_ids_by_db: list[tuple["HasDatabase", set[UUID] | None]],
) -> None:
    """Synchronizes local PG databases to OpenSearch. Mirror only the given record ids if given."""
    from bench.sql.engine import (
        PostgresConditionalOp,
        SqlComparison,
        pg_select,
        pg_unpack_record_row,
    )

    log = logger.bind(module=module, databases=[r[0] for r in record_ids_by_db])
    log.debug("os.sync_pg_databases_to_os")
    os_name = module.os_name
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
            records_data = [pg_unpack_record_row(database, row) for row in records_data]
            # delete table by query
            await os_client.delete_by_query(
                index=os_name, body={"query": {"term": {"statement_key": database.key}}}
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
            records_data = [pg_unpack_record_row(database, row) for row in records_data]
            records_by_id = {r.id: r for r in records_data}
            missing_ids = record_ids - records_by_id.keys()
            # delete missing ids
            for deleted_record_id in missing_ids:
                ops.append({"delete": {"_index": os_name, "_id": str(deleted_record_id)}})

        # upsert records
        for record_data in records_data:
            record_mirror = mirror.unpack_node_flat(module, record_data, database)
            ops.append({"index": {"_index": os_name, "_id": str(record_data.id)}})
            ops.append(record_mirror.to_dict())

    await _flush()
    log.debug("os.sync_pg_databases_to_os.done")
