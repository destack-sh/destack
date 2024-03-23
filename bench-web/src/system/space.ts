import { BenchType, NodeReferenceData, NodeType, SpaceData } from "@/proto/wire";
import { toNodeReference } from "@/proto/wiring";
import { spaceGraphLocal, useGetNodes, useActiveConnection } from "@/system/connection";
import { ProxyNodeGraph } from "@/system/graph";
import { LOADED_SOURCE_NODE_TYPES } from "@/system/lang";
import { LOCAL_PACKAGE_PTR, LOCAL_SPACE_ID, benchPtr, packagePtr, setSpaceToLocal, spacePtr } from "@/system/local";
import { log } from "@/utils/log";
import { ViewCanvas, setupEmptyCanvas } from "@/views/canvas";
import { computed, nextTick, watch } from "vue";

// bench/packages
export const { graph: benchGraph, connection: benchConnection } = useGetNodes(
  computed(() => ({
    name: "bench",
    roots: [benchPtr.value!],
    options: { descendantTypes: [NodeType.ENVIRONMENT, NodeType.BRANCH, NodeType.PACKAGE] },
    enabled: benchPtr.value != null,
    watch: true,
  })),
);
export const bench = benchGraph.getRef(benchPtr);
export const { graph: pkgGraph, connection: pkgConnection } = useGetNodes(
  computed(() => ({
    name: "package",
    roots: [packagePtr.value!],
    options: { descendantTypes: LOADED_SOURCE_NODE_TYPES },
    enabled: packagePtr.value != null,
    watch: true,
  })),
);
export const pkg = pkgGraph.getRef(packagePtr);
export const hasBench = computed(() => bench.value != null);

// space (local if we don't have a Space in that Bench, otherwise from the current Package)
export const spaceRemote = pkgGraph.getRef(spacePtr);
export const spaceGraph = new ProxyNodeGraph(null);
export const space = spaceGraph.getRef(spacePtr);
export const spaceConnection = useActiveConnection(spacePtr);
export const canvas = new ViewCanvas(spacePtr, spaceGraph, () => spaceConnection.connection.sideTx);

// setup/connect local space as needed
watch(
  spaceRemote,
  () => {
    if (spaceRemote.value == null) {
      // local
      spaceGraph.graph = spaceGraphLocal;
      if (spaceGraphLocal.size == 0) {
        const space = { metatype: BenchType.SPACE, id: LOCAL_SPACE_ID, packagePtr: LOCAL_PACKAGE_PTR } as SpaceData;
        spaceGraphLocal.add(space);
        setSpaceToLocal();
        log.debug("space.setupEmptyCanvas", { space });
        nextTick(() => setupEmptyCanvas(spaceConnection.connection.sideTx, space)); // spaceConnection is prepared lazily
      }
    } else {
      // remote
      log.debug("space.useRemoteCanvas", { space: spaceRemote.value });
      spaceGraph.graph = pkgGraph;
    }
    canvas.restoreComponentFocus();
  },
  { immediate: true },
);

export async function goToBench(bench: NodeReferenceData) {
  throw new Error("not implemented");
}
