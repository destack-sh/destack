import { ACTIVE_PROCESS_STATUSES, getBaseFromNode, isRunnableNode } from "@/language/core/const";
import { makeExpression } from "@/language/core/expression";
import type { ReadNodeGraph } from "@/language/core/graph";
import { makeNode } from "@/language/core/node";
import { timesortNode } from "@/language/core/order";
import { isProcessActive, isProcessPaused } from "@/language/runtime/process";
import { newChangeId, type Transaction } from "@/language/core/transaction";
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
  RunOptionsData,
  RunProperty,
  SpanData,
  ProcessStatus,
  Timestamp,
  type RunData,
  RunnableNodeData,
  AgentData,
  TransitionData,
} from "@/proto/wire";
import {
  describeNode,
  isNode,
  makeDefaultObject,
  propertyReference,
  toNodeRef,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { supergraph, useGetConnection, useSearchConnection, type Connection } from "@/system/connection";
import { benchConnection, benchGraph, space, spaceConnection } from "@/system/space";
import { declareCommands } from "@/ui/command";
import { makeIcon } from "@/ui/icon";
import { log } from "@/utils/log";
import { computedValue } from "@/utils/ref";
import { assertNever } from "@protobuf-ts/runtime";
import { computed, type Ref } from "vue";
import { getRunType } from "@/language/runtime/run";

/** A reactive Run with all its descendants */
let treeId = 0;
export class RunTree {
  id: number = treeId++;
  runGraph: ReadNodeGraph;
  runPtr: Ref<NodeReferenceData | null | undefined>;
  runRef: Ref<RunData | null>;
  runsRef: Ref<(RunData | SpanData)[]>;
  runBasePtr: Ref<NodeReferenceData | null>;
  runBaseRef: Ref<RunnableNodeData | null>;
  runConnection: Connection<"get", NodeType.RUN>;
  runsByBaseId: Ref<Record<string, RunData[]>>;
  basesPtrs: Ref<NodeReferenceData[]>;
  basesRef: Ref<RunnableNodeData[]>;
  basesByIdRef: Ref<Record<string, RunnableNodeData>>;
  interruptionsRef: Ref<InterruptionData[]>;

  constructor(graph: ReadNodeGraph, runPtr: Ref<NodeReferenceData | null>) {
    this.runPtr = runPtr;
    const { graph: runGraph, connection: runConnection } = useGetConnection(
      { name: "runtime.run." + this.id, live: true },
      computed(() => ({
        scope: graph.scope,
        roots: [this.runPtr.value!],
        ancestorTypes: [NodeType.THREAD, NodeType.RUN],
        descendantTypes: [NodeType.THREAD, NodeType.RUN, NodeType.SPAN, NodeType.INTERRUPTION],
        isOptional: true,
        isEnabled: this.runPtr.value != null,
      })),
    );
    this.runGraph = runGraph;
    this.runRef = runGraph.getRef(this.runPtr, {
      id: "runtime.run." + this.id,
      ignoreAncestors: false,
    }) as Ref<RunData | null>; // :NodeRefStability
    this.runsRef = runGraph.getDescendantsRef(this.runPtr, {
      metatypes: [NodeType.RUN, NodeType.SPAN],
      includeSelf: true,
    });
    this.runBasePtr = computedValue(
      () => this.runRef.value?.transitionPtr ?? this.runRef.value?.actionPtr ?? this.runRef.value?.flowPtr ?? null,
    );
    this.runBaseRef = graph.getRef(this.runBasePtr) as Ref<RunnableNodeData | null>;
    this.runConnection = runConnection as Connection<"get", NodeType.RUN>;
    this.runsByBaseId = computed(() => {
      const runByBaseId: Record<string, RunData[]> = {};
      for (const run of this.runsRef.value) {
        if (!isNode(run, NodeType.RUN)) continue;
        const base = getBaseFromNode(run);
        if (base?.id != null) {
          if (runByBaseId[base.id] == null) {
            runByBaseId[base.id] = [];
          }
          runByBaseId[base.id].push(run);
        }
      }
      // sort ascending
      for (const runBaseId of Object.keys(runByBaseId)) {
        timesortNode(runByBaseId[runBaseId]);
      }
      return runByBaseId;
    });
    this.basesPtrs = computed(() => {
      const basePtrs: NodeReferenceData[] = [];
      for (const run of this.runsRef.value) {
        const base = getBaseFromNode(run);
        if (base != null) basePtrs.push(base);
      }
      return basePtrs;
    });
    this.basesRef = graph.getManyRef(this.basesPtrs) as Ref<RunnableNodeData[]>;
    this.basesByIdRef = computed(() => {
      const basesById: Record<string, RunnableNodeData> = {};
      for (const base of this.basesRef.value) {
        basesById[base.id] = base;
      }
      return basesById;
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

  hasBase(base: { id?: string }) {
    return this.runsByBaseId.value[base.id!] != null;
  }

  getBase(base: { id?: string }) {
    return this.basesByIdRef.value[base.id!];
  }

  /** Gets the last (active) Runs for the given base node. */
  getLastActiveRuns(base: { id?: string }): RunData[] {
    return this.runsByBaseId.value[base.id!] ?? [];
  }

  /** Gets the last (active) Run for the given base node. */
  getLastActiveRun(base: { id?: string }): RunData | null {
    const runs = this.runsByBaseId.value[base.id!];
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
              value: ACTIVE_PROCESS_STATUSES,
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
    runnable: RunnableNodeData,
    options?: {
      focus?: boolean;
      options?: RunOptionsData;
      resourcesPacked?: Record<string, any>;
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
    if (!isRunnableNode(node)) throw new Error(`no node for base ${describeNode(basePtr)} of ${describeNode(run)}`);
    this.start(node, {
      focus: true,
      benchPtr: run.benchPtr,
      inputsPacked: run.inputsPacked as Record<string, any> | undefined,
      options: run.options,
      tx: options?.tx,
    });
  }

  /** Pause a Run. */
  pause(run: RunData, options?: { tx?: Transaction }) {
    if (!isProcessActive(run)) return;
    log.trace("runtime.pause", run);
    const tx = options?.tx ?? this.tx;
    tx.update(run, { requestedPauseAt: Timestamp.now() });
  }

  /** Resume a Run. */
  resume(run: RunData, options?: { tx?: Transaction }) {
    if (run.status != ProcessStatus.PAUSED) return;
    log.trace("runtime.resume", run);
    const tx = options?.tx ?? this.tx;
    tx.update(run, { requestedResumeAt: Timestamp.now() });
  }

  /** Stop a Run. */
  stop(run: RunData, options?: { tx?: Transaction }) {
    if (!isProcessActive(run)) return;
    log.trace("runtime.stop", run);
    const tx = options?.tx ?? this.tx;
    tx.update(run, { requestedStopAt: Timestamp.now() });
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
export const runtime = new Runtime(benchGraph, () => benchConnection.tx, runPtr);

export function makeRunOptions(options?: Partial<RunOptionsData>): RunOptionsData {
  return makeDefaultObject({ metatype: ObjectType.RUN_OPTIONS, ...options }) as RunOptionsData;
}

/** Make a new Run for some runnable node */
export function makeRun(
  graph: ReadNodeGraph,
  runnable: RunnableNodeData,
  options?: {
    resourcesPacked?: Record<string, any>;
    inputsPacked?: Record<string, any>;
    packagePtr?: NodeReferenceData;
    options?: RunOptionsData;
    mode?: NodeMode;
  },
): RunData {
  let packagePtr: NodeReferenceData | undefined = undefined;
  let agent: AgentData | undefined = undefined;
  let flow: FlowData | undefined = undefined;
  let command: ActionData | undefined = undefined;
  let transition: TransitionData | undefined = undefined;
  if (isNode(runnable, NodeType.AGENT)) {
    agent = runnable;
    packagePtr = options?.packagePtr ?? runnable.packagePtr;
  } else if (isNode(runnable, NodeType.FLOW)) {
    flow = runnable;
    packagePtr = options?.packagePtr ?? runnable.packagePtr;
  } else if (isNode(runnable, NodeType.ACTION)) {
    command = runnable;
    flow = graph.getAncestors(command, { includeSelf: true }).find((node) => isNode(node, NodeType.FLOW));
    packagePtr = options?.packagePtr ?? command.packagePtr;
  } else if (isNode(runnable, NodeType.TRANSITION)) {
    transition = runnable;
    flow = graph.getAncestors(transition, { includeSelf: true }).find((node) => isNode(node, NodeType.FLOW));
    packagePtr = options?.packagePtr ?? transition.packagePtr;
  } else {
    assertNever(runnable);
  }
  const page = graph.getAncestors(runnable, { includeSelf: true }).find((node) => isNode(node, NodeType.PAGE));
  if (page == null) {
    throw new Error(`no containing page for ${describeNode(runnable)}`);
  }
  const run = makeNode({
    metatype: NodeType.RUN,
    parentPtr: packagePtr,
    packagePtr,
    type: getRunType(runnable),
    status: ProcessStatus.SCHEDULED,
    mode: options?.mode ?? space.value?.mode ?? NodeMode.MAIN,
    flowPtr: flow != null ? toNodeRef(flow) : undefined,
    actionPtr: isNode(runnable, NodeType.ACTION) ? toNodeRef(runnable) : undefined,
    transitionPtr: isNode(runnable, NodeType.TRANSITION) ? toNodeRef(runnable) : undefined,
    inputsPacked: options?.inputsPacked ?? undefined,
    options: makeRunOptions(options?.options),
  });
  return run;
}

type RuntimeCommand = { title: string; isPrimary?: boolean; icon: IconData; command: () => void };

/** Gets the available commands for a Run */
export function getRunCommands(run: RunData): RuntimeCommand[] {
  const commands: RuntimeCommand[] = [];
  if (isProcessActive(run)) {
    if (isProcessPaused(run)) {
      commands.push({
        title: "Resume",
        icon: makeIcon("fas fa-play"),
        command: () => {
          runtime.resume(run);
        },
      });
    } else {
      commands.push({
        title: "Pause",
        icon: makeIcon("fas fa-pause"),
        command: () => {
          runtime.pause(run);
        },
      });
    }
    commands.push({
      title: "Stop",
      icon: makeIcon("fas fa-stop"),
      command: () => {
        runtime.stop(run);
      },
    });
  }
  return commands;
}

export const CLEAR_RUN_COMMAND: RuntimeCommand = {
  title: "Hide",
  icon: makeIcon("fas fa-xmark"),
  command: () => {
    if (space.value == null) throw new Error("no current space");
    spaceConnection.tx.update(space.value, { runPtr: undefined });
  },
};

export function getInterruptCommands(interrupt: InterruptionData): RuntimeCommand[] {
  const commands: RuntimeCommand[] = [];
  if (interrupt.status == InterruptionStatus.OPEN) {
    commands.push({
      title: "Complete",
      icon: makeIcon("fas fa-check"),
      isPrimary: true,
      command: () => {
        runtime.complete(interrupt);
      },
    });
    commands.push({
      title: "Cancel",
      icon: makeIcon("fas fa-xmark"),
      command: () => {
        runtime.cancel(interrupt);
      },
    });
  }
  return commands;
}

export function getInputType(node: RunnableNodeData) {
  if (isNode(node, NodeType.ACTION)) {
    return actionToType(node, "value", [FieldType.INPUT]);
  } else if (isNode(node, NodeType.FLOW)) {
    return flowToType(node, "value", [FieldType.INPUT]);
  } else {
    return undefined;
  }
}

export function getOutputType(node: RunnableNodeData) {
  if (isNode(node, NodeType.ACTION)) {
    return actionToType(node, "value", [FieldType.OUTPUT]);
  } else if (isNode(node, NodeType.FLOW)) {
    return flowToType(node, "value", [FieldType.OUTPUT]);
  } else {
    return undefined;
  }
}

// runtime
declareCommands<"runtime">({
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
    isEnabled: (command, context) =>
      context?.nodes?.every((n) => isNode(n, NodeType.RUN)) &&
      context?.nodes?.some((n) => isProcessActive(n as RunData)),
    command: (command, context) => {
      const tx = runtime.tx.with({ change: { key: newChangeId(), title: "Pause" } });
      context?.nodes?.filter((n) => isProcessActive(n as RunData)).forEach((n) => runtime.pause(n as RunData, { tx }));
    },
  },
  "runtime.run.resume": {
    icon: "fas fa-play",
    title: "Resume",
    text: "Resume this Run",
    isEnabled: (command, context) =>
      context?.nodes?.every((n) => isNode(n, NodeType.RUN)) &&
      context?.nodes?.some((n) => isProcessPaused(n as RunData)),
    command: (command, context) => {
      const tx = runtime.tx.with({ change: { key: newChangeId(), title: "Resume" } });
      context?.nodes?.filter((n) => isProcessPaused(n as RunData)).forEach((n) => runtime.resume(n as RunData, { tx }));
    },
  },
  "runtime.run.kill": {
    icon: "fas fa-stop",
    title: "Kill",
    text: "Kill this Run",
    isEnabled: (command, context) =>
      context?.nodes?.every((n) => isNode(n, NodeType.RUN)) &&
      context?.nodes?.some((n) => isProcessActive(n as RunData)),
    command: (command, context) => {
      const tx = runtime.tx.with({ change: { key: newChangeId(), title: "Kill" } });
      context?.nodes?.filter((n) => isProcessActive(n as RunData)).forEach((n) => runtime.stop(n as RunData, { tx }));
    },
  },
  // interrupt
  "runtime.interruption.resume": {
    icon: "fas fa-check",
    title: "Resume",
    text: "Resume this Interruption",
    isEnabled: (command, context) =>
      context?.nodes?.every((n) => isNode(n, NodeType.INTERRUPTION)) &&
      context?.nodes?.some((n) => n.status == InterruptionStatus.OPEN),
    command: (command, context) => {
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
    command: (command, context) => {
      const tx = runtime.tx.with({ change: { key: newChangeId(), title: "Cancel" } });
      context?.nodes
        ?.filter((n) => (n as InterruptionData).status == InterruptionStatus.OPEN)
        .forEach((n) => runtime.cancel(n as InterruptionData, { tx }));
    },
  },
});
