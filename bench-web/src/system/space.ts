import { supervisor } from "@/proto/services";
import { BenchData, BranchData, EditType, NodeType } from "@/proto/wire";
import {
  nodeReference,
  toNodeReference,
  typeNodeReference,
  typeNodeReferenceMaybe,
  unwrapSomeNode,
  type SomeNodeReferenceData,
} from "@/proto/wiring";
import local, { LOCAL_SPACE_ID, spaceGraphLocal, spacePtr } from "@/system/client";
import { makeReadOptions, useExistingConnection, useGetNodes } from "@/system/connection";
import { NodeGraph, ProxyNodeGraph } from "@/system/graph";
import { LOADED_SOURCE_NODE_TYPES } from "@/system/lang";
import { toaster } from "@/system/toast";
import { log } from "@/utils/log";
import { ViewCanvas, setupEmptyCanvas } from "@/views/canvas";
import { computed, nextTick, watch } from "vue";

// bench/packages
export const {
  graph: benchGraph,
  access: benchAccess,
  connection: benchConnection,
} = useGetNodes(
  { name: "bench", live: true },
  computed(() => ({
    roots: [local.benchPtr.value!],
    options: { descendantTypes: [NodeType.ENVIRONMENT, NodeType.BRANCH, NodeType.PACKAGE] },
    enabled: local.benchPtr.value != null,
  })),
);
export const bench = benchGraph.getRef(local.benchPtr);
export const {
  graph: pkgGraph,
  access: pkgAccess,
  connection: pkgConnection,
} = useGetNodes(
  { name: "pkg", live: true },
  computed(() => ({
    roots: [local.packagePtr.value!],
    options: { ancestorTypes: [NodeType.BENCH], descendantTypes: LOADED_SOURCE_NODE_TYPES },
    enabled: local.packagePtr.value != null,
  })),
);
export const pkg = pkgGraph.getRef(local.packagePtr);
export const hasLocalBench = computed(() => bench.value != null);

// space (local if we don't have a Space in that Bench, otherwise from the current Package)
export const spaceGraph = new ProxyNodeGraph(null);
export const space = spaceGraph.getRef(local.spacePtr);
export const { connection: spaceConnection } = useExistingConnection(local.spacePtr);
export const canvas = new ViewCanvas(local.spacePtr, spaceGraph, () => spaceConnection.tx);
export const allSpaces = pkgGraph.getChildrenRef(pkg, NodeType.SPACE);
export const ownedSpacesInPkg = computed(() =>
  local.userInfo.value == null ? [] : allSpaces.value.filter((s) => s.createdByPtr?.id == local.userInfo.value?.id),
);

// find or create space for package
watch(
  [pkgConnection.isConnected, ownedSpacesInPkg, spacePtr],
  () => {
    /** Finds an owned Space or creates a new one if allowed */
    const findOrCreateSpace = () => {
      const spaceInPkg = ownedSpacesInPkg.value[0];
      if (spaceInPkg != null) {
        // switch to our space in the package
        log.debug("space.switchToLocalSpace", { space: spaceInPkg });
        local.setSpace(toNodeReference(spaceInPkg));
        spaceGraph.graph = pkgGraph;
      } else if (pkg.value != null && pkgAccess.can(EditType.CREATE, NodeType.SPACE)) {
        // create new space
        const pkgPtr = toNodeReference(pkg.value);
        log.debug("space.createNeededSpace", { pkg: pkg.value });
        const space = pkgConnection.tx.create({
          metatype: NodeType.SPACE,
          parentPtr: pkgPtr,
          packagePtr: pkgPtr,
        });
        setupEmptyCanvas(pkgConnection.tx, space);
        spaceGraph.graph = pkgGraph;
      } else if (spacePtr.value.id != LOCAL_SPACE_ID) {
        // reset to local space
        local.setSpaceToLocal();
        spaceGraph.graph = spaceGraphLocal;
      }
    };

    // switch spaces if needed
    const prevSpacePtr = spacePtr.value;
    if (spacePtr.value.id == LOCAL_SPACE_ID) {
      // current space is local
      if (pkgConnection.isConnected.value && pkg.value != null) findOrCreateSpace();
    } else {
      // current space is 'remote' (comes from the package)
      const spaceInPkg = spaceGraph.get(spacePtr.value);
      if (spaceInPkg == null) {
        // current space has been deleted, notify and switch
        toaster.warning({ title: "Space deleted", text: "Your Space is gone. Switching." });
        findOrCreateSpace();
      } else {
        // current space is remote
        spaceGraph.graph = pkgGraph;
      }
    }

    // refocus if we switched spaces
    if (prevSpacePtr?.id != spacePtr.value?.id) {
      nextTick(() => canvas.restoreComponentFocus());
    }
  },
  { immediate: true },
);

/** 'Goes' to a Bench and sets it as the current main Bench. **/
export async function goToBench(go: {
  bench: SomeNodeReferenceData<NodeType.BENCH>;
  branch?: SomeNodeReferenceData<NodeType.BRANCH>;
  pkg?: SomeNodeReferenceData<NodeType.PACKAGE>;
  space?: SomeNodeReferenceData<NodeType.SPACE>;
}) {
  log.info("space.goToBench", go);

  // connect to bench/package
  const {
    response: { nodes },
  } = await supervisor.getNodes({
    roots: [go.bench],
    options: makeReadOptions({ descendantTypes: [NodeType.BRANCH] }),
  });
  const graph = new NodeGraph();
  graph.extend(...nodes.map(unwrapSomeNode));
  const bench = graph.roots[0] as BenchData;
  const branch = graph.get(go.branch ?? bench.mainBranchPtr!) as BranchData;
  const pkg = go.pkg ?? branch.mainPackagePtr!;
  local.setBench({
    pkg: typeNodeReference(NodeType.PACKAGE, pkg),
    space: typeNodeReferenceMaybe(NodeType.SPACE, go.space),
  });
}

/** 'Goes' to a Space and sets it as the current main Space. */
export async function goToSpace(go: { space: SomeNodeReferenceData<NodeType.SPACE> }) {
  log.info("space.goToSpace", go);

  if (go.space.benchId != local.benchPtr.value?.id) {
    await goToBench({ bench: nodeReference(NodeType.BENCH, go.space.benchId!), space: go.space });
  } else {
    local.setSpace(typeNodeReference(NodeType.SPACE, go.space));
  }
}
