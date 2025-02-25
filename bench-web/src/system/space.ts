import { setCanvas, setSpace, supergraph } from "@/globals";
import { SOURCE_NODE_TYPES, STATIC_RESOURCE_NODE_TYPES } from "@/language/core/const";
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
import local, { BENCH_SCOPE, LOCAL_SPACE_ID, PACKAGE_SCOPE, spaceGraphLocal, spacePtr } from "@/system/client";
import { useExistingConnection, useGetConnection } from "@/system/connection";
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
    descendantTypes: [NodeType.PACKAGE, ...STATIC_RESOURCE_NODE_TYPES],
    isEnabled: local.benchPtr.value != null,
  })),
);
export const bench = benchGraph.getRef(local.benchPtr);
export const { graph: pkgGraph, connection: pkgConnection } = useGetConnection(
  { name: "current.pkg", live: true, paramsPretty: computed(() => ({ id: local.packagePtr.value?.id })) },
  computed(() => ({
    scope: PACKAGE_SCOPE.value,
    roots: [local.packagePtr.value!],
    descendantTypes: SOURCE_NODE_TYPES,
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
export const spaceGraph = new ProxyNodeGraph({ graph: spaceGraphLocal, filter: DEFAULT_NODE_FILTER });
export const space = spaceGraph.getRef(local.spacePtr);
setSpace(space);
export const { connection: spaceConnection } = useExistingConnection(local.spacePtr, { isRequired: false });
export const canvas = new SpaceCanvas(local.spacePtr, spaceGraph, () =>
  spaceConnection.tx.with({ category: ChangeCategory.SPACE }),
);
setCanvas(canvas);
export const allSpaces = pkgGraph.getChildrenRef(pkg, NodeType.SPACE);
export const ownedSpacesInPkg = computed(() =>
  local.userInfo.value == null ? [] : allSpaces.value.filter((s) => s.createdByPtr?.id == local.userInfo.value?.id),
);

// selection
export const inspectionPtr = computed(() => space.value?.inspectionPtr);
export const channelPtr = computed(() => space.value?.channelPtr);
export const threadPtr = computed(() => space.value?.threadPtr);

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
  const spaceInPkg = pkgGraph.get(spacePtr.value);
  if (spaceInPkg != null) {
    // current space is already good
    spaceGraph.graph = pkgGraph;
  }

  if (ownedSpacesInPkg.value.length > 0) {
    // we already have a space in the package
    const space = ownedSpacesInPkg.value[0];
    local.setSpace(toNodeRef(ownedSpacesInPkg.value[0]));
    if (pkgGraph.getChildren(spacePtr.value, NodeType.VIEW).length == 0) {
      // setup default canvas if needed
      createDesktopDefaultSpace(pkgConnection.tx, space);
    }
    spaceGraph.graph = pkgGraph;
  } else {
    // create a new space
    const space = pkgConnection.tx.create({
      metatype: NodeType.SPACE,
      type: SpaceType.DESKTOP, // should derive this later :HeterogenousClients
      parentPtr: toNodeRef(pkg),
      packagePtr: toNodeRef(pkg),
      name: "MySpace",
      orderKey: "a0",
    });
    createDesktopDefaultSpace(pkgConnection.tx, space);
    local.setSpace(toNodeRef(space));
    spaceGraph.graph = pkgGraph;
    await pkgConnection.txBuffer.commit();
  }
}

/** 'Goes' to a Bench and sets it as the current main Bench. **/
export async function goToBench(go: {
  bench: TypedNodeReferenceData<NodeType.BENCH>;
  pkg?: TypedNodeReferenceData<NodeType.PACKAGE>;
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
  const packagePtr = go.pkg ?? bench.mainPackagePtr!;
  local.setBench({
    pkg: typeNodeReference(NodeType.PACKAGE, packagePtr),
    space: typeNodeReferenceMaybe(NodeType.SPACE, go.space),
  });

  // figure out space once package is loaded
  await pkgConnection.waitUntil((result) => result?.graph.get({ id: packagePtr.id }) != null);
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
