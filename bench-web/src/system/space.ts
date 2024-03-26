import { supervisor } from "@/proto/services";
import { BenchData, BenchType, BranchData, NodeReferenceData, NodeType, SpaceData } from "@/proto/wire";
import { unwrapSomeNode, type TypedNodeReferenceData } from "@/proto/wiring";
import { makeReadOptions, spaceGraphLocal, useActiveConnection, useGetNodes } from "@/system/connection";
import { NodeGraph, ProxyNodeGraph } from "@/system/graph";
import { LOADED_SOURCE_NODE_TYPES } from "@/system/lang";
import local, { LOCAL_PACKAGE_PTR, LOCAL_SPACE_ID } from "@/system/client";
import { log } from "@/utils/log";
import { ViewCanvas, setupEmptyCanvas } from "@/views/canvas";
import { computed, nextTick, watch } from "vue";
// bench/packages
export const { graph: benchGraph, connection: benchConnection } = useGetNodes(
  computed(() => ({
    name: "bench",
    roots: [local.benchPtr.value!],
    options: { descendantTypes: [NodeType.ENVIRONMENT, NodeType.BRANCH, NodeType.PACKAGE] },
    enabled: local.benchPtr.value != null,
    live: true,
  })),
);
export const bench = benchGraph.getRef(local.benchPtr);
export const { graph: pkgGraph, connection: pkgConnection } = useGetNodes(
  computed(() => ({
    name: "package",
    roots: [local.packagePtr.value!],
    options: { descendantTypes: LOADED_SOURCE_NODE_TYPES },
    enabled: local.packagePtr.value != null,
    live: true,
  })),
);
export const pkg = pkgGraph.getRef(local.packagePtr);
export const hasLocalBench = computed(() => bench.value != null);

// space (local if we don't have a Space in that Bench, otherwise from the current Package)
export const spaceRemote = pkgGraph.getRef(local.spacePtr);
export const spaceGraph = new ProxyNodeGraph(null);
export const space = spaceGraph.getRef(local.spacePtr);
export const spaceConnection = useActiveConnection(local.spacePtr);
export const canvas = new ViewCanvas(local.spacePtr, spaceGraph, () => spaceConnection.connection.sideTx);

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
        local.setSpaceToLocal();
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

/** 'Goes' to a Bench and sets it as the current main Bench */
export async function goToBench(go: { bench: NodeReferenceData }) {
  log.info("space.goToBench", go);
  const {
    response: { nodes },
  } = await supervisor.getNodes({
    roots: [go.bench],
    options: makeReadOptions({ descendantTypes: [NodeType.BRANCH] }),
  });
  const graph = new NodeGraph();
  graph.extend(...nodes.map(unwrapSomeNode));
  const bench = graph.roots[0] as BenchData;
  const mainBranch = graph.get(bench.mainBranchPtr!) as BranchData;
  local.setPackage({ pkg: mainBranch.mainPackagePtr as TypedNodeReferenceData<NodeType.PACKAGE> });
}
