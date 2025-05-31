from abc import ABC
from datetime import date
from typing import TYPE_CHECKING, ClassVar, NamedTuple, Sequence

from bench.language import (
    Agent,
    CustomObject,
    FileType,
    ModelDeveloper,
    ModelProvider,
    Runnable,
    RunType,
    TypeBase,
)
from bench.runtime.core import RunIn, Runner, Runtime

if TYPE_CHECKING:
    from bench.runtime.model import ChatModelRunner


class ModelSettings(NamedTuple):
    model_cls: type["ChatModelRunner"]
    model_developer: ModelDeveloper
    model_provider: ModelProvider
    model_id: str
    model_name: str
    supported_file_types: Sequence[FileType]
    knowledge_cutoff: date


class ModelRunner[R: Runnable = Runnable](Runner[R], ABC):
    """
    Run a Model that takes Prompts and returns some Model-specific output (maybe streaming).
    """

    runner_type: ClassVar[RunType] = RunType.ACTION

    def __init__(
        self,
        *,
        runtime: Runtime,
        node: R,
        run: RunIn,
        model_id: str,
        parent: Runner | None = None,
        inputs: CustomObject | None = None,
        outputs: TypeBase | CustomObject | None = None,
        agent: "Agent | None" = None,
    ) -> None:
        super().__init__(
            runtime=runtime,
            node=node,
            parent=parent,
            inputs=inputs,
            outputs=outputs,
            run=run,
            agent=agent,
        )
        self.model_id = model_id
