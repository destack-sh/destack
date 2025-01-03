from bench.language import (
    Action,
    ActionType,
    Block,
    BlockType,
    Expression,
    Field,
    Package,
    Run,
    S,
    Session,
    SortType,
    apply_sort,
    code,
    evaluate_conditional,
)
from bench.runtime.runner import make_run_from_node

# NOTE :Test: generate expressions to test with hypothesis


def test_evaluate_conditional_property_stringy(session: Session):
    NAME = Field.get_property("name")
    ORDER_KEY = Field.get_property("order_key")

    field_0 = Field.new("apple", bool, order_key="a0")
    field_1 = Field.new("banana", bool, order_key="a1")
    field_2 = Field.new("applepie", bool, order_key="a2")
    field_3 = Field.new("banoffee", bool, order_key="a2")
    field_4 = Field.new("edelweiss", bool, order_key="a3")
    fields = [field_0, field_1, field_2, field_3, field_4]

    def _get_matches(cond: Expression):
        return [f for f in fields if evaluate_conditional(cond, f)]

    assert _get_matches(NAME.is_equal("apple")) == [field_0]
    assert _get_matches(NAME.ends_with("e")) == [field_0, field_2, field_3]
    assert _get_matches(NAME.matches_regex(".*e.*") & (ORDER_KEY > "a1")) == [
        field_2,
        field_3,
        field_4,
    ]


def test_evaluate_conditional_property_node(session: Session, package: Package):
    Code1 = Action.new(ActionType.CODE, "Code1", code=code("pass"))
    page = package.blocks.append(Block.new(BlockType.PAGE, "Page"))
    page.actions.append(Code1)
    Run1 = make_run_from_node(Code1)

    cond = Run.get_property("block").is_equal(page)
    assert evaluate_conditional(cond, Run1) is True


def test_evaluate_sort_property_stringy(session: Session):
    field_0 = Field.new("b", bool, order_key="a0")
    field_1 = Field.new("a", bool, order_key="a1")
    field_2 = Field.new("c", bool, order_key="a2")
    field_3 = Field.new("d", bool, order_key="a2")
    field_4 = Field.new("d", bool, order_key="a3")

    # 1 sort, asc
    assert apply_sort(
        (S(SortType.ASCENDING, property=Field.order_key),),
        [field_0, field_1, field_2, field_3, field_4],
    ) == [field_0, field_1, field_2, field_3, field_4]
    # 1 sort, desc
    # (field_2/field_3 share 'a2' so they'll remain in the given order)
    assert apply_sort(
        (S(SortType.DESCENDING, property=Field.order_key),),
        [field_0, field_1, field_2, field_3, field_4],
    ) == [field_4, field_2, field_3, field_1, field_0]

    # 2 sorts
    assert apply_sort(
        (
            S(SortType.ASCENDING, property=Field.order_key),
            S(SortType.ASCENDING, property=Field.name),
        ),
        [field_0, field_1, field_2, field_3, field_4],
    ) == [field_0, field_1, field_2, field_3, field_4]
    assert apply_sort(
        (
            S(SortType.ASCENDING, property=Field.order_key),
            S(SortType.DESCENDING, property=Field.name),
        ),
        [field_0, field_1, field_2, field_3, field_4],
    ) == [field_0, field_1, field_3, field_2, field_4]


def test_evaluate_conditional_field(session: Session, package: Package):
    Choice1 = Block.new(
        BlockType.CHOICE,
        "Choice1",
        fields=(
            Field.option("Option1"),
            Field.option("Option2"),
            Field.option("Option3"),
            Field.option("Option4"),
        ),
    )
    Database1 = Block.new(
        BlockType.DATABASE,
        "Database1",
        fields=(
            Field.member("Rating", int),
            Field.member("Name", str),
            Field.member("Choice", Choice1),
        ),
    )
    Record1 = Database1.records.create(Rating=1, Name="Alice", Choice=Choice1.fields.Option1)
    Record2 = Database1.records.create(Rating=2, Name="Bob", Choice=Choice1.fields.Option2)
    Record3 = Database1.records.create(Rating=3, Name="Charlie", Choice=Choice1.fields.Option3)

    # basic number
    cond = Database1.fields.Rating.is_equal(1)
    assert evaluate_conditional(cond, Record1) is True
    assert evaluate_conditional(cond, Record2) is False
    assert evaluate_conditional(cond, Record3) is False

    # basic string
    cond = Database1.fields.Name.matches_regex(".*ob.*")
    assert evaluate_conditional(cond, Record1) is False
    assert evaluate_conditional(cond, Record2) is True
    assert evaluate_conditional(cond, Record3) is False

    # basic node
    cond = Database1.fields.Choice.is_equal(Choice1.fields.Option1)
    assert evaluate_conditional(cond, Record1) is True
    assert evaluate_conditional(cond, Record2) is False
    assert evaluate_conditional(cond, Record3) is False

    # compound
    cond = Database1.fields.Rating.is_equal(1) & Database1.fields.Name.matches_regex(".*ob.*")
    assert evaluate_conditional(cond, Record1) is False
    assert evaluate_conditional(cond, Record2) is False
    assert evaluate_conditional(cond, Record3) is False
    cond = Database1.fields.Rating.gte(2) | Database1.fields.Name.matches_regex(".*ob.*")
    assert evaluate_conditional(cond, Record1) is False
    assert evaluate_conditional(cond, Record2) is True
    assert evaluate_conditional(cond, Record3) is True
