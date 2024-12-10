// TODO :Architecture: figure out proper all-encompassing event system/bus

import {
  Orientation,
  TypeInfoData,
  Variant,
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
  /** The supported variants (if any) */
  variants?: Variant[];
  /** Focus the element at the given anchor inside the view OR return the element to focus. May be a view or any element. */
  focus?: (
    anchor?: FocusAnchor | NodeReferenceData,
  ) => void | boolean | ViewComponent | HTMLElement | SVGElement | null;
  /** 'Interact's with the primary interaction of the view. */
  interact?: () => void;
  /** Map the relevant node at the given element. */
  mapToNode?: (element: HTMLElement | SVGElement | ViewComponent) => NodeReferenceData | null;
} & {};

// TODO :Architecture :Performance: revisit content view wrapper for vapor mode
export const ViewContentWrapper: FunctionalComponent<{
  type?: ViewType;
  title?: string;
  variant?: Variant;
  orientation?: Orientation;
  valueType?: TypeInfoData;
}> = (props, { slots }) => {
  const classBase =
    props.title == null
      ? ""
      : props.orientation === Orientation.HORIZONTAL
        ? "flex flex-row items-center justify-between gap-x-5"
        : "flex flex-col";
  const labelClass =
    props.variant !== Variant.STEALTH ? "mb-0.5 block font-semibold text-gray-900" : "mb-0.5 block text-gray-700";
  const isUnsupported = props.valueType?.isList && !LISTABLE_VIEW_TYPES.has(props.type!);

  return (
    <div class={classBase}>
      {props.title && <label class={labelClass}>{props.title}</label>}
      {isUnsupported ? (
        <div class="text-red-600">{IS_DEV ? (props.type ?? "<no view type>") : "???"}</div>
      ) : slots.default ? (
        slots.default()
      ) : null}
    </div>
  );
};
ViewContentWrapper.props = ["type", "title", "variant", "orientation", "valueType"];

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
