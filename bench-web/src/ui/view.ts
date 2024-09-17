import { isEnumType, isNodeType, NAME_CONSTRAINT } from "@/language/const";
import { getPropertyType, getStorageKey, makeTypeInfo, resolveType, type TypeIdentity } from "@/language/field";
import type { ReadNodeGraph } from "@/language/graph";
import { type DebounceLevel, type Transaction } from "@/language/transaction";
import { packBuiltinObject, packValue, unpackBuiltinObject, unpackValue } from "@/language/value";
import {
  Anchor,
  BenchType,
  BlockProperty,
  FieldData,
  FieldZone,
  FileType,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PrimitiveType,
  PROPERTY_INFOS_BY_TYPE,
  SelectionData,
  SelectionKind,
  SelectionTarget,
  StructType,
  TransformData,
  TypeConstraintData,
  TypeInfoData,
  TypeKind,
  Variant,
  Vector2Data,
  Vector3Data,
  Vector4Data,
  ViewData,
  ViewProperty,
  ViewType,
  type AnyNodeData,
  type AnyNodeReferenceData,
  type AnyTypeMapping,
} from "@/proto/wire";
import {
  isNodeRef,
  isProtoJson,
  makeStruct,
  packProtoJson,
  toNodeRef,
  toNodeRefOneOf,
  typeNodeReferenceMaybe,
  unpackProtoJson,
  type SomeNodeReferenceData,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { ICONS_BY_ENUM_TYPE } from "@/ui/icon";
import { FULL_WIDTH_VIEW_TYPES, getEnumOptions } from "@/ui/inspect";
import { isFocusableElement } from "@/utils/element";
import { computedValue } from "@/utils/ref";
import { getViewTypeByComponentName, type ViewComponent, type ViewProps } from "@/views/common";
import type { MaybeElement } from "@vueuse/core";
import { computed, toRef, type ComponentInstance, type MaybeRef, type Ref } from "vue";

export const SPACE_DEFAULT_BAR_POSITION = Anchor.TOP;
export const VIEW_DEFAULT_HEADER_HEIGHT = 36;
export const VIEW_DEFAULT_MIN_WIDTH = 320;
export const VIEW_DEFAULT_MAX_WIDTH = 800;

export type Vector2 = Omit<Vector2Data, "metatype">;
export type Vector3 = Omit<Vector3Data, "metatype">;
export type Vector4 = Omit<Vector4Data, "metatype">;

export function getVueComponentType(component: ComponentInstance<any>): string {
  return component.__name ?? (component as any).type.__name;
}

export function describeVueComponent(component: ComponentInstance<any>): string {
  const id = (component as any).exposed?.self?.value?.id ?? (component as any).exposed?.id?.value;
  return `${getVueComponentType(component)}:${id}`;
}

/** Gets a top down 'path' of a vue component (like Space:id->Split:id->Tabbed:id->Button:id) */
export function describeVueComponentPath(component: ComponentInstance<any>): string {
  const components = collectViewComponentsUp(component).reverse();
  return components.map((c) => getVueComponentType(c) + ":" + getViewComponentId(c)).join("->");
}

export function isVueComponent(component: ComponentInstance<any>): boolean {
  return (component as any).uid != null;
}

export function isVueInstanceOf(component: ComponentInstance<any>, type: string | { __name?: string }): boolean {
  const componentType = (component as any).type;
  return typeof type === "string" ? componentType.__name === type : componentType === type;
}

export function isViewComponent(component: ComponentInstance<any>): component is ViewComponent {
  return (component as any).exposed?.self != null || (component as any).exposed?.id != null;
}

export function isIdentifiedViewComponent(
  component: ComponentInstance<any>,
): component is ViewComponent & { exposed: { self: Ref<NodeReferenceData> } } {
  return (component as any).exposed?.self?.value != null;
}

export function isViewComponentIn(component: ViewComponent, viewTypes: Set<ViewType>): boolean {
  const componentType = getVueComponentType(component);
  const viewType = getViewTypeByComponentName(componentType);
  if (viewType == null) throw new Error(`no view type for component: ${componentType}`);
  return viewTypes.has(viewType);
}

export function getViewComponentId(component: ViewComponent): string {
  if (component.exposed?.self?.value != null) return component.exposed.self.value.id!;
  else if (component.exposed?.id?.value != null) return component.exposed.id.value;
  else throw new Error(`no id on component ${getVueComponentType(component)}: ${component}`);
}

/** Finds the closest ViewComponent ancestor. */
export function findViewComponentUp(
  el: HTMLElement | SVGElement | ComponentInstance<any>,
  where?: (component: ViewComponent) => boolean,
): ViewComponent | null {
  while (el != null) {
    if (el instanceof HTMLElement || el instanceof SVGElement) {
      // first find vue component
      if ((el as any).__viewComponent != null) el = (el as any).__viewComponent;
      else el = el.parentElement!;
    } else {
      if (where == null || where(el)) return el;
      else el = el.parent;
    }
  }
  return null;
}

/** Collect all view components from the given component upwards (inclusive) */
export function collectViewComponentsUp(
  componentOrEl: ComponentInstance<any> | HTMLElement | SVGElement,
): ViewComponent[] {
  let component =
    componentOrEl instanceof HTMLElement || componentOrEl instanceof SVGElement
      ? findViewComponentUp(componentOrEl)
      : componentOrEl;
  const components = [];
  while (component != null) {
    if (isViewComponent(component)) components.push(component);
    component = component.parent;
  }
  return components;
}

/**
 * Gets all child View components of a given component in DOM order.
 * Walks the DOM descendants until the first layer of child components.
 * */
export function getViewComponentChildren(instance: ComponentInstance<any>): ViewComponent[] {
  const elements = [instance.subTree.el];
  const components = [];

  // traverse the DOM
  while (elements.length > 0) {
    const el = elements.pop()!;
    for (const child of el.children) {
      if (child instanceof HTMLElement) {
        const component = (child as any).__viewComponent as ComponentInstance<any> | null;
        if (component != null && component !== instance && isViewComponent(component)) components.push(component);
        else elements.push(child);
      }
    }
  }

  return components;
}

export function getViewComponentPtrMaybe(
  component: ViewComponent | null | undefined,
): TypedNodeReferenceData<NodeType.VIEW> | null {
  return typeNodeReferenceMaybe(NodeType.VIEW, component?.exposed.self?.value ?? null);
}

/** Traverses the DOM up to check if any element is marked as outside any view */
export function isOutsideView(el: HTMLElement | SVGElement): boolean {
  while (el != null) {
    if (el.hasAttribute("data-outside-view")) return true;
    el = el.parentElement!;
  }
  return false;
}

export function focusInElement(element: MaybeElement): boolean {
  while (element != null) {
    if (element instanceof HTMLElement || element instanceof SVGElement) {
      if (!isFocusableElement(element)) {
        return false; // don't try to magically find a focusable element, this shouldbe explicit
      } else {
        element.focus();
      }
      return true;
    } else if ("focus" in element) {
      const focusResult = (element as any).focus();
      if (focusResult === true || focusResult === undefined) return true;
      else element = focusResult;
    } else {
      return false;
    }
  }
  return false;
}

export const VIEW_TYPE_BY_BENCH_TYPE: Partial<Record<BenchType, ViewType>> = {
  [BenchType.ICON]: ViewType.ICON,
  [BenchType.CODE]: ViewType.CODE,
  [BenchType.TEXT]: ViewType.TEXT,
  [BenchType.FILE]: ViewType.FILE,
  [BenchType.COLOR]: ViewType.COLOR,
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
export const LISTABLE_VIEW_TYPES = new Set([ViewType.PICKER, ViewType.OBJECT, ViewType.STRING, ViewType.NUMBER]); // nocheckin: list pickers

export function getViewForValueType(type: Omit<TypeIdentity, "kind"> & Partial<TypeInfoData>): ViewProps | null {
  if (type.kind == TypeKind.OBJECT) {
    // object
    return { type: ViewType.OBJECT, valueType: type as TypeInfoData };
  } else if (type.benchType != null) {
    if (VIEW_TYPE_BY_BENCH_TYPE[type.benchType] != null) {
      if (
        type.benchType == BenchType.FILE &&
        type.constraint?.fileType != null &&
        VIEW_TYPE_BY_FILE_TYPE[type.constraint.fileType] != null
      ) {
        // specific file type view
        return {
          type: VIEW_TYPE_BY_FILE_TYPE[type.constraint.fileType]!,
          valueType: makeTypeInfo(type),
          isInline: [FileType.IMAGE, FileType.AUDIO, FileType.VIDEO].includes(type.constraint.fileType),
        };
      } else if (type.benchType == BenchType.FILE) {
        // generic file type view
        return { type: ViewType.FILE, valueType: makeTypeInfo(type), isInline: true };
      }

      // specific bench type view
      return { type: VIEW_TYPE_BY_BENCH_TYPE[type.benchType]!, valueType: makeTypeInfo(type) };
    } else if (isEnumType(type.benchType)) {
      // enum type -> picker
      if (!type.isList && getEnumOptions(type.benchType).length <= 5 && ICONS_BY_ENUM_TYPE[type.benchType] != null) {
        // prefer inline picker for small scalar enums
        const variant = ICONS_BY_ENUM_TYPE[type.benchType] != null ? Variant.STEALTH : Variant.COMPACT;
        return { type: ViewType.PICKER, valueType: makeTypeInfo(type), variant, isInline: true };
      } else {
        // regular picker
        return { type: ViewType.PICKER, valueType: makeTypeInfo(type) };
      }
    } else if (isNodeType(type.benchType)) {
      // node picker
      return { type: ViewType.PICKER, valueType: makeTypeInfo(type) };
    }
  } else if (type.primitiveType != null && VIEW_TYPE_BY_PRIMITIVE_TYPE[type.primitiveType] != null) {
    // primitive
    return { type: VIEW_TYPE_BY_PRIMITIVE_TYPE[type.primitiveType!]!, valueType: makeTypeInfo(type) };
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
      viewType: view?.type,
      viewProps: { ...view, isInput: options?.isInput },
      isFullWidth: FULL_WIDTH_VIEW_TYPES.includes(view?.type!),
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
 * Guards and coerces an event listener with a constraint.
 * Forward the value if it passes, otherwise revert the event target to the old value
 */
export function guardNativeInput<T extends string | number | bigint>(
  type: TypeIdentity,
  constraint: Partial<TypeConstraintData> | undefined,
  event: Event,
  oldValue: T | undefined,
  onAccept: (T: string | number) => void,
) {
  // coerce
  let newValue: string | number = (event.target as HTMLInputElement).value ?? "";
  if (
    type.kind == TypeKind.PRIMITIVE &&
    [
      PrimitiveType.INT16,
      PrimitiveType.INT32,
      PrimitiveType.INT64,
      PrimitiveType.FLOAT32,
      PrimitiveType.FLOAT64,
    ].includes(type.primitiveType!)
  ) {
    newValue = parseFloat(newValue);
    if (isNaN(newValue)) {
      newValue = 0;
    }
  }

  if (constraint == null) {
    // nothing to check
    onAccept(newValue);
    return;
  }

  // check
  let isValid = true;
  if (typeof newValue == "string") {
    if (constraint.minLength != null && newValue.length < constraint.minLength) {
      isValid = false;
    } else if (constraint.maxLength != null && newValue.length > constraint.maxLength) {
      isValid = false;
    } else if (constraint.regex != null && !new RegExp(constraint.regex, "u").test(newValue)) {
      isValid = false;
    }
  }
  if (typeof newValue == "number") {
    if (constraint.minValue != null && newValue < constraint.minValue) {
      isValid = false;
    } else if (constraint.maxValue != null && newValue > constraint.maxValue) {
      isValid = false;
    }
  }
  if (!isValid) {
    const input = event.target as HTMLInputElement;
    input.value = oldValue?.toString() ?? "";
  } else {
    onAccept(newValue);
  }
}
export function guardNativeNameInput(event: Event, oldValue: string | undefined, onAccept: (name: string) => void) {
  return guardNativeInput(
    getPropertyType(PROPERTY_INFOS_BY_TYPE[ObjectType.BLOCK][BlockProperty.name]),
    NAME_CONSTRAINT,
    event,
    oldValue,
    (newValue: any) => onAccept(newValue as string),
  );
}
export function guardNativeTitleInput(event: Event, oldValue: string | undefined, onAccept: (title: string) => void) {
  return guardNativeInput(
    getPropertyType(PROPERTY_INFOS_BY_TYPE[ObjectType.VIEW][ViewProperty.title]),
    NAME_CONSTRAINT,
    event,
    oldValue,
    (newValue: any) => onAccept(newValue as string),
  );
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

export function makeSelection(
  nodes: AnyNodeData | AnyNodeReferenceData | (AnyNodeData | AnyNodeReferenceData)[],
): SelectionData {
  nodes = Array.isArray(nodes) ? nodes : [nodes];
  return {
    metatype: ObjectType.SELECTION,
    target: SelectionTarget.NODE,
    kind: SelectionKind.LIST,
    nodesPtr: nodes.map((n) => (isNodeRef(n) ? n : toNodeRef(n as AnyNodeData))),
  };
}

export function makeSelectionMaybe(
  nodes: AnyNodeData | AnyNodeReferenceData | (AnyNodeData | AnyNodeReferenceData)[] | null | undefined,
): SelectionData | undefined {
  if (nodes == null) return undefined;
  return makeSelection(Array.isArray(nodes) ? nodes : [nodes]);
}

export function expandSelection(
  selection: SelectionData | undefined | null,
  nodes: (AnyNodeData | NodeReferenceData)[],
): SelectionData {
  return {
    ...(selection ?? { metatype: ObjectType.SELECTION, target: SelectionTarget.NODE, kind: SelectionKind.LIST }),
    nodesPtr: [...(selection?.nodesPtr ?? []), ...nodes.map((n) => (isNodeRef(n) ? n : toNodeRef(n as AnyNodeData)))],
  };
}

export function collapseSelection(selection: SelectionData, nodes: (AnyNodeData | NodeReferenceData)[]): SelectionData {
  return {
    ...selection,
    nodesPtr: selection.nodesPtr.filter((n) => !nodes.some((m) => m.id == n.id)),
  };
}

export function useViewExpansion(options: {
  graph: ReadNodeGraph;
  tx: () => Transaction;
  self?: Ref<AnyNodeReferenceData | null | undefined>;
  props: Pick<ViewData, "expansion">;
  emit: (event: string, ...args: any[]) => void;
  isDefaultExpanded?: MaybeRef<boolean | undefined>;
  isExclusive?: boolean;
}) {
  const isDefaultExpandedRef = toRef(options.isDefaultExpanded) as Ref<boolean>;
  const expandedNodesById = computedValue(() => {
    const expanded: Record<string, NodeReferenceData> = {};
    for (const node of options.props.expansion?.nodesPtr ?? []) {
      expanded[node.id!] = node;
    }
    return expanded;
  });

  function isExpanded(node: { id?: string; ck?: string }): boolean {
    return isDefaultExpandedRef.value || expandedNodesById.value[node.id!] != null;
  }

  function toggleExpanded(node: AnyNodeData | AnyNodeReferenceData) {
    if (isDefaultExpandedRef.value) return; // nothing to do

    let newExpansion: SelectionData | null;
    if (isExpanded(node)) {
      newExpansion = collapseSelection(options.props.expansion!, [node]);
    } else {
      if (options.isExclusive) {
        newExpansion = makeSelection([node]);
      } else {
        newExpansion = expandSelection(options.props.expansion, [node]);
      }
    }
    if (options.self?.value != null) {
      const self = options.graph.getOrError(options.self.value!);
      options.tx().update(self, { expansion: newExpansion }, { debounce: "tick" });
    } else {
      options.emit("update:self", { expansion: newExpansion });
    }
  }

  return { toggleExpanded, isExpanded };
}

export function addTransform(transform: TransformData | null | undefined, add: Partial<TransformData>) {
  if (transform == null) {
    return makeStruct({ ...add, metatype: StructType.TRANSFORM });
  }
  const updated: TransformData = transform != null ? { ...transform } : { metatype: ObjectType.TRANSFORM };
  for (const key in add) {
    if (key == "metatype") continue;
    (updated as any)[key] = ((transform as any)[key] ?? 0) + (add as any)[key];
  }
  return updated;
}

export function addVector2(vec: Vector2Data | null | undefined, add: Partial<Vector2Data>): Vector2Data {
  return {
    metatype: ObjectType.VECTOR2,
    x: (vec?.x ?? 0) + (add.x ?? 0),
    y: (vec?.y ?? 0) + (add.y ?? 0),
  };
}

/**
 * Use the typed state in the View.value of a builtin view type.
 **/
export function useViewState<T extends ObjectType>(use: {
  selfPtr: Ref<TypedNodeReferenceData<NodeType.VIEW> | undefined | null>;
  graph: ReadNodeGraph;
  stateType: T;
  props: Pick<ViewData, "valuePacked">;
  emit: (event: string, ...args: any[]) => void;
}) {
  const state = computedValue(() => {
    if (use.props.valuePacked == null) {
      return { metatype: use.stateType } as AnyTypeMapping[T];
    } else {
      // we don't pack proto json structs inside proto json structs,
      //  so while valuePacked should be a proto struct (as per the type) it may not be (see :ProtoStructMapping)
      const valuePacked = isProtoJson(use.props.valuePacked)
        ? unpackProtoJson(use.props.valuePacked)
        : use.props.valuePacked;
      const unpacked = unpackBuiltinObject(valuePacked, use.stateType);
      return unpacked;
    }
  });

  function updateState(tx: Transaction, value: Partial<AnyTypeMapping[T]>, options?: { debounce?: DebounceLevel }) {
    const valuePacked = packStateUpdate(value);
    if (use.selfPtr.value != null) {
      const self = use.graph.getOrError(use.selfPtr.value);
      tx.update(self, { valuePacked: packProtoJson(valuePacked) }, options);
    } else {
      use.emit("update:self", { valuePacked: packProtoJson(valuePacked) });
    }
  }

  function packStateUpdate(value: Partial<AnyTypeMapping[T]>) {
    const newState = { ...state.value, ...value } as AnyTypeMapping[T];
    const valuePacked = packBuiltinObject(newState);
    return valuePacked;
  }

  function useStateProp<P extends keyof AnyTypeMapping[T]>(
    txFactory: () => Transaction,
    prop: P,
    defaultValue: AnyTypeMapping[T][P],
    options?: { debounce?: DebounceLevel },
  ): Ref<Required<AnyTypeMapping[T]>[P]>;
  function useStateProp<P extends keyof AnyTypeMapping[T]>(
    txFactory: () => Transaction,
    prop: P,
  ): Ref<AnyTypeMapping[T][P] | undefined>;
  function useStateProp<P extends keyof AnyTypeMapping[T]>(
    txFactory: () => Transaction,
    prop: P,
    defaultValue?: AnyTypeMapping[T][P],
    options?: { debounce?: DebounceLevel },
  ): Ref<AnyTypeMapping[T][P]> {
    return computed({
      get: () => (state.value?.[prop] ?? defaultValue) as any,
      set: (value: AnyTypeMapping[T][P]) => updateState(txFactory(), { [prop]: value } as any, options),
    });
  }

  return { state, updateState, packStateUpdate, useStateProp };
}
