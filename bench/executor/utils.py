from typing import List, Optional
from uuid import UUID

from bench.models import Flow, Function


def collect_functions(flow: Flow) -> List[Function]:
    return [node.function for node in flow.nodes]


def get_versioned_model_id(model_id: UUID, version: Optional[str]) -> str:
    return f"{model_id.hex}-{version or 'current'}"
