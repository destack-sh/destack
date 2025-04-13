from abc import ABC
from typing import TYPE_CHECKING, ClassVar

from bench.language import Agent, CustomObject, IsType, ModelType, Runnable, RunType
from bench.runtime.core import RunIn, Runner, Runtime

if TYPE_CHECKING:
    pass


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
        model_type: ModelType,
        run: RunIn,
        parent: Runner | None = None,
        inputs: CustomObject | None = None,
        outputs: IsType | CustomObject | None = None,
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
        self.model_type = model_type
