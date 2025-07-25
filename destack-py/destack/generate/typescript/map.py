from collections.abc import Mapping

from destack.language import PrimitiveType

TYPESCRIPT_TYPE_BY_PRIMITIVE_TYPE: Mapping[PrimitiveType, str] = {
    # boolean
    PrimitiveType.BOOLEAN: "boolean",
    # integer
    PrimitiveType.SINT8: "number",
    PrimitiveType.SINT16: "number",
    PrimitiveType.SINT32: "number",
    PrimitiveType.SINT64: "number",
    PrimitiveType.SINT128: "number",
    PrimitiveType.UINT8: "number",
    PrimitiveType.UINT16: "number",
    PrimitiveType.UINT32: "number",
    PrimitiveType.UINT64: "number",
    PrimitiveType.UINT128: "number",
    # float
    PrimitiveType.FLOAT32: "number",
    PrimitiveType.FLOAT64: "number",
    # time
    PrimitiveType.DATETIME: "Temporal.ZonedDateTime",
    PrimitiveType.DATE: "Temporal.PlainDate",
    PrimitiveType.TIME: "Temporal.PlainTime",
    PrimitiveType.DURATION: "Temporal.Duration",
    # string
    PrimitiveType.STRING: "string",
    PrimitiveType.UUID: "string",
    PrimitiveType.BYTES: "Uint8Array",
    # json
    PrimitiveType.JSON: "any",
}
