import { BenchType, NodeType, Orientation, SpaceData, StructType, ViewData, ViewType } from "@/proto/wire";
import { copyNode, makeNode, makeStruct, toNodeReference } from "@/proto/wiring";
import { spaceGraphLocal, useGetNodes } from "@/system/connection";
import { NodeGraph, ProxyNodeGraph, type ReadNodeGraph } from "@/system/graph";
import { makeIcon } from "@/system/icon";
import { LOADED_SOURCE_NODE_TYPES, ROOT_VIEW_TYPES, getOrderKey, updateOrderKey } from "@/system/lang";
import { LOCAL_PACKAGE_PTR, LOCAL_SPACE_ID, benchPtr, packagePtr, spacePtr } from "@/system/local";
import type { Transaction } from "@/system/transaction";
import type { SplitAnchor } from "@/utils/drag";
import { log } from "@/utils/log";
import { DEFAULT_ORIENTATION, splitBox } from "@/utils/layout";
import { computed, watch } from "vue";
import { ACTION_COMING_SOON, contributeActionMap } from "@/system/action";
import { toaster } from "@/system/toast";

// bench/packages
export const { graph: benchGraph, connection: benchConnection } = useGetNodes(
  computed(() => ({
    roots: [benchPtr.value!],
    options: { descendantTypes: [NodeType.ENVIRONMENT, NodeType.BRANCH, NodeType.PACKAGE] },
    enabled: benchPtr.value != null,
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

// space (local if we don't have a bench or a space in that bench, otherwise from the current package)
export const spaceRemote = pkgGraph.getRef(spacePtr);
export const spaceGraph = new ProxyNodeGraph(null);
export const space = spaceGraph.getRef(spacePtr);

// setup/connect local space as needed
watch(
  spaceRemote,
  () => {
    if (spaceRemote.value == null) {
      // local
      spaceGraph.graph = spaceGraphLocal;
      if (spaceGraphLocal.size == 0) {
        const { space } = setupLocalSpace(spaceGraphLocal);
        spacePtr.value = toNodeReference(space);
      }
    } else {
      spaceGraph.graph = pkgGraph;
    }
  },
  { immediate: true },
);

function setupLocalSpace(graph: NodeGraph): { space: SpaceData } {
  log.info("setupLocalSpace");
  const space = { metatype: BenchType.SPACE, id: LOCAL_SPACE_ID, packagePtr: LOCAL_PACKAGE_PTR } as SpaceData;
  // const side = makeNode({
  //   metatype: NodeType.VIEW,
  //   parentPtr: toNodeReference(space),
  //   packagePtr: LOCAL_PACKAGE_PTR,
  //   type: ViewType.TABBED,
  //   name: "Side",
  //   title: "Side Window",
  //   orderKey: "a0",
  //   orientation: Orientation.VERTICAL,
  //   size: makeStruct({ metatype: StructType.BOX, width: 280 }),
  // });
  const primary = makeNode({
    metatype: NodeType.VIEW,
    parentPtr: toNodeReference(space),
    packagePtr: LOCAL_PACKAGE_PTR,
    type: ViewType.TABBED,
    name: "Primary",
    title: "Primary Window",
    orderKey: "a1",
    size: makeStruct({ metatype: StructType.BOX, widthRelative: 1 }),
  });
  const secondary = makeNode({
    metatype: NodeType.VIEW,
    parentPtr: toNodeReference(space),
    packagePtr: LOCAL_PACKAGE_PTR,
    type: ViewType.TABBED,
    name: "Secondary",
    title: "Secondary Window",
    orderKey: "a2",
    size: makeStruct({ metatype: StructType.BOX, widthRelative: 1 }),
  });
  graph.extend(space, primary, secondary);
  let ord = 0;
  for (const node of graph.nodes) {
    if ((node as ViewData).type == ViewType.TABBED) {
      for (const i of [0, 1, 2]) {
        graph.add(
          makeNode({
            metatype: NodeType.VIEW,
            parentPtr: toNodeReference(node),
            packagePtr: LOCAL_PACKAGE_PTR,
            type: ViewType.PAGE,
            icon: makeIcon({ name: "fas fa-right-from-bracket" }),
            title: `Test Page ${ord++}`,
            orderKey: "a0",
            isInput: true,
          }),
        );
      }
    }
  }

  return { space };
}

export function addViewToCurrentRoot(view: Partial<Omit<ViewData, "metatype">> & Pick<ViewData, "type">) {
  const root = spaceGraph.nodes.find(
    (n) => n.metatype == BenchType.VIEW && ROOT_VIEW_TYPES.includes((n as ViewData).type),
  );
  if (root == null) throw new Error("no root view");
  // tx.create(
  //   makeNode({
  //     metatype: NodeType.VIEW,
  //     ...view,
  //     packagePtr: space.value?.packagePtr,
  //     parentPtr: toNodeReference(root),
  //   }),
  // );
  throw new Error("nocheckin: handle & assign current root view etc.");
}

/**
 * Removes the given view from the space graph, taking care to clean up.
 */
export function removeView(tx: Transaction, graph: ReadNodeGraph, view: ViewData) {
  log.debug("view.remove", view);
  const parent = graph.get(view.parentPtr!) as ViewData;
  tx.delete(view); // should soft delete?
  cleanupRootView(tx, graph, parent);
}

/**
 * Cleanup previously split root views that are no longer needed.
 */
export function cleanupRootView(tx: Transaction, graph: ReadNodeGraph, view: ViewData) {
  if (!ROOT_VIEW_TYPES.includes(view.type)) return;
  if (
    graph.getChildren(view, NodeType.VIEW).length == 0 &&
    graph.getChildren(view.parentPtr!, NodeType.VIEW).length > 1
  ) {
    removeView(tx, graph, view);
  }
}

/**
 * Adds the given view into this view at the target/anchor.
 */
export function addView(
  tx: Transaction,
  graph: ReadNodeGraph,
  self: ViewData,
  child: ViewData,
  anchor: "start" | "end",
  referenceId: string | null,
) {
  log.debug("view.add", self, child, anchor, referenceId);
  // move & update order
  if (child.id != referenceId) {
    updateOrderKey({
      tx,
      target: child,
      position: anchor == "start" ? "before" : "after",
      referenceId,
      nodes: () => graph.getChildren(self, NodeType.VIEW),
    });
  }
  if (child.parentPtr?.id != self.id) {
    tx.move({ ...child, parentPtr: toNodeReference(self) });
  }
  cleanupRootView(tx, graph, graph.get(child.parentPtr!) as ViewData);
}

/**
 * 'Splits' the 'self' view to accomodate a new equally sized subview 'seed' (at the anchor).
 * If we're already split alongside the given orientation, the seed is added to the existing split.
 */
export function splitView(
  tx: Transaction,
  graph: ReadNodeGraph,
  self: ViewData,
  child: ViewData,
  anchor: Omit<SplitAnchor, "center">,
) {
  log.debug("view.split", self, child, anchor);

  // determine if we need a new split
  const parent = graph.get(self.parentPtr!) as ViewData;
  const isHorizontal = anchor == "left" || anchor == "right";
  const orientation = isHorizontal ? Orientation.HORIZONTAL : Orientation.VERTICAL;
  const isOrderFlipped = anchor == "right" || anchor == "bottom";
  const needsNewSplit = (parent.orientation ?? DEFAULT_ORIENTATION) != orientation;

  // duplicate seed if it belongs to self
  if (child.parentPtr?.id == self.id) {
    child = copyNode(child);
    tx.create(child);
  }

  if (needsNewSplit) {
    // insert a new split in place of 'self'
    const split = makeNode({
      metatype: NodeType.VIEW,
      type: ViewType.WINDOWED,
      parentPtr: self.parentPtr,
      packagePtr: self.packagePtr,
      orderKey: self.orderKey,
      size: self.size,
      name: "Split",
      orientation,
    });
    tx.create(split);
    tx.move({ ...self, parentPtr: toNodeReference(split) });
    tx.update({ ...self, metatype: NodeType.VIEW, size: undefined, orderKey: isOrderFlipped ? "a0" : "a1" });

    // and a new tabbed wrapper
    const viewParent = makeNode({
      metatype: NodeType.VIEW,
      type: ViewType.TABBED,
      parentPtr: toNodeReference(split),
      packagePtr: self.packagePtr,
      orderKey: isOrderFlipped ? "a1" : "a0",
    });
    tx.create(viewParent);
    tx.move({ ...child, parentPtr: toNodeReference(viewParent) });
    tx.update({ ...child, metatype: NodeType.VIEW, size: undefined, orderKey: "a0" });
  } else {
    // 'split' size between self and child with a new tabbed wrapper
    const halfSize = splitBox(self.size!);
    const viewParent = makeNode({
      metatype: NodeType.VIEW,
      type: ViewType.TABBED,
      parentPtr: self.parentPtr,
      packagePtr: self.packagePtr,
      size: halfSize,
      orderKey: getOrderKey({
        nodes: graph.getChildren(parent, NodeType.VIEW),
        position: isOrderFlipped ? "after" : "before",
        reference: self,
      }),
    });
    tx.create(viewParent);
    tx.move({ ...child, parentPtr: toNodeReference(viewParent) });
    tx.update({ ...child, metatype: NodeType.VIEW, size: undefined, orderKey: "a0" });
    tx.update({ ...self, metatype: NodeType.VIEW, size: halfSize });
  }
  cleanupRootView(tx, graph, graph.get(child.parentPtr!) as ViewData);
}

const DISCORD_URL = "https://discord.gg/pSBdq6XC";
contributeActionMap<"space">({
  "space.open.inspector": {
    title: "Inspect Node",
    text: "Open the Inspector View",
    icon: "fas fa-eye-dropper",
    action: ACTION_COMING_SOON,
  },
  "space.open.library": {
    title: "Open Library",
    text: "Get building blocks from the library",
    icon: "fas fa-books",
    action: ACTION_COMING_SOON,
  },
  "space.open.docs": {
    title: "Read the Docs",
    text: "Get help from our examples and guides",
    icon: "fas fa-book-open",
    action: ACTION_COMING_SOON,
  },
  "space.open.discord": {
    title: "Join Discord",
    text: "Join our community on Discord",
    icon: "fab fa-discord",
    url: DISCORD_URL,
    action: () => {
      // open in new tab
      window.open(DISCORD_URL, "_blank");
    },
  },
});
