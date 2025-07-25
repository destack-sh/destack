import type { Temporal } from "temporal-polyfill";

// :PrimitiveType
export type Boolean = boolean;
// integer
export type SInt8 = number;
export type SInt16 = number;
export type SInt32 = number;
export type SInt64 = number;
export type SInt128 = number;
export type UInt8 = number;
export type UInt16 = number;
export type UInt32 = number;
export type UInt64 = number;
export type UInt128 = number;
// float
export type Float16 = number;
export type Float32 = number;
export type Float64 = number;
// time
export type Datetime = Temporal.ZonedDateTime;
export type Date = Temporal.PlainDate;
export type Time = Temporal.PlainTime;
export type Duration = Temporal.Duration;
// string
export type String = string;
export type UUID = string;
export type Bytes = Uint8Array;
export type Json = any;
export type Cson = Json;
export type Kompakt = Uint8Array;
