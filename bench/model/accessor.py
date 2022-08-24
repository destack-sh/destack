from bench.model.base import load_model
from bench.models import ModelVersion


def get_model_version_handler(model: ModelVersion):
    return load_model(
        handler_id=model.handler_id,
        version=model.version,
        storage_uri=model.storage_uri,
        arguments=model.config_arguments,
        spec=model.spec,
    )
