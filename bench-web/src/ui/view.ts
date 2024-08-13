import {
  BenchType,
  FieldData,
  FieldZone,
  FileType,
  NodeType,
  PrimitiveType,
  TypeConstraintData,
  TypeInfoData,
  TypeKind,
  Variant,
  ViewType,
  type AnyNodeData,
} from "@/proto/wire";
import type { ReadNodeGraph } from "@/language/graph";
import { ICONS_BY_ENUM_TYPE } from "@/ui/icon";
import { isEnumType, isNodeType } from "@/language/const";
import type { ViewProps } from "@/views/common";
import { FULL_WIDTH_VIEW_TYPES, getEnumOptions } from "@/ui/inspect";
import { type TypeIdentity, makeTypeInfo, resolveType, getStorageKey } from "@/language/field";
import { unpackValue, packValue } from "@/language/value";
import type { Transaction } from "@/language/transaction";
import { toNodeRefOneOf, type SomeNodeReferenceData, type TypedNodeReferenceData } from "@/proto/wiring";

export const VIEW_TYPE_BY_BENCH_TYPE: Partial<Record<BenchType, ViewType>> = {
  [BenchType.ICON]: ViewType.ICON,
  [BenchType.CODE]: ViewType.CODE,
  [BenchType.TEXT]: ViewType.TEXT,
  [BenchType.FILE]: ViewType.FILE,
};
export const VIEW_TYPE_BY_FILE_TYPE: Partial<Record<FileType, ViewType>> = {
  [FileType.IMAGE]: ViewType.IMAGE,
  [FileType.AUDIO]: ViewType.AUDIO,
  [FileType.VIDEO]: ViewType.VIDEO,
  [FileType.DOCUMENT]: ViewType.DOCUMENT,
};
export const FILE_TYPE_BY_VIEW_TYPE: Partial<Record<ViewType, FileType>> = Object.fromEntries(
  Object.entries(VIEW_TYPE_BY_FILE_TYPE).map(([k, v]) => [v, k]),
);

export const VIEW_TYPE_BY_PRIMITIVE_TYPE: Partial<Record<PrimitiveType, ViewType>> = {
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

export function getViewForValueType(type: Omit<TypeIdentity, "kind"> & Partial<TypeInfoData>): {
  viewType: ViewType;
  props?: ViewProps;
} | null {
  if (type.kind == TypeKind.OBJECT) {
    // object
    return { viewType: ViewType.OBJECT, props: { valueType: type as TypeInfoData } };
  } else if (type.benchType != null) {
    if (VIEW_TYPE_BY_BENCH_TYPE[type.benchType] != null) {
      if (
        type.benchType == BenchType.FILE &&
        type.constraint?.fileType != null &&
        VIEW_TYPE_BY_FILE_TYPE[type.constraint.fileType] != null
      ) {
        // specific file type view
        return {
          viewType: VIEW_TYPE_BY_FILE_TYPE[type.constraint.fileType]!,
          props: {
            valueType: makeTypeInfo(type),
            isInline: [FileType.IMAGE, FileType.AUDIO, FileType.VIDEO].includes(type.constraint.fileType),
          },
        };
      } else if (type.benchType == BenchType.FILE) {
        // generic file type view
        return { viewType: ViewType.FILE, props: { valueType: makeTypeInfo(type), isInline: true } };
      }

      // specific bench type view
      return { viewType: VIEW_TYPE_BY_BENCH_TYPE[type.benchType]!, props: { valueType: makeTypeInfo(type) } };
    } else if (isEnumType(type.benchType)) {
      // enum type -> picker
      if (getEnumOptions(type.benchType).length <= 5 && ICONS_BY_ENUM_TYPE[type.benchType] != null) {
        // prefer inline picker for small enums
        const variant = ICONS_BY_ENUM_TYPE[type.benchType] != null ? Variant.STEALTH : Variant.COMPACT;
        return {
          viewType: ViewType.PICKER,
          props: { valueType: makeTypeInfo(type), variant, isInline: true },
        };
      } else {
        // regular picker
        return { viewType: ViewType.PICKER, props: { valueType: makeTypeInfo(type) } };
      }
    } else if (isNodeType(type.benchType)) {
      // node picker
      return { viewType: ViewType.PICKER, props: { valueType: makeTypeInfo(type) } };
    }
  } else if (type.primitiveType != null && VIEW_TYPE_BY_PRIMITIVE_TYPE[type.primitiveType] != null) {
    // primitive
    return { viewType: VIEW_TYPE_BY_PRIMITIVE_TYPE[type.primitiveType!]!, props: { valueType: makeTypeInfo(type) } };
  }

  return null;
}

export type FieldView = {
  field: FieldData;
  fieldType: TypeIdentity;
  storageKey: string;
  isSet: boolean;
  value: any;
  prepareUpdate: (value: any) => Record<string, any>;
  viewType?: ViewType;
  viewProps?: any;
  isFullWidth?: boolean;
};

/** View the values of a value object */
export function getFieldViews(
  fields: FieldData[],
  objectValuePacked: Record<string, any>,
  graph: ReadNodeGraph,
  options?: { zones?: FieldZone[]; isInput?: boolean },
): FieldView[] {
  const fieldViews: FieldView[] = [];
  for (const field of fields) {
    if (options?.zones != null && !options.zones.includes(field.zone)) continue;
    const fieldType = resolveType(field, graph);
    const storageKey = getStorageKey(field, fieldType);
    let value;
    if (fieldType.kind == TypeKind.OBJECT) {
      value = objectValuePacked?.[storageKey]; // keep packed for object types
    } else {
      value = unpackValue(objectValuePacked?.[storageKey], field, {
        graph: graph,
        unwrapScalar: false,
        recurseValueObject: false,
      });
    }
    const prepareUpdate = (newValue: any) => ({
      ...objectValuePacked,
      [storageKey]: packValue(newValue, field, { graph: graph, wrapScalar: false, recurseValueObject: false }),
    });
    const isSet = value != null && !(Array.isArray(value) && value.length === 0);
    const view = getViewForValueType(fieldType);
    fieldViews.push({
      field,
      fieldType,
      storageKey,
      isSet,
      value,
      prepareUpdate,
      viewType: view?.viewType,
      viewProps: { ...view?.props, isInput: options?.isInput },
      isFullWidth: FULL_WIDTH_VIEW_TYPES.includes(view?.viewType!),
    });
  }
  return fieldViews;
}

/** Turn a type constraint into props for an Html input element */
export function getNativeConstraintProps(constraint?: Partial<TypeConstraintData>) {
  if (constraint == null) return {};
  const props: Partial<Pick<HTMLInputElement, "minLength" | "maxLength" | "pattern" | "min" | "max">> = {};
  if (constraint.minLength != null) {
    props.minLength = constraint.minLength;
  }
  if (constraint.maxLength != null) {
    props.maxLength = constraint.maxLength;
  }
  if (constraint.regex != null) {
    props.pattern = constraint.regex;
  }
  if (constraint.minValue != null) {
    props.min = constraint.minValue.toString();
  }
  if (constraint.maxValue != null) {
    props.max = constraint.maxValue.toString();
  }
  return props;
}

/**
 * Guards an event listener with a constraint
 * Forward the value if it passes, otherwise revert the event target to the old value
 */
export function guardNativeInput<T extends string | number>(
  constraint: Partial<TypeConstraintData> | undefined,
  event: Event,
  oldValue: T | undefined,
  onAccept: (T: string) => void,
) {
  if (constraint == null) {
    onAccept((event.target as HTMLInputElement).value);
    return;
  }
  const input = event.target as HTMLInputElement;
  const newValue = (input.value ?? "") as string;
  let isValid = true;
  if (constraint.minLength != null && newValue.length < constraint.minLength) {
    isValid = false;
  } else if (constraint.maxLength != null && newValue.length > constraint.maxLength) {
    isValid = false;
  } else if (constraint.regex != null && !new RegExp(constraint.regex).test(newValue)) {
    isValid = false;
  } else if (constraint.minValue != null && parseFloat(newValue) < constraint.minValue) {
    isValid = false;
  } else if (constraint.maxValue != null && parseFloat(newValue) > constraint.maxValue) {
    isValid = false;
  }
  if (!isValid) {
    input.value = oldValue?.toString() ?? "";
  } else {
    onAccept(newValue);
  }
}

/** Set or unset the pinned 'nodePtr' for a Helper View (they normally default to some active node or some other empty state). */
export function toggleHelperViewPin(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: { self: TypedNodeReferenceData<NodeType.VIEW>; nodePtr: AnyNodeData | SomeNodeReferenceData | null },
) {
  const self = graph.getOrError(options.self);
  if (self.nodePtr?.oneofKind == null) {
    if (options.nodePtr == null) return; // shouldn't happen?
    // strip title so it's dynamically tied to the node
    tx.update(self, { nodePtr: toNodeRefOneOf(options.nodePtr), title: undefined }, { debounce: "tick" });
  } else {
    const title = self.name?.replace(/\d+$/, ""); // title = name without postfix numbers
    tx.update(self, { nodePtr: { oneofKind: undefined }, title }, { debounce: "tick" });
  }
}
