import {
  BenchType,
  EnumType,
  PrimitiveType,
  Struct as ProtoStruct,
  StructType,
  Variant,
  ViewType,
  type TypeInfoData,
} from "@/proto/wire";
import { makeDefaultStruct } from "@/proto/wiring";
import { ENUM_ICONS_BY_TYPE } from "@/system/icon";
import { getEnumOptions, isEnumType } from "@/system/lang";
import type { ViewProps } from "@/views/common";

export type TypeIdentity = Pick<TypeInfoData, "primitiveType" | "benchType" | "baseTypePtr" | "formatHint">;

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

/** Pack the value into robust wire format. */
export function packValue(value: any, type: TypeInfoData): ProtoStruct {
  // nocheckin: store encoded (use field/type key, even for single Variable so we can change types)
  return ProtoStruct.fromJson({ test: value });
}

/** Unpack the value from robust wire format. */
export function unpackValue(value: ProtoStruct | undefined | null, type: TypeInfoData): any {
  if (value == null) return null;
  return (ProtoStruct.toJson(value) as any)?.test; // nocheckin
}
