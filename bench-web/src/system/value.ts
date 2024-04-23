import { BenchType, EnumType, PrimitiveType, StructType, Variant, ViewType, type TypeInfoData } from "@/proto/wire";
import { makeDefaultStruct } from "@/proto/wiring";
import { isEnumType } from "@/system/lang";
import type { ViewProps } from "@/views";

export function makeTypeInfo(partial: Partial<Omit<TypeInfoData, "metatype">>): TypeInfoData {
  return makeDefaultStruct({ metatype: StructType.TYPE_INFO, ...partial });
}

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
const COMPACT_PICKER_ENUM_TYPES: EnumType[] = [
  EnumType.REGION,
  EnumType.NODE_VISIBILITY,
  EnumType.ORIENTATION,
  EnumType.ALIGNMENT,
  EnumType.VARIANT,
];

export function getViewComponentForValueType(type: Pick<TypeInfoData, "primitiveType" | "benchType" | "baseTypePtr">): {
  viewType: ViewType;
  props?: ViewProps;
} {
  // TODO :Incomplete: getViewComponentForValueType
  if (type.benchType != null) {
    if (VIEW_TYPE_BY_BENCH_TYPE[type.benchType] != null) {
      return { viewType: VIEW_TYPE_BY_BENCH_TYPE[type.benchType]! };
    } else {
      if (isEnumType(type.benchType) && COMPACT_PICKER_ENUM_TYPES.includes(type.benchType)) {
        return {
          viewType: ViewType.PICKER,
          props: { valueType: makeTypeInfo(type), variant: Variant.COMPACT, isInline: true },
        };
      } else {
        return { viewType: ViewType.PICKER, props: { valueType: makeTypeInfo(type) } };
      }
    }
  } else if (VIEW_TYPE_BY_PRIMITIVE_TYPE[type.primitiveType!] != null) {
    return { viewType: VIEW_TYPE_BY_PRIMITIVE_TYPE[type.primitiveType!]! };
  } else {
    throw new Error(`no view component for type: ${type}`);
  }
}
