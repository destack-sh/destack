import { useMetaStore } from "@/stores";
import type { FlowNode } from "@/types";
import { CheckCircleIcon, ChipIcon, QuestionMarkCircleIcon, ScaleIcon, VariableIcon } from "@heroicons/vue/outline";

export function useIcons() {
  const metaStore = useMetaStore();

  function forNode(node: FlowNode) {
    const functionHandlerSpec = metaStore.functionHandlersById[node.function_id];

    if (node.function_id == "bench.model") {
      return ChipIcon;
    } else if (functionHandlerSpec.type == "RecordTransform") {
      return VariableIcon;
    } else if (functionHandlerSpec.type == "Metric") {
      return ScaleIcon;
    } else if (functionHandlerSpec.type == "Test") {
      return CheckCircleIcon;
    } else {
      return QuestionMarkCircleIcon;
    }
  }

  return { forNode };
}
