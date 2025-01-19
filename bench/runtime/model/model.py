from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, ClassVar, Sequence

from bench.language import (
    CustomObject,
    HasContext,
    ModelType,
    Run,
    RunnableNode,
    RunOptions,
    RunType,
    TypeBase,
)
from bench.runtime.core import Runner, Runtime

from .prompt import Prompt, PromptElement

if TYPE_CHECKING:
    pass


class ModelRunner[R: RunnableNode = RunnableNode](Runner[R], ABC):
    """
    Run a Model that takes Prompts and returns some Model-specific output (maybe streaming).
    """

    runner_type: ClassVar[RunType] = RunType.ACTION

    def __init__(
        self,
        *,
        runtime: Runtime,
        node: R,
        model_type: ModelType,
        track: bool,
        options: RunOptions,
        context: HasContext,
        parent: Runner | None = None,
        inputs: CustomObject | None = None,
        variables: CustomObject | None = None,
        output_type: TypeBase | None = None,
        run: Run | None = None,
    ) -> None:
        super().__init__(
            runtime=runtime,
            node=node,
            track=track,
            options=options,
            context=context,
            parent=parent,
            inputs=inputs,
            variables=variables,
            output_type=output_type,
            run=run,
        )
        self.model_type = model_type

    @abstractmethod
    async def build(self, prompt: Prompt, budget: float) -> Sequence[PromptElement]:
        """Compile the Prompt into a list of basic prompt parts."""
        ...
