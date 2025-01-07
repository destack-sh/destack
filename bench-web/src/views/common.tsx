// TODO :Architecture: figure out proper all-encompassing event system/bus

import {
  FieldData,
  Orientation,
  TypeData,
  ViewData,
  ViewType,
  type NodeReferenceData,
  type NodeType,
} from "@/proto/wire";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import type { ActionMapImplementation } from "@/ui/action";
import { LISTABLE_VIEW_TYPES } from "@/ui/view";
import { IS_DEV } from "@/utils/globals";
import { Casing, toCasing } from "@/utils/string";
import { type ComponentInstance, type FunctionalComponent, type Ref } from "vue";

export type ViewProps = {
  self?: NodeReferenceData;
  modelValue?: any;
  placeholder?: string;
  isPopover?: boolean;
} & Partial<Omit<ViewData, "metatype" | "ck">>;
export type ViewComponent = {
  new (): ComponentInstance<any>;
  props: ViewProps;
  exposed: ViewExposed;
};

export const VIEW_EMITS = {
  apply: null,
  cancel: null,
  close: null,
  ["update:modelValue"]: null,
};

export function viewEmits(): Partial<typeof VIEW_EMITS> {
  return VIEW_EMITS;
}

export type FocusAnchor = "left" | "right" | "top" | "bottom" | "center";

export type ModelValueOptions = {
  field: FieldData;
  path: string[];
};

export type ViewExposed = (
  | {
      // always has an identity
      /** The view node identity of a view component */
      self: Ref<TypedNodeReferenceData<NodeType.VIEW>>;
      id?: Ref<string>;
    }
  | {
      // maybe has an identity
      /** The view node identity of a view component, maybe null if anonymous */
      self: Ref<TypedNodeReferenceData<NodeType.VIEW> | null | undefined>;
      /** The anonymous identity of a view component if 'self' is unavailable.  */
      id: Ref<string>;
    }
) & {
  /** The virtual actions implemented by this view */
  actions?: Partial<ActionMapImplementation<any>>;
  /** Focus the element at the given anchor inside the view OR return the element to focus. May be a view or any element. */
  focus?: (
    anchor?: FocusAnchor | NodeReferenceData,
  ) => void | boolean | ViewComponent | HTMLElement | SVGElement | null;
  /** 'Interact's with the primary interaction of the view. */
  interact?: () => void;
  /** Map the relevant node at the given element. */
  mapToNode?: (element: HTMLElement | SVGElement | ViewComponent) => NodeReferenceData | null;
} & {};

// inverse :ViewRegistry for lookups without needing to import the registry
export function getViewTypeByComponentName(name: string): ViewType | null {
  if (name == "NativeInput") {
    return ViewType.STRING;
  } else if (name == "CustomObject") {
    return ViewType.OBJECT;
  } else {
    const capsName = toCasing(name, Casing.ALL_CAPS);
    return (ViewType[capsName as any] as unknown as ViewType) ?? null;
  }
}
