from bench.language.const import SortOp
from bench.language.expression import Expression, S, apply_sort, evaluate_conditional
from bench.language.field import Field
from bench.language.session import Session

# NOTE :Test: generate expressions to test with hypothesis


def test_evaluate_conditional(session: Session):
    NAME = Field._prop("name")
    ORDER_KEY = Field._prop("order_key")

    field_0 = Field.new("apple", bool, order_key="a0")
    field_1 = Field.new("banana", bool, order_key="a1")
    field_2 = Field.new("applepie", bool, order_key="a2")
    field_3 = Field.new("banoffee", bool, order_key="a2")
    field_4 = Field.new("edelweiss", bool, order_key="a3")
    fields = [field_0, field_1, field_2, field_3, field_4]

    def _get_matches(cond: Expression):
        return [f for f in fields if evaluate_conditional(cond, f)]

    assert _get_matches(NAME.equals("apple")) == [field_0]
    assert _get_matches(NAME.ends_with("e")) == [field_0, field_2, field_3]
    assert _get_matches(NAME.matches_regex(".*e.*") & (ORDER_KEY > "a1")) == [
        field_2,
        field_3,
        field_4,
    ]


def test_apply_sort(session: Session):
    field_0 = Field.new("b", bool, order_key="a0")
    field_1 = Field.new("a", bool, order_key="a1")
    field_2 = Field.new("c", bool, order_key="a2")
    field_3 = Field.new("d", bool, order_key="a2")
    field_4 = Field.new("d", bool, order_key="a3")

    # 1 sort, asc
    assert apply_sort(
        (S(SortOp.ASCENDING, property=Field.order_key),),
        [field_0, field_1, field_2, field_3, field_4],
    ) == [field_0, field_1, field_2, field_3, field_4]
    # 1 sort, desc
    # (field_2/field_3 share 'a2' so they'll remain in the given order)
    assert apply_sort(
        (S(SortOp.DESCENDING, property=Field.order_key),),
        [field_0, field_1, field_2, field_3, field_4],
    ) == [field_4, field_2, field_3, field_1, field_0]

    # 2 sorts
    assert apply_sort(
        (S(SortOp.ASCENDING, property=Field.order_key), S(SortOp.ASCENDING, property=Field.name)),
        [field_0, field_1, field_2, field_3, field_4],
    ) == [field_0, field_1, field_2, field_3, field_4]
    assert apply_sort(
        (S(SortOp.ASCENDING, property=Field.order_key), S(SortOp.DESCENDING, property=Field.name)),
        [field_0, field_1, field_2, field_3, field_4],
    ) == [field_0, field_1, field_3, field_2, field_4]
