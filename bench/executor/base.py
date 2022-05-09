import abc
from typing import Dict, Mapping, Optional, Union

from bench.models import ArtifactVersion
from bench.models.flow import FlowNode, FlowVersion
from bench.models.model import ModelVersion
from bench.models.utils import UUIDT
from bench.utils.record import Record, RecordBatch

Resource = str
ResourceRequirements = Dict[Resource, Union[int, float]]
PerNodeResourceRequirements = Dict[UUIDT, ResourceRequirements]


class Executor(abc.ABC):
    """
    Base executor for orchestrating, routing and executing resources.
    """

    async def load_artifact(
        self,
        model: ArtifactVersion,
        requirements: Optional[ResourceRequirements] = None,
    ):
        """
        Make the model available in this executor with the given resources.
        """
        raise NotImplementedError

    async def load_flow_node(
        self, flow_node: FlowNode, requirements: ResourceRequirements
    ):
        """
        Make the flow available in this executor with the given resources.
        """
        raise NotImplementedError

    async def load_flow(
        self, flow: FlowVersion, requirements: PerNodeResourceRequirements
    ):
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

    async def run_flow_node(
        self,
        node: FlowNode,
        inputs: Union[Record, RecordBatch, Mapping[str, ArtifactVersion]],
    ) -> Union[Record, RecordBatch, Mapping[str, ArtifactVersion]]:
        raise NotImplementedError

    async def run_flow(
        self,
        flow: FlowVersion,
        inputs: Union[None, Record, RecordBatch, Mapping[str, ArtifactVersion]],
    ) -> Union[Record, RecordBatch, Mapping[str, ArtifactVersion]]:
        raise NotImplementedError


class SimpleExecutor(Executor, abc.ABC):
    """
    An executor that does not coordinate allocation of artifacts to workers.
    """

    pass
