import { BASED_NODE_TYPES, isBenchNodeType, isUnloadedNodeType } from "@/language/const";
import { NodeSuperGraph } from "@/language/graph";
import { NodeReferenceData, NodeType } from "@/proto/wire";
import { describeNode, makeScope, toNodeRef } from "@/proto/wiring";
import { BENCH_SCOPE } from "@/system/client";
import { acquireConnection, GetConnectionParams, releaseConnection, RemoteGetConnection } from "@/system/connection";
import { groupByList, groupByScalar } from "@/utils/functools";
import { log } from "@/utils/log";
import { Ref, shallowRef, triggerRef } from "vue";

type AutoloadedBatch = {
  id: number;
  nodesById: Record<string, NodeReferenceData>;
  connection: RemoteGetConnection<any>;
};

/** Monitor the supergraph and automatically load (and unload) any remote nodes that we're missing. */
export class NodeAutoloader {
  /** All subscriptions (in the supergraph; we don't use the callbacks, we just count them as references) */
  private nodeSubsById: Record<string, Array<any>> = {};
  /** All currently 'missing' nodes (reference without a node, again, in the supergraph) */
  private missingNodeById: Record<string, NodeReferenceData> = {};
  /** Nodes we're waiting/trying to load */
  private pendingNodesById: Ref<Record<string, NodeReferenceData>> = shallowRef({});
  /** Nodes we'll load next */
  private nextNodesToLoad: Array<NodeReferenceData> = [];
  /** All loaded batches */
  private loadedBatches: Array<AutoloadedBatch> = [];
  /** Nodes we already loaded/checked (even if they are still missing!) */
  private loadedNodesById: Record<string, NodeReferenceData> = {};
  /** Nodes we failed to load */
  private failedNodesById: Record<string, NodeReferenceData> = {};
  /** 'Generation' of the current load batch */
  private batchId: number = 0;

  constructor(private supergraph: NodeSuperGraph) {
    this.supergraph = supergraph;
  }

  /** Handle a subscription event from the supergraph */
  onEvent(event: "miss" | "hit" | "sub" | "unsub", key: NodeReferenceData, callback: any) {
    if (event == "sub") {
      // add to subscriptions
      if (this.nodeSubsById[key.id!] == null) {
        this.nodeSubsById[key.id!] = [];
      }
      this.nodeSubsById[key.id!].push(callback);
    } else if (event == "unsub") {
      // remove from subscriptions
      const subs = this.nodeSubsById[key.id!];
      if (subs != null) {
        const index = subs.indexOf(callback);
        if (index >= 0) {
          subs.splice(index, 1);
          if (subs.length == 0) {
            delete this.nodeSubsById[key.id!];
          }
        }
      }
    } else if (event == "hit") {
      // remove from missing
      this.removeMissing(key);
    } else if (event == "miss") {
      // add to missing
      this.addMissing(key);
    }
  }

  /** Register 'missing' nodes and (if eligible) add to pending */
  addMissing(...keys: NodeReferenceData[]) {
    for (const key of keys) {
      if (this.missingNodeById[key.id!] == null) {
        this.missingNodeById[key.id!] = key;
        if (isUnloadedNodeType(key.nodeType) && !this.failedNodesById[key.id!] && !this.loadedNodesById[key.id!]) {
          if (BASED_NODE_TYPES.includes(key.nodeType) && key.baseCk == null) {
            throw new Error(`missing base in ${describeNode(key)}`);
          }
          this.pendingNodesById.value[key.id!] = key;
          this.nextNodesToLoad.push(key);
        }
      }
    }
    triggerRef(this.pendingNodesById);
  }

  /** Remove 'missing' nodes from the list of pending nodes to load */
  removeMissing(...keys: NodeReferenceData[]) {
    for (const key of keys) {
      if (this.missingNodeById[key.id!] != null) {
        delete this.missingNodeById[key.id!];
        const index = this.nextNodesToLoad.findIndex((p) => p.id == key.id);
        if (index >= 0) {
          this.nextNodesToLoad.splice(index, 1);
        }
      }
      delete this.pendingNodesById.value[key.id!];
    }
    triggerRef(this.pendingNodesById);
  }

  /** On 'successful' loading of nodes (doesn't mean they're no longer missing!) */
  private onLoaded(...keys: NodeReferenceData[]) {
    for (const key of keys) {
      this.loadedNodesById[key.id!] = key;
      delete this.pendingNodesById.value[key.id!];
    }
    triggerRef(this.pendingNodesById);
  }

  /** On failed loading of nodes */
  private onFailed(...keys: NodeReferenceData[]) {
    for (const key of keys) {
      this.failedNodesById[key.id!] = key;
      delete this.pendingNodesById.value[key.id!];
    }
    triggerRef(this.pendingNodesById);
  }

  /** Whether the node is currently pending loading */
  isPending(key: NodeReferenceData): boolean {
    return this.pendingNodesById.value[key.id!] != null;
  }

  /** Purge all loaded batches without any subscribers */
  gc() {
    const inactiveBatches = this.loadedBatches.filter((batch) =>
      Object.keys(batch.nodesById).every((id) => this.nodeSubsById[id] == null),
    );
    if (inactiveBatches.length > 0) {
      log.trace("autoload.gcBatches", { inactiveBatches });
      for (const batch of inactiveBatches) {
        releaseConnection(batch.connection);
      }
      this.loadedBatches = this.loadedBatches.filter((b) => !inactiveBatches.includes(b));
    }
  }

  /** Load any missing nodes with new connections (as feasible) */
  async loadAll() {
    if (this.nextNodesToLoad.length > 0) {
      const nodesByBase = groupByList(this.nextNodesToLoad, (ptr) => ptr.nodeType + "." + ptr.baseCk);
      this.nextNodesToLoad = [];
      await Promise.all(Object.values(nodesByBase).map(this.load.bind(this)));
    }
  }

  /** Load a batch of missing nodes with new connections */
  private async load(missingNodes: NodeReferenceData[]) {
    const batchId = this.batchId++;
    log.trace("autoload.load", { batchId, missingNodes });

    // build query
    const nodeType = missingNodes[0].nodeType;
    const baseCk = missingNodes[0].baseCk;
    const block = baseCk != null ? this.supergraph.get({ nodeType: NodeType.BLOCK, ck: baseCk }) : null;
    const blockPtr = block != null ? toNodeRef(block) : block;
    if (baseCk != null && blockPtr == null) {
      this.onFailed(...missingNodes);
      // nocheckin: retry once base is found
      log.trace("autoload.load.fail", { batchId, missingNodes });
      return; // can't load this right now (but retry later?)
    }

    // load
    const scope = isBenchNodeType(nodeType) ? BENCH_SCOPE.value : makeScope({});
    const params: GetConnectionParams<any> = {
      roots: missingNodes as NodeReferenceData[],
      scope,
      blockPtr: blockPtr as NodeReferenceData | undefined,
      isOptional: true,
    };
    try {
      const connection = (await acquireConnection(
        "get",
        { name: `autoload.${batchId}`, live: true },
        params,
      )) as RemoteGetConnection<any>;
      const batch: AutoloadedBatch = {
        id: batchId,
        nodesById: groupByScalar(missingNodes, (ptr) => ptr.id!),
        connection,
      };
      this.loadedBatches.push(batch);
      log.trace("autoload.load.complete", { batchId, missingNodes });
      this.onLoaded(...missingNodes);
    } catch (e) {
      log.trace("autoload.load.fail", { e, batchId, missingNodes });
      this.onFailed(...missingNodes);
    }
  }
}
