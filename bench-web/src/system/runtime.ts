import { ACTIVE_RUN_STATUSES } from "@/language/const";
import { makeExpression } from "@/language/expression";
import type { ReadNodeGraph } from "@/language/graph";
import { timesortNode } from "@/language/order";
import { makeRun, type RunnableObject } from "@/language/session";
import { CONNECTION_IGNORE, type Transaction } from "@/language/transaction";
import {
  BlockData,
  ChangeCategory,
  ExpressionOp,
  NodeType,
  ObjectType,
  RunProperty,
  StepData,
  type NodeReferenceData,
  type RunData,
} from "@/proto/wire";
import { propertyReference, toNodeRef, type SomeNodeReferenceData, type TypedNodeReferenceData } from "@/proto/wiring";
import { useGetConnection, useSearchConnection, type Connection } from "@/system/connection";
import { canvas, pkgConnection, pkgGraph, space } from "@/system/space";
import { computedValue } from "@/utils/ref";
import { computed, type Ref } from "vue";

/** A reactive Run with all its descendants */
let treeId = 0;
export class RunTree {
  id: number = treeId++;
  runGraph: ReadNodeGraph;
  runPtr: Ref<TypedNodeReferenceData<NodeType.RUN> | null>;
  runRef: Ref<RunData | null>;
  runsRef: Ref<RunData[]>;
  runBasePtr: Ref<TypedNodeReferenceData<NodeType.BLOCK | NodeType.STEP> | null>;
  runBaseRef: Ref<BlockData | StepData | null>;
  runConnection: Connection<"get", NodeType.RUN>;
  runsByBaseCk: Ref<Record<string, RunData[]>>;

  constructor(graph: ReadNodeGraph, runPtr: Ref<TypedNodeReferenceData<NodeType.RUN> | null>) {
    this.runPtr = runPtr as Ref<TypedNodeReferenceData<NodeType.RUN> | null>;
    const { graph: runGraph, connection: runConnection } = useGetConnection(
      { name: "runtime.run." + this.id, live: true },
      computed(() => ({
        scope: graph.scope,
        roots: [this.runPtr.value!],
        options: { ancestorTypes: [NodeType.RUN], descendantTypes: [NodeType.RUN] },
        isOptional: true,
        isEnabled: this.runPtr.value != null,
      })),
    );
    this.runGraph = runGraph;
    this.runRef = runGraph.getRef(this.runPtr, { id: "runtime.run." + this.id, ignoreAncestors: true });
    this.runsRef = runGraph.getDescendantsRef(this.runPtr, { metatypes: [NodeType.RUN], includeSelf: true });
    this.runBasePtr = computedValue(
      () =>
        (this.runRef.value?.stepPtr ?? this.runRef.value?.blockPtr) as TypedNodeReferenceData<
          NodeType.BLOCK | NodeType.STEP
        >,
    );
    this.runBaseRef = graph.getRef(this.runBasePtr);
    this.runConnection = runConnection;
    this.runsByBaseCk = computed(() => {
      const runByBaseCk: Record<string, RunData[]> = {};
      for (const run of this.runsRef.value) {
        const base = run.stepPtr ?? run.blockPtr;
        if (base?.ck != null) {
          if (runByBaseCk[base.ck] == null) {
            runByBaseCk[base.ck] = [];
          }
          runByBaseCk[base.ck].push(run);
        }
      }
      // sort ascending
      for (const runBaseCk of Object.keys(runByBaseCk)) {
        timesortNode(runByBaseCk[runBaseCk]);
      }
      return runByBaseCk;
    });
  }

  /** The root Run. */
  get run() {
    return this.runRef.value;
  }

  /** The current active Run tree (preorder). */
  get runs() {
    return this.runsRef.value;
  }

  /** The base node of the current active Run. */
  get base() {
    return this.runBaseRef.value;
  }

  /** Gets the last (active) Runs for the given base node. */
  getLastActiveRuns(base: { ck?: string }): RunData[] {
    return this.runsByBaseCk.value[base.ck!] ?? [];
  }

  /** Gets the last (active) Run for the given base node. */
  getLastActiveRun(base: { ck?: string }): RunData | null {
    const runs = this.runsByBaseCk.value[base.ck!];
    return runs?.[runs.length - 1] ?? null;
  }
}

/**
 * Runtime for managing runtime Nodes, wrapping the local runtime and related stuff.
 */
export class Runtime {
  graph: ReadNodeGraph;
  txFactory: () => Transaction;
  focusedRunTree: RunTree;
  activeRootRuns: Ref<RunData[]>;

  constructor(graph: ReadNodeGraph, txFactory: () => Transaction, runPtr: Ref<SomeNodeReferenceData | null>) {
    this.graph = graph;
    this.txFactory = txFactory;

    this.focusedRunTree = new RunTree(graph, runPtr as Ref<TypedNodeReferenceData<NodeType.RUN> | null>);
    const { roots: activeRuns } = useSearchConnection(
      { name: "runtime.activeRuns", live: true },
      computed(() => ({
        scope: graph.scope,
        nodeType: NodeType.RUN,
        filter: makeExpression({
          op: ExpressionOp.AND,
          clauses: [
            makeExpression({
              op: ExpressionOp.NOT_EXISTS,
              propertyPtr: propertyReference(NodeType.RUN, RunProperty.rootPtr),
            }),
            makeExpression({
              op: ExpressionOp.IN,
              propertyPtr: propertyReference(ObjectType.RUN, RunProperty.status),
              value: ACTIVE_RUN_STATUSES,
            }),
          ],
        }),
        isEnabled: graph.scope?.benchId != null,
      })),
    );
    this.activeRootRuns = activeRuns;
  }

  get tx() {
    return this.txFactory();
  }

  /** The current Run. */
  get focusedRun() {
    return this.focusedRunTree.run;
  }

  /** The current Run base node. */
  get focusedRunBase() {
    return this.focusedRunTree.base;
  }

  /** The current active Run roots. */
  get activeRuns() {
    return this.activeRootRuns.value;
  }

  /** Creates a new Run and makes it the current active Run in the Space. */
  createRun(
    runnable: RunnableObject,
    options?: { inputsPacked?: Record<string, any>; packagePtr?: NodeReferenceData },
  ): RunData {
    const run = makeRun(this.graph, runnable, options);
    this.txFactory().with({ connectionId: CONNECTION_IGNORE, category: ChangeCategory.SESSION }).create(run);
    if (canvas.space.value != null) {
      canvas.tx().update(canvas.space.value, { runPtr: toNodeRef(run) }, { debounce: "short" });
    }
    return run;
  }
}

const runPtr = computedValue(() => space.value?.runPtr ?? null);
export const runtime = new Runtime(pkgGraph, () => pkgConnection.tx, runPtr);
