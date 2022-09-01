import { useMetaStore } from "@/stores";
import type { FlowNode } from "@/types";
import {
  CheckCircleIcon,
  CpuChipIcon,
  QuestionMarkCircleIcon,
  ScaleIcon,
  VariableIcon,
} from "@heroicons/vue/24/outline";

export function useIcons() {
  const metaStore = useMetaStore();

  function forNode(node: FlowNode) {
    const functionHandlerSpec = metaStore.functionHandlersById[node.function_id];

    if (node.function_id == "bench.model") {
      return CpuChipIcon;
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
