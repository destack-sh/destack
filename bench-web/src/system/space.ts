import type { NodeReferenceData } from "@/proto/wire";
import { useStorage } from "@vueuse/core";
import { NodeType } from "@/proto/wire";
import auth from "@/system/auth";
import { getNodesRef, nodeReference, toNodeReferenceRef } from "@/system/graph";
import { computed } from "vue";

const { graph: userGraph } = getNodesRef(
  computed(() => ({
    roots: [nodeReference(NodeType.USER, auth.userInfo.value?.id!)],
    options: { descendantTypes: [NodeType.CLIENT] },
    enabled: auth.isAuthenticated,
    live: true,
  })),
);
export const user = userGraph.getRef(
  computed(() => (auth.isAuthenticated ? { type: NodeType.USER, id: auth.userInfo.value?.id! } : null)),
);
export const client = userGraph.getRef(
  computed(() => (auth.clientAccess.value?.id == null ? null : { type: NodeType.CLIENT, id: auth.clientAccess.value.id })),
);
export const clients = userGraph.getChildrenRef(toNodeReferenceRef(user), NodeType.CLIENT);

export const spacePtr = useStorage<NodeReferenceData | null>("spacePtr", null);
export const packageIdByBenchId = useStorage<{ [benchId: string]: string }>("packageIdByBenchId", {});
