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
    Module,
    QueryEngine,
    SortMode,
    SortOp,
    TypeHint,
    TypeTag,
    symbolx_lib,
)
from bench.language.const import RUNNABLE_STATEMENT_TYPES, BenchType, EditKind, TypeFlag
from bench.language.database import HasDatabase
from bench.language.expression import (
    TYPE_DISCRIMINATOR_KEY,
    Expression,
    ExpressionOps,
    QueryEngineError,
    QueryEngineIncapableError,
    S,
)
from bench.language.field import TYPE_TAG_BY_TYPE_HINT
from bench.language.node import BENCH_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE, Node, Property, Struct
from bench.language.run import HasRun
from bench.os import core as os
from bench.os.client import get_os_errors, os_client, os_client_sync
from bench.os.core import SubfieldType
from bench.proto import wire, wiring
from bench.proto.wire import EditData
from bench.sql.core import ColumnType

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


TypeSignature = NamedTuple("TypeSignature", [("tag", TypeTag), ("hint", Optional[TypeHint])])

field_mappers: dict[TypeSignature, FieldMapper] = {}


def register_mapper(
    mapper: os.Field | FieldMapper,
    tags: list[TypeTag] = None,
    hints: list[TypeHint] = None,
) -> None:
    if not tags and not hints:
        raise ValueError("at least one tag or hint must be specified")
    if isinstance(mapper, os.Field):
        mapper = StaticFieldMapper(mapper)
    tags = tags or []
    hints = hints or []
    for tag in tags:
        field_mappers[TypeSignature(tag, None)] = mapper
    for hint in hints:
        tag = TYPE_TAG_BY_TYPE_HINT[hint]
        field_mappers[TypeSignature(tag, hint)] = mapper


def get_mapper(type: Union[lang.Field, lang.Statement]) -> FieldMapper:
    if type.tag == TypeTag.TYPE_REFERENCE and isinstance(type.reference, lang.Statement):
        return get_mapper(type.reference)  # skip the reference
    exact_signature = TypeSignature(type.tag, type.hint)
    mapping = field_mappers.get(exact_signature)
    if mapping is not None:
        return mapping
    # no exact match, try generic without hint
    stripped_signature = TypeSignature(type.tag, None)
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
            # (these don't get reported up to every reference, but we still can't map it)
            return os.Field(os.FT.OBJECT, properties=subfields)

        for f in type.fields:
            if f._effective_tag != TypeTag.STRUCT or depth < MAXIMUM_NESTING_DEPTH:
                subfields[f._storage_key] = get_mapper(f).to_os_type(f, depth + 1)
            else:
                # treat as json (but not as flattened yet.. :BadJsonMapping)
                subfields[f._storage_key] = os.Field(os.FT.OBJECT, dynamic=True, enabled=False)
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
register_mapper(os.Field(os.FT.DOUBLE), tags=[TypeTag.NUMBER], hints=[TypeHint.DURATION])
register_mapper(os.Field(os.FT.LONG), hints=[TypeHint.INTEGER])
register_mapper(os.Field(os.FT.BOOLEAN), tags=[TypeTag.BOOLEAN])
register_mapper(VectorFieldMapper(), tags=[TypeTag.VECTOR])
register_mapper(os.Field(os.FT.FLAT_OBJECT), tags=[TypeTag.JSON])
register_mapper(StructFieldMapper(), tags=[TypeTag.STRUCT])
register_mapper(os.Field(os.FT.KEYWORD), tags=[TypeTag.ENUM])


def map_to_os_field(field: lang.Field) -> os.Field:
    os_field = get_mapper(field).to_os_type(field, depth=0)
    if field.flags & TypeFlag.IS_STORE_ONLY:
        os_field.index = False
    return os_field


async def update_os_schema(module: Module, dynamic: str = "strict") -> None:
    """
    Updates *all* OpenSearch field mappings for a module
    TODO @Performance: update OS field mappings more efficiently on field edit
      (especially for library/dependency mappings)
    """
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
    for node in symbolx_lib._nodes:
        if HasRun in node._components:
            for field in node.fields:
                if field.flags & TypeFlag.IS_OUTPUT:
                    outputs_mappings[field._storage_key] = _map_to_os_field_safe(field)
                else:
                    inputs_mappings[field._storage_key] = _map_to_os_field_safe(field)
    # ensure library vectors are not indexed (would be pointless waste of resources)
    for field in (*inputs_mappings.values(), *outputs_mappings.values()):
        for f in field.walk():
            if f.type == os.FieldType.KNN_VECTOR:
                f.index = False

    # add dynamic user mappings
    for node in module._nodes:
        if not isinstance(node, lang.Statement):
            continue
        if node.self_errors:
            continue  # ignore nodes with issues
        elif node.type == lang.StatementType.DATABASE:
            # all fields go into Record.value
            for field in node.fields:
                value_mappings[field._storage_key] = _map_to_os_field_safe(field)
        elif node.type in RUNNABLE_STATEMENT_TYPES:
            # inputs into Execution.inputs, outputs into Execution.outputs
            for field in node.fields:
                if field.flags & TypeFlag.IS_OUTPUT:
                    outputs_mappings[field._storage_key] = _map_to_os_field_safe(field)
                else:
                    inputs_mappings[field._storage_key] = _map_to_os_field_safe(field)

    # actually update mappings
    mappings = {}
    for key, sub_mappings in (
        ("value", value_mappings),
        ("inputs", inputs_mappings),
        ("outputs", outputs_mappings),
    ):
        sub_mappings = {k: v.to_dict() for (k, v) in sub_mappings.items() if v is not None}
        mappings[key] = {"type": "object", "dynamic": dynamic, "properties": sub_mappings}

    await os_client.indices.put_mapping(index=module.os_name, body={"properties": mappings})
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
    ConditionalOp.GREATER_THAN: "gt",
    ConditionalOp.GREATER_THAN_OR_EQUALS: "gte",
    ConditionalOp.LESS_THAN: "lt",
    ConditionalOp.LESS_THAN_OR_EQUALS: "lte",
}


def _compile_expression_ref(expr: Expression) -> str:
    if expr.property_ptr is not None:
        return expr._stored_property_resolved.name
    if expr.field is not None:
        assert expr.field._reflected_from is None, f"cannot use reflected: {expr!r}->{expr.field!r}"
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
        # TODO @Performance @Robustness: tune knn k relative to database and query limit
        return {"knn": {key: {"vector": cond.value, "k": ctx.root_limit * 3}}}
    elif cond.op == ConditionalOp.EXISTS:
        return {"exists": {"field": key}}
    elif cond.op == ConditionalOp.NOT_EXISTS:
        return {"bool": {"must_not": {"exists": {"field": key}}}}
    raise QueryEngineIncapableError(QueryEngine.LOCAL_OPENSEARCH, cond, "unsupported conditional")


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
    if sort.property_ptr is not None:
        return {sort._stored_property_resolved.name: props}
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
        clauses=[C(ConditionalOp.EQUALS, field_key=TYPE_DISCRIMINATOR_KEY, value=metatype.name)],
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
    e: Exception, expr: lang.Expression | list[lang.Expression]
) -> QueryEngineError | Exception:
    return QueryEngineError(QueryEngine.LOCAL_OPENSEARCH, expr, str(e))


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


def os_search_sync(
    os_name: str,
    metatype: BenchType,
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
    search = os_compile_search(metatype, filter, sort, limit, skip, count, after)
    logger.debug("os.search", os_name=os_name, search=search)
    try:
        os_results = os_client_sync.search(index=os_name, body=search.to_dict())
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

FIELD_TYPE_BY_COLUMN_TYPE = {
    ColumnType.BOOLEAN: os.FieldType.BOOLEAN,
    ColumnType.DATETIME: os.FieldType.DATE,
    ColumnType.FLOAT: os.FieldType.DOUBLE,
    ColumnType.BIGINT: os.FieldType.LONG,
    ColumnType.JSON: os.FieldType.OBJECT,
    ColumnType.STRING: os.FieldType.TEXT,
    ColumnType.UUID: os.FieldType.KEYWORD,
}


def _is_property_indexed(prop: lang.Property) -> bool:
    return prop.is_stored and not prop.is_encrypted and not prop.is_deferred


def map_struct_type_to_os_document(struct: type[Struct]) -> os.Document:
    fields: dict[str, os.Field] = {}

    for prop in struct.__stored_properties__.values():
        if not _is_property_indexed(prop):
            continue
        if prop.is_enum:
            field = os.Field(os.FieldType.KEYWORD)
        elif prop.is_struct:
            struct_cls = STRUCT_CLASS_BY_TYPE[prop.struct_type]
            field = os.Field(
                os.FieldType.OBJECT, properties=map_struct_type_to_os_document(struct_cls).fields
            )
        elif prop.column_type == ColumnType.JSON:
            field = os.Field(os.FieldType.OBJECT, dynamic="strict")
        else:
            field_type = FIELD_TYPE_BY_COLUMN_TYPE.get(prop.column_type)
            if field_type is None:
                raise ValueError(f"unsupported column type in {prop!r}: {prop.column_type}")
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
    if struct.__is_indexed_in_os__
)


def _pack_struct_prop(prop: Property, value: Any, ignore_array: bool) -> Any:
    if value is None:
        return None
    elif prop.is_array and not ignore_array:
        return [_pack_struct_prop(prop, v, ignore_array=True) for v in value]
    elif prop.is_struct:
        return pack_struct(value)
    elif prop.column_type == ColumnType.JSON:
        return wiring.pack_json_value(value)
    elif prop.is_enum:
        return wiring.pack_enum(value)
    else:
        return value


def _unpack_struct_prop(prop: Property, value: Any, ignore_array: bool) -> Any:
    if value is None:
        return None
    elif prop.is_array and not ignore_array:
        return [_unpack_struct_prop(prop, v, ignore_array=True) for v in value]
    elif prop.is_struct:
        return unpack_struct(value)
    elif prop.column_type == ColumnType.JSON:
        return wiring.unpack_json_value(value)
    elif prop.is_enum:
        return wiring.unpack_enum(prop.enum_cls, value)
    else:
        return value


def pack_struct(node: wire.AnyNodeData | wire.AnyStructData) -> dict:
    metatype = wiring.unpack_enum(BenchType, node.metatype)
    bench_cls = BENCH_CLASS_BY_TYPE[metatype]
    document: dict[str, Any] = {TYPE_DISCRIMINATOR_KEY: metatype.name}
    for prop in bench_cls.__stored_properties__.values():
        if not _is_property_indexed(prop):
            continue
        value = getattr(node, prop.name)
        document[prop.name] = _pack_struct_prop(prop, value, ignore_array=False)
    return document


def unpack_struct(source: dict) -> wire.AnyNodeData | wire.AnyStructData:
    metatype = BenchType(source[TYPE_DISCRIMINATOR_KEY])
    bench_cls = BENCH_CLASS_BY_TYPE[metatype]
    proto_cls = wiring.PROTO_CLASS_BY_TYPE[metatype]
    proto_kwargs = {}
    for prop in bench_cls.__stored_properties__.values():
        if not _is_property_indexed(prop):
            continue
        value = source.get(prop.name)
        proto_kwargs[prop.name] = _unpack_struct_prop(prop, value, ignore_array=False)
    return proto_cls(**proto_kwargs)


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
        pg_unpack_record_data_row,
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
            records_data = [pg_unpack_record_data_row(database, row) for row in records_data]
            # delete table by query
            await os_client.delete_by_query(
                index=os_name, body={"query": {"term": {"statement_key": database.dynamic_key}}}
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
    TYPE_DISCRIMINATOR_KEY: os.TYPE_DISCRIMINATOR_FIELD,
}
GLOBAL_INDEX_SHARDS = 5
GLOBAL_INDEX_REPLICAS = 1
GLOBAL_READ_ONLY_ROLE = "global-ro"
BENCH_INDEX_SHARDS = 1
BENCH_INDEX_REPLICAS = 0
BENCH_MAPPING_TOTAL_FIELDS_LIMIT = 10000  # TODO @Performance: reconsider OS mapping limit


def _collect_fields(docs: list[os.Document]) -> dict[str, os.Field]:
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


def _create_os_index(
    index_name: str,
    *,
    shards: int,
    replicas: int,
    documents: list[os.Document],
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
    result = os_client_sync.indices.create(
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
        os_client_sync.indices.close(index=index_name)
        # update mutable settings
        os_client_sync.indices.put_settings(
            index=index_name,
            body={
                "analysis": {"tokenizer": tokenizers, "analyzer": analyzers},
                "mapping": {"total_fields": {"limit": BENCH_MAPPING_TOTAL_FIELDS_LIMIT}},
            },
        )
        os_client_sync.indices.put_mapping(
            index=index_name, body={"dynamic": "strict", "properties": mappings}
        )
        # reopen index
        os_client_sync.indices.open(index=index_name)
    log.info("os.create_index.done")


def create_global_os_index(upsert: bool = False) -> None:
    logger.info("os.create_global_index")
    _create_os_index(
        os.GLOBAL_INDEX_NAME,
        shards=GLOBAL_INDEX_SHARDS,
        replicas=GLOBAL_INDEX_REPLICAS,
        documents=[],  # nothing yet
        upsert=upsert,
    )


def create_global_os_role() -> None:
    # creates a global role (that doesn't do anything yet)
    # every user has this role to read public indices
    rep = os_client_sync.security.get_role(role=GLOBAL_READ_ONLY_ROLE, ignore=404)
    if rep.get("status") == "NOT_FOUND":
        rep = os_client_sync.security.create_role(
            role=GLOBAL_READ_ONLY_ROLE,
            body={
                "cluster_permissions": [],
                "index_permissions": [],
                "tenant_permissions": [],
            },
        )
        if rep.get("error"):
            raise RuntimeError(f"failed to create global-ro role: {rep['error']}")
        logger.info("os.create_global_role", name=GLOBAL_READ_ONLY_ROLE, rep=rep)
    else:
        logger.info("os.global_role_exists", name=GLOBAL_READ_ONLY_ROLE, rep=rep)


def create_local_os_index(
    *, os_name: str, os_username: str, os_password: str, is_public: bool, upsert: bool
) -> None:
    """
    Creates the OpenSearch index and corresponding roles/user for a bench.
    """
    log = logger.bind(os_name=os_name)
    log.info("os.create_local_index")
    # create index
    _create_os_index(
        index_name=os_name,
        shards=BENCH_INDEX_SHARDS,
        replicas=BENCH_INDEX_REPLICAS,
        documents=LOCAL_DOCUMENTS,
        upsert=upsert,
    )

    # get global read only role to modify
    rep = os_client_sync.security.get_role(role=GLOBAL_READ_ONLY_ROLE, ignore=404)
    role = rep.get(GLOBAL_READ_ONLY_ROLE)
    assert role is not None, f"failed to get role {GLOBAL_READ_ONLY_ROLE}: {rep}"
    if is_public:
        # TODO @Robustness: fix race condition between read and patch global role
        #  (unfortunately the 'add' op doesn't seem to be actually additive?)
        # grant read access to global read only role
        log.info("os.grant_global_read_access")
        rep = os_client_sync.security.patch_role(
            role=GLOBAL_READ_ONLY_ROLE,
            body=[
                {
                    "op": "add",
                    "path": "/index_permissions",
                    "value": [
                        *(r for r in role["index_permissions"] if r["index_patterns"] != [os_name]),
                        {
                            "index_patterns": [os_name],
                            "fls": [],
                            "masked_fields": [],
                            "allowed_actions": ["read"],
                        },
                    ],
                }
            ],
        )
        if rep.get("error"):
            raise RuntimeError(f"failed to grant read access to global-ro: {rep['error']}")
        log.info("os.grant_global_read_access.done")
    else:
        # revoke read access from global read only role (if exists)
        log.info("os.revoke_global_read_access")

        # find index permission for this bench
        permission_idx = -1
        for i, index_permission in enumerate(role["index_permissions"]):
            if index_permission["index_patterns"] == [os_name]:
                permission_idx = i
                break
        if permission_idx >= 0:
            rep = os_client_sync.security.patch_role(
                role=GLOBAL_READ_ONLY_ROLE,
                body={"op": "remove", "path": f"/index_permissions/{permission_idx}"},
            )
            if rep.get("error"):
                raise RuntimeError(f"failed to revoke read access from global-ro: {rep['error']}")
            log.info("os.revoke_global_read_access.done")
        else:
            log.info("os.revoke_global_read_access.not_found")

    # create write access role for bench owner
    log.info("os.create_owner_role")
    owner_role_name = f"{os_name}-rw"
    rep = os_client_sync.security.get_role(role=owner_role_name, ignore=404)
    owner_role = rep.get(owner_role_name)
    if owner_role is not None:
        os_client_sync.security.delete_role(role=owner_role_name)
    rep = os_client_sync.security.create_role(
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
    rep = os_client_sync.security.get_user(username=os_username, ignore=404)
    user = rep.get(os_username)
    if user is not None:
        os_client_sync.security.delete_user(username=os_username)
    rep = os_client_sync.security.create_user(
        username=os_username,
        body={
            "password": os_password,
            "opendistro_security_roles": [owner_role_name, GLOBAL_READ_ONLY_ROLE],
        },
    )
    if rep.get("error"):
        raise RuntimeError(f"failed to create user {os_username}: {rep['error']}")
    log.info("os.create_user.done", username=os_username)


async def os_write_edits(module: Module, edits: list[EditData], *, refresh: bool = False) -> None:
    """
    Writes/mirrors any relevant edit to OpenSearch.
    All regular DB edit come this way.
    """
    log = logger.bind(module=module, edits=edits)
    log.debug("os.write_edits")
    if not edits:
        if refresh:
            # just refresh the index
            os_client_sync.indices.refresh(index=module.os_name)
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
        if not node_cls.__is_indexed_in_os__:
            continue  # ignore
        elif edit.type.kind in (
            EditKind.CREATE,
            EditKind.UPDATE,
            EditKind.MOVE,
            EditKind.SOFT_DELETE,
            EditKind.RESTORE,
        ):
            ops.append({"index": {"_index": index, "_id": str(edit.node.id)}})
            ops.append(pack_struct(wiring.unwrap_some_node(edit.node)))
        elif edit.type.kind == EditKind.DELETE:
            ops.append({"delete": {"_index": index, "_id": str(edit.node.id)}})
        else:
            raise ValueError(f"unexpected edit type: {edit!r}")

    await _flush()  # flush all remaining edits


async def os_sync_databases(module: Module, databases: list[HasDatabase]) -> None:
    """
    Mirrors the given databases to OpenSearch, replacing any existing data.
    Obviously not scalable yet because it just selects everything in one go (no streaming).
    TODO @Robustness: race condition in syncing database because OS has no transactions?
    """
    from bench.sql.engine import async_pg_cursor, pg_select_records_data

    log = logger.bind(module=module, databases=databases)

    all_records: list[wire.RecordData] = []
    async with async_pg_cursor(module.pg_name) as cur:
        for database in databases:
            where = C(
                ConditionalOp.EQUALS, field_key="statement_key", value=database.dynamic_key
            ) & ~C(ConditionalOp.EXISTS, field_key="deleted_at")
            records_data, _, _ = await pg_select_records_data(cur, database, where=where)
            all_records.extend(records_data)

    log.debug("os.write_edits.flush", records=len(all_records))
    # wipe all databases by query
    filter = [
        {"term": {TYPE_DISCRIMINATOR_KEY: BenchType.RECORD}},
        {"terms": {"statement_key": [d.key for d in databases]}},
    ]
    await os_client.delete_by_query(module.os_name, body={"query": {"bool": {"filter": filter}}})
    await os_write_records(module, all_records)


async def os_write_records(module: Module, records: list[wire.RecordData]) -> None:
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


def enable_os_strict_mapping(index_name: str) -> None:
    logger.info("os.enable_strict_dynamic_mapping", index=index_name)
    rep = os_client_sync.indices.put_mapping(index=index_name, body={"dynamic": "strict"})
    if rep.get("error"):
        raise RuntimeError(f"failed to enable strict dynamic mapping: {rep['error']}")
