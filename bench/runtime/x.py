import enum
import inspect
import typing

from bench.language import XBlock
from bench.language.type import (
    EMPTY_FUNC_TYPE,
    Code,
    Dataset,
    LiteralValue,
    Model,
    Record,
    TypeNode,
    XBlockContent,
    XKind,
    XSource,
)
from bench.language.typer import check_type
from bench.utils.fractional import generate_n_keys_between


class GenerationErrorType(enum.Enum):
    INTERNAL = 0, "internal error"
    EXCEEDED_CONTEXT = 1, "ran out of tokens"
    INVALID_OUTPUT = 2, "invalid output"

    def __init__(self, code: int, message: str):
        self.code = code
        self.message = message


class GenerationError(ValueError):
    def __init__(
        self,
        _t: GenerationErrorType,
        message_detail: str | None = None,
        cause: Exception | None = None,
    ):
        super().__init__(_t.message)
        self.type = _t
        self.cause = cause
        self.message_detail = message_detail


ValueT = typing.TypeVar("ValueT", bound=typing.Any)


def xsettings(value: ValueT, source: XSource = XSource.System, path: str = None) -> XBlock[ValueT]:
    return XBlock(kind=XKind.Settings, source=source, value=value, path=path)


def xstatic(value: ValueT, source: XSource = XSource.Developer, path: str = None) -> XBlock[ValueT]:
    return XBlock(kind=XKind.Static, source=source, value=value, path=path)


def xinput(value: ValueT, source: XSource = XSource.User, path: str = None) -> XBlock[ValueT]:
    return XBlock(kind=XKind.Input, source=source, value=value, path=path)


def xoutput(value: ValueT, source: XSource = XSource.Model, path: str = None) -> XBlock[ValueT]:
    return XBlock(kind=XKind.Output, source=source, value=value, path=path)


def xcode(
    callable: typing.Callable,
    type: TypeNode = EMPTY_FUNC_TYPE,
    language: str = "python",
    xblocks: list[XBlock] = None,
    name: str = None,
) -> Code:
    """Creates a Code instance from the source and name of the given callable"""
    source = inspect.getsource(callable)
    name = name or callable.__name__
    return Code(
        code=source,
        name=name,
        type=type,
        type_node=type,
        description=None,
        language=language,
        xblocks=xblocks,
    )


class XBuilder:
    """Build a structured X prompt."""

    def __init__(self, name: str, type: TypeNode, model: Model):
        self.name = name
        self.type = type
        self.model = model
        self.xblocks: list[XBlock] = []
        self.input_modifiers: list[Code] = []
        self.output_parsers: list[Code] = []

    def use_handler(self, code: typing.Callable):
        if "input" in code.__name__:
            self.input_modifiers.append(xcode(code))
        elif "output" in code.__name__:
            self.output_parsers.append(xcode(code))
        else:
            raise ValueError(f"unknown code type: {code.__name__}")

    def append(self, xblock: XBlock):
        self.xblocks.append(xblock)

    def extend(self, xblocks: list[XBlock]):
        for xblock in xblocks:
            self.append(xblock)

    def to_symbol(self) -> Code:
        order_keys = generate_n_keys_between(None, None, len(self.xblocks))
        xblocks = [
            XBlockContent(order_key=ok, **x.__dict__) for ok, x in zip(order_keys, self.xblocks)
        ]

        input_modifiers_names = [x.name for x in self.input_modifiers]
        output_parsers_names = [x.name for x in self.output_parsers]

        def x():
            pass

        return xcode()


class DataBuilder:
    """Build a dataset."""

    def __init__(self, name: str, type_node: TypeNode):
        self.name = name
        self.type_node = type_node
        self.records: list[LiteralValue] = []

    def append(self, record: LiteralValue):
        check_type(record, self.type_node)
        self.records.append(record)

    def extend(self, records: list[LiteralValue], ignore_type_errors: bool):
        # ignore_type_errors is a stopgap since records should already be checked here
        for record in records:
            try:
                self.append(record)
            except TypeError as e:
                if not ignore_type_errors:
                    raise e

    def to_symbol(self) -> Dataset:
        order_keys = generate_n_keys_between(None, None, len(self.records))
        records = [Record(order_key=ok, data=d) for ok, d in zip(order_keys, self.records)]
        dataset = Dataset(
            name=self.name,
            type_node=self.type_node,
            type=self.type_node.to_type(),
            records=records,
            description=None,
            language="jsonl",
        )
        # type check records
        for record in dataset.records:
            check_type(record, dataset.type)
        return dataset
