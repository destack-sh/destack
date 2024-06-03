import { BenchType, ViewType, PrimitiveType, TypeInfoData, TypeKind, Variant } from "@/proto/wire";
import { ENUM_ICONS_BY_TYPE } from "@/system/icon";
import { isEnumType, getEnumOptions, isNodeType } from "@/system/lang";
import { type TypeIdentity, makeTypeInfo } from "@/system/value";
import type { ViewProps } from "@/views/common";

const VIEW_TYPE_BY_BENCH_TYPE: Partial<Record<BenchType, ViewType>> = {
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

export function getViewForValueType(type: TypeIdentity & Partial<TypeInfoData>): {
  viewType: ViewType;
  props?: ViewProps;
} | null {
  if (type.kind == TypeKind.OBJECT) {
    return { viewType: ViewType.OBJECT, props: { valueType: type as TypeInfoData } };
  } else if (type.benchType != null) {
    if (VIEW_TYPE_BY_BENCH_TYPE[type.benchType] != null) {
      return { viewType: VIEW_TYPE_BY_BENCH_TYPE[type.benchType]! };
    } else if (isEnumType(type.benchType)) {
      // prefer inline picker if possible
      if (getEnumOptions(type.benchType).length <= 5) {
        const variant = ENUM_ICONS_BY_TYPE[type.benchType] != null ? Variant.STEALTH : Variant.COMPACT;
        return {
          viewType: ViewType.PICKER,
          props: { valueType: makeTypeInfo(type), variant, isInline: true },
        };
      } else {
        return { viewType: ViewType.PICKER, props: { valueType: makeTypeInfo(type) } };
      }
    } else if (isNodeType(type.benchType)) {
      return { viewType: ViewType.PICKER, props: { valueType: makeTypeInfo(type) } };
    }
  } else if (VIEW_TYPE_BY_PRIMITIVE_TYPE[type.primitiveType!] != null) {
    return { viewType: VIEW_TYPE_BY_PRIMITIVE_TYPE[type.primitiveType!]! };
  }

  return null;
}
