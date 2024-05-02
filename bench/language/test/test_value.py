from bench.language.block import Block
from bench.language.const import BlockType, NodeType, PrimitiveType, StructType
from bench.language.field import FieldZone, TypeInfo, TypeKind
from bench.language.notice import on_notice_ignore
from bench.language.text import Text
from bench.language.value import Object, pack_value, unpack_value


def test_roundtrip_simple_value():
    choice1 = Block(type=BlockType.CHOICE, name="Choice1")
    choice1.fields.create(name="Option1", zone=FieldZone.OPTION)
    choice1.fields.create(name="Option2", zone=FieldZone.OPTION)
    choice1.fields.create(name="Option3", zone=FieldZone.OPTION)

    class1 = Block(type=BlockType.CLASS, name="Class1")
    class1.fields.create(
        name="Field1", bench_type=NodeType.FIELD, base_type=choice1, kind=TypeKind.BASED_NODE
    )
    class1.fields.create(
        name="Field2", primitive_type=PrimitiveType.BOOLEAN, kind=TypeKind.PRIMITIVE
    )
    class1.fields.create(name="Field3", bench_type=StructType.TEXT, kind=TypeKind.STRUCT)

    choice1._interp_rec(None, on_notice_ignore)
    class1._interp_rec(None, on_notice_ignore)
    type1 = TypeInfo(kind=TypeKind.ALIAS, base_type=class1)
    type1._interp_rec(None, on_notice_ignore)

    value = Object.new({}, type1)
    value.field1 = choice1.fields.Option1
    value.field2 = False
    value.field3 = Text.plain("hello bench!")

    value_packed, secret_value_packed = pack_value(value, type1)
    unpacked_value = unpack_value(value_packed, secret_value_packed, type1)
    assert unpacked_value == value


# nocheckin: auto generate :Test types & values
