import {
  BenchType,
  EnumType,
  NodeType,
  ObjectType,
  PrimitiveType,
  Struct as ProtoStruct,
  StructType,
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
  "primitiveType" | "benchType" | "baseTypePtr" | "formatHint" | "isList" | "isSecret"
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

/** The 'kind' of a Type. Only for encoding for now. :TypeInfoEncoding */
enum TypeKind {
  PRIMITIVE = "p",
  STRUCT = "s",
  NODE = "n",
  ENUM = "e",
  BASE = "b",
  ALIAS = "a",
}

/**
 * Encodes the type identity into a key for storage & implicit typing.
 * Format is <kind>[id] (with id encoded as base64).
 * :TypeInfoEncoding
 */
export function encodeTypeIdentity(type: TypeIdentity): string {
  let kind: TypeKind | null = null;
  let value: string | null = null;
  if (type.primitiveType != null) {
    kind = TypeKind.PRIMITIVE;
    value = encodeB64VLQ(type.primitiveType);
  } else if (type.benchType != null) {
    if (isNodeType(type.benchType)) {
      if (type.baseTypePtr == null) {
        kind = TypeKind.NODE;
        value = encodeB64VLQ(type.benchType);
      } else {
        kind = TypeKind.BASE;
        value = `${getTkB64FromPtr(type.baseTypePtr)}${encodeB64VLQ(type.benchType)}`;
      }
    } else if (isStructType(type.benchType)) {
      kind = TypeKind.STRUCT;
      value = encodeB64VLQ(type.benchType);
    } else if (isEnumType(type.benchType)) {
      kind = TypeKind.ENUM;
      value = encodeB64VLQ(type.benchType);
    }
  } else if (type.baseTypePtr != null) {
    kind = TypeKind.ALIAS;
    value = getTkB64FromPtr(type.baseTypePtr);
  }
  if (kind == null) {
    throw new Error(`unsupported type ${type}`);
  }

  const prefix = type.isList ? kind.toUpperCase() : kind;
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
    kind = key[0].toLowerCase() as TypeKind;
  } else {
    isList = false;
    kind = key[0] as TypeKind;
  }
  const value = key.slice(1);

  if (kind === TypeKind.PRIMITIVE) {
    return { primitiveType: decodeB64VLQ(value) as PrimitiveType, isList, isSecret };
  } else if (kind === TypeKind.NODE || kind === TypeKind.STRUCT || kind === TypeKind.ENUM) {
    return { benchType: decodeB64VLQ(value) as BenchType, isList, isSecret };
  } else if (kind === TypeKind.BASE) {
    const baseTypePtr = {
      metatype: ObjectType.NODE_REFERENCE,
      type: NodeType.BLOCK,
      ck: padCkFromTkB64(value.slice(0, TK_LENGTH_B64)),
    };
    const benchType = decodeB64VLQ(value.slice(TK_LENGTH_B64)) as BenchType;
    return { baseTypePtr, benchType, isList, isSecret };
  } else if (kind === TypeKind.ALIAS) {
    const baseTypePtr = { metatype: ObjectType.NODE_REFERENCE, type: NodeType.BLOCK, ck: padCkFromTkB64(value) };
    return { baseTypePtr, isList, isSecret };
  } else {
    throw new Error(`unsupported type kind ${kind}`);
  }
}

/** Pack the value into robust wire format. If previous is passed, old values with different types will be retained. */
export function packValue(
  value: any,
  type: TypeIdentity,
  graph: ReadNodeGraph,
  previous?: { valuePacked?: ProtoStruct; secretValuePacked?: ProtoStruct | undefined },
): { valuePacked: ProtoStruct; secretValuePacked: ProtoStruct | undefined } {
  // nocheckin: packValue
  const identityKey = encodeTypeIdentity(type);
  const valuePacked = { [identityKey]: value };
  const secretValuePacked = undefined; // TODO :Incomplete: handle :SecretValues

  // merge in previous values
  if (previous?.valuePacked != null) {
    const previousValue = ProtoStruct.toJson(previous.valuePacked) as any;
    for (const key in previousValue) {
      if (key !== identityKey) {
        valuePacked[key] = previousValue[key];
      }
    }
  }

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
