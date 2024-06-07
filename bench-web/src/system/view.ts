import {
  BenchType,
  ViewType,
  PrimitiveType,
  TypeInfoData,
  TypeKind,
  Variant,
  FieldData,
  FieldZone,
} from "@/proto/wire";
import type { ReadNodeGraph } from "@/system/graph";
import { ENUM_ICONS_BY_TYPE } from "@/system/icon";
import { isEnumType, getEnumOptions, isNodeType, FULL_WIDTH_VIEW_TYPES } from "@/system/lang";
import { type TypeIdentity, makeTypeInfo, resolveType, getStorageKey } from "@/system/value";
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
      // prefer inline picker if it fits
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

export type FieldView = {
  field: FieldData;
  fieldType: TypeIdentity;
  storageKey: string;
  isSet: boolean;
  value: any;
  viewType?: ViewType;
  viewProps?: any;
  isFullWidth?: boolean;
};

export function getFieldViews(
  fields: FieldData[],
  modelValue: Record<string, any>,
  pkgGraph: ReadNodeGraph,
  options?: {
    zones?: FieldZone[];
    isInput?: boolean;
  },
): FieldView[] {
  const fieldViews: FieldView[] = [];
  for (const field of fields) {
    if (options?.zones != null && !options.zones.includes(field.zone)) continue;
    const fieldType = resolveType(field, pkgGraph);
    const storageKey = getStorageKey(field, fieldType);
    const fieldValue = modelValue?.[storageKey];
    const isSet = fieldValue != null && !(Array.isArray(fieldValue) && fieldValue.length === 0);
    const view = getViewForValueType(fieldType);
    fieldViews.push({
      field,
      fieldType,
      storageKey,
      isSet,
      value: fieldValue,
      viewType: view?.viewType,
      viewProps: { ...view?.props, isInput: options?.isInput },
      isFullWidth: FULL_WIDTH_VIEW_TYPES.includes(view?.viewType!),
    });
  }
  return fieldViews;
}
