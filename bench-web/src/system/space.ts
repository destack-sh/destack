import {
  BenchType,
  NodeReferenceData,
  NodeType,
  Orientation,
  SpaceData,
  StructType,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { copyNode, makeNode, makeStruct, toNodeReference } from "@/proto/wiring";
import { spaceGraphLocal, useGetNodes, useLoadedGraph } from "@/system/connection";
import { NodeGraph, ProxyNodeGraph, type ReadNodeGraph } from "@/system/graph";
import { makeIcon } from "@/system/icon";
import { LOADED_SOURCE_NODE_TYPES, ROOT_VIEW_TYPES, getOrderKey, updateOrderKey } from "@/system/lang";
import { LOCAL_PACKAGE_PTR, LOCAL_SPACE_ID, benchPtr, packagePtr, spacePtr } from "@/system/local";
import type { Transaction } from "@/system/transaction";
import type { SplitAnchor } from "@/utils/drag";
import { DEFAULT_ORIENTATION, splitBox } from "@/utils/layout";
import { log } from "@/utils/log";
import { ViewCanvas } from "@/views/canvas";
import { computed, watch } from "vue";

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
export const spaceConnection = useLoadedGraph(spacePtr);
export const canvas = new ViewCanvas(spacePtr, spaceGraph, () => spaceConnection.connection.sideTx);

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
    canvas.restoreComponentFocus();
  },
  { immediate: true },
);

function setupLocalSpace(graph: NodeGraph): { space: SpaceData } {
  log.info("setupLocalSpace");
  const space = { metatype: BenchType.SPACE, id: LOCAL_SPACE_ID, packagePtr: LOCAL_PACKAGE_PTR } as SpaceData;
  graph.add(space);
  for (let i = 0; i < 2; i++) {
    const root = makeNode({
      metatype: NodeType.VIEW,
      parentPtr: toNodeReference(space),
      packagePtr: LOCAL_PACKAGE_PTR,
      type: ViewType.TABBED,
      name: `Window ${i}`,
      title: `Window ${i}`,
      orderKey: `a${i}`,
      orientation: Orientation.HORIZONTAL,
    });
    graph.add(root);
  }
  let ord = 0;
  for (const node of graph.nodes) {
    if ((node as ViewData).type == ViewType.TABBED) {
      for (const i of [0, 1]) {
        graph.add(
          makeNode({
            metatype: NodeType.VIEW,
            parentPtr: toNodeReference(node),
            packagePtr: LOCAL_PACKAGE_PTR,
            type: ViewType.USER_WIZARD,
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

