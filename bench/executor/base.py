import abc
from typing import Dict, Union

from bench.models import Function, Model
from bench.models.flow import Flow
from bench.utils.record import Record, RecordBatch

Resource = str
ResourceRequirements = Dict[Resource, Union[int, float]]


class Executor(abc.ABC):
    """
    Base executor for hosting and routing between consumers, functions and models.
    """

    async def prepare_model(self, model: Model, requirements: ResourceRequirements):
        """
        Make model available in this executor with the given requirements.
        """
        raise NotImplementedError

    async def prepare_function(
        self, function: Function, requirements: ResourceRequirements
    ) -> Union[Record, RecordBatch]:
        """
        Make arguments to the function available in this executor with the given requirements.
        """
        pass

    async def prepare_flow(
        self, flow: Flow, requirements: ResourceRequirements
    ) -> Union[Record, RecordBatch]:
        """
        Make arguments to the flow available in this executor with the given requirements.
        """
        pass

    async def submit_dag(
        self, record: Union[Record, RecordBatch]
    ) -> Union[Record, RecordBatch]:
        pass

    async def submit_model(
        self, record: Union[Record, RecordBatch]
    ) -> Union[Record, RecordBatch]:
        pass
