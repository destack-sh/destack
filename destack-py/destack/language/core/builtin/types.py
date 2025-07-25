from datetime import date, datetime, time, timedelta
from typing import Any

from destack.utils.uuid import UUID

# :PrimitiveType
type Boolean = bool
# integer
type SInt8 = int
type SInt16 = int
type SInt32 = int
type SInt64 = int
type SInt128 = int
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
type Datetime = datetime
type Date = date
type Time = time
type Duration = timedelta
# string
type String = str
# type UUID = UUID
type Bytes = bytes
type Json = Any
type Cson = Json  # just an alias
type Kompakt = bytes  # just an alias

__all__ = [  # noqa: RUF022
    "Boolean",
    # integer
    "SInt8",
    "SInt16",
    "SInt32",
    "SInt64",
    "SInt128",
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
    "Datetime",
    "Date",
    "Time",
    "Duration",
    # string
    "String",
    "UUID",
    "Bytes",
    "Json",
    "Cson",
    "Kompakt",
]
