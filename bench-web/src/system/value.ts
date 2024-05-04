import {
  BenchType,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PrimitiveType,
  Struct as ProtoStruct,
  StructType,
  TypeKind,
  Variant,
  ViewType,
  type AnyNodeData,
  type AnyStructData,
  type TypeInfoData,
} from "@/proto/wire";
import { describeNode, fromRobustJson, isStruct, makeDefaultStruct, toRobustJson } from "@/proto/wiring";
import type { ReadNodeGraph } from "@/system/graph";
import { ENUM_ICONS_BY_TYPE } from "@/system/icon";
import { TK_LENGTH_B64, getEnumOptions, getTkB64FromPtr, isEnumType, padCkFromTkB64, toCamelName } from "@/system/lang";
import { decodeB64VLQ, encodeB64VLQ } from "@/utils/functools";
import { toCamelCase } from "@/utils/string";
import type { ViewProps } from "@/views/common";

export type TypeIdentity = Pick<
  TypeInfoData,
  "kind" | "primitiveType" | "benchType" | "baseTypePtr" | "formatHint" | "isList" | "isSecret" | "constraint"
> & { ck?: string };

export function describeTypeIdentity(type: TypeIdentity & Partial<AnyNodeData>): string {
  if (type.kind == null) return "<empty>";
  const typeParts: string[] = [];
  if ("id" in type) typeParts.push(`id=${type.id}`);
  if ("ck" in type) typeParts.push(`ck=${type.ck}`);
  if ("revision" in type) typeParts.push(`r=${type.revision}`);
  if (type.primitiveType != null) typeParts.push(toCamelName(PrimitiveType, type.primitiveType!));
  if (type.benchType != null) typeParts.push(toCamelName(BenchType, type.benchType!));
  if (type.baseTypePtr != null) typeParts.push(`base=${describeNode(type.baseTypePtr)}`);
  if (type.isList) typeParts.push("list");
  if (type.isSecret) typeParts.push("secret");
  const kindName = toCamelName(TypeKind, type.kind);
  return `${kindName}[${typeParts.join(", ")}]`;
}

export type JsonPrimimtive = string | number | boolean | null;
export type JsonValue = JsonPrimimtive | { [key: string]: JsonValue } | JsonValue[];
export type PrimitiveValue = JsonPrimimtive;
export type ScalarValue = PrimitiveValue | ProtoStruct | AnyStructData | AnyNodeData;
export type SomeValue = ScalarValue | ScalarValue[];

export function makeTypeInfo(partial: Partial<Omit<TypeInfoData, "metatype">>): TypeInfoData {
  return makeDefaultStruct({ metatype: StructType.TYPE_INFO, ...partial });
}

const VIEW_TYPE_BY_OBJECT_TYPE: Partial<Record<BenchType, ViewType>> = {
  [BenchType.ICON]: ViewType.ICON,
  [BenchType.CODE]: ViewType.CODE,
  [BenchType.TEXT]: ViewType.TEXT,
};
const VIEW_TYPE_BY_PRIMITIVE_TYPE: Partial<Record<PrimitiveType, ViewType>> = {
  [PrimitiveType.STRING]: ViewType.STRING,
  [PrimitiveType.INT16]: ViewType.NUMBER,
  [PrimitiveType.INT32]: ViewType.NUMBER,
  [PrimitiveType.INT64]: ViewType.NUMBER,
  [PrimitiveType.FLOAT32]: ViewType.NUMBER,
  [PrimitiveType.FLOAT64]: ViewType.NUMBER,
  [PrimitiveType.BOOLEAN]: ViewType.TOGGLE,
  [PrimitiveType.DATETIME]: ViewType.CALENDAR,
  [PrimitiveType.INTERVAL]: ViewType.CALENDAR,
  [PrimitiveType.JSON]: ViewType.JSON,
};

export function getViewForValueType(type: TypeIdentity): {
  viewType: ViewType;
  props?: ViewProps;
} | null {
  if (type.benchType != null) {
    if (VIEW_TYPE_BY_OBJECT_TYPE[type.benchType] != null) {
      return { viewType: VIEW_TYPE_BY_OBJECT_TYPE[type.benchType]! };
    } else {
      // prefer inline picker if possible
      if (isEnumType(type.benchType) && getEnumOptions(type.benchType).length <= 5) {
        const variant = ENUM_ICONS_BY_TYPE[type.benchType] != null ? Variant.STEALTH : Variant.COMPACT;
        return {
          viewType: ViewType.PICKER,
          props: { valueType: makeTypeInfo(type), variant, isInline: true },
        };
      } else {
        return { viewType: ViewType.PICKER, props: { valueType: makeTypeInfo(type) } };
      }
    }
  } else if (VIEW_TYPE_BY_PRIMITIVE_TYPE[type.primitiveType!] != null) {
    return { viewType: VIEW_TYPE_BY_PRIMITIVE_TYPE[type.primitiveType!]! };
  } else {
    return null;
  }
}

const LETTER_BY_TYPE_KIND: Partial<Record<TypeKind, string>> = {
  [TypeKind.PRIMITIVE]: "p",
  [TypeKind.STRUCT]: "s",
  [TypeKind.NODE]: "n",
  [TypeKind.ENUM]: "e",
  [TypeKind.BASED_NODE]: "b",
  [TypeKind.OBJECT]: "o",
};
const TYPE_KIND_BY_LETTER: Partial<Record<string, TypeKind>> = {
  p: TypeKind.PRIMITIVE,
  s: TypeKind.STRUCT,
  n: TypeKind.NODE,
  e: TypeKind.ENUM,
  b: TypeKind.BASED_NODE,
  o: TypeKind.OBJECT,
};

/**
 * Encodes the type identity into a key for storage & implicit typing.
 * Format is <kind>[id] (with id encoded as base64).
 * :TypeInfoEncoding
 */
export function encodeTypeIdentity(type: TypeIdentity): string {
  let value: string | null = null;
  if (type.kind == TypeKind.PRIMITIVE) {
    value = encodeB64VLQ(type.primitiveType!);
  } else if (type.kind == TypeKind.NODE || type.kind == TypeKind.STRUCT || type.kind == TypeKind.ENUM) {
    value = encodeB64VLQ(type.benchType!);
  } else if (type.kind == TypeKind.BASED_NODE) {
    value = `${getTkB64FromPtr(type.baseTypePtr!)}${encodeB64VLQ(type.benchType!)}`;
  } else if (type.kind == TypeKind.OBJECT) {
    value = getTkB64FromPtr(type.baseTypePtr!);
  } else {
    throw new Error(`unsupported type kind ${type?.kind}`);
  }

  const prefix = type.isList ? LETTER_BY_TYPE_KIND[type.kind]!.toUpperCase() : LETTER_BY_TYPE_KIND[type.kind]!;
  if (type.isSecret) return `!${prefix}${value}`;
  else return `${prefix}${value}`;
}

/** Decodes the type-related info back from the identity key. See encode. :TypeInfoEncoding */
export function decodeTypeIdentity(key: string): TypeIdentity {
  let isSecret: boolean;
  if (key[0] === "!") {
    key = key.slice(1);
    isSecret = true;
  } else {
    isSecret = false;
  }
  let isList: boolean;
  let kind: TypeKind;
  if (key[0].toUpperCase() === key[0]) {
    isList = true;
    kind = TYPE_KIND_BY_LETTER[key[0].toLowerCase()]!;
  } else {
    isList = false;
    kind = TYPE_KIND_BY_LETTER[key[0]]!;
  }
  const value = key.slice(1);

  if (kind === TypeKind.PRIMITIVE) {
    return { kind, primitiveType: decodeB64VLQ(value) as PrimitiveType, isList, isSecret };
  } else if (kind === TypeKind.NODE || kind === TypeKind.STRUCT || kind === TypeKind.ENUM) {
    return { kind, benchType: decodeB64VLQ(value) as BenchType, isList, isSecret };
  } else if (kind === TypeKind.BASED_NODE) {
    const baseTypePtr = {
      metatype: ObjectType.NODE_REFERENCE,
      type: NodeType.BLOCK,
      ck: padCkFromTkB64(value.slice(0, TK_LENGTH_B64)),
    };
    const benchType = decodeB64VLQ(value.slice(TK_LENGTH_B64)) as BenchType;
    return { kind, baseTypePtr, benchType, isList, isSecret };
  } else if (kind === TypeKind.OBJECT) {
    const baseTypePtr = { metatype: ObjectType.NODE_REFERENCE, type: NodeType.BLOCK, ck: padCkFromTkB64(value) };
    return { kind, baseTypePtr, isList, isSecret };
  } else {
    throw new Error(`unsupported type kind ${kind}`);
  }
}

// TODO :Architecture :Performance: encode/decode protoStruct/Json in connections (at the fetch/commit boundary)
//  (Currently, we have to eagerly encode/decode for every single edit, which is possibly every frame or keystroke,
//   It's likely possible to just cheat a little and auto-encode/decode ProtoStruct properties at the boundary
//   without introducing an entire new layer like in the backend).

function _packValueScalar(value: ScalarValue, type: TypeIdentity): JsonValue {
  if (type.kind == TypeKind.PRIMITIVE) {
    return value as JsonPrimimtive;
  } else if (type.kind == TypeKind.NODE || type.kind == TypeKind.BASED_NODE) {
    if ((value as NodeReferenceData).metatype != ObjectType.NODE_REFERENCE) {
      throw new Error(`unexpected value ${JSON.stringify(value)} for type ${describeTypeIdentity(type)}`);
    }
    return toRobustJson(value as NodeReferenceData);
  } else if (type.kind == TypeKind.ENUM) {
    return value as JsonPrimimtive;
  } else if (type.kind == TypeKind.STRUCT) {
    if (!isStruct(value)) {
      throw new Error(`unexpected value ${JSON.stringify(value)} for type ${describeTypeIdentity(type)}`);
    }
    return toRobustJson(value);
  } else {
    throw new Error(`cannot pack value of type ${describeTypeIdentity(type)}`);
  }
}

function _unpackValueScalar(valuePacked: JsonValue, type: TypeIdentity): ScalarValue {
  if (type.kind == TypeKind.PRIMITIVE) {
    return valuePacked as PrimitiveValue;
  } else if (type.kind == TypeKind.NODE || type.kind == TypeKind.BASED_NODE) {
    if (typeof valuePacked !== "object") {
      throw new Error(`unexpected value ${JSON.stringify(valuePacked)} for type ${describeTypeIdentity(type)}`);
    }
    return fromRobustJson(valuePacked as unknown as NodeReferenceData);
  } else if (type.kind == TypeKind.ENUM) {
    return valuePacked as PrimitiveValue;
  } else if (type.kind == TypeKind.STRUCT) {
    if (typeof valuePacked !== "object") {
      throw new Error(`unexpected value ${JSON.stringify(valuePacked)} for type ${describeTypeIdentity(type)}`);
    }
    return fromRobustJson(valuePacked as unknown as AnyStructData);
  } else {
    throw new Error(`cannot unpack value of type ${describeTypeIdentity(type)}`);
  }
}

function _packObjectScalar(
  value: ScalarValue,
  type: TypeIdentity,
  graph: ReadNodeGraph,
): { valuePacked: JsonValue; secretValuePacked: JsonValue | undefined } {
  throw new Error(`nocheckin: packObjectScalar`);
}

function _unpackObjectScalar(
  valuePacked: JsonValue,
  secretValuePacked: JsonValue | undefined,
  type: TypeIdentity,
  graph: ReadNodeGraph,
): ScalarValue {
  throw new Error(`nocheckin: unpackObjectScalar`);
}

/**
 * Pack the value into robust wire format.
 * If previous is passed, old values with different types will be retained.
 * TODO :Incomplete: handle :SecretValues
 * */
export function packValue(
  value: any,
  type: TypeIdentity,
  graph: ReadNodeGraph,
  previous?: { valuePacked?: JsonValue; secretValuePacked?: JsonValue | undefined },
): { valuePacked: JsonValue; secretValuePacked: JsonValue | undefined } {
  // nocheckin: packValue
  if (type.kind == TypeKind.ALIAS) {
    throw new Error(`unresolved type ${describeTypeIdentity(type)}`);
  } else if (type.kind == TypeKind.OBJECT) {
    // nested object
    if (!type.isList) {
      return _packObjectScalar(value, type, graph);
    } else {
      const valuePacked: JsonValue[] = [];
      const secretValuePacked: JsonValue[] = [];
      for (let i = 0; i < value.length; i++) {
        const packed = _packObjectScalar(value[i], type, graph);
        valuePacked.push(packed.valuePacked);
        if (packed.secretValuePacked != null) secretValuePacked.push(packed.secretValuePacked);
      }
      return { valuePacked, secretValuePacked: secretValuePacked.length > 0 ? secretValuePacked : undefined };
    }
  } else {
    // wrap scalar
    let valuePacked;
    if (value == null) {
      valuePacked = null;
    } else if (!type.isList) {
      valuePacked = _packValueScalar(value, type);
    } else {
      valuePacked = value.map((v: any) => _packValueScalar(v, type));
    }
    valuePacked = { [encodeTypeIdentity(type)]: valuePacked };
    return { valuePacked, secretValuePacked: undefined };
  }
}

/**
 * Unpack the value from robust wire format.
 * TODO :Incomplete: handle :SecretValues
 */
export function unpackValue(
  packed: { valuePacked?: JsonValue; secretValuePacked?: JsonValue },
  type: TypeIdentity,
  graph: ReadNodeGraph,
): any {
  if (type.kind == TypeKind.ALIAS) {
    throw new Error(`unresolved type ${describeTypeIdentity(type)}`);
  } else if (type.kind == TypeKind.OBJECT) {
    // nested object
    if (!type.isList) {
      return _unpackObjectScalar(packed.valuePacked!, packed.secretValuePacked, type, graph);
    } else {
      if (!Array.isArray(packed.valuePacked)) {
        throw new Error(`expected array for list type ${describeTypeIdentity(type)}`);
      }
      return packed.valuePacked!.map((v: any, i: number) =>
        _unpackObjectScalar(v, (packed.secretValuePacked as Array<JsonValue>)?.[i], type, graph),
      );
    }
  } else {
    // unwrap scalar
    if (packed.valuePacked == null) {
      return null;
    } else if (typeof packed.valuePacked !== "object" || Array.isArray(packed.valuePacked)) {
      throw new Error(`expected object for scalar type ${describeTypeIdentity(type)}`);
    }
    const valuePacked = packed.valuePacked[encodeTypeIdentity(type)];
    if (valuePacked == null) {
      return null;
    } else if (!type.isList) {
      return _unpackValueScalar(valuePacked, type);
    } else {
      if (!Array.isArray(valuePacked)) {
        throw new Error(`expected array for list type ${describeTypeIdentity(type)}`);
      }
      return valuePacked.map((v: any) => _unpackValueScalar(v, type));
    }
  }
}
