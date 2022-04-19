from typing import Dict, Optional, Union, cast

import structlog

from bench.executor.base import ResourceRequirements, UncoordinatedExecutor
from bench.executor.utils import get_model_iid
from bench.model.base import ModelHandler, load_model
from bench.models import Model
from bench.utils.record import Record, RecordBatch, is_record

logger = structlog.stdlib.get_logger()


class LocalExecutor(UncoordinatedExecutor):
    """
    A locally executed implementation of Executor without coordination or parallelism.
    """

    def __init__(self):
        self._loaded_models_by_iid: Dict[str, ModelHandler] = {}

    async def _get_prepared_model(
        self, model: Model, version: Optional[str], prepare_if_needed: bool
    ) -> ModelHandler:
        model_iid = get_model_iid(model, version)
        if model_iid not in self._loaded_models_by_iid:
            if not prepare_if_needed:
                raise RuntimeError("model " + model_iid + " is not prepared")
            else:
                await self.prepare_model(model, version)
        return self._loaded_models_by_iid[model_iid]

    async def prepare_model(
        self,
        model: Model,
        version: Optional[str] = None,
        requirements: Optional[ResourceRequirements] = None,
    ):
        model_iid = get_model_iid(model=model, version=version)
        if model_iid in self._loaded_models_by_iid:
            return

        log = logger.bind(
            model_id=model.id,
            model_iid=model_iid,
            arguments=model.arguments,
            version=version,
            requirements=requirements,
        )
        log.info("model_load")
        model_handler: ModelHandler = load_model(
            model.handler_id,
            storage_uri=model.storage_uri,
            arguments=model.arguments,
            version=version,
            spec=model.spec,
        )
        self._loaded_models_by_iid[model_iid] = model_handler
        log.info("model_loaded")

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
