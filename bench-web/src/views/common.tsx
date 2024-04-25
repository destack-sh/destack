// TODO :Architecture: figure out proper all-encompassing event system/bus

import { Orientation, Variant, type NodeReferenceData, type NodeType } from "@/proto/wire";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import type { ActionMapImplementation } from "@/system/action";
import type { ViewComponent } from "@/views";
import type { FunctionalComponent, Ref } from "vue";

export const VIEW_EMITS = {
  apply: null,
  cancel: null,
  ["update:modelValue"]: null,
};

export function viewEmits(): Partial<typeof VIEW_EMITS> {
  return VIEW_EMITS;
}

export type FocusAnchor = "left" | "right" | "top" | "bottom" | "center";

export type ViewExposed = (
  | {
      // always identity
      /** The view node identity of a view component */
      self: Ref<TypedNodeReferenceData<NodeType.VIEW>>;
      id?: Ref<string>;
    }
  | {
      // maybe anonymous identity
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
  focus?: (anchor: FocusAnchor | NodeReferenceData) => void | boolean | ViewComponent | HTMLElement | null;
  /** Map the relevant node at the given element. */
  mapToNode?: (element: HTMLElement | ViewComponent) => NodeReferenceData | null;
} & {};

// TODO :Architecture :Performance: revisit content view wrapper for vapor mode
export const ViewContentWrapper: FunctionalComponent<{
  title?: string;
  variant?: Variant;
  orientation?: Orientation;
}> = (props, { slots }) => {
  const classBase =
    props.title == null
      ? ""
      : props.orientation === Orientation.HORIZONTAL
        ? "flex flex-row items-center justify-between gap-x-5"
        : "flex flex-col";

  const labelClass =
    props.variant === Variant.PRIMARY ? "mb-0.5 block font-semibold text-gray-900" : "mb-0.5 block text-gray-700";

  return (
    <div class={classBase}>
      {props.title && <label class={labelClass}>{props.title}</label>}
      {slots.default ? slots.default() : null}
    </div>
  );
};
ViewContentWrapper.props = ["title", "variant", "orientation"];
