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
import { log } from "@/utils/log";
import { ViewCanvas, setupEmptyCanvas } from "@/views/canvas";
import { computed, nextTick, watch } from "vue";

// bench/packages
export const {
  graph: benchGraph,
  access: benchAccess,
  connection: benchConnection,
} = useGetNodes(
  { name: "bench", live: true, paramsPretty: computed(() => ({ id: local.benchPtr.value?.id })) },
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
  { name: "pkg", live: true, paramsPretty: computed(() => ({ id: local.packagePtr.value?.id })) },
  computed(() => ({
    roots: [local.packagePtr.value!],
    options: { ancestorTypes: [NodeType.BENCH], descendantTypes: LOADED_SOURCE_NODE_TYPES },
    enabled: local.packagePtr.value != null,
  })),
);
export const pkg = pkgGraph.getRef(local.packagePtr);
export const hasLocalBench = computed(() => bench.value != null);

// space (local if we don't have a Space in that Bench, otherwise from the current Package)
export const spaceGraph = new ProxyNodeGraph({ graph: spaceGraphLocal });
export const space = spaceGraph.getRef(local.spacePtr);
export const { connection: spaceConnection } = useExistingConnection(local.spacePtr, {
  isGlobal: true,
});
export const canvas = new ViewCanvas(local.spacePtr, spaceGraph, () => spaceConnection.tx);
export const allSpaces = pkgGraph.getChildrenRef(pkg, NodeType.SPACE);
export const ownedSpacesInPkg = computed(() =>
  local.userInfo.value == null ? [] : allSpaces.value.filter((s) => s.createdByPtr?.id == local.userInfo.value?.id),
);

// spaceGraph should point to current space
watch(
  spacePtr,
  () => {
    if (spacePtr.value.id == LOCAL_SPACE_ID) {
      spaceGraph.graph = spaceGraphLocal;
    } else {
      spaceGraph.graph = pkgGraph;
    }
  },
  { immediate: true },
);

// refocus whenever space changes
watch(
  spacePtr,
  async () => {
    await spaceConnection.waitForResult((result) => result?.graph.get({ id: spacePtr.value.id }) != null);
    nextTick(() => canvas.restoreComponentFocus());
  },
  { immediate: true },
);

/** Assigns a space in the current Package */
async function assignSpaceInPackage() {
  if (pkg.value == null) throw new Error(`package not loaded`);
  log.debug("space.assignSpaceInPackage", { pkg: pkg.value, space: spacePtr.value });

  const spaceInPkg = pkgGraph.get(spacePtr.value);
  if (spaceInPkg != null) {
    // current space is already good
    spaceGraph.graph = pkgGraph;
  }

  if (ownedSpacesInPkg.value.length > 0) {
    // we already have a space in the package
    local.setSpace(toNodeReference(ownedSpacesInPkg.value[0]));
    spaceGraph.graph = pkgGraph;
  } else if (pkgAccess.can(EditType.CREATE, NodeType.SPACE)) {
    // we can create a new space
    const space = pkgConnection.tx.create({
      metatype: NodeType.SPACE,
      parentPtr: toNodeReference(pkg.value),
      packagePtr: toNodeReference(pkg.value),
    });
    setupEmptyCanvas(pkgConnection.tx, space);
    local.setSpace(toNodeReference(space));
    spaceGraph.graph = pkgGraph;
    await pkgConnection.txBuffer.commit();
  } else {
    // we can't create, so just use a local space
    local.setSpaceToLocal();
    spaceGraph.graph = spaceGraphLocal;
  }
}

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

  // figure out space once package is loaded
  await pkgConnection.waitForResult((result) => result?.graph.get({ id: pkg.id }) != null);
  await assignSpaceInPackage();
  nextTick(() => canvas.restoreComponentFocus());
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
