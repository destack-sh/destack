import { isBenchNodeType, isUnloadedNodeType } from "@/language/core/const";
import { NodeSuperGraph } from "@/language/core/graph";
import { NodeReferenceData, NodeType } from "@/proto/wire";
import { describeNode, makeScope, toNodeRef } from "@/proto/wiring";
import { BENCH_SCOPE } from "@/system/client";
import { acquireConnection, GetConnectionParams, releaseConnection, RemoteGetConnection } from "@/system/connection";
import { groupByList, groupByScalar } from "@/utils/functools";
import { log } from "@/utils/log";
import { Ref, shallowRef, triggerRef } from "vue";

// automatically included descendants :AutoLoading
export const AUTOLOAD_DESCENDANT_TYPES: Partial<Record<NodeType, NodeType[]>> = {
  [NodeType.THREAD]: [NodeType.FILE, NodeType.MEMBERSHIP, NodeType.CLAIM, NodeType.IDENTITY],
};

type AutoloadedBatch = {
  id: number;
  nodesById: Record<string, NodeReferenceData>;
  connection: RemoteGetConnection<any>;
};

/** Monitor the supergraph and automatically load (and unload) any remote nodes that we're missing. */
export class NodeAutoloader {
  /** All subscriptions (in the supergraph; we don't use the callbacks, we just count them as references) */
  private nodeSubsById: Record<string, Array<any>> = {};
  /** All currently 'missing' nodes (reference without a node, again, in the supergraph) with reference count */
  private missingNodeById: Record<string, { node: NodeReferenceData; count: number }> = {};
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
    const newPending: NodeReferenceData[] = [];
    for (const key of keys) {
      if (this.missingNodeById[key.id!] == null) {
        this.missingNodeById[key.id!] = { node: key, count: 1 };
        if (isUnloadedNodeType(key.nodeType) && !this.failedNodesById[key.id!] && !this.loadedNodesById[key.id!]) {
          newPending.push(key);
        }
      } else {
        this.missingNodeById[key.id!].count++;
      }
    }
    this.addPending(...newPending);
  }

  /** Add pending nodes to the list of nodes to load */
  addPending(...keys: NodeReferenceData[]) {
    for (const key of keys) {
      if ([NodeType.RECORD].includes(key.nodeType) && key.baseId == null) {
        throw new Error(`missing base in ${describeNode(key)}`);
      }
      this.pendingNodesById.value[key.id!] = key;
      this.nextNodesToLoad.push(key);
    }
    triggerRef(this.pendingNodesById);
  }

  /** Remove 'missing' nodes from the list of pending nodes to load */
  removeMissing(...keys: NodeReferenceData[]) {
    for (const key of keys) {
      const missing = this.missingNodeById[key.id!];
      if (missing != null) {
        missing.count--;
        if (missing.count <= 0) {
          delete this.missingNodeById[key.id!];
          const index = this.nextNodesToLoad.findIndex((p) => p.id == key.id);
          if (index >= 0) {
            this.nextNodesToLoad.splice(index, 1);
          }
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

  /** Load any missing nodes with new connections (as feasible) */
  async loadAll() {
    if (this.nextNodesToLoad.length > 0) {
      const nodesByBase = groupByList(this.nextNodesToLoad, (ptr) => ptr.nodeType + "." + ptr.baseId);
      this.nextNodesToLoad = [];
      await Promise.all(Object.values(nodesByBase).map(this.load.bind(this)));
    }
  }

  /** Load a batch of missing nodes with new connections */
  private async load(nodeRefs: NodeReferenceData[]) {
    const batchId = this.batchId++;
    log.trace("autoload.load", { batchId, nodeRefs });

    // build query
    const nodeType = nodeRefs[0].nodeType;
    const baseId = nodeRefs[0].baseId;
    const block = baseId != null ? this.supergraph.get({ nodeType: NodeType.BLOCK, id: baseId }) : null;
    const blockPtr = block != null ? toNodeRef(block) : block;

    // retry later if base is missing
    if (baseId != null && blockPtr == null) {
      this.onFailed(...nodeRefs);
      log.trace("autoload.load.fail", { batchId, nodeRefs });
      const basePtr = { nodeType: NodeType.BLOCK, id: baseId };
      // NOTE :Cleanup: we're leaking this wait-to-auto-reload subscription, but it shouldn't matter for now
      //  (it _should_ stop when none of the missing nodes are subscribed to anymore)
      this.supergraph.subscribeUntilFound(basePtr, () => {
        // remove from failed
        nodeRefs.forEach((ptr) => delete this.failedNodesById[ptr.id!]);
        // add to pending again (if still missing)
        nodeRefs = nodeRefs.filter((ptr) => this.missingNodeById[ptr.id!] != null);
        if (nodeRefs.length > 0) {
          this.addPending(...nodeRefs);
        }
        log.trace("autoload.load.retry", { batchId, basePtr, nodeRefs });
      });

      return; // can't load this right now
    }

    // load
    const scope = isBenchNodeType(nodeType) ? BENCH_SCOPE.value : makeScope({});
    const params: GetConnectionParams<any> = {
      roots: nodeRefs as NodeReferenceData[],
      scope,
      baseTypePtr: blockPtr as NodeReferenceData | undefined,
      descendantTypes: AUTOLOAD_DESCENDANT_TYPES[nodeType],
      // NOTE :Architecture: bypass memory cache for autoloaded nodes
      //  (since if we're autoloading, they key shouldn't be from the loaded Bench/Packages)
      noMemory: true,
      isOptional: true,
    };
    try {
      const connection = await acquireConnection("get", { name: `autoload.${batchId}`, live: true }, params);
      const batch: AutoloadedBatch = {
        id: batchId,
        nodesById: groupByScalar(nodeRefs, (ptr) => ptr.id!),
        connection: connection as RemoteGetConnection<any>,
      };
      this.loadedBatches.push(batch);
      const nodes = nodeRefs.map((ptr) => this.supergraph.get(ptr));
      log.trace("autoload.load.complete", { batchId, nodeRefs, nodes, connection });
      this.onLoaded(...nodeRefs);
    } catch (e) {
      log.trace("autoload.load.fail", { e, batchId, nodeRefs });
      this.onFailed(...nodeRefs);
    }
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
}
