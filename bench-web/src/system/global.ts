import { NodeType, type NodeReferenceData } from "@/proto/wire";
import { nodeReference } from "@/system/graph";
import { useStorage } from "@vueuse/core";
import { computed, type Ref } from "vue";

// client-local 'space' state
export const spacePtr = useStorage<NodeReferenceData | null>("spacePtr", null);
export const packageIdByBenchId = useStorage<{ [benchId: string]: string }>("packageIdByBenchId", {});
export const benchPtr: Ref<NodeReferenceData | null> = computed(() => {
  if (spacePtr.value == null) return null;
  else return nodeReference(NodeType.BENCH, spacePtr.value.benchId!);
});
export const packagePtr: Ref<NodeReferenceData | null> = computed(() => {
  if (spacePtr.value == null) return null;
  else return nodeReference(NodeType.PACKAGE, packageIdByBenchId.value[spacePtr.value.benchId!]);
});
