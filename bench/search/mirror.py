from dataclasses import dataclass
from typing import Any, Optional, TypeVar
from uuid import UUID

import bench.search.core as os
from bench.sql.core import Table


@dataclass
class ModuleInfo:
    id: UUID
    bench_id: UUID


NAME_FIELD = os.Field(
    os.FT.TEXT,
    fields={
        os.SubfieldType.starts_with: os.Field(os.FieldType.SEARCH_AS_YOU_TYPE),
        os.SubfieldType.key: os.Field(os.FieldType.KEYWORD),
    },
)
HTML_FIELD = os.Field(os.FieldType.TEXT, analyzer=os.Analyzer.HTML)
TEXT_FIELD = HTML_FIELD

TableT = TypeVar("TableT", bound=Table)
MirrorT = TypeVar("MirrorT", bound=os.Document)
DataT = TypeVar("DataT", bound=Any)

DOCUMENT_CLASS_BY_TYPE: dict[os.DocumentType, type[os.Document]] = {}


# nocheckin: generate os mirror schema
#  (like pg schema but only at runtime, no generated output since this is not a source of truth)


def has_mirror(node: TableT) -> bool:
    return type(node) in _packers_by_model


def mirror_node(module: ModuleInfo, node: TableT) -> MirrorT:
    packer = _packers_by_model[type(node)]
    return packer.mirror(module, node)


def pack_node_flat(node: MirrorT) -> DataT:
    packer = _packers_by_mirror[type(node)]
    return packer.pack(node)


def unpack_node_flat(module: ModuleInfo, data: DataT, parent: Optional[TableT]) -> MirrorT:
    packer = _packers_by_data[type(data)]
    return packer.unpack(module, data, parent)
