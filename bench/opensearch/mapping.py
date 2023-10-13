from dataclasses import dataclass
from typing import NamedTuple, Optional, Union

import bench.language as lang
import bench.opensearch.core as os
from bench.language import TypeHint, TypeTag
from bench.language.const import TypeFlag
from bench.language.field import TYPE_TAG_BY_TYPE_HINT
from bench.language.query import SubfieldType
from bench.language.typing import TYPENAME_SENTINEL
from bench.opensearch import mirror

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
            **mirror.RemoteObject.__fields__,
            "id": os.Field(os.FT.KEYWORD),
            TYPENAME_SENTINEL: os.Field(os.FT.KEYWORD),
        },
    ),
    tags=[TypeTag.FILE],
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
