import type { Session, StructDefinition, Supergraph } from "@destack/language/core";
import { ACTIVE_SESSION, StructType } from "@destack/language/core";
import { BuiltinObject, BuiltinObjectClass } from "@destack/language/core/builtin/object";
import type { StructTypeMapping } from "@destack/language/mapping";
import { AnyStructProto } from "@destack/proto";

/** A Struct is an ordered collection of Properties. */
export abstract class Struct extends BuiltinObject {
  static readonly __isStruct__: boolean = true;
  static readonly metatype: StructType;
  static readonly __definition__: StructDefinition;

  constructor(_session: Session | null, _supergraph: Supergraph | null) {
    _session = _session ?? ACTIVE_SESSION.get();
    super(_supergraph ?? (_session != null ? _session.supergraph : null));
  }

  get metatype(): StructType {
    return (this.constructor as typeof Struct).metatype;
  }
}

/** A frozen Struct is a Struct that is immutable. */
export abstract class StructFrozen extends Struct {
  static readonly __isFrozen__: boolean = true;

  readonly _hash: number | null = null;
  readonly _repr: string | null = null;
  readonly _proto: AnyStructProto | null = null;
  readonly _value: Record<string, any> | null = null;

  _invalidateFrozenCache(): void {
    // @ts-ignore
    this._hash = null;
    // @ts-ignore
    this._repr = null;
    // @ts-ignore
    this._proto = null;
    // @ts-ignore
    this._value = null;
  }
}

/** A Struct class. */
type StructConstructor<S extends Struct = Struct> = new (...args: any[]) => S;
export type StructClass<S extends Struct = Struct> = StructConstructor &
  BuiltinObjectClass<any, any> & {
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
