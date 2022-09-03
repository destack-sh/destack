from typing import Type, TypeVar

from bench.artifact.base import ArtifactHandler
from bench.utils.spec import convert_to_config_type_spec, infer_config_type

ArtifactT = TypeVar("ArtifactT", bound=ArtifactHandler)


def map_to_artifact_cls(artifact_cls: Type[ArtifactT], ignore_keys: set[str]) -> Type[ArtifactT]:
    if hasattr(artifact_cls, "config_spec"):
        declared_config_spec = convert_to_config_type_spec(artifact_cls.config_spec)
    else:
        declared_config_spec = None
    # TODO @Robustness: check declared_config_spec against inferred_config_spec
    inferred_config_spec = infer_config_type(artifact_cls.__init__)  # noqa

    # overwrite config spec with clean config
    config_spec = declared_config_spec or inferred_config_spec
    # filter to exclude ignored keys
    config_spec = {key: typ for key, typ in config_spec.items() if key not in ignore_keys}
    artifact_cls.config_spec = config_spec
    return artifact_cls
