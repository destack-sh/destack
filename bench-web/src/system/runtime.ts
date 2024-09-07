import type { ReadNodeGraph } from "@/language/graph";
import { makeRun, type RunnableObject } from "@/language/session";
import type { Transaction } from "@/language/transaction";
import { ChangeCategory, NodeType, type NodeReferenceData, type RunData } from "@/proto/wire";
import { toNodeRef, type SomeNodeReferenceData, type TypedNodeReferenceData } from "@/proto/wiring";
import { useGetConnection, type Connection } from "@/system/connection";
import { canvas, pkgConnection, pkgGraph, space } from "@/system/space";
import { computedValue } from "@/utils/ref";
import { computed, type Ref } from "vue";

/**
 * Runtime for managing runtime Nodes, wrapping the local runtime and related stuff.
 */
export class Runtime {
  graph: ReadNodeGraph;
  txFactory: () => Transaction;
  runGraph: ReadNodeGraph;
  runPtr: Ref<TypedNodeReferenceData<NodeType.RUN> | null>; // the current active Run
  runRef: Ref<RunData | null>; // the current active Run
  runConnection: Connection<"get", NodeType.RUN>;

  constructor(graph: ReadNodeGraph, txFactory: () => Transaction, runPtr: Ref<SomeNodeReferenceData | null>) {
    this.graph = graph;
    this.txFactory = txFactory;

    // active run
    this.runPtr = runPtr as Ref<TypedNodeReferenceData<NodeType.RUN> | null>;
    const { graph: runGraph, connection: runConnection } = useGetConnection(
      { name: "runtime.run", live: true },
      computed(() => ({
        scope: graph.scope,
        roots: [this.runPtr.value!],
        options: { descendantTypes: [NodeType.RUN] },
        isEnabled: this.runPtr.value != null,
      })),
    );
    this.runGraph = runGraph;
    this.runRef = runGraph.getRef(this.runPtr);
    this.runConnection = runConnection;
  }

  get tx() {
    return this.txFactory();
  }

  /** The current active Run. */
  get run() {
    return this.runRef.value;
  }

  /** Creates a new Run and makes it the current active Run in the Space. */
  createRun(
    runnable: RunnableObject,
    options?: { inputsPacked?: Record<string, any>; packagePtr?: NodeReferenceData },
  ): RunData {
    const run = makeRun(this.graph, runnable, options);
    this.txFactory().with({ category: ChangeCategory.SESSION }).create(run);
    if (canvas.space.value != null) {
      canvas.tx().update(canvas.space.value, { runPtr: toNodeRef(run) }, { debounce: "short" });
    }
    return run;
  }
}

const runPtr = computedValue(() => space.value?.runPtr ?? null);
export const runtime = new Runtime(pkgGraph, () => pkgConnection.tx, runPtr);
