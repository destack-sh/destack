import { setCanvas, setSpace, supergraph } from "@/globals";
import { BENCH_BUILTIN_PACKAGE_PTR, BENCH_BUILTIN_SCOPE } from "@/language/core/builtin";
import { LOADED_PACKAGE_NODE_TYPES } from "@/language/core/const";
import { DEFAULT_NODE_FILTER, NodeGraph, ProxyNodeGraph } from "@/language/core/graph";
import { getHostClient } from "@/proto/services";
import { BenchData, ChangeCategory, NodeType, PackageData, SpaceType } from "@/proto/wire";
import {
  describeNode,
  isNode,
  makeScope,
  nodeReference,
  toNodeRef,
  typeNodeReference,
  typeNodeReferenceMaybe,
  unwrapSomeNode,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import local, { BENCH_SCOPE, LOCAL_SPACE_ID, spaceGraphLocal, spacePtr } from "@/system/client";
import { useAutoConnection, useGetConnection } from "@/system/connection";
import { createDesktopDefaultSpace, SpaceCanvas } from "@/ui/space";
import { toaster } from "@/ui/toast";
import { log } from "@/utils/log";
import { computed, nextTick, watch } from "vue";

// bench/packages
export const { graph: benchGraph, connection: benchConnection } = useGetConnection(
  { name: "current.bench", live: true, paramsPretty: computed(() => ({ id: local.benchPtr.value?.id })) },
  computed(() => ({
    scope: BENCH_SCOPE.value,
    roots: [local.benchPtr.value!],
    descendantTypes: [NodeType.PACKAGE, ...LOADED_PACKAGE_NODE_TYPES],
    isEnabled: local.benchPtr.value != null,
  })),
);
export const bench = benchGraph.getRef(local.benchPtr);
export const pkg = benchGraph.getRef(local.packagePtr);
export const hasLocalPkg = computed(() => pkg.value != null);
export const hasLocalBench = computed(() => bench.value != null);
benchConnection.onError((e) => {
  if (e == "NOT_FOUND" || e == "PERMISSION_DENIED") {
    toaster.error({
      title: "Bench unavailable",
      text: "That Bench is no longer accessible.",
    });
    local.clearBench();
  }
});

// dependencies (hard-coded to just the builtin bench for now)
export const { graph: builtinGraph, connection: builtinConnection } = useGetConnection(
  { name: "dependency.bench.builtin", live: true },
  computed(() => ({
    scope: BENCH_BUILTIN_SCOPE,
    roots: [BENCH_BUILTIN_PACKAGE_PTR],
    descendantTypes: [NodeType.PACKAGE, ...LOADED_PACKAGE_NODE_TYPES],
    isEnabled: local.benchPtr.value != null,
  })),
);
export const builtinBench = builtinGraph.getRef(BENCH_BUILTIN_PACKAGE_PTR);

// space (local if we don't have a Space in that Bench, otherwise from the current Package)
export const spaceGraph = new ProxyNodeGraph({ graph: spaceGraphLocal, filter: DEFAULT_NODE_FILTER });
export const space = spaceGraph.getRef(local.spacePtr);
setSpace(space);
export const { connection: spaceConnection } = useAutoConnection(local.spacePtr);
export const canvas = new SpaceCanvas(local.spacePtr, spaceGraph, () =>
  spaceConnection.tx.with({ category: ChangeCategory.SPACE }),
);
setCanvas(canvas);
export const allSpaces = benchGraph.getChildrenRef(pkg, NodeType.SPACE);
export const ownedSpacesInPkg = computed(() =>
  local.userInfo.value == null ? [] : allSpaces.value.filter((s) => s.createdByPtr?.id == local.userInfo.value?.id),
);

// selection
export const inspectionPtr = computed(() => space.value?.inspectionPtr);

// spaceGraph should point to current space
watch(
  spacePtr,
  () => {
    if (spacePtr.value.id == LOCAL_SPACE_ID) {
      spaceGraph.graph = spaceGraphLocal;
    } else {
      spaceGraph.graph = benchGraph;
    }
  },
  { immediate: true },
);

// refocus whenever space changes
watch(
  spacePtr,
  async () => {
    await spaceConnection.waitUntil((result) => result?.graph.get({ id: spacePtr.value.id }) != null);
    // NOTE :Robustness :Cleanup: why doesn't nextTick work to restoreComponentFocus on space change?
    //  (all the views should get rendered immediately, right..?)
    setTimeout(() => {
      try {
        canvas.restoreComponentFocus();
        log.trace("space.restoreFocus", spacePtr.value, spaceConnection.result.value);
      } catch (e) {
        log.warn("space.restoreFocus.error", spacePtr.value, spaceConnection.result.value, e);
      }
    }, 250);
  },
  { immediate: true },
);

/** Sets (and creates if needed) a space in the current Package */
export async function assignSpaceInPackage(pkg: PackageData) {
  const spaceInPkg = benchGraph.get(spacePtr.value);
  if (spaceInPkg != null) {
    // current space is already good
    spaceGraph.graph = benchGraph;
  }

  if (ownedSpacesInPkg.value.length > 0) {
    // we already have a space in the package
    const space = ownedSpacesInPkg.value[0];
    local.setSpace(toNodeRef(ownedSpacesInPkg.value[0]));
    if (benchGraph.getChildren(spacePtr.value, NodeType.VIEW).length == 0) {
      // setup default canvas if needed
      createDesktopDefaultSpace(benchConnection.tx, space);
    }
    spaceGraph.graph = benchGraph;
  } else {
    // create a new space
    const space = benchConnection.tx.create({
      metatype: NodeType.SPACE,
      type: SpaceType.DESKTOP, // should derive this later :HeterogenousClients
      parentPtr: toNodeRef(pkg),
      packagePtr: toNodeRef(pkg),
      name: "MySpace",
      orderKey: "a0",
    });
    createDesktopDefaultSpace(benchConnection.tx, space);
    local.setSpace(toNodeRef(space));
    spaceGraph.graph = benchGraph;
    await benchConnection.txBuffer.commit();
  }
}

/** 'Goes' to a Bench and sets it as the current main Bench. **/
export async function goToBench(go: {
  bench: TypedNodeReferenceData<NodeType.BENCH>;
  space?: TypedNodeReferenceData<NodeType.SPACE>;
}) {
  log.info("space.goToBench", go);

  // connect to bench/package
  const scope = makeScope({ benchId: go.bench.id! });
  const host = await getHostClient({ id: go.bench.id! });
  const {
    response: { nodes },
  } = await host.getNodes({ roots: [go.bench], scope, descendantTypes: [NodeType.PACKAGE], ancestorTypes: [] });
  const graph = new NodeGraph({ scope, nodeTypes: new Set([NodeType.PACKAGE]) });
  graph.extend(...nodes.map(unwrapSomeNode));
  const bench = graph.roots[0] as BenchData;
  const packagePtr = bench.mainPackagePtr!;
  local.setBench({
    pkg: typeNodeReference(NodeType.PACKAGE, packagePtr),
    space: typeNodeReferenceMaybe(NodeType.SPACE, go.space),
  });

  // figure out space once package is loaded
  await benchConnection.waitUntil((result) => result?.graph.get({ id: packagePtr.id }) != null);
  const pkg = supergraph.get(packagePtr);
  if (!isNode(pkg, NodeType.PACKAGE)) {
    throw new Error(`could not load package: ${describeNode(packagePtr)}`);
  }
  await assignSpaceInPackage(pkg);
  nextTick(() => {
    try {
      canvas.restoreComponentFocus();
    } catch (e) {
      log.warn("space.restoreFocus.error", e);
    }
  });
}

/** 'Goes' to a Space and sets it as the current main Space. */
export async function goToSpace(go: { space: TypedNodeReferenceData<NodeType.SPACE> }) {
  log.info("space.goToSpace", go);

  if (go.space.benchId != local.benchPtr.value?.id) {
    await goToBench({ bench: nodeReference(NodeType.BENCH, go.space.benchId!), space: go.space });
  } else {
    local.setSpace(typeNodeReference(NodeType.SPACE, go.space));
  }
}
