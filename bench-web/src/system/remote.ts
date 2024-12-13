import { isInBenchNodeType, isLocalNodeType, isSourceNodeType, isVirtualResourceNodeType } from "@/language/const";
import { NodeSuperGraph, TypedNodeKey } from "@/language/graph";
import { NodeReferenceData, NodeType } from "@/proto/wire";
import { makeScope, toNodeRef } from "@/proto/wiring";
import { BENCH_SCOPE } from "@/system/client";
import { acquireConnection } from "@/system/connection";
import { groupByList } from "@/utils/functools";

/** Monitor the supergraph and automatically load (and unload) any remote nodes that we're missing. */
export class RemoteNodeLoader {
  private nodeSubsById: Record<string, Array<any>> = {}; // we don't do anything with these, we just track them
  private missingNodeById: Record<string, TypedNodeKey<any>> = {};
  private missingLoadableNodes: Array<TypedNodeKey<any>> = [];

  constructor(private supergraph: NodeSuperGraph) {
    this.supergraph = supergraph;
  }

  /** Handle a subscription event from the supergraph */
  onEvent(event: "miss" | "hit" | "sub" | "unsub", key: TypedNodeKey<any>, callback: any) {
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

  /** Add a missing node to the list of nodes to load */
  addMissing(key: TypedNodeKey<any>) {
    if (this.missingNodeById[key.id!] != null) return;
    this.missingNodeById[key.id!] = key;
    if (!isSourceNodeType(key.nodeType) && !isVirtualResourceNodeType(key.nodeType)) {
      this.missingLoadableNodes.push(key);
    }
  }

  /** Remove a missing node from the list of nodes to load */
  removeMissing(key: TypedNodeKey<any>) {
    delete this.missingNodeById[key.id!];
  }

  /** Load any missing nodes with new connections (as feasible) */
  async loadMissing() {
    if (this.missingLoadableNodes.length == 0) return;

    const missingNodesByBase = groupByList(this.missingLoadableNodes, (key) => key.nodeType + "." + key.baseCk);
    for (const missingNodes of Object.values(missingNodesByBase)) {
      const nodeType = missingNodes[0].nodeType;
      const baseCk = missingNodes[0].baseCk;
      const block = baseCk != null ? this.supergraph.get({ nodeType: NodeType.BLOCK, ck: baseCk }) : null;
      const blockPtr = block != null ? toNodeRef(block) : block;
      if (baseCk != null && blockPtr == null) {
        continue; // can't load this right now
      }

      console.log("missing", NodeType[nodeType], missingNodes);
      const scope = isInBenchNodeType(nodeType) ? BENCH_SCOPE.value : makeScope({});

      const connection = await acquireConnection(
        "get",
        { name: "remote", live: true },
        { roots: missingNodes as NodeReferenceData[], scope },
      );

      // nocheckin: remove from missing / add to pending while loading so we don't load it twice?
    }
  }
}
