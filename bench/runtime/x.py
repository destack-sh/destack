import enum
import inspect
import textwrap
import typing
from dataclasses import dataclass

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
from bench.runtime.type import Modality
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


def xsettings(
    value: ValueT, source: XSource = XSource.System, path: str = None
) -> XBlockContent[ValueT]:
    return XBlockContent(kind=XKind.Settings, source=source, value=value, path=path)


def xstatic(
    value: ValueT, source: XSource = XSource.Developer, path: str = None
) -> XBlockContent[ValueT]:
    return XBlockContent(kind=XKind.Static, source=source, value=value, path=path)


def xinput(
    value: ValueT, source: XSource = XSource.User, path: str = None
) -> XBlockContent[ValueT]:
    return XBlockContent(kind=XKind.Input, source=source, value=value, path=path)


def xoutput(
    value: ValueT, source: XSource = XSource.Model, path: str = None
) -> XBlockContent[ValueT]:
    return XBlockContent(kind=XKind.Output, source=source, value=value, path=path)


def xcode(
    source: typing.Callable | str,
    type: TypeNode = EMPTY_FUNC_TYPE,
    language: str = "python",
    xblocks: list[XBlock] = None,
    name: str = None,
) -> Code:
    """Creates a Code instance from the source and name of the given callable"""
    source = inspect.getsource(source) if inspect.isfunction(source) else source
    name = name or source.__name__
    return Code(
        code=source,
        name=name,
        type=type,
        type_node=type,
        description=None,
        language=language,
        xblocks=xblocks,
    )


XInputHandler = typing.Callable[[XBlock, typing.Any], None]
XOutputHandler = typing.Callable[[XBlock], typing.Any]


@dataclass
class DynamicXBlock:
    xblock: XBlockContent
    handler: XInputHandler | XOutputHandler


class XBuilder:
    """Build a structured X prompt."""

    def __init__(self, name: str, type: TypeNode, model: Model, modality: Modality):
        self.name = name
        self.type = type
        self.model = model
        self.modality = modality
        self.xblocks: list[XBlockContent] = []
        self.dynamic_xblocks: list[DynamicXBlock] = []

    def append(self, xblock: XBlockContent | DynamicXBlock):
        if isinstance(xblock, DynamicXBlock):
            self.dynamic_xblocks.append(xblock)
            self.xblocks.append(xblock.xblock)
        else:
            self.xblocks.append(xblock)

    def extend(self, xblocks: list[XBlockContent | DynamicXBlock]):
        for xblock in xblocks:
            self.append(xblock)

    def to_symbol(self) -> Code:
        order_keys = generate_n_keys_between(None, None, len(self.xblocks))
        # assign order keys
        for xblock, order_key in zip(self.xblocks, order_keys):
            xblock.order_key = order_key

        model_call = (
            f"input_blocks = [xblock for xblock in self.xblocks if xblock.kind == XKind.Input]\n"
            f"settings = last([xblock.value for xblock in self.xblocks if xblock.kind == XKind.Settings])\n"
            f"model_output = await model.{self.modality}(input_blocks, settings)"
        )

        # inline handler methods
        input_handler_defs: list[str] = []
        input_handler_calls: list[str] = []
        output_handler_defs: list[str] = []
        output_handler_calls: list[str] = []
        for i, block in enumerate(self.dynamic_xblocks):
            if block.xblock.kind == XKind.Input:
                handler_def = f"def input_handler_{i}(xblock, value):" + textwrap.indent(
                    inspect.getsource(block.handler), "    "
                )
                input_handler_defs.append(handler_def)
                handler_call = f"value = " f"input_handler_{i}(self.xblocks[{i}], value)"
                input_handler_calls.append(handler_call)
            elif block.xblock.kind == XKind.Output:
                handler_def = f"def output_handler_{i}(xblock):" + textwrap.indent(
                    inspect.getsource(block.handler), "    "
                )
                output_handler_defs.append(handler_def)
                handler_call = f"output_handler_{i}(self.xblocks[{i}])"

        x_source = (
            # context
            f"model = context[{self.model.name}]",
            *input_handler_defs,
            *output_handler_defs,
            # run input handlers
            *input_handler_calls,
            # run model
            *model_call,
            # run output handlers
            *output_handler_calls,  # TODO @Broken: return output handler value(s?)
        )
        return xcode("\n".join(x_source), name=self.name, type=self.type, xblocks=self.xblocks)


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
