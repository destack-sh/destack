import { getHostClient } from "@/proto/services";
import { BenchData, BranchData, NodeType, SpaceType } from "@/proto/wire";
import {
  makeScope,
  nodeReference,
  toNodeReference,
  typeNodeReference,
  typeNodeReferenceMaybe,
  unwrapSomeNode,
  type SomeNodeReferenceData,
} from "@/proto/wiring";
import local, { LOCAL_SPACE_ID, spaceGraphLocal, spacePtr } from "@/system/client";
import { makeReadOptions, useExistingConnection, useGetConnection } from "@/system/connection";
import { NodeGraph, ProxyNodeGraph } from "@/system/graph";
import { SOURCE_NODE_TYPES } from "@/system/lang";
import { toaster } from "@/system/toast";
import { log } from "@/utils/log";
import { ViewCanvas, createDefaultDesktopSpace, createEmptySpace } from "@/views/canvas";
import { computed, nextTick, watch } from "vue";

// bench/packages
export const { graph: benchGraph, connection: benchConnection } = useGetConnection(
  { name: "bench", live: true, paramsPretty: computed(() => ({ id: local.benchPtr.value?.id })) },
  computed(() => ({
    roots: [local.benchPtr.value!],
    options: { descendantTypes: [NodeType.ENVIRONMENT, NodeType.BRANCH, NodeType.PACKAGE] },
    isEnabled: local.benchPtr.value != null,
  })),
);
export const bench = benchGraph.getRef(local.benchPtr);
export const { graph: pkgGraph, connection: pkgConnection } = useGetConnection(
  { name: "pkg", live: true, paramsPretty: computed(() => ({ id: local.packagePtr.value?.id })) },
  computed(() => ({
    roots: [local.packagePtr.value!],
    options: { descendantTypes: SOURCE_NODE_TYPES },
    isEnabled: local.packagePtr.value != null,
  })),
);
export const pkg = pkgGraph.getRef(local.packagePtr);
export const hasLocalPkg = computed(() => pkg.value != null);
export const hasLocalBench = computed(() => bench.value != null);
pkgConnection.onError((e) => {
  if (e == "NOT_FOUND" || e == "PERMISSION_DENIED") {
    toaster.error({
      title: "Bench unavailable",
      text: "That Bench is no longer accessible.",
    });
    local.clearBench();
  }
});

// space (local if we don't have a Space in that Bench, otherwise from the current Package)
export const spaceGraph = new ProxyNodeGraph({ graph: spaceGraphLocal });
export const space = spaceGraph.getRef(local.spacePtr);
export const inspectionPtr = computed(() => space.value?.inspectionPtr);
export const inspectionBasePtr = computed(() => space.value?.basePtr);
export const { connection: spaceConnection } = useExistingConnection(local.spacePtr, {
  isOptional: true,
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
    // NOTE :Robustness :Cleanup: why doesn't nextTick work to restoreComponentFocus on space change?
    //  (All the views should get rendered immediately, right?)
    setTimeout(() => {
      try {
        canvas.restoreComponentFocus();
        log.debug("space.restoreFocus", spacePtr.value, spaceConnection.result.value);
      } catch (e) {
        log.warn("space.restoreFocus.error", spacePtr.value, spaceConnection.result.value, e);
      }
    }, 100);
  },
  { immediate: true },
);

/** Sets (and creates if needed) a space in the current Package */
export async function assignSpaceInPackage() {
  if (pkg.value == null) throw new Error(`package not loaded`);
  log.debug("space.assignSpaceInPackage", { pkg: pkg.value, space: spacePtr.value });

  const spaceInPkg = pkgGraph.get(spacePtr.value);
  if (spaceInPkg != null) {
    // current space is already good
    spaceGraph.graph = pkgGraph;
  }

  if (ownedSpacesInPkg.value.length > 0) {
    // we already have a space in the package
    const space = ownedSpacesInPkg.value[0];
    local.setSpace(toNodeReference(ownedSpacesInPkg.value[0]));
    if (pkgGraph.getChildren(spacePtr.value, NodeType.VIEW).length == 0) {
      // setup default canvas if needed
      createDefaultDesktopSpace(pkgConnection.tx, space);
    }
    spaceGraph.graph = pkgGraph;
  } else {
    // create a new space
    const space = pkgConnection.tx.create({
      metatype: NodeType.SPACE,
      type: SpaceType.DESKTOP, // should derive this later :HeterogenousClients
      parentPtr: toNodeReference(pkg.value),
      packagePtr: toNodeReference(pkg.value),
      name: "MySpace",
      orderKey: "a0",
    });
    createEmptySpace(pkgConnection.tx, space);
    local.setSpace(toNodeReference(space));
    spaceGraph.graph = pkgGraph;
    await pkgConnection.txBuffer.commit();
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
  const scope = makeScope({ benchId: go.bench.id! });
  const host = await getHostClient({ id: go.bench.id! });
  const {
    response: { nodes },
  } = await host.getNodes({
    roots: [go.bench],
    scope,
    options: makeReadOptions({ descendantTypes: [NodeType.BRANCH] }),
  });
  const graph = new NodeGraph({ scope, nodeTypes: [NodeType.BRANCH] });
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
