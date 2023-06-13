from dataclasses import dataclass
from typing import Any, NamedTuple, Optional

import bench.bench as lang
import bench.opensearch.type as os
from bench.bench import TypeHint, TypeTag, wire
from bench.bench.const import TYPE_TAG_BY_TYPE_HINT, TypeFlag
from bench.opensearch import mirror


class FieldMapper:
    """
    Maps a Bench Field to an OpenSearch field (possibly nested).
    Don't bother with lists and optional here.
    """

    def to_os_type(self, field: lang.Field) -> os.Field:
        raise NotImplementedError

    def to_os_value(self, field: lang.Field, value: Any) -> Any:
        raise NotImplementedError

    def from_os_value(self, field: lang.Field, value: Any) -> Any:
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
):
    if not tags and not hints:
        raise ValueError("at least one tag or hint must be specified")
    if isinstance(mapper, os.Field):
        mapper = StaticFieldMapper(mapper)
    tags = tags or []
    hints = hints or []
    flags = flags or TypeFlag.Zero
    for tag in tags:
        field_mappers[TypeSignature(tag, None, flags)] = mapper
    for hint in hints:
        tag = TYPE_TAG_BY_TYPE_HINT[hint]
        field_mappers[TypeSignature(tag, hint, flags)] = mapper


@dataclass
class StaticFieldMapper(FieldMapper):
    field: os.Field | os.FieldType

    def to_os_type(self, field: lang.Field) -> os.Field:
        return self.field


# string
register_mapper(
    os.Field(
        os.FieldType.TEXT,
        fields={os.FieldType.TOKEN_COUNT: os.Field(os.FieldType.TOKEN_COUNT)},
    ),
    tags=[TypeTag.STRING],
)
register_mapper(
    os.Field(
        os.FieldType.TEXT,
        fields={
            os.FieldType.KEYWORD: os.Field(os.FieldType.KEYWORD),
            os.FieldType.SEARCH_AS_YOU_TYPE: os.Field(os.FieldType.SEARCH_AS_YOU_TYPE),
            os.FieldType.TOKEN_COUNT: os.Field(os.FieldType.TOKEN_COUNT),
        },
    ),
    hints=[TypeHint.NAME],
)
register_mapper(os.Field(os.FieldType.KEYWORD), hints=[TypeHint.UUID, TypeHint.KEY])
# number
register_mapper(os.Field(os.FieldType.DOUBLE), tags=[TypeTag.NUMBER])
register_mapper(os.Field(os.FieldType.LONG), hints=[TypeHint.INTEGER])
# boolean
register_mapper(os.Field(os.FieldType.BOOLEAN), tags=[TypeTag.BOOLEAN])
# vector
register_mapper(os.Field(os.FieldType.KNN_VECTOR), tags=[TypeTag.VECTOR])
# file
register_mapper(
    os.Field(os.FieldType.OBJECT, fields=mirror.RemoteObject.fields()), tags=[TypeTag.FILE]
)
# secret
register_mapper(
    os.Field(os.FieldType.OBJECT, fields=mirror.Secret.fields()),
    tags=[TypeTag.STRING, TypeTag.NUMBER],
    flags=TypeFlag.IsSecret,
)


def pack_record(record: mirror.Record) -> wire.RecordData:
    return wire.RecordData(
        id=record.id,
        parent_id=record.statement_id,
        order_key=record.order_key,
        revision=record.revision,
        data=record.data,
    )


def unpack_record(data: wire.RecordData) -> mirror.Record:
    return mirror.Record(
        id=data.id,
        statement_id=data.parent_id,
        order_key=data.order_key,
        revision=data.revision,
        data=data.data,
    )
