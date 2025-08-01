import type {
  BinaryReader,
  BinaryWriter,
  BuiltinObject,
  ObjectKind,
  Session,
  Type,
} from "@destack/language";

export enum EncoderOptions {
  DEFAULT = 0,
  OMIT_METATYPE = 1,
  OMIT_KEY = 1 << 1,
  OMIT_TYPE = 1 << 2,
  OMIT_NONE = 1 << 3,
  UNWRAP_VALUE = 1 << 4,
}

/** Encoder for packing/unpacking Objects. */
export abstract class Encoder<T = any> {
  /** Pack an Object into some encoded format. */
  abstract packObject(object: BuiltinObject, options: EncoderOptions): T;

  /** Unpack an Object from some encoded format. */
  abstract unpackObject(
    kind: ObjectKind | null,
    type: number | null,
    value: T,
    session: Session | null,
    options: EncoderOptions,
  ): BuiltinObject;

  /** Pack an Object into the byte representation of its encoded format. */
  abstract packObjectBinary(
    object: BuiltinObject,
    writer: BinaryWriter,
    options: EncoderOptions,
  ): void;

  /** Unpack an Object from the byte representation of its encoded format. */
  abstract unpackObjectBinary(
    kind: ObjectKind | null,
    type: number | null,
    reader: BinaryReader,
    session: Session | null,
    options: EncoderOptions,
  ): BuiltinObject;

  /** Pack a Type into some encoded format. */
  abstract packType(type: Type, options: EncoderOptions): T;

  /** Unpack a Type from some encoded format. */
  abstract unpackType(value: T, options: EncoderOptions): any;

  /** Pack a Type into the byte representation of its encoded format. */
  abstract packTypeBinary(type: Type, writer: BinaryWriter, options: EncoderOptions): void;

  /** Unpack a Type from the byte representation of its encoded format. */
  abstract unpackTypeBinary(reader: BinaryReader, options: EncoderOptions): any;

  /** Pack a value into some encoded format. */
  abstract packValue(type: Type, value: any, options: EncoderOptions): T;

  /** Unpack a value from some encoded format. */
  abstract unpackValue(type: Type, value: T, session: Session | null, options: EncoderOptions): any;

  /** Pack a value into the byte representation of its encoded format. */
  abstract packValueBinary(
    type: Type,
    value: any,
    writer: BinaryWriter,
    options: EncoderOptions,
  ): void;

  /** Unpack a value from the byte representation of its encoded format. */
  abstract unpackValueBinary(
    type: Type,
    reader: BinaryReader,
    session: Session | null,
    options: EncoderOptions,
  ): any;
}
