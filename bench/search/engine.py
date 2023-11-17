import base64
import json
from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Any, NamedTuple, Optional, Union

import structlog

from bench import language as lang
from bench.language import (
    C,
    Conditional,
    ConditionalOp,
    HasRun,
    Module,
    Sort,
    SortMode,
    SortOp,
    TypeHint,
    TypeTag,
)
from bench.language.const import RUNNABLE_STATEMENT_TYPES, TypeFlag
from bench.language.edit import MET, MNT
from bench.language.expression import (
    TYPE_DISCRIMINATOR_KEY,
    ComparisonConditional,
    CompoundConditional,
    ExistenceConditional,
    VectorConditional,
    get_default_sort,
)
from bench.language.field import TYPE_TAG_BY_TYPE_HINT
from bench.language.packer import TYPENAME_SENTINEL
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
        subfields = {TYPENAME_SENTINEL: os.Field(os.FT.KEYWORD)}
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
        # see https://aws.amazon.com/blogs/big-data/choose-the-k-nn-algorithm-for-your-billion-scale-use-case-with-opensearch/
        # see https://github.com/nmslib/hnswlib/blob/master/ALGO_PARAMS.md#construction-parameters
        # ideally we would use the Lucene engine with byte vectors here, but it only goes to 1024 dims
        method = os.KnnMethod(
            # assumes normalized vectors with a :FixedEmbeddingDimension
            name=os.KnnMethodName.HNSW,
            engine=os.KnnEngine.NMSLIB,
            space_type=os.KnnSpaceType.DOT_PRODUCT,
            parameters=os.HnswParameters(ef_construction=512, m=64),
        )
        return os.Field(os.FT.KNN_VECTOR, dimension=type.dimensions, method=method)


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
register_mapper(os.Field(os.FT.KEYWORD), hints=[TypeHint.UUID, TypeHint.KEY])
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
# file
register_mapper(
    os.Field(
        os.FT.OBJECT,
        properties={
            **mirror.Blob.__fields__,
            "id": os.Field(os.FT.KEYWORD),
            TYPENAME_SENTINEL: os.Field(os.FT.KEYWORD),
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
            TYPENAME_SENTINEL: os.Field(os.FT.KEYWORD),
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
    IndexType.LOCAL: [
        mirror.Record,
        mirror.Session,
        mirror.Run,
        mirror.LogEntry,
    ],
}
SEARCH_SEMANTIC_EDIT_TYPES = {
    MET.CREATE_FIELD,
    MET.UPDATE_FIELD,
    MET.UPDATE_FIELD_TYPE,
    MET.DELETE_FIELD,
    MET.TRUNCATE_RESOLVED_FIELDS,
    MET.CREATE_RESOLVED_FIELD,
}
BENCH_LOCAL_MNTS = (MNT.RECORD,)


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

    # get library mappings
    for lib in libs.DEFAULT_MODULES.values():
        for node in lib._nodes:
            if HasRun in node._components:
                for field in node.resolved_fields:
                    if field.flags & TypeFlag.IS_OUTPUT:
                        outputs_mappings[field._typed_key] = map_to_os_field(field)
                    else:
                        inputs_mappings[field._typed_key] = map_to_os_field(field)
    # ensure library vectors are not indexed (would be pointless waste of resources)
    for field in (*inputs_mappings.values(), *outputs_mappings.values()):
        for f in field.walk():
            if f.type == os.FieldType.KNN_VECTOR:
                f.index = False

    # and 'static' value mappings (hard-coded)
    for value_type in (libs.symbolx_lib.resolve(".reflect.RunMetadata"),):
        for field in value_type.resolved_fields:
            value_mappings[field._typed_key] = map_to_os_field(field)

    # add dynamic user mappings
    for node in module._nodes:
        if not isinstance(node, lang.Statement):
            continue
        if node.self_errors:
            continue  # ignore symbols with issues
        elif node.type == lang.StatementType.DATABASE:
            # all fields go into Record.value
            for field in node.resolved_fields:
                value_mappings[field._typed_key] = map_to_os_field(field)
        elif node.type in RUNNABLE_STATEMENT_TYPES:
            # inputs into Execution.inputs, outputs into Execution.outputs
            for field in node.resolved_fields:
                if field.flags & TypeFlag.IS_OUTPUT:
                    outputs_mappings[field._typed_key] = map_to_os_field(field)
                else:
                    inputs_mappings[field._typed_key] = map_to_os_field(field)

    # actually update mappings
    mappings = {}
    for key, sub_mappings in (
        ("value", value_mappings),
        ("inputs", inputs_mappings),
        ("outputs", outputs_mappings),
    ):
        sub_mappings = {k: v.to_dict() for (k, v) in sub_mappings.items()}
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


_SUPPORTED_SUBFIELDS_BY_TYPE: dict[TypeHint | TypeTag, set[SubfieldType]] = {
    # cumulative supported subfields by type
    TypeHint.EMAIL: {SubfieldType.key, SubfieldType.starts_with},
    TypeHint.NAME: {SubfieldType.key, SubfieldType.starts_with},
}


@dataclass
class CompilationInfo:
    root_limit: Optional[int]


class Compiler(ABC):
    @abstractmethod
    def compile(self, info: CompilationInfo, obj: Any) -> dict[str, Any]:
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


@compiler(CompoundConditional)
class CompoundQueryCompiler(Compiler):
    MAPPING = {
        ConditionalOp.NOT: "must_not",
        ConditionalOp.AND: "must",
        ConditionalOp.OR: "should",
    }

    def compile(self, info: CompilationInfo, query: CompoundConditional) -> dict[str, Any]:
        return {"bool": {self.MAPPING[query.op]: compile_to_os(info, query.clauses)}}


@compiler(ComparisonConditional)
class ComparisonQueryCompiler(Compiler):
    def compile(self, info: CompilationInfo, query: ComparisonConditional) -> dict[str, Any]:
        if query.op == ConditionalOp.EQUALS:
            if isinstance(query.value, list):
                return {"terms": {query.field_key: query.value}}
            else:
                return {"term": {query.field_key: query.value}}
        elif query.op == ConditionalOp.NOT_EQUALS:
            return {"bool": {"must_not": {"term": {query.field_key: query.value}}}}
        elif query.op == ConditionalOp.GREATER_THAN:
            return {"range": {query.field_key: {"gt": query.value}}}
        elif query.op == ConditionalOp.GREATER_THAN_OR_EQUALS:
            return {"range": {query.field_key: {"gte": query.value}}}
        elif query.op == ConditionalOp.LESS_THAN:
            return {"range": {query.field_key: {"lt": query.value}}}
        elif query.op == ConditionalOp.LESS_THAN_OR_EQUALS:
            return {"range": {query.field_key: {"lte": query.value}}}
        elif query.op == ConditionalOp.MATCHES:
            return {"match": {query.field_key: query.value}}
        elif query.op == ConditionalOp.STARTS_WITH:
            return {"prefix": {query.field_key: query.value}}
        else:
            raise RuntimeError(f"unexpected query: {query}")


@compiler(ExistenceConditional)
class ExistenceQueryCompiler(Compiler):
    def compile(self, info: CompilationInfo, query: ExistenceConditional) -> dict[str, Any]:
        if query.op == ConditionalOp.EXISTS:
            return {"exists": {"field": query.field_key}}
        elif query.op == ConditionalOp.NOT_EXISTS:
            return {"bool": {"must_not": {"exists": {"field": query.field_key}}}}
        else:
            raise RuntimeError(f"unexpected query: {query}")


@compiler(VectorConditional)
class VectorQueryCompiler(Compiler):
    def compile(self, info: CompilationInfo, query: VectorConditional) -> dict[str, Any]:
        if query.approximate:
            # TODO @Performance @Robustness: tune knn k relative to database and query limit
            return {"knn": {query.field_key: {"vector": query.value, "k": info.root_limit * 2}}}
        else:
            raise NotImplementedError(f"exact knn not implemented: {query}")


@compiler(Sort)
class SortCompiler(Compiler):
    SORT_ORDERS = {
        SortOp.ASCENDING: "asc",
        SortOp.DESCENDING: "desc",
    }
    SORT_MODES = {
        SortMode.MIN: "min",
        SortMode.MAX: "max",
        SortMode.AVERAGE: "avg",
        SortMode.MEDIAN: "median",
        SortMode.SUM: "sum",
    }

    def compile(self, info: CompilationInfo, sort: Sort) -> dict[str, Any]:
        props = {"order": self.SORT_ORDERS[sort.op]}
        if sort.mode:
            props["mode"] = self.SORT_MODES[sort.mode]
        return {sort.field_key: props}


DslObj = Union[Conditional, Sort]


def compile_to_os(
    info: CompilationInfo, obj: DslObj | list[DslObj]
) -> dict[str, Any] | list[dict[str, Any]]:
    if isinstance(obj, list):
        return [compile_to_os(info, o) for o in obj]
    else:
        compiler = _COMPILERS[type(obj)]
        return compiler.compile(info, obj)


@dataclass
class SearchQuery:
    """Compiled search query for OS."""

    type: DocumentType
    limit: int | None = None
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
        return search


def compile_os_query(
    type: "DocumentType",
    query: Optional[Conditional] = None,
    sort: Optional[list[Sort]] = None,
    limit: int | None = None,
    count: bool = True,
    after: Optional[str] = None,
) -> SearchQuery:
    combined_query = C(
        ConditionalOp.AND,
        clauses=[
            C(ConditionalOp.EQUALS, TYPE_DISCRIMINATOR_KEY, value=type.value),
            ~C(ConditionalOp.EXISTS, "deleted_at"),
        ],
    )
    if query is not None:
        combined_query &= query
    # add id to sort as tiebreaker if not already present
    if sort and not any(s.field_key == "_id" for s in sort):
        sort = sort + [Sort(field="_id")]
    compilation = CompilationInfo(root_limit=limit)
    compiled_query = compile_to_os(compilation, combined_query)
    compiled_sort = compile_to_os(compilation, sort or get_default_sort(combined_query))
    return SearchQuery(
        type=type,
        limit=limit,
        count=count,
        after=after,
        sort=compiled_sort,
        query=compiled_query,
    )


async def os_search(os_name: str, query: SearchQuery) -> dict:
    """
    Executes a search query against OpenSearch.
    """
    return await os_client.search(index=os_name, body=query.to_dict())


def os_search_sync(os_name: str, query: SearchQuery) -> dict:
    """
    Executes a search query against OpenSearch.
    """
    return os_client_sync.search(index=os_name, body=query.to_dict())


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
