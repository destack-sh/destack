import sklearn

from bench.function.base import MetricFunction, functions
from bench.utils.record import Record, RecordBatch


@functions.register("bench.metric.accuracy")
class MetricAccuracy(MetricFunction):
    def __init__(
        self,
        prediction_key: str = None,
        reference_key: str = None,
    ):
        self.prediction_key = prediction_key
        self.reference_key = reference_key

    def compute(self, predictions: RecordBatch, references: RecordBatch) -> Record:
        if self.prediction_key:
            predictions = predictions[self.prediction_key]
        if self.reference_key:
            references = references[self.reference_key]

        accuracy = sklearn.metrics.accuracy_score(references, predictions)
        return {"accuracy": accuracy}
