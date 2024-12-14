import { isInBenchNodeType, isSourceNodeType, isUnloadedNodeType, isVirtualResourceNodeType } from "@/language/const";
import { NodeSuperGraph } from "@/language/graph";
import { NodeReferenceData, NodeType } from "@/proto/wire";
import { makeScope, toNodeRef } from "@/proto/wiring";
import { BENCH_SCOPE } from "@/system/client";
import { acquireConnection, GetConnectionParams } from "@/system/connection";
import { groupByList } from "@/utils/functools";
import { Ref, shallowRef, triggerRef } from "vue";

/** Monitor the supergraph and automatically load (and unload) any remote nodes that we're missing. */
export class NodeAutoloader {
  /** All subscriptions (in the supergraph; we don't use the callbacks, we just count them as references) */
  private nodeSubsById: Record<string, Array<any>> = {};
  /** All currently 'missing' nodes (reference without a node, again, in the supergraph) */
  private missingNodeById: Record<string, NodeReferenceData> = {};
  /** Nodes we're waiting/trying to load */
  private pendingNodesById: Ref<Record<string, NodeReferenceData>> = shallowRef({});
  /** Nodes we'll load next */
  private nodesToLoad: Array<NodeReferenceData> = [];
  /** Nodes we already loaded (even if they are still missing!) */
  private loadedNodesById: Record<string, NodeReferenceData> = {};
  /** Nodes we failed to load */
  private failedNodesById: Record<string, NodeReferenceData> = {};
  /** 'Generation' of the current load batch */
  private loadGeneration: number = 0;

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
        if (index >= 0) subs.splice(index, 1);
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
        if (isUnloadedNodeType(key.nodeType) && !this.failedNodesById[key.id!]) {
          this.pendingNodesById.value[key.id!] = key;
          this.nodesToLoad.push(key);
          triggerRef(this.pendingNodesById);
        }
      }
    }
  }

  /** Remove 'missing' nodes from the list of nodes to load */
  removeMissing(...keys: NodeReferenceData[]) {
    for (const key of keys) {
      if (this.missingNodeById[key.id!] != null) {
        delete this.missingNodeById[key.id!];
        const index = this.nodesToLoad.findIndex((p) => p.id == key.id);
        if (index >= 0) this.nodesToLoad.splice(index, 1);
      }
    }
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
    if (this.nodesToLoad.length > 0) {
      const nodesByBase = groupByList(this.nodesToLoad, (ptr) => ptr.nodeType + "." + ptr.baseCk);
      await Promise.all(Object.values(nodesByBase).map(this.loadBatch.bind(this)));
      this.nodesToLoad = [];
    }
  }

  /** Load a batch of missing nodes with new connections */
  private async loadBatch(missingNodes: NodeReferenceData[]) {
    const generation = this.loadGeneration++;

    // build query
    const nodeType = missingNodes[0].nodeType;
    const baseCk = missingNodes[0].baseCk;
    const block = baseCk != null ? this.supergraph.get({ nodeType: NodeType.BLOCK, ck: baseCk }) : null;
    const blockPtr = block != null ? toNodeRef(block) : block;
    if (baseCk != null && blockPtr == null) {
      this.onFailed(...missingNodes);
      // nocheckin: retry once base is found
      return; // can't load this right now (but retry later?)
    }

    // load
    const scope = isInBenchNodeType(nodeType) ? BENCH_SCOPE.value : makeScope({});
    const params: GetConnectionParams<any> = {
      roots: missingNodes as NodeReferenceData[],
      scope,
      blockPtr: blockPtr as NodeReferenceData | undefined,
      isOptional: true,
    };
    console.log("missing." + this.loadGeneration, NodeType[nodeType], missingNodes, scope, params);
    try {
      const connection = await acquireConnection("get", { name: `remote.${generation}`, live: true }, params);
      // nocheckin: release connection somehow?
      this.onLoaded(...missingNodes);
    } catch (e) {
      this.onFailed(...missingNodes);
    }
  }
}
