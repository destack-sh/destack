import { Session, StructFrozen, StructType, Supergraph, Type } from "@/language";

/* ==== DESTACK_GENERATED_START:STRUCT:2500 ==== */
export class Value extends StructFrozen {
  static metatype: StructType = StructType.VALUE;
  static __isFrozen__: boolean = true;

  readonly type: Type;
  readonly value: any;

  constructor(options: { type: Type; value: any; _session?: Session | null; _supergraph?: Supergraph | null }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    this.type = options.type;
    this.value = options.value;
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:2500 ==== */

/**
 * Convert an arbitrary (legal) value to a Value.
 * If Type isn't provided, it will be inferred from the value.
 */
export function toValue(valueUnpacked: any, type: Type | null = null, nodeAsValue: boolean = false): Value {
  throw new Error("not implemented");
}

/**
 * Pack a generic typed value to a JSON object.
 */
export function packValue(value: any, type: Type): any {
  throw new Error("not implemented");
}

/**
 * Unpack a JSON object to a generic typed value.
 */
export function unpackValue(value: any, type: Type): any {
  throw new Error("not implemented");
}
