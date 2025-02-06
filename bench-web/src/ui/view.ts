import { isEnumType, isNodeType } from "@/language/core/const";
import { getEnumOptions } from "@/language/core/enum";
import { packSubnode } from "@/language/core/node";
import { makeType, type TypeIdentity } from "@/language/core/type";
import {
  Alignment,
  Anchor,
  BenchType,
  FileType,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PickerVariant,
  PrimitiveType,
  SelectionData,
  StructType,
  TransformData,
  TypeConstraintData,
  TypeData,
  TypeKind,
  Vector2Data,
  Vector3Data,
  Vector4Data,
  ViewData,
  ViewType,
  type AnyNodeData
} from "@/proto/wire";
import { isNodeRef, makeStruct, toNodeRef, typeNodeReferenceMaybe, type TypedNodeReferenceData } from "@/proto/wiring";
import { isFocusableElement } from "@/utils/element";
import { getViewTypeByComponentName, type ViewComponent, type ViewProps } from "@/views/common";
import type { MaybeElement } from "@vueuse/core";
import { type ComponentInstance, type Ref } from "vue";

export const SPACE_DEFAULT_BAR_POSITION = Anchor.TOP;
export const VIEW_DEFAULT_ROOT_HEADER_HEIGHT = 44;
export const VIEW_DEFAULT_HEADER_HEIGHT = 36;
export const VIEW_DEFAULT_MIN_WIDTH = 240;
export const VIEW_DEFAULT_MAX_WIDTH = 800;

export const TEXT_DIRECTION_BY_ALIGNMENT: Partial<Record<Alignment, string>> = {
  [Alignment.START]: "text-left",
  [Alignment.MIDDLE]: "text-center",
  [Alignment.END]: "text-right",
};

export type Vector2 = Omit<Vector2Data, "metatype">;
export type Vector3 = Omit<Vector3Data, "metatype">;
export type Vector4 = Omit<Vector4Data, "metatype">;

export function getVueComponentType(component: ComponentInstance<any>): string {
  return component.__name ?? (component as any).type?.__name ?? (component as any).type?.name;
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
  return viewType != null && viewTypes.has(viewType);
}

export function getViewComponentId(component: ViewComponent): string {
  if (component.exposed?.self?.value != null) return component.exposed.self.value.id!;
  else if (component.exposed?.id?.value != null) return component.exposed.id.value;
  else throw new Error(`no id on component ${getVueComponentType(component)}: ${component}`);
}

/** Finds the closest ViewComponent ancestor. */
export function findViewComponentUp(
  el: HTMLElement | SVGElement | ComponentInstance<any>,
  where?: ViewType[] | Set<ViewType> | ((component: ViewComponent) => boolean),
): ViewComponent | null {
  if (Array.isArray(where) || where instanceof Set) {
    const viewTypes = where instanceof Set ? where : new Set(where);
    where = (component: ViewComponent) => {
      const viewType = getViewTypeByComponentName(getVueComponentType(component));
      return viewType != null && viewTypes.has(viewType);
    };
  }
  while (el != null) {
    if (el instanceof HTMLElement || el instanceof SVGElement) {
      // first find vue component
      if ((el as any).__viewComponent != null) el = (el as any).__viewComponent;
      else el = el.parentElement!;
    } else {
      if (isViewComponent(el) && (where == null || where(el))) return el;
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
  if (instance.subTree == null) return [];
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
  [BenchType.PATH]: ViewType.PATH,
  [BenchType.TYPE]: ViewType.TYPE,
  [BenchType.COMPUTED_VALUE]: ViewType.COMPUTED_VALUE,
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
  [PrimitiveType.JSON]: ViewType.JSON,
  [PrimitiveType.DATETIME]: ViewType.DATETIME,
  [PrimitiveType.DATE]: ViewType.DATETIME,
  [PrimitiveType.TIME]: ViewType.DATETIME,
  [PrimitiveType.DURATION]: ViewType.DURATION,
};
export const LISTABLE_VIEW_TYPES = new Set([ViewType.PICKER, ViewType.OBJECT, ViewType.STRING, ViewType.NUMBER]);
export const FULL_WIDTH_VIEW_TYPES = [
  ViewType.OBJECT,
  ViewType.TEXT,
  ViewType.CODE,
  ViewType.FILE,
  ViewType.IMAGE,
  ViewType.AUDIO,
  ViewType.VIDEO,
];

/** Gets the View for some value type */
export function getViewForType(
  type: Omit<TypeIdentity, "kind"> & Partial<TypeData>,
  options?: { forcePickerDropdown?: boolean },
): ViewProps | null {
  const valueType = makeType(type);
  if (type.kind == TypeKind.CUSTOM_OBJECT || type.kind == TypeKind.PARTIAL_OBJECT) {
    // object
    return { type: ViewType.OBJECT, valueType };
  } else if (type.benchType != null || type.kind == TypeKind.NODE) {
    if (VIEW_TYPE_BY_BENCH_TYPE[type.benchType!] != null) {
      if (
        type.benchType == BenchType.FILE &&
        type.constraint?.nodeSubtypes?.length == 1 &&
        VIEW_TYPE_BY_FILE_TYPE[type.constraint.nodeSubtypes[0] as FileType] != null
      ) {
        // specific file type view
        return {
          type: VIEW_TYPE_BY_FILE_TYPE[type.constraint.nodeSubtypes[0] as FileType]!,
          valueType,
          isInline: [FileType.IMAGE, FileType.AUDIO, FileType.VIDEO].includes(
            type.constraint.nodeSubtypes[0] as FileType,
          ),
        };
      } else if (type.benchType == BenchType.FILE) {
        // generic file type view
        return { type: ViewType.FILE, valueType, isInline: true };
      }

      // specific bench type view
      return { type: VIEW_TYPE_BY_BENCH_TYPE[type.benchType!]!, valueType };
    } else if (isEnumType(type.benchType)) {
      // enum type -> picker
      if (!options?.forcePickerDropdown && !type.isList && getEnumOptions(type.benchType).length <= 5) {
        // prefer inline picker for small scalar enums
        return {
          type: ViewType.PICKER,
          valueType,
          isInline: true,
          subnodePacked: packSubnode(NodeType.VIEW, ViewType.PICKER, { variant: PickerVariant.MULTI_TOGGLE }),
        };
      } else {
        // regular picker
        return { type: ViewType.PICKER, valueType };
      }
    } else if (type.kind == TypeKind.NODE || isNodeType(type.benchType)) {
      // node picker
      return { type: ViewType.PICKER, valueType };
    }
  } else if (type.primitiveType != null && VIEW_TYPE_BY_PRIMITIVE_TYPE[type.primitiveType] != null) {
    // primitive
    return { type: VIEW_TYPE_BY_PRIMITIVE_TYPE[type.primitiveType!]!, valueType };
  }

  return null;
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

export function makeSelection(
  nodes: AnyNodeData | NodeReferenceData | (AnyNodeData | NodeReferenceData)[],
): SelectionData {
  nodes = Array.isArray(nodes) ? nodes : [nodes];
  return {
    metatype: ObjectType.SELECTION,
    nodesPtr: nodes.map((n) => (isNodeRef(n) ? n : toNodeRef(n as AnyNodeData))),
    fieldsPtr: [],
    propertiesPtr: [],
  };
}

export function makeSelectionMaybe(
  nodes: AnyNodeData | NodeReferenceData | (AnyNodeData | NodeReferenceData)[] | null | undefined,
): SelectionData | undefined {
  if (nodes == null) return undefined;
  return makeSelection(Array.isArray(nodes) ? nodes : [nodes]);
}

export function expandSelection(
  selection: SelectionData | undefined | null,
  nodes: (AnyNodeData | NodeReferenceData)[],
): SelectionData {
  const newNodes = nodes.filter((n) => !selection?.nodesPtr.some((m) => m.id == n.id));
  return {
    ...(selection ?? { metatype: ObjectType.SELECTION }),
    nodesPtr: [
      ...(selection?.nodesPtr ?? []),
      ...newNodes.map((n) => (isNodeRef(n) ? n : toNodeRef(n as AnyNodeData))),
    ],
    fieldsPtr: [],
    propertiesPtr: [],
  };
}

export function collapseSelection(selection: SelectionData, nodes: (AnyNodeData | NodeReferenceData)[]): SelectionData {
  return {
    ...selection,
    nodesPtr: selection.nodesPtr.filter((n) => !nodes.some((m) => m.id == n.id)),
  };
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

export function addVector2(vec: Vector2 | null | undefined, add: Partial<Vector2>): Vector2Data {
  return {
    metatype: ObjectType.VECTOR2,
    x: (vec?.x ?? 0) + (add.x ?? 0),
    y: (vec?.y ?? 0) + (add.y ?? 0),
  };
}

export function subVector2(vec: Vector2 | null | undefined, sub: Partial<Vector2>): Vector2Data {
  return {
    metatype: ObjectType.VECTOR2,
    x: (vec?.x ?? 0) - (sub.x ?? 0),
    y: (vec?.y ?? 0) - (sub.y ?? 0),
  };
}

export function lengthVector2(vec: Vector2 | null | undefined): number {
  return Math.sqrt((vec?.x ?? 0) ** 2 + (vec?.y ?? 0) ** 2);
}

//
// History
//

export const HISTORY_STATE_KEY = Symbol("history");
export type HistoryState = {
  history: Ref<ViewData[]>;
  focusedViewIdx: Ref<number | null>;
  focusedView: Ref<ViewData | null>;
  canGoBackward: Ref<boolean>;
  canGoForward: Ref<boolean>;
  go: (delta: number) => void;
};
