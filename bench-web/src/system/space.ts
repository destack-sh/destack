import { BenchType, NodeType, SpaceData, StructType, ViewType } from "@/proto/wire";
import { makeNode, makeStruct, nodeReference, toNodeReference } from "@/proto/wiring";
import { spaceGraphLocal, useGetNodes } from "@/system/connection";
import { NodeGraph, ProxyNodeGraph } from "@/system/graph";
import { LOADED_SOURCE_NODE_TYPES } from "@/system/lang";
import { LOCAL_BENCH_ID, LOCAL_PACKAGE_ID, LOCAL_SPACE_ID, benchPtr, packagePtr, spacePtr } from "@/system/local";
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
      spaceGraph.graph.value = spaceGraphLocal;
      if (spaceGraphLocal.size == 0) {
        const { space } = setupLocalSpace(spaceGraphLocal);
        spacePtr.value = toNodeReference(space);
      }
    } else {
      spaceGraph.graph.value = pkgGraph;
    }
  },
  { immediate: true },
);

function setupLocalSpace(graph: NodeGraph): { space: SpaceData } {
  log.info("setupLocalSpace");
  const packagePtr = nodeReference(NodeType.PACKAGE, LOCAL_PACKAGE_ID, LOCAL_BENCH_ID);
  const space = { metatype: BenchType.SPACE, id: LOCAL_SPACE_ID, packagePtr } as SpaceData;
  const side = makeNode({
    metatype: NodeType.VIEW,
    parentPtr: toNodeReference(space),
    packagePtr,
    type: ViewType.TABBED,
    name: "side",
    title: "Side Window",
    size: makeStruct({ metatype: StructType.BOX, width: 240 }),
  });
  const primary = makeNode({
    metatype: NodeType.VIEW,
    parentPtr: toNodeReference(space),
    packagePtr,
    type: ViewType.TABBED,
    name: "primary",
    title: "Primary Window",
    size: makeStruct({ metatype: StructType.BOX, widthRelative: 1.5 }),
  });
  const secondary = makeNode({
    metatype: NodeType.VIEW,
    parentPtr: toNodeReference(space),
    packagePtr,
    type: ViewType.TABBED,
    name: "secondary",
    title: "Secondary Window",
    size: makeStruct({ metatype: StructType.BOX, widthRelative: 1 }),
  });
  graph.extend(space, side, primary, secondary);
  return { space };
}
