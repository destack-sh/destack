import { ACTIVE_RUN_STATUSES, getBaseFromNode, isResourceNodeType, TK_LENGTH_B64 } from "@/language/core/const";
import { makeExpression } from "@/language/core/expression";
import type { ReadNodeGraph } from "@/language/core/graph";
import { makeNode } from "@/language/core/node";
import { timesortNode } from "@/language/core/order";
import { decodeTypeIdentity } from "@/language/core/type";
import { unpackValue } from "@/language/core/value";
import {
  getRunType,
  isRunActive,
  isRunnable,
  isRunPaused,
  isRunTerminal,
  RunnableNode,
  RunnableNodeType,
} from "@/language/runtime/run";
import { newChangeId, type Transaction } from "@/language/runtime/transaction";
import { actionToType } from "@/language/source/action";
import { flowToType } from "@/language/source/flow";
import {
  ActionData,
  ChangeCategory,
  ExpressionType,
  FieldType,
  FlowData,
  IconData,
  InterruptionData,
  InterruptionStatus,
  NodeMode,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PipeData,
  RunOptionsData,
  RunProperty,
  RunSpanData,
  RunStatus,
  StructType,
  Timestamp,
  type RunData,
} from "@/proto/wire";
import {
  describeNode,
  isNode,
  isStruct,
  makeDefaultObject,
  propertyReference,
  toNodeRef,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { supergraph, useGetConnection, useSearchConnection, type Connection } from "@/system/connection";
import { pkgConnection, pkgGraph, space, spaceConnection } from "@/system/space";
import { declareActions } from "@/ui/action";
import { makeIcon } from "@/ui/icon";
import { log } from "@/utils/log";
import { computedValue } from "@/utils/ref";
import { assertNever } from "@protobuf-ts/runtime";
import { computed, type Ref } from "vue";

/** A reactive Run with all its descendants */
let treeId = 0;
export class RunTree {
  id: number = treeId++;
  runGraph: ReadNodeGraph;
  runPtr: Ref<TypedNodeReferenceData<NodeType.RUN> | null>;
  runRef: Ref<RunData | null>;
  runsRef: Ref<(RunData | RunSpanData)[]>;
  runBasePtr: Ref<TypedNodeReferenceData<RunnableNodeType> | null>;
  runBaseRef: Ref<RunnableNode | null>;
  runConnection: Connection<"get", NodeType.RUN>;
  runsByBaseCk: Ref<Record<string, RunData[]>>;
  basesPtrs: Ref<TypedNodeReferenceData<RunnableNodeType>[]>;
  basesRef: Ref<RunnableNode[]>;
  baseByCkRef: Ref<Record<string, RunnableNode>>;
  interruptionsRef: Ref<InterruptionData[]>;

  constructor(graph: ReadNodeGraph, runPtr: Ref<TypedNodeReferenceData<NodeType.RUN> | null>) {
    this.runPtr = runPtr as Ref<TypedNodeReferenceData<NodeType.RUN> | null>;
    const { graph: runGraph, connection: runConnection } = useGetConnection(
      { name: "runtime.run." + this.id, live: true },
      computed(() => ({
        scope: graph.scope,
        roots: [this.runPtr.value!],
        ancestorTypes: [NodeType.RUN],
        descendantTypes: [NodeType.RUN, NodeType.RUN_SPAN, NodeType.INTERRUPTION],
        isOptional: true,
        isEnabled: this.runPtr.value != null,
      })),
    );
    this.runGraph = runGraph;
    this.runRef = runGraph.getRef(this.runPtr, { id: "runtime.run." + this.id, ignoreAncestors: false }); // :NodeRefStability
    this.runsRef = runGraph.getDescendantsRef(this.runPtr, {
      metatypes: [NodeType.RUN, NodeType.RUN_SPAN],
      includeSelf: true,
    });
    this.runBasePtr = computedValue(
      () =>
        (this.runRef.value?.pipePtr ??
          this.runRef.value?.actionPtr ??
          this.runRef.value?.flowPtr) as TypedNodeReferenceData<RunnableNodeType>,
    );
    this.runBaseRef = graph.getRef(this.runBasePtr);
    this.runConnection = runConnection;
    this.runsByBaseCk = computed(() => {
      const runByBaseCk: Record<string, RunData[]> = {};
      for (const run of this.runsRef.value) {
        if (!isNode(run, NodeType.RUN)) continue;
        const base = getBaseFromNode(run);
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
    this.basesPtrs = computed(() => {
      const basePtrs: TypedNodeReferenceData<RunnableNodeType>[] = [];
      for (const run of this.runsRef.value) {
        const base = getBaseFromNode(run);
        if (base != null) basePtrs.push(base as TypedNodeReferenceData<RunnableNodeType>);
      }
      return basePtrs;
    });
    this.basesRef = graph.getManyRef(this.basesPtrs);
    this.baseByCkRef = computed(() => {
      const basesByCk: Record<string, RunnableNode> = {};
      for (const base of this.basesRef.value) {
        basesByCk[base.ck] = base;
      }
      return basesByCk;
    });
    this.interruptionsRef = runGraph.getOfTypeRef(NodeType.INTERRUPTION);
  }

  /** A transaction for this RunTree. */
  get tx() {
    return this.runConnection.tx.with({ category: ChangeCategory.RUNTIME });
  }

  /** The root Run. */
  get run() {
    return this.runRef.value;
  }

  /** The current active Run tree (preorder). */
  get runs() {
    return this.runsRef.value;
  }

  /** The interruptions for the current active Run. */
  get interruptions() {
    return this.interruptionsRef.value;
  }

  /** The base node of the current active Run. */
  get base() {
    return this.runBaseRef.value;
  }

  hasBase(base: { ck?: string }) {
    return this.runsByBaseCk.value[base.ck!] != null;
  }

  getBase(base: { ck?: string }) {
    return this.baseByCkRef.value[base.ck!];
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
    return this.txFactory().with({ connectionId: null, category: ChangeCategory.RUNTIME });
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
    runnable: RunnableNode,
    options?: {
      focus?: boolean;
      options?: RunOptionsData;
      variablesPacked?: Record<string, any>;
      inputsPacked?: Record<string, any>;
      benchPtr?: NodeReferenceData;
      tx?: Transaction;
    },
  ): RunData {
    const tx = options?.tx ?? this.tx;
    const run = makeRun(this.graph, runnable, options);
    tx.create(run);
    if (options?.focus) {
      spaceConnection.tx.update(space.value!, { runPtr: toNodeRef(run) });
    }
    log.trace("runtime.start", run);
    return run;
  }

  /** Replay a Run. */
  replay(run: RunData, options?: { tx?: Transaction }) {
    const basePtr = getBaseFromNode(run);
    if (basePtr == null) throw new Error(`no base for ${describeNode(run)}`);
    const node = supergraph.get(basePtr);
    if (!isRunnable(node)) throw new Error(`no node for base ${describeNode(basePtr)} of ${describeNode(run)}`);
    let variablesPacked: Record<string, any> | undefined = undefined;
    for (const [key, valuePacked] of Object.entries(run.variablesPacked ?? {})) {
      const type = decodeTypeIdentity(key.slice(TK_LENGTH_B64 + 1));
      const value = unpackValue(valuePacked, type);
      if (!(isStruct(value, StructType.NODE_REFERENCE) && isResourceNodeType(value.nodeType))) {
        // ignore resources
        if (variablesPacked == null) variablesPacked = {};
        variablesPacked[key] = valuePacked;
      }
    }
    this.start(node, {
      focus: true,
      benchPtr: run.benchPtr,
      variablesPacked,
      inputsPacked: run.inputsPacked as Record<string, any> | undefined,
      options: run.options,
      tx: options?.tx,
    });
  }

  /** Pause a Run. */
  pause(run: RunData, options?: { tx?: Transaction }) {
    if (!isRunActive(run)) return;
    log.trace("runtime.pause", run);
    const tx = options?.tx ?? this.tx;
    tx.update(run, { pausedAt: Timestamp.now() });
  }

  /** Resume a Run. */
  resume(run: RunData, options?: { tx?: Transaction }) {
    if (run.status != RunStatus.PAUSED) return;
    log.trace("runtime.resume", run);
    const tx = options?.tx ?? this.tx;
    tx.update(run, { resumedAt: Timestamp.now() });
  }

  /** Stop a Run. */
  stop(run: RunData, options?: { tx?: Transaction }) {
    if (!isRunActive(run)) return;
    log.trace("runtime.stop", run);
    const tx = options?.tx ?? this.tx;
    tx.update(run, { stoppedAt: Timestamp.now() });
  }

  /** Complete an Interrupt */
  complete(interrupt: InterruptionData, options?: { tx?: Transaction }) {
    if (interrupt.status != InterruptionStatus.OPEN) return;
    log.trace("runtime.complete", interrupt);
    const tx = options?.tx ?? this.tx;
    tx.update(interrupt, { status: InterruptionStatus.COMPLETED, closedAt: Timestamp.now() }, { debounce: "tick" });
  }

  /*+ Cancel an Interrupt */
  cancel(interrupt: InterruptionData, options?: { tx?: Transaction }) {
    if (interrupt.status != InterruptionStatus.OPEN) return;
    log.trace("runtime.cancel", interrupt);
    const tx = options?.tx ?? this.tx;
    tx.update(interrupt, { status: InterruptionStatus.CANCELLED, closedAt: Timestamp.now() }, { debounce: "tick" });
  }
}

const runPtr = computedValue(() => space.value?.runPtr ?? null);
export const runtime = new Runtime(pkgGraph, () => pkgConnection.tx, runPtr);

export function makeRunOptions(options?: Partial<RunOptionsData>): RunOptionsData {
  return makeDefaultObject({ metatype: ObjectType.RUN_OPTIONS, ...options }) as RunOptionsData;
}

/** Make a new Run for some runnable node */
export function makeRun(
  graph: ReadNodeGraph,
  runnable: RunnableNode,
  options?: {
    variablesPacked?: Record<string, any>;
    inputsPacked?: Record<string, any>;
    benchPtr?: NodeReferenceData;
    options?: RunOptionsData;
    mode?: NodeMode;
  },
): RunData {
  let benchPtr: NodeReferenceData | undefined = undefined;
  let flow: FlowData | undefined = undefined;
  let action: ActionData | undefined = undefined;
  let pipe: PipeData | undefined = undefined;
  if (isNode(runnable, NodeType.FLOW)) {
    flow = runnable;
    benchPtr = options?.benchPtr ?? runnable.benchPtr;
  } else if (isNode(runnable, NodeType.ACTION)) {
    action = runnable;
    flow = graph.getAncestors(action, { includeSelf: true }).find((node) => isNode(node, NodeType.FLOW));
    benchPtr = options?.benchPtr ?? action.benchPtr;
  } else if (isNode(runnable, NodeType.PIPE)) {
    pipe = runnable;
    flow = graph.getAncestors(pipe, { includeSelf: true }).find((node) => isNode(node, NodeType.FLOW));
    benchPtr = options?.benchPtr ?? pipe.benchPtr;
  } else {
    assertNever(runnable);
  }
  const run = makeNode({
    metatype: NodeType.RUN,
    parentPtr: benchPtr,
    type: getRunType(runnable),
    status: RunStatus.SCHEDULED,
    mode: options?.mode ?? space.value?.mode ?? NodeMode.PRODUCTION,
    flowPtr: flow != null ? toNodeRef(flow) : undefined,
    actionPtr: isNode(runnable, NodeType.ACTION) ? toNodeRef(runnable) : undefined,
    pipePtr: isNode(runnable, NodeType.PIPE) ? toNodeRef(runnable) : undefined,
    variablesPacked: options?.variablesPacked ?? undefined,
    inputsPacked: options?.inputsPacked ?? undefined,
    options: makeRunOptions(options?.options),
  });
  return run;
}

type RuntimeAction = { title: string; isPrimary?: boolean; icon: IconData; action: () => void };

/** Gets the available actions for a Run */
export function getRunActions(run: RunData): RuntimeAction[] {
  const actions: RuntimeAction[] = [];
  if (isRunActive(run)) {
    if (isRunPaused(run)) {
      actions.push({
        title: "Resume",
        icon: makeIcon("fas fa-play"),
        action: () => {
          runtime.resume(run);
        },
      });
    } else {
      actions.push({
        title: "Pause",
        icon: makeIcon("fas fa-pause"),
        action: () => {
          runtime.pause(run);
        },
      });
    }
    actions.push({
      title: "Stop",
      icon: makeIcon("fas fa-stop"),
      action: () => {
        runtime.stop(run);
      },
    });
  } else if (isRunTerminal(run) && run.rootPtr == null) {
    actions.push({
      title: "Replay",
      isPrimary: true,
      icon: makeIcon("fas fa-redo"),
      action: () => {
        runtime.replay(run);
      },
    });
  }
  return actions;
}

export const CLEAR_RUN_ACTION: RuntimeAction = {
  title: "Hide",
  icon: makeIcon("fas fa-xmark"),
  action: () => {
    if (space.value == null) throw new Error("no current space");
    spaceConnection.tx.update(space.value, { runPtr: undefined });
  },
};

export function getInterruptActions(interrupt: InterruptionData): RuntimeAction[] {
  const actions: RuntimeAction[] = [];
  if (interrupt.status == InterruptionStatus.OPEN) {
    actions.push({
      title: "Complete",
      icon: makeIcon("fas fa-check"),
      isPrimary: true,
      action: () => {
        runtime.complete(interrupt);
      },
    });
    actions.push({
      title: "Cancel",
      icon: makeIcon("fas fa-xmark"),
      action: () => {
        runtime.cancel(interrupt);
      },
    });
  }
  return actions;
}

export function getInputType(node: RunnableNode) {
  if (isNode(node, NodeType.ACTION)) {
    return actionToType(node, "value", [FieldType.INPUT]);
  } else if (isNode(node, NodeType.FLOW)) {
    return flowToType(node, "value", [FieldType.INPUT]);
  } else {
    return undefined;
  }
}

export function getOutputType(node: RunnableNode) {
  if (isNode(node, NodeType.ACTION)) {
    return actionToType(node, "value", [FieldType.OUTPUT]);
  } else if (isNode(node, NodeType.FLOW)) {
    return flowToType(node, "value", [FieldType.OUTPUT]);
  } else {
    return undefined;
  }
}

// runtime
declareActions<"runtime">({
  // run
  "runtime.run.start": {
    icon: "fas fa-play",
    title: "Run",
    text: "Start this Run",
    shortcuts: ["ctrl+r", "meta+enter"],
  },
  "runtime.run.pause": {
    icon: "fas fa-pause",
    title: "Pause",
    text: "Pause this Run",
    isEnabled: (action, context) =>
      context?.nodes?.every((n) => isNode(n, NodeType.RUN)) && context?.nodes?.some((n) => isRunActive(n as RunData)),
    action: (action, context) => {
      const tx = runtime.tx.with({ change: { key: newChangeId(), title: "Pause" } });
      context?.nodes?.filter((n) => isRunActive(n as RunData)).forEach((n) => runtime.pause(n as RunData, { tx }));
    },
  },
  "runtime.run.resume": {
    icon: "fas fa-play",
    title: "Resume",
    text: "Resume this Run",
    isEnabled: (action, context) =>
      context?.nodes?.every((n) => isNode(n, NodeType.RUN)) && context?.nodes?.some((n) => isRunPaused(n as RunData)),
    action: (action, context) => {
      const tx = runtime.tx.with({ change: { key: newChangeId(), title: "Resume" } });
      context?.nodes?.filter((n) => isRunPaused(n as RunData)).forEach((n) => runtime.resume(n as RunData, { tx }));
    },
  },
  "runtime.run.kill": {
    icon: "fas fa-stop",
    title: "Kill",
    text: "Kill this Run",
    isEnabled: (action, context) =>
      context?.nodes?.every((n) => isNode(n, NodeType.RUN)) && context?.nodes?.some((n) => isRunActive(n as RunData)),
    action: (action, context) => {
      const tx = runtime.tx.with({ change: { key: newChangeId(), title: "Kill" } });
      context?.nodes?.filter((n) => isRunActive(n as RunData)).forEach((n) => runtime.stop(n as RunData, { tx }));
    },
  },
  // interrupt
  "runtime.interruption.resume": {
    icon: "fas fa-check",
    title: "Resume",
    text: "Resume this Interruption",
    isEnabled: (action, context) =>
      context?.nodes?.every((n) => isNode(n, NodeType.INTERRUPTION)) &&
      context?.nodes?.some((n) => n.status == InterruptionStatus.OPEN),
    action: (action, context) => {
      const tx = runtime.tx.with({ change: { key: newChangeId(), title: "Resume" } });
      context?.nodes
        ?.filter((n) => (n as InterruptionData).status == InterruptionStatus.OPEN)
        .forEach((n) => runtime.resume(n as RunData, { tx }));
    },
  },
  "runtime.interruption.cancel": {
    icon: "fas fa-xmark",
    title: "Cancel",
    text: "Cancel this Interruption",
    action: (action, context) => {
      const tx = runtime.tx.with({ change: { key: newChangeId(), title: "Cancel" } });
      context?.nodes
        ?.filter((n) => (n as InterruptionData).status == InterruptionStatus.OPEN)
        .forEach((n) => runtime.cancel(n as InterruptionData, { tx }));
    },
  },
});
