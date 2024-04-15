import { NodeReferenceData, NodeType } from "@/proto/wire";
import { describeNode, isNode } from "@/proto/wiring";
import type { ActionContext, ActionMapImplementation } from "@/system/action";
import { moveNode, type NodeTreeItem, type ReadNodeGraph } from "@/system/graph";
import { updateOrder } from "@/system/lang";
import { pkgGraph } from "@/system/space";
import type { Transaction } from "@/system/transaction";
import type { Ref } from "vue";

export function useHierarchicalNodeMoveActions<T extends NodeType>(options: {
  graph: ReadNodeGraph;
  basePtr: Ref<NodeReferenceData | null | undefined>;
  expandedItems: Ref<NodeTreeItem<T>[]>;
  txFactory: () => Transaction;
  getItemFromContext(context: ActionContext | undefined): { item: NodeTreeItem<T> | null; idx: number };
  enabled?: Ref<boolean>;
}): ActionMapImplementation<"common.move"> {
  type ItemT = NodeTreeItem<T>;

  const { graph, basePtr, expandedItems, txFactory, getItemFromContext, enabled } = options;

  /** Finds the closest 'right' (more indented / deeper) parent */
  function findRightParent(item: ItemT, idx: number): ItemT | null {
    const prevItems = expandedItems.value.slice(0, idx).reverse();
    for (const other of prevItems) {
      if (other.depth < item.depth) return null;
      else if (other.depth == item.depth) return other;
    }
    for (const other of prevItems) {
      if (other.depth == item.depth + 1) return other;
    }
    return null;
  }

  /** Moves the item one level to the right */
  function moveNodeRight(tx: Transaction, item: ItemT, idx: number) {
    if (!isNode(item.node, NodeType.BLOCK)) throw new Error(`can't move hierarchically: ${describeNode(item.node)}`);
    const rightParent = findRightParent(item, idx);
    if (rightParent == null) return false;
    updateOrder({
      tx,
      node: item.node,
      position: "after",
      reference: graph.getChildren(rightParent.node, NodeType.BLOCK).slice(-1)[0],
      getNodes: () => graph.getChildren(rightParent.node, NodeType.BLOCK),
    });
    tx.move(item.node, rightParent.nodePtr);
  }

  /** Moves the item one level to the left */
  function moveNodeLeft(tx: Transaction, item: ItemT, idx: number) {
    if (!isNode(item.node, NodeType.BLOCK)) throw new Error(`can't move hierarchically: ${describeNode(item.node)}`);
    const parent = pkgGraph.getMaybe(item?.node.parentPtr);
    if (item == null || !isNode(parent, NodeType.BLOCK) || parent.id == basePtr.value?.id) return false;
    moveNode(txFactory(), pkgGraph, item.node, parent, "end");
  }

  return {
    "common.move.up": {
      enabled,
      action: (action, context) => {
        // move block one closer in indent or above the previous item in linear order
        const { item, idx } = getItemFromContext(context);
        const prev = expandedItems.value[idx - 1];
        if (item == null || prev == null) return false;
        const tx = txFactory();
        if (prev.node.parentPtr?.id == item.node.parentPtr?.id || prev.node?.id == item.node.parentPtr?.id) {
          moveNode(tx, pkgGraph, item.node, prev.node, "start");
          return true;
        } else if (!isNode(item.node, NodeType.BLOCK)) {
          return false;
        } else if (prev.depth > item.depth) {
          return moveNodeRight(tx, item, idx);
        } else {
          return moveNodeLeft(tx, item, idx);
        }
      },
    },
    "common.move.down": {
      enabled,
      action: (action, context) => {
        // move block one further in indent or below the next item in linear order
        const { item, idx } = getItemFromContext(context);
        const next = expandedItems.value[idx + 1];
        if (item == null || next == null) return false;
        const tx = txFactory();
        if (next.node.parentPtr?.id == item.node.parentPtr?.id) {
          moveNode(tx, pkgGraph, item.node, next.node, "end");
          return true;
        } else if (!isNode(item.node, NodeType.BLOCK)) {
          return false;
        } else if (next.depth > item.depth) {
          return moveNodeRight(tx, item, idx);
        } else {
          return moveNodeLeft(tx, item, idx);
        }
      },
    },
    "common.move.left": {
      enabled,
      action: (action, context) => {
        // move block one 'higher' in indent (to after parent in its siblings)
        const { item, idx } = getItemFromContext(context);
        if (item == null || !isNode(item.node, NodeType.BLOCK)) return false;
        return moveNodeLeft(txFactory(), item, idx);
      },
    },
    "common.move.right": {
      enabled,
      action: (action, context) => {
        // move block one 'lower' in indent (to before the next sibling of the closest parent)
        const { item, idx } = getItemFromContext(context);
        if (item == null || idx < 1 || !isNode(item.node, NodeType.BLOCK)) return false;
        return moveNodeRight(txFactory(), item, idx);
      },
    },
  };
}
