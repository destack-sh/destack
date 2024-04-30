from uuid import UUID

import pytest

from bench.language.const import EnumType, NodeType, PrimitiveType, StructType
from bench.language.expression import NodeReference
from bench.language.field import TypeInfo, decode_type_identity, encode_type_identity
from bench.language.node import get_tk_from_ptr_maybe

TYPE_IDENTITIES = (
    TypeInfo(primitive_type=PrimitiveType.DATETIME),
    TypeInfo(bench_type=NodeType.USER, is_list=True),
    TypeInfo(bench_type=StructType.TEXT),
    TypeInfo(bench_type=EnumType.OBJECT_TYPE, is_secret=True),
    TypeInfo(
        bench_type=NodeType.FIELD,
        base_type_ptr=NodeReference(
            type=NodeType.BLOCK, ck=UUID("12345678-ffff-0000-0000-000000000000")
        ),
        is_secret=True,
        is_list=True,
    ),
    TypeInfo(
        base_type_ptr=NodeReference(
            type=NodeType.BLOCK, ck=UUID("12345678-ffff-0000-0000-000000000000")
        ),
    ),
)


@pytest.mark.parametrize("type_info", TYPE_IDENTITIES)
def test_roundtrip_type_identity(type_info: TypeInfo):
    identity_key = encode_type_identity(type_info)
    decoded = decode_type_identity(identity_key)
    assert decoded.primitive_type == type_info.primitive_type
    assert decoded.bench_type == type_info.bench_type
    assert get_tk_from_ptr_maybe(decoded.base_type_ptr) == get_tk_from_ptr_maybe(
        type_info.base_type_ptr
    )
    assert decoded.is_list == type_info.is_list
    assert decoded.is_secret == type_info.is_secret
