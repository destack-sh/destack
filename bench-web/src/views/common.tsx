// TODO :Architecture: figure out proper all-encompassing event system/bus

import type { NodeReferenceData } from "@/proto/wire";
import type { ActionMapImplementation } from "@/system/action";
import type { Ref } from "vue";

export const VIEW_EMITS = {};

export function viewEmits() {
  return VIEW_EMITS;
}

export type ViewExposed = (
  | { self: Ref<NodeReferenceData> }
  | { self?: Ref<NodeReferenceData | null | undefined>; id: Ref<string> }
) & {
  actions?: Partial<ActionMapImplementation<any>>;
} & {
  // any additional exposed methods
  [key: string]: any;
};
