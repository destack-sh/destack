from typing import Optional, Type, Union

from bench.utils.spec import ClassLabelType, FieldSpec, convert_to_record_spec

TOKEN_SPEC = FieldSpec(
    name="token",
    description="a single token",
    type=convert_to_record_spec({"text": str, "start": int, "end": int}),
)
TOKENS_SPEC = [TOKEN_SPEC]


def make_entity_spec(label_type: Union[Type[str], ClassLabelType]):
    return convert_to_record_spec(
        name="entity",
        description="a single entity",
        spec={"text": str, "start": int, "end": int, "label": label_type, "score": Optional[float]},
    )


def make_entities_spec(label_type: Union[Type[str], ClassLabelType]):
    return [make_entity_spec(label_type)]


def make_scored_label_spec(label_type: Union[Type[str], ClassLabelType]):
    return FieldSpec(
        name="label",
        description="a scored label",
        type=convert_to_record_spec({"label": label_type, "score": float}),
    )


def make_scored_labels_spec(label_type: Union[Type[str], ClassLabelType]):
    return [make_scored_label_spec(label_type)]
