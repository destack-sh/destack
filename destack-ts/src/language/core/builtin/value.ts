import { packJsonc, unpackJsonc } from "@destack/encoder/jsonc/wiring";
import { StructType } from "@destack/language/core/builtin/builtin";
import { ScalarType, TypeCardinality } from "@destack/language/core/builtin/common";
import { isNode } from "@destack/language/core/builtin/node";
import type { PackedObjectCache } from "@destack/language/core/builtin/object";
import { ImmutableStruct } from "@destack/language/core/builtin/struct";
import type { Type } from "@destack/language/core/builtin/type";
import { toType } from "@destack/language/core/builtin/type";
import type { Json } from "@destack/language/core/builtin/types";
import type { Session } from "@destack/language/core/runtime/session";
import { registerStructClass } from "@destack/language/registry";
import { hashString } from "@destack/utils/hash";

/* ==== DESTACK_GENERATED_START:STRUCT:100 ==== */
/**
 * A generic Value of any Type.
 * Values are used to represent any generic or user-provided data.
 */
export class Value extends ImmutableStruct {
  static metatype: StructType = StructType.VALUE;
  static __isFrozen__: boolean = true;

  /**
   * Value.type
   */
  readonly type: Type;

  /**
   * Value.value
   */
  readonly value: Json | null;

  constructor(options: {
    type: Type;
    value?: Json | null;
    _session?: Session | null;
    _hash?: number | null;
    _repr?: string | null;
    _PackedObjectCache?: PackedObjectCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
    let _type = options.type;
    if (_type == null) {
      throw new Error(`Value.type is required`);
    }
    this.type = _type;
    let _value = options.value ?? null;
    this.value = _value;

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._PackedObjectCache = options._PackedObjectCache ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!this.type.equals(other.type)) {
      return false;
    }
    if (!(this.value === other.value)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${this.type.repr()}`);
      // @ts-expect-error(readonly) */
      this._repr = `<Value ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type.hash()) & 0xffffffff;
    if (this.value != null) {
      h = (h * 31 + hashString(JSON.stringify(this.value))) & 0xffffffff;
    }
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  _unpacked: any | null = null;

  /** Get the unpacked value of this generic Value. */
  unpack(): any {
    if (this._unpacked === null) {
      this._unpacked = unpackJsonc(this.type, this.value, this._session);
    }
    return this._unpacked;
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.VALUE, Value);
/* ==== DESTACK_GENERATED_END:STRUCT:100 ==== */

/**
 * Convert an arbitrary (legal) value to a Value.
 * If Type isn't provided, it will be inferred from the value.
 */
export function toValue(
  valueUnpacked: any,
  type: Type | null = null,
  options?: { nodeAsValue: boolean },
): Value {
  // infer type
  if (type === null) {
    if (valueUnpacked === null) {
      throw new Error("cannot infer type for null");
    }
    type = toType(valueUnpacked, options);
  }
  // coerce nodes into node references
  if (type.scalarType == ScalarType.NODE_REFERENCE) {
    if (type.cardinality == TypeCardinality.SCALAR && isNode(valueUnpacked)) {
      valueUnpacked = valueUnpacked.toRef();
    } else if (type.cardinality == TypeCardinality.LIST && valueUnpacked.length > 0) {
      valueUnpacked = valueUnpacked.map((item: any) => (isNode(item) ? item.toRef() : item));
    }
  }
  // pack value
  const valuePacked = packJsonc(type, valueUnpacked);
  const value = new Value({ type: type, value: valuePacked });
  return value;
}
