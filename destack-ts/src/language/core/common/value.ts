import { NODE_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE, toType } from "@destack/language";
import {
  BuiltinObject,
  Node,
  NodeReference,
  NodeType,
  PrimitiveType,
  ScalarType,
  Session,
  StructFrozen,
  StructType,
  Supergraph,
  Type,
  TypeCardinality,
} from "@destack/language/core";
import { ValueProto } from "@destack/proto";
import { assertNever, timedeltaFromISOFormat, timedeltaToISOFormat } from "@destack/utils";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:STRUCT:2500 ==== */
/**
 * A generic Value of any Type.
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
  readonly value: any;

  constructor(options: {
    type: Type;
    value: any;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    let _value = options.value;
    if (_value === null) {
      throw new Error(`Value.value is required`);
    }
    this.value = _value;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Value.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Value): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 2500;
    objectValue["30"] = object.type.toValue();
    objectValue["40"] = object.value;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Value {
    return new Value({
      type: Type.fromValue(objectValue["30"], _session, _supergraph, _graph, _connection),
      value: objectValue["40"],
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Value {
    return Value.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): ValueProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Value.__packProto__(this);
    }
    return this._proto as ValueProto;
  }

  static __packProto__(object: Value): ValueProto {
    const objectProto: Partial<ValueProto> = { metatype: 2500 };
    objectProto.type = object.type.toProto();
    objectProto.value = object.value;
    return objectProto as ValueProto;
  }

  static __unpackProto__(
    objectProto: ValueProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Value {
    return new Value({
      type: Type.fromProto(objectProto.type!, _session, _supergraph, _graph, _connection),
      value: objectProto.value,
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
}
/* ==== DESTACK_GENERATED_END:STRUCT:2500 ==== */

/**
 * Convert an arbitrary (legal) value to a Value.
 * If Type isn't provided, it will be inferred from the value.
 */
export function toValue(valueUnpacked: any, type: Type | null = null, nodeAsValue: boolean = false): Value {
  // infer type
  if (type === null) {
    if (valueUnpacked === null) {
      throw new Error("cannot infer type for null");
    }
    type = toType(valueUnpacked, nodeAsValue);
  }
  // coerce nodes into node references
  if (type.scalarType == ScalarType.NODE_REFERENCE) {
    if (type.cardinality == TypeCardinality.SCALAR && valueUnpacked instanceof Node) {
      valueUnpacked = valueUnpacked.toRef();
    } else if (type.cardinality == TypeCardinality.LIST) {
      valueUnpacked = valueUnpacked.map((item: any) => (item instanceof Node ? item.toRef() : item));
    }
  }
  // pack value
  const valuePacked = packValue(valueUnpacked, type);
  const value = new Value({ type, value: valuePacked });
  return value;
}

/**
 * Pack a generic typed value to a JSON object.
 */
export function packValue(value: any, type: Type): any {
  if (type.cardinality == TypeCardinality.SCALAR) {
    return _packScalarValue(value, type);
  } else if (type.cardinality == TypeCardinality.LIST) {
    if (!value) {
      return [];
    }
    const packedList: any[] = [];
    for (const item of value) {
      packedList.push(_packScalarValue(item, type));
    }
    return packedList;
  } else if (type.cardinality == TypeCardinality.MAP) {
    if (!value) {
      return {};
    }
    if (type.keyType === null) {
      throw new Error(`no key type for ${type.repr()}`);
    }
    const packedMap: { [key: string]: any } = {};
    for (const [key, val] of Object.entries(value)) {
      const packedKey = _packScalarValue(key, type.keyType);
      const packedVal = _packScalarValue(val, type);
      packedMap[String(packedKey)] = packedVal;
    }
    return packedMap;
  } else {
    assertNever(type.cardinality);
  }
}

/**
 * Unpack a JSON object to a generic typed value.
 */
export function unpackValue(
  value: any,
  type: Type,
  _session?: Session | null,
  _graph?: any | null,
  _supergraph?: Supergraph | null,
  _connection?: any | null,
): any {
  if (type.cardinality == TypeCardinality.SCALAR) {
    return _unpackScalarValue(value, type, _session, _supergraph, _graph, _connection);
  } else if (type.cardinality == TypeCardinality.LIST) {
    if (value === null) {
      return [];
    }
    const unpackedList = [];
    for (const item of value) {
      unpackedList.push(_unpackScalarValue(item, type, _session, _supergraph, _graph, _connection));
    }
    return unpackedList;
  } else if (type.cardinality == TypeCardinality.MAP) {
    if (value === null) {
      return {};
    }
    const unpackedMap: { [key: string]: any } = {};
    for (const [key, val] of Object.entries(value)) {
      const unpackedKey = type.keyType ? _unpackScalarValue(key, type.keyType) : key;
      const unpackedVal = _unpackScalarValue(val, type, _session, _supergraph, _graph, _connection);
      unpackedMap[unpackedKey] = unpackedVal;
    }
    return unpackedMap;
  } else {
    assertNever(type.cardinality);
  }
}

/** Pack a scalar value to a JSON object. */
function _packScalarValue(value: any, type: Type): any {
  if (type.scalarType == ScalarType.PRIMITIVE) {
    if (type.primitiveType == PrimitiveType.BYTES) {
      return Buffer.from(value as Uint8Array).toString("base64");
    } else if (type.primitiveType == PrimitiveType.DATETIME) {
      return (value as Temporal.ZonedDateTime).toString();
    } else if (type.primitiveType == PrimitiveType.DATE) {
      return (value as Temporal.PlainDate).toString();
    } else if (type.primitiveType == PrimitiveType.TIME) {
      return (value as Temporal.PlainTime).toString();
    } else if (type.primitiveType == PrimitiveType.DURATION) {
      return timedeltaToISOFormat(value as Temporal.Duration);
    } else {
      return value;
    }
  } else if (type.scalarType == ScalarType.ENUM) {
    return value;
  } else if (
    type.scalarType == ScalarType.NODE_REFERENCE ||
    type.scalarType == ScalarType.NODE_VALUE ||
    type.scalarType == ScalarType.STRUCT
  ) {
    return (value as BuiltinObject).toValue();
  } else {
    assertNever(type.scalarType);
  }
}

/** Unpack a JSON object to a scalar value. */
function _unpackScalarValue(
  value: any,
  type: Type,
  _session?: Session | null,
  _supergraph?: Supergraph | null,
  _graph?: any | null,
  _connection?: any | null,
): any {
  if (type.scalarType == ScalarType.PRIMITIVE) {
    if (type.primitiveType == PrimitiveType.BYTES) {
      return Buffer.from(value, "base64");
    } else if (type.primitiveType == PrimitiveType.DATETIME) {
      return Temporal.ZonedDateTime.from(value);
    } else if (type.primitiveType == PrimitiveType.DATE) {
      return Temporal.PlainDate.from(value);
    } else if (type.primitiveType == PrimitiveType.TIME) {
      return Temporal.PlainTime.from(value);
    } else if (type.primitiveType == PrimitiveType.DURATION) {
      return timedeltaFromISOFormat(value);
    } else {
      return value;
    }
  } else if (type.scalarType == ScalarType.ENUM) {
    return value;
  } else if (type.scalarType == ScalarType.NODE_REFERENCE) {
    return NodeReference.fromValue(value, _session, _supergraph, _graph, _connection);
  } else if (type.scalarType == ScalarType.NODE_VALUE) {
    const nodeType = Number(value["1"]) as NodeType;
    const nodeClass = NODE_CLASS_BY_TYPE[nodeType];
    return nodeClass.fromValue(value, _session, _supergraph, _graph, _connection);
  } else if (type.scalarType == ScalarType.STRUCT) {
    if (type.structType === null) {
      throw new Error(`missing struct type for ${type.repr()}`);
    }
    const structClass = STRUCT_CLASS_BY_TYPE[type.structType];
    return structClass.fromValue(value, _session, _supergraph, _graph, _connection);
  } else {
    assertNever(type.scalarType);
  }
}
