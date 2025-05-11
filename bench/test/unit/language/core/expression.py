from bench.language import (
    Action,
    ActionType,
    Choice,
    Expression,
    Field,
    FieldType,
    Flow,
    Option,
    Package,
    Run,
    S,
    Session,
    SortType,
    Table,
    apply_sort,
    code,
    evaluate_conditional,
    text_line,
)
from bench.runtime.core import create_run

# NOTE :Test: generate expressions to test with hypothesis


def test_evaluate_conditional_property_stringy(session: Session):
    NAME = Field.get_property("name")
    ORDER_KEY = Field.get_property("order_key")

    field_0 = Field.new("apple", FieldType.MEMBER, bool, order_key="a0")
    field_1 = Field.new("banana", FieldType.MEMBER, bool, order_key="a1")
    field_2 = Field.new("applepie", FieldType.MEMBER, bool, order_key="a2")
    field_3 = Field.new("banoffee", FieldType.MEMBER, bool, order_key="a2")
    field_4 = Field.new("edelweiss", FieldType.MEMBER, bool, order_key="a3")
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
    Page1 = package.pages.create(title=text_line("Page1"))
    Flow1 = Flow.new("Flow1")
    Page1.append(Flow1)
    Code1 = Action.new(ActionType.CODE, "Code1", code=code("pass"))
    Flow1.append(Code1)
    run, _ = create_run(Code1, parent=package)

    cond = Run.get_property("action").is_equal(Code1)
    assert evaluate_conditional(cond, run) is True


def test_evaluate_sort_property_stringy(session: Session):
    field_0 = Field.new("b", FieldType.MEMBER, bool, order_key="a0")
    field_1 = Field.new("a", FieldType.MEMBER, bool, order_key="a1")
    field_2 = Field.new("c", FieldType.MEMBER, bool, order_key="a2")
    field_3 = Field.new("d", FieldType.MEMBER, bool, order_key="a2")
    field_4 = Field.new("d", FieldType.MEMBER, bool, order_key="a3")

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
    Choice1 = Choice.new(
        "Choice1",
        Option.new("Option1"),
        Option.new("Option2"),
        Option.new("Option3"),
        Option.new("Option4"),
    )
    Table1 = Table.new(
        "Table1",
        Field.member("Rating", int),
        Field.member("Name", str),
        Field.member("Choice", Choice1),
    )
    Record1 = Table1.records.create(Rating=1, Name="Alice", Choice=Choice1.options.Option1)
    Record2 = Table1.records.create(Rating=2, Name="Bob", Choice=Choice1.options.Option2)
    Record3 = Table1.records.create(Rating=3, Name="Charlie", Choice=Choice1.options.Option3)

    # basic number
    cond = Table1.fields.Rating.is_equal(1)
    assert evaluate_conditional(cond, Record1) is True
    assert evaluate_conditional(cond, Record2) is False
    assert evaluate_conditional(cond, Record3) is False

    # basic string
    cond = Table1.fields.Name.matches_regex(".*ob.*")
    assert evaluate_conditional(cond, Record1) is False
    assert evaluate_conditional(cond, Record2) is True
    assert evaluate_conditional(cond, Record3) is False

    # basic node
    cond = Table1.fields.Choice.is_equal(Choice1.options.Option1)
    assert evaluate_conditional(cond, Record1) is True
    assert evaluate_conditional(cond, Record2) is False
    assert evaluate_conditional(cond, Record3) is False

    # compound
    cond = Table1.fields.Rating.is_equal(1) & Table1.fields.Name.matches_regex(".*ob.*")
    assert evaluate_conditional(cond, Record1) is False
    assert evaluate_conditional(cond, Record2) is False
    assert evaluate_conditional(cond, Record3) is False
    cond = Table1.fields.Rating.gte(2) | Table1.fields.Name.matches_regex(".*ob.*")
    assert evaluate_conditional(cond, Record1) is False
    assert evaluate_conditional(cond, Record2) is True
    assert evaluate_conditional(cond, Record3) is True
