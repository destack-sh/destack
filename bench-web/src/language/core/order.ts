import type { Transaction } from "@/language/runtime/transaction";
import type { AnyNodeData } from "@/proto/wire";
import { generateOrderKey, generateOrderKeys, isValidOrderKey } from "@/utils/fractional";

/** Sorts the nodes using createdAt, then id. */
export function timesortNode<T extends AnyNodeData>(nodes: T[]): void {
  nodes.sort((a, b) => {
    if (a.createdAt != null && b.createdAt != null) {
      if (a.createdAt.seconds != b.createdAt.seconds) {
        return Number(a.createdAt.seconds - b.createdAt.seconds);
      } else if (a.createdAt.nanos != b.createdAt.nanos) {
        return Number(a.createdAt.nanos - b.createdAt.nanos);
      }
    }
    return a.id > b.id ? 1 : -1;
  });
}

/** Sorts the given nodes using explicit order keys if available, createdAt otherwise, then id. */
export function defaultSortNode<T extends AnyNodeData>(nodes: T[]): void {
  nodes.sort((a, b) => {
    if ((a as any).orderKey != null && (b as any).orderKey != null && (a as any).orderKey != (b as any).orderKey) {
      return (a as any).orderKey > (b as any).orderKey ? 1 : -1;
    } else if (a.createdAt != null && b.createdAt != null) {
      if (a.createdAt.seconds != b.createdAt.seconds) {
        return Number(a.createdAt.seconds - b.createdAt.seconds);
      } else if (a.createdAt.nanos != b.createdAt.nanos) {
        return Number(a.createdAt.nanos - b.createdAt.nanos);
      }
    }
    return a.id > b.id ? 1 : -1;
  });
}

/**
 * Sets the node order keys so that the target is position relative to the reference. Nodes must be in order.
 * Also applies any 'fixes' due to duplicate order keys in the same transaction.
 */
export function updateOrder<T extends AnyNodeData & { orderKey?: string | undefined }>(order: {
  tx: Transaction;
  node: T;
  position: "before" | "after";
  reference: string | T | null;
  getNodes: () => T[];
}) {
  let orderKey;
  const getReference = (nodes: T[]) => {
    if (order.reference == null) return order.position == "before" ? null : nodes[nodes.length - 1];
    else if (typeof order.reference == "string") return nodes.find((n) => n.id == order.reference) ?? null;
    else return order.reference;
  };
  try {
    const nodes = order.getNodes();
    orderKey = getOrderKey<T>({ nodes, position: order.position, reference: getReference(nodes) });
  } catch (e) {
    // 'fix' order keys if we couldn't generate one
    //  (usually because of duplicates, we don't enforce uniqueness per order key in backend for simplicity)
    let nodes = order.getNodes();
    fixOrderKeys(order.tx, nodes);
    nodes = order.getNodes(); // 'refresh' to apply tx changes
    orderKey = getOrderKey<T>({
      nodes,
      position: order.position,
      reference: getReference(nodes),
    });
  }
  // @ts-expect-error: orderKey must exist
  order.tx.update(order.node, { orderKey }, { debounce: "tick" });
}

/**
 * Gets the order key relative to the reference. Nodes must be in order.
 */
export function getOrderKey<T extends { id: string; orderKey?: string | undefined }>(order: {
  position: "before" | "above" | "after" | "below";
  reference: T | null;
  nodes: T[];
}) {
  let orderKey;
  if (order.position == "before" || order.position == "above") {
    const a =
      order.reference?.id == null ? null : order.nodes[order.nodes.findIndex((n) => n.id == order.reference!.id) - 1];
    orderKey = generateOrderKey(a?.orderKey ?? null, order.reference?.orderKey ?? null);
  } else {
    const b =
      order.reference?.id == null ? null : order.nodes[order.nodes.findIndex((n) => n.id == order.reference!.id) + 1];
    orderKey = generateOrderKey(order.reference?.orderKey ?? null, b?.orderKey ?? null);
  }
  return orderKey;
}

/**
 * Patches any 'broken' order keys to put the nodes in the given order.
 */
export function fixOrderKeys<T extends AnyNodeData & { orderKey?: string | undefined }>(tx: Transaction, nodes: T[]) {
  // ensure nodes are in current order
  defaultSortNode(nodes);

  // scan for successive duplicates (they must be successive now)
  let i = 0;
  while (i < nodes.length) {
    const prevOrderKey = i == 0 ? null : nodes[i - 1].orderKey;
    const node = nodes[i];
    if (node.orderKey == null || !isValidOrderKey(node.orderKey)) {
      // just patch in place
      const orderKey = generateOrderKey(prevOrderKey ?? null, nodes[i + 1]?.orderKey ?? null);
      // @ts-expect-error: orderKey must exist
      tx.update(node, { orderKey }, { debounce: "tick" });
    } else if (node.orderKey == prevOrderKey) {
      // find all duplicates with same key from here and fix them in one go
      const numDuplicates = nodes.slice(i).filter((n) => n.orderKey == node.orderKey).length;
      const duplicates = nodes.slice(i, i + numDuplicates);
      const orderKeys = generateOrderKeys(prevOrderKey, nodes[i + numDuplicates]?.orderKey ?? null, numDuplicates);
      for (let j = 0; j < numDuplicates; j++) {
        // @ts-expect-error: orderKey must exist
        tx.update(duplicates[j], { orderKey: orderKeys[j] }, { debounce: "tick" });
      }
      i += numDuplicates;
    } else {
      i++;
    }
  }
}
