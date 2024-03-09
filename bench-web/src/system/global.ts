import { NodeType } from "@/proto/wire";
import { nodeReference, type TypedNodeReferenceData } from "@/proto/wiring";
import { useStorage } from "@vueuse/core";
import { computed, type Ref } from "vue";

export const LOCAL_BENCH_ID = /* uuidv4 */ "00000000-0000-0000-0000-000000000000";
export const LOCAL_PACKAGE_ID = /* uuidv4 */ "00000000-0000-0000-0000-000000000001";
export const LOCAL_SPACE_ID = /* uuidv4 */ "00000000-0000-0000-0000-000000000002";

export const LOCAL_BENCH_PTR = nodeReference(NodeType.BENCH, LOCAL_BENCH_ID);
export const LOCAL_PACKAGE_PTR = nodeReference(NodeType.PACKAGE, LOCAL_PACKAGE_ID);
export const LOCAL_SPACE_PTR = nodeReference(NodeType.SPACE, LOCAL_SPACE_ID);

// client-local 'space' state
export const spacePtr = useStorage<TypedNodeReferenceData<NodeType.SPACE> | null>("spacePtr", null);
export const packageIdByBenchId = useStorage<{ [benchId: string]: string }>("packageIdByBenchId", {});
export const benchPtr: Ref<TypedNodeReferenceData<NodeType.BENCH> | null> = computed(() => {
  if (spacePtr.value == null) return null;
  else return nodeReference(NodeType.BENCH, spacePtr.value.benchId!);
});
export const packagePtr: Ref<TypedNodeReferenceData<NodeType.PACKAGE> | null> = computed(() => {
  if (spacePtr.value == null) return null;
  else return nodeReference(NodeType.PACKAGE, packageIdByBenchId.value[spacePtr.value.benchId!]);
});
