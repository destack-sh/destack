from collections.abc import Mapping

from destack.language import PrimitiveType

TYPESCRIPT_TYPE_BY_PRIMITIVE_TYPE: Mapping[PrimitiveType, str] = {
    PrimitiveType.BOOLEAN: "boolean",
    PrimitiveType.SINT16: "number",
    PrimitiveType.SINT32: "number",
    PrimitiveType.SINT64: "number",
    PrimitiveType.FLOAT32: "number",
    PrimitiveType.FLOAT64: "number",
    PrimitiveType.STRING: "string",
    PrimitiveType.UUID: "string",
    PrimitiveType.BYTES: "Uint8Array",
    PrimitiveType.DATETIME: "Temporal.ZonedDateTime",
    PrimitiveType.DATE: "Temporal.PlainDate",
    PrimitiveType.TIME: "Temporal.PlainTime",
    PrimitiveType.DURATION: "Temporal.Duration",
    PrimitiveType.JSON: "any",
}
