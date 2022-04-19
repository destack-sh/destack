from typing import Dict, Optional, Union, cast

import structlog

from bench.executor.base import ResourceRequirements, UncoordinatedExecutor
from bench.executor.utils import get_versioned_model_id
from bench.model.base import ModelHandler, load_model
from bench.models import Model
from bench.utils.record import Record, RecordBatch, is_record

logger = structlog.stdlib.get_logger()


class LocalExecutor(UncoordinatedExecutor):
    """
    A locally executed implementation of Executor without coordination or parallelism.
    """

    def __init__(self):
        self._loaded_models: Dict[str, ModelHandler] = {}

    async def _get_prepared_model(
        self, model: Model, version: Optional[str], prepare_if_needed: bool
    ) -> ModelHandler:
        model_vid = get_versioned_model_id(model_id=model.id, version=version)
        if model_vid not in self._loaded_models:
            if not prepare_if_needed:
                raise RuntimeError("model " + model_vid + " is not prepared")
            else:
                await self.prepare_model(model, version)
        return self._loaded_models[model_vid]

    async def prepare_model(
        self,
        model: Model,
        version: Optional[str] = None,
        requirements: Optional[ResourceRequirements] = None,
    ):
        model_vid = get_versioned_model_id(model_id=model.id, version=version)
        if model_vid in self._loaded_models:
            return

        log = logger.bind(model_id=model.id, version=version, requirements=requirements)
        log.info("executor.model.load")
        model_handler: ModelHandler = load_model(
            model.handler_id,
            storage_uri=model.storage_uri,
            arguments=model.arguments,
            version=version,
            spec=model.spec,
        )
        self._loaded_models[model_vid] = model_handler
        log.info("executor.model.loaded")

    async def run_model(
        self,
        model: Model,
        version: Optional[str],
        record: Union[Record, RecordBatch],
        prepare_if_needed: bool = False,
    ) -> Union[Record, RecordBatch]:
        model_handler = await self._get_prepared_model(
            model, version, prepare_if_needed
        )
        if is_record(record):
            return model_handler.predict(cast(Record, record))
        else:
            return model_handler.predict_batch(cast(RecordBatch, record))
