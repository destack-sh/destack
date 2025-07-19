import { packProtoJson, unpackProtoJson } from "@destack/grpc";
import { ScalarType, StructType, TypeCardinality } from "@destack/language/core/builtin/common";
import { isNode } from "@destack/language/core/builtin/node";
import { StructFrozen } from "@destack/language/core/builtin/struct";
import { packCson, unpackCson } from "@destack/language/core/common/cson";
import { PropertyDefinition } from "@destack/language/core/common/definition";
import { CustomProperty } from "@destack/language/core/common/property";
import type { Type } from "@destack/language/core/common/type";
import { toType } from "@destack/language/core/common/type";
import type { Supergraph } from "@destack/language/core/runtime/graph";
import type { Session } from "@destack/language/core/runtime/session";
import { STRUCT_CLASS_BY_TYPE, registerStructClass } from "@destack/language/registry";
import { ValueProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashString } from "@destack/utils/hash";

/* ==== DESTACK_GENERATED_START:STRUCT:100 ==== */
/**
 * A generic Value of any Type.
 * Values are used to represent any generic or user-provided data.
 */
export class Value extends StructFrozen {
  static metatype: StructType = StructType.VALUE;
  static __isFrozen__: boolean = true;

  /**
   * Value.type
   */
  readonly type: Type;

  /**
   * Value.value
   */
  readonly value: any | null;

  constructor(options: {
    type: Type;
    value?: any | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Value.type is required`);
    }
    this.type = _type;
    let _value = options.value ?? null;
    this.value = _value;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._cson = options._cson ?? null;
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
      // @ts-expect-error(readonly)
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

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = Value.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: Value): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 100;
    objectCson["100"] = object.type.toCson();
    if (object.value != null) {
      objectCson["110"] = object.value;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Value {
    const _Type = STRUCT_CLASS_BY_TYPE[StructType.TYPE] as typeof Type;
    const valueValue = objectCson["110"];
    const unpackedValue = valueValue != undefined ? valueValue : null;
    return new Value({
      type: _Type.fromCson(objectCson["100"], _session, _supergraph, _graph, _connection),
      value: unpackedValue,
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Value {
    return Value.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): ValueProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Value.__packProto__(this);
    }
    return this._proto as ValueProto;
  }

  static __packProto__(object: Value): ValueProto {
    const objectProto: Partial<ValueProto> = { metatype: 100 };
    objectProto.type = object.type.toProto();
    if (object.value != null) {
      objectProto.value = packProtoJson(object.value);
    }
    return objectProto as ValueProto;
  }

  static __unpackProto__(
    objectProto: ValueProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Value {
    const _Type = STRUCT_CLASS_BY_TYPE[StructType.TYPE] as typeof Type;
    return new Value({
      type: _Type.fromProto(objectProto.type!, _session, _supergraph, _graph, _connection),
      value: objectProto.value != undefined ? unpackProtoJson(objectProto.value!) : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: ValueProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Value {
    return Value.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Value {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ValueProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  _unpacked: any | null = null;

  /** Get the unpacked value of this generic Value. */
  unpack(): any {
    if (this._unpacked === null) {
      this._unpacked = unpackCson(this.value, this.type);
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
  type: Type | PropertyDefinition | CustomProperty | null = null,
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
  const _Type = STRUCT_CLASS_BY_TYPE[StructType.TYPE] as typeof Type;
  const valuePacked = packCson(valueUnpacked, type);
  const value = new Value({
    type: type instanceof _Type ? type : type.toType(),
    value: valuePacked,
  });
  return value;
}
