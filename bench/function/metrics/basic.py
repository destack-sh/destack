import sklearn

from bench.function.base import Metric, functions
from bench.utils.func import dict_to_ordered
from bench.utils.record import Record, RecordBatch
from bench.utils.spec import convert_to_record_spec


@functions.register("bench.metric.accuracy")
class MetricAccuracy(Metric):
    input_spec = {
        "predictions": convert_to_record_spec(int),
        "references": convert_to_record_spec(int),
    }
    output_spec = dict_to_ordered({"*": {"accuracy": convert_to_record_spec(float)}})

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
            predictions = predictions[self.prediction_key]
        if self.reference_key:
            references = references[self.reference_key]

        accuracy = sklearn.metrics.accuracy_score(references, predictions)
        return {"accuracy": accuracy}
