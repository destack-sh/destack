// TODO :Architecture: figure out proper all-encompassing event system/bus

import type { NodeReferenceData, Variant } from "@/proto/wire";
import type { ActionMapImplementation } from "@/system/action";
import type { ViewComponent } from "@/views";
import type { Ref } from "vue";

export const VIEW_EMITS = {};

export function viewEmits(): Partial<typeof VIEW_EMITS> {
  return VIEW_EMITS;
}

export type FocusAnchor = "left" | "right" | "top" | "bottom" | "center";

export type ViewExposed = (
  | {
      // always identity
      /** The view node identity of a view component */
      self: Ref<NodeReferenceData>;
      id?: Ref<string>;
    }
  | {
      // maybe anonymous identity
      /** The view node identity of a view component, maybe null if anonymous */
      self: Ref<NodeReferenceData | null | undefined>;
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
