import { ACTIVE_RUN_STATUSES } from "@/language/const";
import { makeExpression } from "@/language/expression";
import type { ReadNodeGraph } from "@/language/graph";
import { timesortNode } from "@/language/order";
import {
  getRunBasePtr as getRunBasePtr,
  isRunActive,
  isRunInterrupted,
  isRunnable,
  isRunTerminal,
  makeRun,
  type RunnableObject,
} from "@/language/session";
import { CONNECTION_IGNORE, type Transaction } from "@/language/transaction";
import {
  BlockData,
  ChangeCategory,
  ExpressionType,
  IconData,
  InterruptData,
  InterruptStatus,
  NodeReferenceData,
  NodeType,
  ObjectType,
  RunOptionsData,
  RunProperty,
  RunStatus,
  StepData,
  Timestamp,
  type RunData,
} from "@/proto/wire";
import { Duration } from "@/proto/wire/google/protobuf/duration";
import { describeNode, propertyReference, toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { supergraph, useGetConnection, useSearchConnection, type Connection } from "@/system/connection";
import { pkgConnection, pkgGraph, space, spaceConnection } from "@/system/space";
import { makeIcon } from "@/ui/icon";
import { log } from "@/utils/log";
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
  interruptsRef: Ref<InterruptData[]>;

  constructor(graph: ReadNodeGraph, runPtr: Ref<TypedNodeReferenceData<NodeType.RUN> | null>) {
    this.runPtr = runPtr as Ref<TypedNodeReferenceData<NodeType.RUN> | null>;
    const { graph: runGraph, connection: runConnection } = useGetConnection(
      { name: "runtime.run." + this.id, live: true },
      computed(() => ({
        scope: graph.scope,
        roots: [this.runPtr.value!],
        ancestorTypes: [NodeType.RUN],
        descendantTypes: [NodeType.RUN, NodeType.INTERRUPT],
        isOptional: true,
        isEnabled: this.runPtr.value != null,
      })),
    );
    this.runGraph = runGraph;
    this.runRef = runGraph.getRef(this.runPtr, { id: "runtime.run." + this.id, ignoreAncestors: false }); // :NodeRefStability
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
        const base = getRunBasePtr(run);
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
    this.interruptsRef = runGraph.getOfTypeRef(NodeType.INTERRUPT);
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

  hasBase(base: { ck?: string }) {
    return this.runsByBaseCk.value[base.ck!] != null;
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

  constructor(graph: ReadNodeGraph, txFactory: () => Transaction, runPtr: Ref<NodeReferenceData | null>) {
    this.graph = graph;
    this.txFactory = txFactory;

    this.focusedRunTree = new RunTree(graph, runPtr as Ref<TypedNodeReferenceData<NodeType.RUN> | null>);
    const { roots: activeRuns } = useSearchConnection(
      { name: "runtime.runs.active", live: true },
      computed(() => ({
        scope: graph.scope,
        nodeType: NodeType.RUN,
        filter: makeExpression({
          type: ExpressionType.AND,
          clauses: [
            makeExpression({
              type: ExpressionType.NOT_EXISTS,
              propertyPtr: propertyReference(NodeType.RUN, RunProperty.rootPtr),
            }),
            makeExpression({
              type: ExpressionType.IN,
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
    return this.txFactory().with({ connectionId: CONNECTION_IGNORE, category: ChangeCategory.RUNTIME });
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

  /** Creates a new Run. */
  start(
    runnable: RunnableObject,
    options?: {
      focus?: boolean;
      options?: RunOptionsData;
      inputsPacked?: Record<string, any>;
      packagePtr?: NodeReferenceData;
    },
  ): RunData {
    const run = makeRun(this.graph, runnable, options);
    this.tx.create(run);
    if (options?.focus) {
      spaceConnection.tx.update(space.value!, { runPtr: toNodeRef(run) });
    }
    log.trace("runtime.start", run);
    return run;
  }

  /** Pause a Run. */
  pause(run: RunData) {
    log.trace("runtime.pause", run);
    this.tx.update(run, { pausedAt: Timestamp.now() });
  }

  /** Resume a Run. */
  resume(run: RunData) {
    log.trace("runtime.resume", run);
    this.tx.update(run, { resumedAt: Timestamp.now() });
  }

  /** Stop a Run. */
  kill(run: RunData) {
    log.trace("runtime.kill", run);
    this.tx.update(run, { killedAt: Timestamp.now() });
  }

  /** Complete an Interrupt */
  complete(interrupt: InterruptData) {
    log.trace("runtime.complete", interrupt);
    const tx = this.tx;
    tx.update(interrupt, { status: InterruptStatus.COMPLETED, closedAt: Timestamp.now() }, { debounce: "tick" });
  }

  /*+ Cancel an Interrupt */
  cancel(interrupt: InterruptData) {
    log.trace("runtime.cancel", interrupt);
    const tx = this.tx;
    tx.update(interrupt, { status: InterruptStatus.CANCELLED, closedAt: Timestamp.now() }, { debounce: "tick" });
  }
}

const runPtr = computedValue(() => space.value?.runPtr ?? null);
export const runtime = new Runtime(pkgGraph, () => pkgConnection.tx, runPtr);

type RuntimeAction = { title: string; isPrimary?: boolean; icon: IconData; action: () => void };

/** Gets the available actions for a Run */
export function getRunActions(run: RunData): RuntimeAction[] {
  const actions: RuntimeAction[] = [];
  if (isRunActive(run)) {
    if (run.status == RunStatus.PAUSED) {
      actions.push({
        title: "Resume",
        icon: makeIcon("fas fa-pause"),
        action: () => {
          runtime.pause(run);
        },
      });
    } else if (!isRunInterrupted(run)) {
      actions.push({
        title: "Pause",
        icon: makeIcon("fas fa-play"),
        action: () => {
          runtime.pause(run);
        },
      });
    }
    actions.push({
      title: "Stop",
      icon: makeIcon("fas fa-stop"),
      action: () => {
        runtime.kill(run);
      },
    });
  } else if (isRunTerminal(run)) {
    actions.push({
      title: "Restart",
      icon: makeIcon("fas fa-redo"),
      action: () => {
        const basePtr = getRunBasePtr(run);
        if (basePtr == null) throw new Error(`no base for ${describeNode(run)}`);
        const node = supergraph.get(basePtr);
        if (!isRunnable(node)) throw new Error(`no node for base ${describeNode(basePtr)} of ${describeNode(run)}`);
        runtime.start(node, { focus: true, packagePtr: run.packagePtr, options: run.options });
      },
    });
  }
  return actions;
}

export const CLEAR_RUN_ACTION: RuntimeAction = {
  title: "Clear",
  icon: makeIcon("fas fa-xmark"),
  action: () => {
    if (space.value == null) throw new Error("no current space");
    spaceConnection.tx.update(space.value, { runPtr: undefined });
  },
};

export function getInterruptActions(interrupt: InterruptData): RuntimeAction[] {
  const actions: RuntimeAction[] = [];
  if (interrupt.status == InterruptStatus.OPEN) {
    actions.push({
      title: "Complete",
      icon: makeIcon("fas fa-play"),
      action: () => {
        runtime.complete(interrupt);
      },
    });
  }
  return actions;
}
