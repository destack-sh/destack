import enum
import typing
from dataclasses import asdict, dataclass, is_dataclass

from bench.language import XBlock
from bench.language.type import (
    Data,
    LiteralValue,
    Model,
    Record,
    Type,
    TypeNode,
    TypeTag,
    XBlockContent,
    XKind,
    XSource,
)
from bench.language.typer import check_type
from bench.runtime.inference import (
    EmbeddingSettings,
    ImageGenerationSettings,
    IncapableError,
    Modality,
    TextGenerationSettings,
)
from bench.runtime.instance import (
    AsyncCodeInstance,
    CodeTransformation,
    ModelInstance,
    Session,
    TaskInstance,
)
from bench.utils.fractional import generate_n_keys_between
from bench.utils.utils import get_method_source


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
    if is_dataclass(value):
        value = asdict(value)
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


X_BUILTINS = {
    "xinput": xinput,
    "xoutput": xoutput,
    "xstatic": xstatic,
    "xsettings": xsettings,
    "XKind": XKind,
    "XSource": XSource,
    "XBlock": XBlock,
    "Modality": Modality,
    "TextGenerationSettings": TextGenerationSettings,
    "ImageGenerationSettings": ImageGenerationSettings,
    "EmbeddingSettings": EmbeddingSettings,
}

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

    def build(
        self, task: TaskInstance, model: ModelInstance, session: Session
    ) -> AsyncCodeInstance:
        async def _invoke(*args, **kwargs) -> dict[str, LiteralValue]:
            combined_kwargs = {**kwargs}
            for input_t, input in zip(self.type.inputs, args):
                combined_kwargs[input_t.name] = input
            raise IncapableError("not yet implemented")  # nocheckin

        _invoke.__name__ = self.name
        return AsyncCodeInstance(
            id=task.id,
            task=task,
            name=self.name,
            type=self.type,
            type_nodes=self.type.type_nodes,
            session=session,
            tracer=session.tracer,
            code_callable=_invoke,
            transform=CodeTransformation(
                method_name=_invoke.__name__,
                original_code=get_method_source(_invoke),
                transformed_code=get_method_source(_invoke),
                start_offset=0,
            ),
            tag=TypeTag.FUNCTION,
        )


class DataBuilder:
    """Build a dataset."""

    def __init__(self, name: str, type_node: Type):
        self.name = name
        self.type = type_node
        self.records: list[LiteralValue] = []

    def append(self, record: LiteralValue):
        check_type(record, self.type)
        self.records.append(record)

    def extend(self, records: list[LiteralValue], ignore_type_errors: bool):
        # ignore_type_errors is a stopgap since records should already be checked here
        for record in records:
            try:
                self.append(record)
            except TypeError as e:
                if not ignore_type_errors:
                    raise e

    def to_symbol(self) -> Data:
        order_keys = generate_n_keys_between(None, None, len(self.records))
        records = [Record(order_key=ok, data=d) for ok, d in zip(order_keys, self.records)]
        dataset = Data(
            name=self.name,
            type_node=self.type,
            type=self.type.deepcopy(keep_id=False, keep_reference=True),
            records=records,
            description=None,
            language="jsonl",
        )
        # type check records
        for record in dataset.records:
            check_type(record, dataset.type)
        return dataset
