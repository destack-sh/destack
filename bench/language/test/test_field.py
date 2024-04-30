import pytest

from bench.language.const import EnumType, NodeType, PrimitiveType, StructType
from bench.language.field import TypeInfo

TYPE_IDENTITIES = (
    TypeInfo(primitive_type=PrimitiveType.DATETIME),
    TypeInfo(bench_type=NodeType.USER),
    TypeInfo(bench_type=StructType.TEXT),
    TypeInfo(bench_type=EnumType.OBJECT_TYPE),
)


@pytest.mark.parametrize("type_info", TYPE_IDENTITIES)
def test_roundtrip_type_identity(type_info: TypeInfo):
    identity_key = type_info.identity_key
    print(identity_key)
