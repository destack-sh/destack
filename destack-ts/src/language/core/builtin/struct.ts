import type { StructDefinition } from "@destack/language/core";
import { StructType } from "@destack/language/core/builtin/common";
import { ACTIVE_SESSION } from "@destack/language/core/builtin/const";
import { BuiltinObject, BuiltinObjectClass } from "@destack/language/core/builtin/object";
import type { Supergraph } from "@destack/language/core/runtime/graph";
import type { Session } from "@destack/language/core/runtime/session";
import type { StructTypeMapping } from "@destack/language/mapping";
import { registerStructClass } from "@destack/language/registry";
import { AnyStructProto } from "@destack/proto";

/** A Struct is an ordered collection of Properties. */
export abstract class Struct extends BuiltinObject {
  static readonly metatype: StructType;
  static readonly __isStruct__: boolean = true;
  static readonly __definition__: StructDefinition;

  constructor(_session: Session | null, _supergraph: Supergraph | null) {
    _session = _session ?? ACTIVE_SESSION.get();
    super(_supergraph ?? (_session != null ? _session.supergraph : null));
  }

  get metatype(): StructType {
    return (this.constructor as typeof Struct).metatype;
  }
}
registerStructClass(StructType.STRUCT, Struct);

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
type AbstractStructConstructor<S extends Struct = Struct> = abstract new (...args: any[]) => S;
export type StructClass<S extends Struct = Struct> = (
  | StructConstructor<S>
  | AbstractStructConstructor<S>
) &
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
