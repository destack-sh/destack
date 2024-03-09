import { BenchType, NodeType, SpaceData, StructType, ViewType } from "@/proto/wire";
import { makeNode, makeStruct, nodeReference, toNodeReference, toNodeReferenceRef } from "@/proto/wiring";
import auth from "@/system/auth";
import { useGetNodes } from "@/system/connection";
import { LOCAL_SPACE_ID, benchPtr, packagePtr, spacePtr } from "@/system/global";
import { NodeGraph, ProxyNodeGraph } from "@/system/graph";
import { LOADED_SOURCE_NODE_TYPES } from "@/system/lang";
import { log } from "@/utils/log";
import { computed, watch } from "vue";

// user
export const { graph: userGraph, connection: userConnection } = useGetNodes(
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
export const { graph: benchGraph, connection: benchConnection } = useGetNodes(
  computed(() => ({
    roots: [benchPtr.value!],
    options: { descendantTypes: [NodeType.ENVIRONMENT, NodeType.BRANCH, NodeType.PACKAGE] },
    enabled: spacePtr.value != null,
    watch: true,
  })),
);
export const bench = benchGraph.getRef(benchPtr);
export const { graph: pkgGraph, connection: pkgConnection } = useGetNodes(
  computed(() => ({
    roots: [packagePtr.value!],
    options: { descendantTypes: LOADED_SOURCE_NODE_TYPES },
    enabled: packagePtr.value != null,
    watch: true,
  })),
);
export const pkg = pkgGraph.getRef(packagePtr);

// space (local if we don't have a bench or a space in that bench, otherwise in the package)
export const spaceGraphLocal = new NodeGraph();
export const spaceRemote = pkgGraph.getRef(spacePtr);
export const spaceGraph = new ProxyNodeGraph(null);
export const space = spaceGraph.getRef(spacePtr);

// setup local space if needed
watch(
  spaceRemote,
  () => {
    if (spaceRemote.value == null) {
      spaceGraph.graph.value = spaceGraphLocal;
      if (spaceGraphLocal.size == 0) {
        const { space } = setupLocalSpace(spaceGraphLocal);
        spacePtr.value = toNodeReference(space);
      }
    } else {
      // spaceGraph.graph.value = pkgGraph;
      throw new Error("not implemented");
    }
  },
  { immediate: true },
);

function setupLocalSpace(graph: NodeGraph): { space: SpaceData } {
  log.info("setupLocalSpace");
  const space = { metatype: BenchType.SPACE, id: LOCAL_SPACE_ID } as SpaceData;
  const side = makeNode({
    metatype: NodeType.VIEW,
    parentPtr: toNodeReference(space),
    type: ViewType.TABBED,
    size: makeStruct({ metatype: StructType.BOX, width: 250 }),
  });
  const primary = makeNode({
    metatype: NodeType.VIEW,
    parentPtr: toNodeReference(space),
    type: ViewType.TABBED,
    size: makeStruct({ metatype: StructType.BOX, widthRelative: 1.5 }),
  });
  const secondary = makeNode({
    metatype: NodeType.VIEW,
    parentPtr: toNodeReference(space),
    type: ViewType.TABBED,
    size: makeStruct({ metatype: StructType.BOX, widthRelative: 1 }),
  });
  graph.extend(space, side, primary, secondary);
  return { space };
}
