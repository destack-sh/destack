from typing import Dict, Optional, Union, cast

import structlog

from bench.executor.base import ResourceRequirements, SimpleExecutor
from bench.executor.utils import get_model_iid
from bench.model.base import ModelHandler, load_model
from bench.models.model import ModelVersion
from bench.utils.record import Record, RecordBatch, is_record

logger = structlog.stdlib.get_logger()


class LocalExecutor(SimpleExecutor):
    """
    A locally executed implementation of Executor without coordination or parallelism.
    """

    def __init__(self):
        self._loaded_models_by_iid: Dict[str, ModelHandler] = {}

    async def _get_loaded_model(
        self, model: ModelVersion, load_if_needed: bool
    ) -> ModelHandler:
        model_iid = get_model_iid(model)
        if model_iid not in self._loaded_models_by_iid:
            if not load_if_needed:
                raise RuntimeError("model " + model_iid + " is not load")
            else:
                await self.load_model(model)
        return self._loaded_models_by_iid[model_iid]

    async def load_model(
        self,
        model: ModelVersion,
        requirements: Optional[ResourceRequirements] = None,
    ):
        model_iid = get_model_iid(model=model)
        if model_iid in self._loaded_models_by_iid:
            return

        log = logger.bind(
            model_id=model.id,
            model_iid=model_iid,
            arguments=model.arguments,
            requirements=requirements,
        )
        log.info("model_load")
        model_handler: ModelHandler = load_model(
            model.handler_id,
            version=model.version,
            storage_uri=model.storage_uri,
            arguments=model.arguments,
            spec=model.spec,
        )
        self._loaded_models_by_iid[model_iid] = model_handler
        log.info("model_loaded")

    async def run_model(
        self,
        model: ModelVersion,
        record: Union[Record, RecordBatch],
        load_if_needed: bool = False,
    ) -> Union[Record, RecordBatch]:
        model_handler = await self._get_loaded_model(model, load_if_needed)
        if is_record(record):
            return model_handler.predict(cast(Record, record))
        else:
            return model_handler.predict_batch(cast(RecordBatch, record))
