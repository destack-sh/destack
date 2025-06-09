import { setCanvas, setSpace, supergraph } from "@/globals";
import { DESTACK_PTR, DESTACK_SCOPE } from "@/language/core/builtin";
import { LOADED_PACKAGE_NODE_TYPES } from "@/language/core/const";
import { DEFAULT_NODE_FILTER, NodeGraph, ProxyNodeGraph } from "@/language/core/graph";
import { getHostClient } from "@/proto/services";
import { DestackData, ChangeCategory, NodeType, PackageData, SpaceType } from "@/proto/wire";
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
import local, { CURRENT_DESTACK_SCOPE, LOCAL_SPACE_ID, spaceGraphLocal, spacePtr } from "@/system/client";
import { useAutoConnection, useGetConnection } from "@/system/connection";
import { createDesktopDefaultSpace, SpaceCanvas } from "@/ui/space";
import { toaster } from "@/ui/toast";
import { log } from "@/utils/log";
import { computed, nextTick, watch } from "vue";

// destack/packages
export const { graph: destackGraph, connection: destackConnection } = useGetConnection(
  { name: "current.destack", live: true, paramsPretty: computed(() => ({ id: local.destackPtr.value?.id })) },
  computed(() => ({
    scope: CURRENT_DESTACK_SCOPE.value,
    roots: [local.destackPtr.value!],
    descendantTypes: [NodeType.PACKAGE, ...LOADED_PACKAGE_NODE_TYPES],
    isEnabled: local.destackPtr.value != null,
  })),
);
export const destack = destackGraph.getRef(local.destackPtr);
export const pkg = destackGraph.getRef(local.packagePtr);
export const hasLocalPkg = computed(() => pkg.value != null);
export const hasLocalDestack = computed(() => destack.value != null);
destackConnection.onError((e) => {
  if (e == "NOT_FOUND" || e == "PERMISSION_DENIED") {
    toaster.error({
      title: "Destack unavailable",
      text: "That Destack is no longer accessible.",
    });
    local.clearDestack();
  }
});

// dependencies (hard-coded to just the builtin destack for now)
export const { graph: builtinGraph, connection: builtinConnection } = useGetConnection(
  { name: "destack.builtin", live: true, isReadOnly: true },
  computed(() => ({
    scope: DESTACK_SCOPE,
    roots: [DESTACK_PTR],
    descendantTypes: [NodeType.PACKAGE, ...LOADED_PACKAGE_NODE_TYPES],
    isEnabled: local.destackPtr.value != null,
  })),
);
export const builtinDestack = builtinGraph.getRef(DESTACK_PTR);

// space (local if we don't have a Space in that Destack, otherwise from the current Package)
export const spaceGraph = new ProxyNodeGraph({ graph: spaceGraphLocal, filter: DEFAULT_NODE_FILTER });
export const space = spaceGraph.getRef(local.spacePtr);
setSpace(space);
export const { connection: spaceConnection } = useAutoConnection(local.spacePtr);
export const canvas = new SpaceCanvas(local.spacePtr, spaceGraph, () =>
  spaceConnection.tx.with({ category: ChangeCategory.SPACE }),
);
setCanvas(canvas);
export const allSpaces = destackGraph.getChildrenRef(pkg, NodeType.SPACE);
export const ownedSpacesInPkg = computed(() =>
  local.userInfo.value == null ? [] : allSpaces.value.filter((s) => s.createdByPtr?.id == local.userInfo.value?.id),
);

// global pointers
export const inspectionPtr = computed(() => space.value?.inspectionPtr);
export const containerPtr = computed(() => space.value?.containerPtr);
export const threadPtr = computed(() => space.value?.threadPtr);
export const pagePtr = computed(() => space.value?.pagePtr);

// spaceGraph should point to current space
watch(
  spacePtr,
  () => {
    if (spacePtr.value.id == LOCAL_SPACE_ID) {
      spaceGraph.graph = spaceGraphLocal;
    } else {
      spaceGraph.graph = destackGraph;
    }
  },
  { immediate: true },
);

// refocus whenever space changes
watch(
  spacePtr,
  async () => {
    await spaceConnection.waitUntil((result) => result?.graph.get({ id: spacePtr.value.id }) != null);
    // NOTE :Cleanup: why doesn't nextTick work to restoreComponentFocus on space change?
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
  const spaceInPkg = destackGraph.get(spacePtr.value);
  if (spaceInPkg != null) {
    // current space is already good
    spaceGraph.graph = destackGraph;
  }

  if (ownedSpacesInPkg.value.length > 0) {
    // we already have a space in the package
    const space = ownedSpacesInPkg.value[0];
    local.setSpace(toNodeRef(ownedSpacesInPkg.value[0]));
    if (destackGraph.getChildren(spacePtr.value, NodeType.VIEW).length == 0) {
      // setup default canvas if needed
      createDesktopDefaultSpace(destackConnection.tx, space);
    }
    spaceGraph.graph = destackGraph;
  } else {
    // create a new space
    const space = destackConnection.tx.create({
      metatype: NodeType.SPACE,
      type: SpaceType.DESKTOP, // should derive this later :HeterogenousClients
      parentPtr: toNodeRef(pkg),
      packagePtr: toNodeRef(pkg),
      name: "MySpace",
      orderKey: "a0",
    });
    createDesktopDefaultSpace(destackConnection.tx, space);
    local.setSpace(toNodeRef(space));
    spaceGraph.graph = destackGraph;
    await destackConnection.txBuffer.commit();
  }
}

/** 'Goes' to a Destack and sets it as the current main Destack. **/
export async function goToDestack(go: {
  destack: TypedNodeReferenceData<NodeType.SPACE>;
  space?: TypedNodeReferenceData<NodeType.SPACE>;
}) {
  log.info("space.goToDestack", go);

  // connect to destack/package
  const scope = makeScope({ destackId: go.destack.id! });
  const host = await getHostClient({ id: go.destack.id! });
  const {
    response: { nodes },
  } = await host.getNodes(
    { roots: [go.destack], scope, descendantTypes: [NodeType.PACKAGE], ancestorTypes: [] },
    { timeout: 5000 },
  );
  const graph = new NodeGraph({ scope, nodeTypes: new Set([NodeType.PACKAGE]) });
  graph.extend(...nodes.map(unwrapSomeNode));
  const destack = graph.roots[0] as DestackData;
  const packagePtr = destack.packagePtr!;
  local.setDestack({
    pkg: typeNodeReference(NodeType.PACKAGE, packagePtr),
    space: typeNodeReferenceMaybe(NodeType.SPACE, go.space),
  });

  // figure out space once package is loaded
  await destackConnection.waitUntil((result) => result?.graph.get({ id: packagePtr.id }) != null);
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

  if (go.space.destackId != local.destackPtr.value?.id) {
    await goToDestack({ destack: nodeReference(NodeType.SPACE, go.space.destackId!), space: go.space });
  } else {
    local.setSpace(typeNodeReference(NodeType.SPACE, go.space));
  }
}
