import hashlib
import json
from typing import List, Optional
from uuid import UUID

from bench.model.base import get_static_config_keys
from bench.models import Flow, Model
from bench.models.function import FunctionVersion
from bench.models.model import ModelVersion
from bench.settings import MODEL_IID_HASH_LENGTH


def collect_functions(flow: Flow) -> List[FunctionVersion]:
    return [node.function for node in flow.nodes.all()]


def get_model_iid(model: ModelVersion) -> str:
    """
    Gets the model instance identifier used for distinguishing loaded models.
    If the iids match, the same ModelHandler instance may be used.
    """
    static_keys = get_static_config_keys(model.handler_id)
    static_config = {
        key: val for key, val in model.arguments.items() if key in static_keys
    }
    if model.storage_uri is not None:
        # Include storage uri in static config since it's not part of the config
        #  arguments given to the model but handled before loading it.
        static_config["storage_uri"] = model.storage_uri
    static_config_str = json.dumps(static_config, sort_keys=True)
    static_config_hash = hashlib.sha3_256(static_config_str.encode()).hexdigest()
    return format_model_iid(
        model_id=model.id,
        version=model.version,
        arguments_hash=static_config_hash[:MODEL_IID_HASH_LENGTH],
    )


def format_model_iid(
    model_id: UUID, version: Optional[str], arguments_hash: Optional[str]
):
    return f"{str(model_id)}-{version or 'current'}-{arguments_hash or '0'}"
