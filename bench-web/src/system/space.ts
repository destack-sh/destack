import { NodeType } from "@/proto/wire";
import auth from "@/system/auth";
import { benchPtr, packagePtr, spacePtr } from "@/system/global";
import { getNodesRef, nodeReference, toNodeReferenceRef, type GraphConnection } from "@/system/graph";
import { LOADED_SOURCE_NODE_TYPES } from "@/system/lang";
import { computed, type Ref } from "vue";


// user
export const { graph: userGraph, connection: userConnection } = getNodesRef(
  computed(() => ({
    roots: [nodeReference(NodeType.USER, auth.userInfo.value?.id!)],
    options: { descendantTypes: [NodeType.CLIENT] },
    enabled: auth.isAuthenticated,
    watch: true,
  })),
);
export const user = userGraph.getRef(
  computed(() => (auth.isAuthenticated ? { type: NodeType.USER, id: auth.userInfo.value?.id! } : null)),
);
export const client = userGraph.getRef(
  computed(() =>
    auth.clientAccess.value?.id == null ? null : { type: NodeType.CLIENT, id: auth.clientAccess.value.id },
  ),
);
export const clients = userGraph.getChildrenRef(toNodeReferenceRef(user), NodeType.CLIENT);

// bench/packages
export const { graph: benchGraph, connection: benchConnection } = getNodesRef(
  computed(() => ({
    roots: [benchPtr.value!],
    options: { descendantTypes: [NodeType.ENVIRONMENT, NodeType.BRANCH, NodeType.PACKAGE] },
    enabled: spacePtr.value != null,
    watch: true,
  })),
);
export const bench = benchGraph.getRef(benchPtr);
export const space = benchGraph.getRef(spacePtr);
export const { graph: packageGraph, connection: packageConnection } = getNodesRef(
  computed(() => ({
    roots: [packagePtr.value!],
    options: { descendantTypes: LOADED_SOURCE_NODE_TYPES },
    enabled: spacePtr.value != null,
    watch: true,
  })),
);
export const pkg = packageGraph.getRef(packagePtr);
export const allGraphs: Ref<GraphConnection[]> = computed(() => {
  const graphs = [userConnection, benchConnection, packageConnection];
  return graphs.filter((g) => g.active.value);
});