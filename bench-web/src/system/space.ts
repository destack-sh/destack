import { BenchType, NodeType, Orientation, SpaceData, StructType, ViewData, ViewType } from "@/proto/wire";
import { makeNode, makeStruct, toNodeReference } from "@/proto/wiring";
import { spaceGraphLocal, useGetNodes } from "@/system/connection";
import { NodeGraph, ProxyNodeGraph } from "@/system/graph";
import { makeIcon } from "@/system/icon";
import { LOADED_SOURCE_NODE_TYPES, ROOT_VIEW_TYPES } from "@/system/lang";
import { LOCAL_PACKAGE_PTR, LOCAL_SPACE_ID, benchPtr, packagePtr, spacePtr } from "@/system/local";
import type { Transaction } from "@/system/transaction";
import { log } from "@/utils/log";
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
  const side = makeNode({
    metatype: NodeType.VIEW,
    parentPtr: toNodeReference(space),
    packagePtr: LOCAL_PACKAGE_PTR,
    type: ViewType.WINDOWED,
    name: "side",
    title: "Side Window",
    orientation: Orientation.VERTICAL,
    size: makeStruct({ metatype: StructType.BOX, width: 280 }),
  });
  const sideTop = makeNode({
    metatype: NodeType.VIEW,
    parentPtr: toNodeReference(side),
    packagePtr: LOCAL_PACKAGE_PTR,
    type: ViewType.TABBED,
  });
  const sideBottom = makeNode({
    metatype: NodeType.VIEW,
    parentPtr: toNodeReference(side),
    packagePtr: LOCAL_PACKAGE_PTR,
    type: ViewType.TABBED,
  });
  const primary = makeNode({
    metatype: NodeType.VIEW,
    parentPtr: toNodeReference(space),
    packagePtr: LOCAL_PACKAGE_PTR,
    type: ViewType.TABBED,
    name: "primary",
    title: "Primary Window",
    size: makeStruct({ metatype: StructType.BOX, widthRelative: 1.5 }),
  });
  const secondary = makeNode({
    metatype: NodeType.VIEW,
    parentPtr: toNodeReference(space),
    packagePtr: LOCAL_PACKAGE_PTR,
    type: ViewType.TABBED,
    name: "secondary",
    title: "Secondary Window",
    size: makeStruct({ metatype: StructType.BOX, widthRelative: 1 }),
  });
  graph.extend(space, side, primary, secondary, sideTop, sideBottom);
  // nocheckin testing
  for (const node of graph.nodes) {
    if ((node as ViewData).type == ViewType.TABBED) {
      graph.add(
        makeNode({
          metatype: NodeType.VIEW,
          parentPtr: toNodeReference(node),
          packagePtr: LOCAL_PACKAGE_PTR,
          type: ViewType.USER_WIZARD,
          icon: makeIcon({ name: "fas fa-right-from-bracket" }),
          title: "Registration 1",
        }),
      );
      graph.add(
        makeNode({
          metatype: NodeType.VIEW,
          parentPtr: toNodeReference(node),
          packagePtr: LOCAL_PACKAGE_PTR,
          type: ViewType.BENCH_WIZARD,
          title: "Bench Wizard! 2 Very Long Title Yes Very Long Indeed (I mean it)",
        }),
      );
      graph.add(
        makeNode({
          metatype: NodeType.VIEW,
          parentPtr: toNodeReference(node),
          packagePtr: LOCAL_PACKAGE_PTR,
          type: ViewType.USER_WIZARD,
        }),
      );
    }
  }

  return { space };
}

export function addViewToCurrentRoot(
  view: Partial<Omit<ViewData, "metatype">> & Pick<ViewData, "type">,
  tx: Transaction,
) {
  // nocheckin: handle & assign current root view etc.
  const root = spaceGraph.nodes.find(
    (n) => n.metatype == BenchType.VIEW && ROOT_VIEW_TYPES.includes((n as ViewData).type),
  );
  if (root == null) throw new Error("no root view");
  tx.create(
    makeNode({
      metatype: NodeType.VIEW,
      ...view,
      packagePtr: space.value?.packagePtr,
      parentPtr: toNodeReference(root),
    }),
  );
}
