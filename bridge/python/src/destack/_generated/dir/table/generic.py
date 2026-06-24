# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_int,
    json_object,
)

import destack._generated.dir.type.generic
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class GenericSegment:
    """Generic templates and parameters added by one DIR phase."""

    # the module id of the generic segment
    module_id: destack._generated.source.file.model.module.ModuleId
    # the first generic template id owned by this table segment
    first_template_id: int
    # the first generic parameter id owned by this table segment
    first_parameter_id: int
    # generic templates
    templates: Sequence[destack._generated.dir.type.generic.GenericTemplate]
    # generic parameters
    parameters: Sequence[destack._generated.dir.type.generic.GenericParameterBinding]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_generic_segment(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GenericSegment:
        """Decode one GenericSegment."""
        return decode_generic_segment(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_generic_segment(self)

    @classmethod
    def from_json(cls, value: Json) -> GenericSegment:
        """Return one GenericSegment from one JSON value."""
        return from_json_generic_segment(value)


def encode_generic_segment(writer: BinaryWriter, value: GenericSegment) -> None:
    """Encode one GenericSegment."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    writer.write_unsigned(value.first_template_id)
    writer.write_unsigned(value.first_parameter_id)
    writer.write_unsigned(len(value.templates))
    for item_value_templates_0 in value.templates:
        destack._generated.dir.type.generic.encode_generic_template(
            writer, item_value_templates_0
        )
    writer.write_unsigned(len(value.parameters))
    for item_value_parameters_0 in value.parameters:
        destack._generated.dir.type.generic.encode_generic_parameter_binding(
            writer, item_value_parameters_0
        )


def decode_generic_segment(reader: BinaryReader) -> GenericSegment:
    """Decode one GenericSegment."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    first_template_id = reader.read_number()
    first_parameter_id = reader.read_number()
    templates = [
        destack._generated.dir.type.generic.decode_generic_template(reader)
        for _ in range(reader.read_number())
    ]
    parameters = [
        destack._generated.dir.type.generic.decode_generic_parameter_binding(reader)
        for _ in range(reader.read_number())
    ]

    return GenericSegment(
        module_id=module_id,
        first_template_id=first_template_id,
        first_parameter_id=first_parameter_id,
        templates=templates,
        parameters=parameters,
    )


def to_json_generic_segment(value: GenericSegment) -> Json:
    """Return one JSON value for one GenericSegment."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "firstTemplateId": value.first_template_id,
        "firstParameterId": value.first_parameter_id,
        "templates": [
            destack._generated.dir.type.generic.to_json_generic_template(item_0)
            for item_0 in value.templates
        ],
        "parameters": [
            destack._generated.dir.type.generic.to_json_generic_parameter_binding(
                item_0
            )
            for item_0 in value.parameters
        ],
    }


def from_json_generic_segment(value: Json) -> GenericSegment:
    """Return one GenericSegment from one JSON value."""
    object_ = json_object(value)

    return GenericSegment(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        first_template_id=json_int(json_field(object_, "firstTemplateId")),
        first_parameter_id=json_int(json_field(object_, "firstParameterId")),
        templates=[
            destack._generated.dir.type.generic.from_json_generic_template(item_0)
            for item_0 in json_array(json_field(object_, "templates"))
        ],
        parameters=[
            destack._generated.dir.type.generic.from_json_generic_parameter_binding(
                item_0
            )
            for item_0 in json_array(json_field(object_, "parameters"))
        ],
    )


__all__ = [
    "GenericSegment",
    "encode_generic_segment",
    "decode_generic_segment",
    "to_json_generic_segment",
    "from_json_generic_segment",
]
