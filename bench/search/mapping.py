import base64
import json
from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Any, NamedTuple, Optional, Union

import bench.language as lang
import bench.search.core as os
from bench.language import (
    C,
    Conditional,
    ConditionalOp,
    Sort,
    SortMode,
    SortOrder,
    TypeHint,
    TypeTag,
)
from bench.language.const import TypeFlag
from bench.language.expression import (
    TYPE_DISCRIMINATOR_KEY,
    ComparisonConditional,
    CompoundConditional,
    ExistenceConditional,
    SubfieldType,
    VectorConditional,
    get_default_sort,
)
from bench.language.field import TYPE_TAG_BY_TYPE_HINT
from bench.language.packer import TYPENAME_SENTINEL
from bench.search import mirror
from bench.search.core import DocumentType

MAXIMUM_NESTING_DEPTH = 3


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
            return os.Field(os.FT.OBJECT, dynamic="strict", properties=subfields)

        for f in type.resolved_fields:
            if f._effective_tag != TypeTag.STRUCT or depth < MAXIMUM_NESTING_DEPTH:
                subfields[f._typed_key] = get_mapper(f).to_os_type(f, depth + 1)
            else:
                # treat as json (but not as flattened yet.. :BadJsonMapping)
                subfields[f._typed_key] = os.Field(os.FT.OBJECT, dynamic=True, enabled=False)
        return os.Field(os.FT.OBJECT, dynamic="strict", properties=subfields)


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
        # :QuerySubfields
        fields={
            SubfieldType.token_count: os.Field(os.FT.TOKEN_COUNT, analyzer=os.Analyzer.STANDARD),
            SubfieldType.char_count: os.Field(os.FT.TOKEN_COUNT, analyzer=os.Analyzer.CHAR_COUNT),
        },
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
            SubfieldType.token_count: os.Field(os.FT.TOKEN_COUNT, analyzer=os.Analyzer.STANDARD),
            SubfieldType.char_count: os.Field(os.FT.TOKEN_COUNT, analyzer=os.Analyzer.CHAR_COUNT),
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
# TODO @Feature: index JSON as flattened object fields (not available in OpenSearch 2.5) :BadJsonMapping
register_mapper(os.Field(os.FT.OBJECT, dynamic=True, enabled=False), tags=[TypeTag.JSON])
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
                return {"terms": {query.key: query.value}}
            else:
                return {"term": {query.key: query.value}}
        elif query.op == ConditionalOp.NOT_EQUALS:
            return {"bool": {"must_not": {"term": {query.key: query.value}}}}
        elif query.op == ConditionalOp.GREATER_THAN:
            return {"range": {query.key: {"gt": query.value}}}
        elif query.op == ConditionalOp.GREATER_THAN_OR_EQUALS:
            return {"range": {query.key: {"gte": query.value}}}
        elif query.op == ConditionalOp.LESS_THAN:
            return {"range": {query.key: {"lt": query.value}}}
        elif query.op == ConditionalOp.LESS_THAN_OR_EQUALS:
            return {"range": {query.key: {"lte": query.value}}}
        elif query.op == ConditionalOp.MATCHES:
            return {"match": {query.key: query.value}}
        elif query.op == ConditionalOp.STARTS_WITH:
            return {"prefix": {query.key: query.value}}
        else:
            raise RuntimeError(f"unexpected query: {query}")


@compiler(ExistenceConditional)
class ExistenceQueryCompiler(Compiler):
    def compile(self, info: CompilationInfo, query: ExistenceConditional) -> dict[str, Any]:
        if query.op == ConditionalOp.EXISTS:
            return {"exists": {"field": query.key}}
        elif query.op == ConditionalOp.DOES_NOT_EXIST:
            return {"bool": {"must_not": {"exists": {"field": query.key}}}}
        else:
            raise RuntimeError(f"unexpected query: {query}")


@compiler(VectorConditional)
class VectorQueryCompiler(Compiler):
    def compile(self, info: CompilationInfo, query: VectorConditional) -> dict[str, Any]:
        if query.approximate:
            # TODO @Performance @Robustness: tune knn k relative to database and query limit
            return {"knn": {query.key: {"vector": query.value, "k": info.root_limit * 2}}}
        else:
            raise NotImplementedError(f"exact knn not implemented: {query}")


@compiler(Sort)
class SortCompiler(Compiler):
    SORT_ORDERS = {
        SortOrder.ASCENDING: "asc",
        SortOrder.DESCENDING: "desc",
    }
    SORT_MODES = {
        SortMode.MIN: "min",
        SortMode.MAX: "max",
        SortMode.AVERAGE: "avg",
        SortMode.MEDIAN: "median",
        SortMode.SUM: "sum",
    }

    def compile(self, info: CompilationInfo, sort: Sort) -> dict[str, Any]:
        props = {"order": self.SORT_ORDERS[sort.order]}
        if sort.mode:
            props["mode"] = self.SORT_MODES[sort.mode]
        return {sort.key: props}


DslObj = Union[Conditional, Sort]


def compile_to_os(
    info: CompilationInfo, obj: DslObj | list[DslObj]
) -> dict[str, Any] | list[dict[str, Any]]:
    if isinstance(obj, list):
        return [compile_to_os(info, o) for o in obj]
    else:
        compiler = _COMPILERS[type(obj)]
        return compiler.compile(info, obj)


def compact_os_queries(queries: list[dict[str, Any]]) -> dict[str, Any]:
    """
    Combine a list of queries into a single query.
    Unroll and un-nest as much as possible
     - two terms queries with different keys into one terms
    TODO @Cleanup @Performance?: actually compact OS queries
    """
    if len(queries) == 1:
        return queries[0]
    else:
        return {"bool": {"must": queries}}


def prepare_os_query(
    type: "DocumentType",
    project_version_id: Union[str, list[str], None],
    limit: int,
    count: bool,
    after: Optional[str] = None,
    sort: Optional[list[Sort]] = None,
    query: Optional[Conditional] = None,
    version: bool = False,
    fields: Optional[list[str]] = None,
    source: bool = True,
) -> dict:
    combined_query = C(
        ConditionalOp.AND,
        clauses=[
            C(ConditionalOp.EQUALS, TYPE_DISCRIMINATOR_KEY, value=type.value),
            ~C(ConditionalOp.EXISTS, "deleted_at"),
        ],
    )
    if project_version_id:
        combined_query &= C(ConditionalOp.EQUALS, "project_version_id", value=project_version_id)
    if query is not None:
        combined_query &= query
    # add id to sort as tiebreaker if not already present
    if sort and not any(s.key == "_id" for s in sort):
        sort = sort + [Sort(field="_id", order=SortOrder.ASCENDING)]
    compilation = CompilationInfo(root_limit=limit)
    compiled_query = compile_to_os(compilation, combined_query)
    compiled_sort = compile_to_os(compilation, sort or get_default_sort(combined_query))
    search = {
        "size": limit,
        "query": compiled_query,
        "sort": compiled_sort,
        "track_total_hits": count,
        "version": version,
        "_source": source,
    }
    if fields is not None:
        search["fields"] = fields
    if after:
        # cursor is base64 encoded json of search after if it exists,
        # otherwise just from for relevance-scored search (opaque to client)
        after = json.loads(base64.b64decode(after).decode())
        if isinstance(after, list):
            search["search_after"] = after
        elif isinstance(after, int):
            search["from"] = after
        else:
            raise ValueError("invalid cursor")
    return search


def encode_cursor(record: dict[str, Any], after: Optional[str], i: int) -> str:
    # for relevance-scored search, cursor is from offset, so add i to it
    # otherwise, cursor is search_after, so encode 'sort' from record
    if "sort" in record:
        return base64.b64encode(json.dumps(record["sort"]).encode()).decode("utf-8")
    else:
        after = json.loads(base64.b64decode(after).decode()) if after else 0
        if not isinstance(after, int):
            raise ValueError("invalid cursor")
        return base64.b64encode(json.dumps(after + i).encode()).decode("utf-8")
