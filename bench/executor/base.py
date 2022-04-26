import abc
from typing import Dict, Optional, Union

from bench.executor.utils import collect_functions
from bench.models import ArtifactVersion, Function, Model
from bench.models.flow import Flow
from bench.models.function import FunctionVersion
from bench.models.model import ModelVersion
from bench.utils.record import Record, RecordBatch

Resource = str
ResourceRequirements = Dict[Resource, Union[int, float]]


class Executor(abc.ABC):
    """
    Base executor for hosting and routing between consumers, functions and models.
    """

    async def load_model(
        self,
        model: ModelVersion,
        requirements: Optional[ResourceRequirements] = None,
    ):
        """
        Make the model available in this executor with the given resources.
        """
        raise NotImplementedError

    async def load_function(
        self, function: FunctionVersion, requirements: Optional[ResourceRequirements]
    ):
        """
        Make the function available in this executor with the given resources.
        """
        raise NotImplementedError

    async def load_flow(self, flow: Flow, requirements: ResourceRequirements):
        """
        Make the flow available in this executor with the given resources.
        """
        raise NotImplementedError

    async def run_model(
        self,
        model: ModelVersion,
        record: Union[Record, RecordBatch],
        load_if_needed: bool = False,
    ) -> Union[Record, RecordBatch]:
        raise NotImplementedError

    async def run_function(
        self,
        function: FunctionVersion,
        inputs: Union[Record, RecordBatch, ArtifactVersion],
    ) -> Union[Record, RecordBatch]:
        raise NotImplementedError

    async def run_flow(
        self, flow: Flow, inputs: Union[None, Record, RecordBatch, ArtifactVersion]
    ) -> Union[Record, RecordBatch]:
        raise NotImplementedError


class SimpleExecutor(Executor, abc.ABC):
    """
    An executor that does not coordinate allocation of artifacts to workers.
    """

    async def load_function(
        self, function: FunctionVersion, requirements: Optional[ResourceRequirements]
    ):
        for model in function.models:
            await self.load_model(model)

    async def load_flow(self, flow: Flow, requirements: Optional[ResourceRequirements]):
        flow_functions = collect_functions(flow)
        for function in flow_functions:
            await self.load_function(function, requirements)
