import abc
from typing import Dict, Mapping, Optional, Tuple, Union
from uuid import UUID

from bench.models import ArtifactVersion, FlowExecution, ModelExecution
from bench.models.flow import FlowNode, FlowVersion
from bench.models.model import ModelVersion
from bench.models.utils import UUIDT
from bench.utils.record import Record, RecordBatch

Resource = str
ResourceRequirements = Dict[Resource, Union[int, float]]
PerNodeResourceRequirements = Dict[UUIDT, ResourceRequirements]

FlowInput = Union[RecordBatch, ArtifactVersion]
FlowArgument = Union[FlowNode, RecordBatch, ArtifactVersion]
FlowOutput = Union[ArtifactVersion]


class Executor(abc.ABC):
    """
    Base executor for orchestrating, routing and executing resources.
    """

    def load_artifact(
        self,
        model: ArtifactVersion,
        requirements: Optional[ResourceRequirements] = None,
    ):
        """
        Make the artifact available in this executor with the given resources.
        """
        raise NotImplementedError

    def load_flow(self, flow: FlowVersion, requirements: PerNodeResourceRequirements):
        """
        Prepare the flow in this executor with the given resources.
        """
        raise NotImplementedError

    def run_model(
        self,
        model: ModelVersion,
        record: Union[Record, RecordBatch],
        blocking: bool = True,
        load_if_needed: bool = False,
    ) -> Tuple[ModelExecution, Union[None, Record, RecordBatch]]:
        """
        Runs the given model on the given records, loading it first if needed.
        The returned execution object encapsulates this run and can be used to retrieve results.

        If run in non-blocking mode, output cannot be returned and is None.
        """
        raise NotImplementedError

    def run_flow(
        self,
        flow: FlowVersion,
        inputs: Mapping[UUID, Mapping[str, FlowInput]],
        arguments: Mapping[UUID, Mapping[str, FlowArgument]],
    ) -> Tuple[FlowExecution, Mapping[UUID, Mapping[str, ArtifactVersion]]]:
        """
        Runs the given flow with the provided inputs and arguments to each node.
        This method is non-blocking and the returned execution object can be used to retrieve results.
        """
        raise NotImplementedError
