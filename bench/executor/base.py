import abc
from typing import Dict, Optional, Union

from bench.executor.utils import collect_functions
from bench.models import Function, Model
from bench.models.flow import Flow
from bench.utils.record import Record, RecordBatch

Resource = str
ResourceRequirements = Dict[Resource, Union[int, float]]


class Executor(abc.ABC):
    """
    Base executor for hosting and routing between consumers, functions and models.
    """

    async def prepare_model(
        self,
        model: Model,
        version: Optional[str] = None,
        requirements: Optional[ResourceRequirements] = None,
    ):
        """
        Make model available in this executor with the given requirements.
        """
        raise NotImplementedError

    async def prepare_function(
        self, function: Function, requirements: Optional[ResourceRequirements]
    ):
        """
        Make arguments to the function available in this executor with the given requirements.
        """
        raise NotImplementedError

    async def prepare_flow(self, flow: Flow, requirements: ResourceRequirements):
        """
        Make arguments to the flow available in this executor with the given requirements.
        """
        raise NotImplementedError

    async def run_model(
        self,
        model: Model,
        version: Optional[str],
        record: Union[Record, RecordBatch],
        prepare_if_needed: bool = False,
    ) -> Union[Record, RecordBatch]:
        raise NotImplementedError

    async def run_function(
        self, function: Function, record: Union[Record, RecordBatch]
    ) -> Union[Record, RecordBatch]:
        raise NotImplementedError

    async def run_flow(
        self, flow: Flow, record: Union[Record, RecordBatch]
    ) -> Union[Record, RecordBatch]:
        raise NotImplementedError


class UncoordinatedExecutor(Executor):
    """
    An executor that does not coordinate allocation of models/datasets to workers.
    """

    async def prepare_function(
        self, function: Function, requirements: Optional[ResourceRequirements]
    ):
        for model in function.models:
            await self.prepare_model(model)

    async def prepare_flow(
        self, flow: Flow, requirements: Optional[ResourceRequirements]
    ):
        flow_functions = collect_functions(flow)
        for function in flow_functions:
            await self.prepare_function(function, requirements)
