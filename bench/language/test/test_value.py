from bench.language.block import Block
from bench.language.const import BlockType, NodeType, PrimitiveType, StructType
from bench.language.field import FieldZone, TypeInfo, TypeKind
from bench.language.notice import on_notice_ignore
from bench.language.text import Text
from bench.language.value import Object, pack_value, unpack_value


def test_roundtrip_simple_value():
    # choice block
    choice1 = Block(type=BlockType.CHOICE, name="Choice1")
    choice1.fields.create(name="Option1", zone=FieldZone.OPTION)
    choice1.fields.create(name="Option2", zone=FieldZone.OPTION)
    choice1.fields.create(name="Option3", zone=FieldZone.OPTION)

    # inner class
    class2 = Block(type=BlockType.CLASS, name="Class2")
    class2.fields.create(
        name="Field1", bench_type=NodeType.FIELD, base_type=choice1, kind=TypeKind.BASED_NODE
    )
    class2.fields.create(name="Field2", bench_type=NodeType.BLOCK, kind=TypeKind.NODE)

    # outer class
    class1 = Block(type=BlockType.CLASS, name="Class1")
    class1.fields.create(
        name="Field1", bench_type=NodeType.FIELD, base_type=choice1, kind=TypeKind.BASED_NODE
    )
    class1.fields.create(
        name="Field2", primitive_type=PrimitiveType.BOOLEAN, kind=TypeKind.PRIMITIVE
    )
    class1.fields.create(
        name="Field3", bench_type=StructType.TEXT, kind=TypeKind.STRUCT, is_list=True
    )
    class1.fields.create(name="Field4", base_type=class2, kind=TypeKind.ALIAS)

    # TODO :Cleanup :Test: interp/to_resolved shit should not be necessary
    #  (run this test in session? or somehow in 'tracked' mode)
    choice1._interp_rec(None, on_notice_ignore)
    class2._interp_rec(None, on_notice_ignore)
    class1._interp_rec(None, on_notice_ignore)
    type1 = TypeInfo(base_type=class1, kind=TypeKind.ALIAS)
    type1._interp_rec(None, on_notice_ignore)
    type1 = type1._to_resolved()
    type2 = TypeInfo(base_type=class2, kind=TypeKind.ALIAS)
    type2._interp_rec(None, on_notice_ignore)
    type2 = type2._to_resolved()

    # outer value
    value = Object.new({}, type1)
    value.field1 = choice1.fields.Option1
    value.field2 = False
    value.field3 = [Text.plain("hello bench!")]
    value.field4 = Object.new({}, type2)

    value_packed, secret_value_packed = pack_value(value, type1)
    unpacked_value = unpack_value(value_packed, secret_value_packed, type1)
    assert unpacked_value == value


# TODO :Test: auto generate :Test types & values
