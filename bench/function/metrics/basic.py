import sklearn

from bench.function.base import MetricFunction, functions
from bench.utils.record import Record, RecordBatch


@functions.register("bench.metric.accuracy")
class MetricAccuracy(MetricFunction):
    input_spec = {"predictions": int, "references": int}

    # TODO @Cleanup @Architecture: move input/output remapping to general connection/function wrapper
    def __init__(
        self,
        prediction_key: str = None,
        reference_key: str = None,
    ):
        self.prediction_key = prediction_key
        self.reference_key = reference_key

    def compute(self, **kwargs: RecordBatch) -> Record:
        predictions: RecordBatch = kwargs["predictions"]
        references: RecordBatch = kwargs["references"]
        if self.prediction_key:
            predictions = predictions[self.prediction_key]  # type: ignore
        if self.reference_key:
            references = references[self.reference_key]  # type: ignore

        accuracy = sklearn.metrics.accuracy_score(references, predictions)
        return {"accuracy": accuracy}
