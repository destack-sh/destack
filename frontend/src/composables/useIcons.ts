import type { FlowNode } from "@/types";
import { ChipIcon, QuestionMarkCircleIcon } from "@heroicons/vue/outline";

export function useIcons() {
  function forNode(node: FlowNode) {
    if (node.function_id == "bench.model") {
      return ChipIcon;
    } else {
      return QuestionMarkCircleIcon;
    }
  }

  return { forNode };
}
