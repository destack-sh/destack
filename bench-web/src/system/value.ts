import { BenchType, PrimitiveType, StructType, ViewType, type TypeInfoData } from "@/proto/wire";
import { makeDefaultStruct } from "@/proto/wiring";
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

export function getViewComponentForValueType(type: Pick<TypeInfoData, "primitiveType" | "benchType" | "baseTypePtr">): {
  viewType: ViewType;
  props?: ViewProps;
} {
  if (type.baseTypePtr != null) {
    return { viewType: ViewType.PICKER };
  } else if (type.benchType != null) {
    if (VIEW_TYPE_BY_BENCH_TYPE[type.benchType] != null) return { viewType: VIEW_TYPE_BY_BENCH_TYPE[type.benchType]! };
    else return { viewType: ViewType.PICKER, props: { valueType: makeTypeInfo(type) } };
  } else if (VIEW_TYPE_BY_PRIMITIVE_TYPE[type.primitiveType!] != null) {
    return { viewType: VIEW_TYPE_BY_PRIMITIVE_TYPE[type.primitiveType!]! };
  } else {
    throw new Error(`no view component for type: ${type}`);
  }
}
