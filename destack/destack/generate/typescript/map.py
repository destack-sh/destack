from collections.abc import Mapping

from destack.language import PrimitiveType

TYPESCRIPT_TYPE_BY_PRIMITIVE_TYPE: Mapping[PrimitiveType, str] = {
    # boolean
    PrimitiveType.BOOLEAN: "boolean",
    # integer
    PrimitiveType.INT8: "Int8",
    PrimitiveType.INT16: "Int16",
    PrimitiveType.INT32: "Int32",
    PrimitiveType.INT64: "Int64",
    PrimitiveType.INT128: "Int128",
    PrimitiveType.UINT8: "UInt8",
    PrimitiveType.UINT16: "UInt16",
    PrimitiveType.UINT32: "UInt32",
    PrimitiveType.UINT64: "UInt64",
    PrimitiveType.UINT128: "UInt128",
    # float
    PrimitiveType.FLOAT16: "Float16",
    PrimitiveType.FLOAT32: "Float32",
    PrimitiveType.FLOAT64: "Float64",
    # time
    PrimitiveType.DATETIME: "Datetime",
    PrimitiveType.DATE: "Date",
    PrimitiveType.TIME: "Time",
    PrimitiveType.DURATION: "Duration",
    # string
    PrimitiveType.STRING: "string",
    PrimitiveType.UUID: "UUID",
    PrimitiveType.BYTES: "Bytes",
    # json
    PrimitiveType.JSON: "Json",
}
