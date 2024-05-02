import {
  BenchType,
  EnumType,
  NodeType,
  ObjectType,
  PrimitiveType,
  Struct as ProtoStruct,
  StructType,
  TypeKind,
  Variant,
  ViewType,
  type TypeInfoData,
} from "@/proto/wire";
import { makeDefaultStruct } from "@/proto/wiring";
import type { ReadNodeGraph } from "@/system/graph";
import { ENUM_ICONS_BY_TYPE } from "@/system/icon";
import {
  TK_LENGTH_B64,
  getEnumOptions,
  getTkB64FromPtr,
  isEnumType,
  isNodeType,
  isStructType,
  padCkFromTkB64,
} from "@/system/lang";
import { decodeB64VLQ, encodeB64VLQ } from "@/utils/functools";
import type { ViewProps } from "@/views/common";

export type TypeIdentity = Pick<
  TypeInfoData,
  "kind" | "primitiveType" | "benchType" | "baseTypePtr" | "formatHint" | "isList" | "isSecret"
> & { ck?: string };

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

/** Pack the value into robust wire format. If previous is passed, old values with different types will be retained. */
export function packValue(
  value: any,
  type: TypeIdentity,
  graph: ReadNodeGraph,
  previous?: { valuePacked?: ProtoStruct; secretValuePacked?: ProtoStruct | undefined },
): { valuePacked: ProtoStruct; secretValuePacked: ProtoStruct | undefined } {
  // nocheckin: packValue
  const identityKey = encodeTypeIdentity(type);
  const valuePacked = {
    ...(previous?.valuePacked != null ? (ProtoStruct.toJson(previous.valuePacked) as object) : {}),
    [identityKey]: value,
  };
  const secretValuePacked = undefined; // TODO :Incomplete: handle :SecretValues

  const packed = { valuePacked: ProtoStruct.fromJson(valuePacked), secretValuePacked: undefined };
  return packed;
}

/** Unpack the value from robust wire format. */
export function unpackValue(
  packed: { valuePacked?: ProtoStruct; secretValuePacked?: ProtoStruct },
  type: TypeIdentity,
  graph: ReadNodeGraph,
): any {
  // nocheckin: unpackValue
  if (packed.valuePacked == null) return null;
  const valuePacked = ProtoStruct.toJson(packed.valuePacked) as any;
  const identityKey = encodeTypeIdentity(type);
  const unpacked = valuePacked[identityKey];
  return unpacked;
}
