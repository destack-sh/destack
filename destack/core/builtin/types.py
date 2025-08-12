from datetime import date, datetime, time, timedelta
from typing import Any
from uuid import UUID

# :PrimitiveType
type Boolean = bool
# integer
type Int8 = int
type Int16 = int
type Int32 = int
type Int64 = int
type Int128 = int
type UInt8 = int
type UInt16 = int
type UInt32 = int
type UInt64 = int
type UInt128 = int
# float
type Float16 = float
type Float32 = float
type Float64 = float
# time
type DateTime = datetime
type Date = date
type Time = time
type Timestamp = UInt64
type Duration = timedelta
# string
type String = str
type Character = str
# type UUID = UUID
type Bytes = bytes
type Json = Any

__all__ = [  # noqa: RUF022
    "Boolean",
    # integer
    "Int8",
    "Int16",
    "Int32",
    "Int64",
    "Int128",
    "UInt8",
    "UInt16",
    "UInt32",
    "UInt64",
    "UInt128",
    # float
    "Float16",
    "Float32",
    "Float64",
    # time
    "DateTime",
    "Date",
    "Time",
    "Timestamp",
    "Duration",
    # string
    "String",
    "Character",
    "UUID",
    "Bytes",
    "Json",
    "Bytes",
]
