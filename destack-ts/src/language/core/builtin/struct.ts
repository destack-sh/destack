import type { Encoding, PackedCache, StructDefinition } from "@destack/language/core";
import { ObjectKind, StructType } from "@destack/language/core/builtin/common";
import { ACTIVE_SESSION, ENCODERS } from "@destack/language/core/builtin/const";
import { BuiltinObject, BuiltinObjectClass } from "@destack/language/core/builtin/object";
import type { Session } from "@destack/language/core/runtime/session";
import type { StructTypeMapping } from "@destack/language/mapping";
import { registerStructClass } from "@destack/language/registry";
import { BinaryWriter } from "@destack/language/core/runtime/binary";

/** A Struct is an ordered collection of Properties. */
export abstract class Struct extends BuiltinObject {
  static readonly metatype: StructType;
  static readonly __kind__: ObjectKind = ObjectKind.STRUCT;
  static readonly __isStruct__: boolean = true;
  static readonly __definition__: StructDefinition;

  constructor(_session: Session | null) {
    _session = _session ?? ACTIVE_SESSION.get();
    super(_session);
  }

  get metatype(): StructType {
    return (this.constructor as typeof Struct).metatype;
  }
}
registerStructClass(StructType.STRUCT, Struct);

/** A frozen Struct is a Struct that is immutable. */
export abstract class StructFrozen extends Struct {
  static readonly __isFrozen__: boolean = true;

  /** Cached hash of the Struct. */
  readonly _hash: number | null = null;
  /** Cached repr of the Struct. */
  readonly _repr: string | null = null;
  /** Cached packed representations (first N = each Encoding, next N = each Encoding as bytes). */
  readonly _packedCache: PackedCache[] | null = null;

  _invalidateFrozenCache(): void {
    // frozen Structs should be immutable, but sometimes we need to break out of that
    // @ts-ignore
    this._hash = null;
    // @ts-ignore
    this._repr = null;
    // @ts-ignore
    this._packedCache = null;
  }

  override pack(encoding: Encoding): any {
    // check if we have a cached packed representation
    if (this._packedCache != null) {
      for (const cached of this._packedCache) {
        if (cached.encoding === encoding && !cached.isBytes) {
          return cached.packed;
        }
      }
    }
    // pack the object
    const encoder = ENCODERS[encoding]!;
    const packedObject = encoder.packObject(ObjectKind.STRUCT, this.metatype, this);
    // cache the result
    const newCache: PackedCache = {
      encoding: encoding,
      isBytes: false,
      packed: packedObject,
    };
    if (this._packedCache == null) {
      // @ts-expect-error(readonly)
      this._packedCache = [newCache];
    } else {
      // @ts-expect-error(readonly)
      this._packedCache = [...this._packedCache, newCache];
    }
    return packedObject;
  }

  packBinary(encoding: Encoding): Uint8Array {
    // check if we have a cached packed bytes representation
    if (this._packedCache != null) {
      for (const cached of this._packedCache) {
        if (cached.encoding === encoding && cached.isBytes) {
          return cached.packed;
        }
      }
    }
    // pack the object as bytes
    const encoder = ENCODERS[encoding]!;
    const writer = new BinaryWriter();
    encoder.packObjectBinary(ObjectKind.STRUCT, this.metatype, this, writer);
    const packedObjectBytes = writer.toBytes();
    // cache the result
    const newCache: PackedCache = {
      encoding: encoding,
      isBytes: true,
      packed: packedObjectBytes,
    };
    if (this._packedCache == null) {
      // @ts-expect-error(readonly)
      this._packedCache = [newCache];
    } else {
      // @ts-expect-error(readonly)
      this._packedCache = [...this._packedCache, newCache];
    }
    return packedObjectBytes;
  }
}

/** A Struct class. */
type StructConstructor<S extends Struct = Struct> = new (...args: any[]) => S;
type AbstractStructConstructor<S extends Struct = Struct> = abstract new (...args: any[]) => S;
export type StructClass<S extends Struct = Struct> = (
  | StructConstructor<S>
  | AbstractStructConstructor<S>
) &
  BuiltinObjectClass<any> & {
    metatype: StructType;
    __definition__: StructDefinition;
  };

/** Check if a value is a Struct of a specific type. */
export function isStruct<T extends StructType>(
  value: any,
  structType?: T,
): value is StructTypeMapping[T] {
  return value instanceof Struct && (structType === undefined || value.metatype === structType);
}
