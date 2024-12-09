import { makeTypeInfo } from "@/language/field";
import { type NodeTreeItem, type ReadNodeGraph } from "@/language/graph";
import { makeNodeName, moveNode, NodeIn } from "@/language/node";
import { getOrderKey, updateOrder } from "@/language/order";
import type { Transaction } from "@/language/transaction";
import {
  BenchType,
  BlockData,
  BlockType,
  FieldType,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PackageData,
  TypeInfoData,
  TypeKind,
  ValueBlockData,
  type NodeTypeMapping,
} from "@/proto/wire";
import { describeNode, isNode, toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import type { ActionContext, ActionMapImplementation } from "@/ui/action";
import { generateOrderKey } from "@/utils/fractional";
import type { Ref } from "vue";

/** Actions to smoothly move up/down/left/right inside a node tree */
export function useHierarchicalNodeMoveActions<T extends NodeType>(options: {
  graph: ReadNodeGraph;
  basePtr: Ref<NodeReferenceData | null | undefined>;
  expandedItems: Ref<NodeTreeItem<T>[]>;
  txFactory: () => Transaction;
  getItemFromContext(context: ActionContext | undefined): { item: NodeTreeItem<T> | null; idx: number };
  enabled?: Ref<boolean>;
}): ActionMapImplementation<"space.move"> {
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
    tx.move(item.node as BlockData, { parentPtr: rightParent.nodePtr! }, { debounce: "tick" });
  }

  /** Moves the item one level to the left */
  function moveNodeLeft(tx: Transaction, item: ItemT, idx: number) {
    if (!isNode(item.node, NodeType.BLOCK)) throw new Error(`can't move hierarchically: ${describeNode(item.node)}`);
    const parent = graph.getMaybe(item?.node.parentPtr);
    if (item == null || !isNode(parent, NodeType.BLOCK) || parent.id == basePtr.value?.id) return false;
    moveNode(tx, graph, item.node, { anchor: "after", target: parent });
  }

  return {
    "space.move.up": {
      isEnabled: enabled,
      action: (action, context) => {
        // move block one closer in indent or above the previous item in linear order
        const { item, idx } = getItemFromContext(context);
        const prev = expandedItems.value[idx - 1];
        if (item == null || prev == null) return false;
        const tx = txFactory();
        if (prev.node.parentPtr?.id == item.node.parentPtr?.id || prev.node?.id == item.node.parentPtr?.id) {
          moveNode(tx, graph, item.node, { anchor: "before", target: prev.node });
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
    "space.move.down": {
      isEnabled: enabled,
      action: (action, context) => {
        // move block one further in indent or below the next item in linear order (skipping own descendants)
        const { item, idx } = getItemFromContext(context);
        let nextIdx = expandedItems.value.slice(idx + 1).findIndex((i) => i.depth <= item!.depth);
        nextIdx = nextIdx == -1 ? expandedItems.value.length : nextIdx + idx + 1;
        const next = expandedItems.value[nextIdx];
        const nextnext = expandedItems.value[nextIdx + 1];
        if (item == null || next == null) return false;
        const tx = txFactory();
        if (next.node.parentPtr?.id == item.node.parentPtr?.id) {
          if (nextnext && nextnext.depth > next!.depth) {
            moveNode(tx, graph, item.node, { anchor: "before", target: nextnext.node });
          } else {
            moveNode(tx, graph, item.node, { anchor: "after", target: next.node });
          }
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
    "space.move.left": {
      isEnabled: enabled,
      action: (action, context) => {
        // move block one 'higher' in indent (to after parent in its siblings)
        const { item, idx } = getItemFromContext(context);
        if (item == null || !isNode(item.node, NodeType.BLOCK)) return false;
        return moveNodeLeft(txFactory(), item, idx);
      },
    },
    "space.move.right": {
      isEnabled: enabled,
      action: (action, context) => {
        // move block one 'lower' in indent (to before the next sibling of the closest parent)
        const { item, idx } = getItemFromContext(context);
        if (item == null || idx < 1 || !isNode(item.node, NodeType.BLOCK)) return false;
        return moveNodeRight(txFactory(), item, idx);
      },
    },
  };
}

/** Actions to move up/down in a flat node level (assumes all the parents are the same, i.e. all nodes are siblings) */
export function useFlatNodeMoveActions<T extends NodeType>(options: {
  graph: ReadNodeGraph;
  txFactory: () => Transaction;
  getNodeFromContext(context: ActionContext | undefined): { node: NodeTypeMapping[T] | null; idx: number };
  enabled?: Ref<boolean>;
}): ActionMapImplementation<"space.move.up" | "space.move.down"> {
  const { graph, txFactory, getNodeFromContext, enabled } = options;

  return {
    "space.move.up": {
      isEnabled: enabled,
      action: (action, context) => {
        // move block one position up
        const { node } = getNodeFromContext(context);
        if (node?.parentPtr == null) return false;
        const siblings = node?.parentPtr ? graph.getChildren(node.parentPtr) : [];
        const prev = siblings[siblings.indexOf(node) - 1];
        if (prev == null) return false;
        const tx = txFactory();
        moveNode(tx, graph, node, { anchor: "before", target: prev });
        return true;
      },
    },
    "space.move.down": {
      isEnabled: enabled,
      action: (action, context) => {
        // move block one position down
        const { node } = getNodeFromContext(context);
        if (node?.parentPtr == null) return false;
        const siblings = node?.parentPtr ? graph.getChildren(node.parentPtr) : [];
        const next = siblings[siblings.indexOf(node) + 1];
        if (next == null) return false;
        const tx = txFactory();
        moveNode(tx, graph, node, { anchor: "after", target: next });
        return true;
      },
    },
  };
}

/** Create a Block relative to another. */
export function createBlock(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: {
    block: Partial<NodeIn<NodeType.BLOCK>> & Required<Pick<NodeIn<NodeType.BLOCK>, "type">>;
    anchor: "before" | "after" | "inside";
    target: BlockData | TypedNodeReferenceData<NodeType.BLOCK> | PackageData | TypedNodeReferenceData<NodeType.PACKAGE>;
  },
): BlockData {
  const target = isNode(options.target) ? options.target : graph.getOrError(options.target);
  const packagePtr = isNode(target, NodeType.PACKAGE) ? toNodeRef(target) : target.packagePtr;

  // position in graph
  let parentPtr: NodeReferenceData;
  let orderKey: string;
  let siblings: BlockData[];
  if (options.anchor == "inside") {
    parentPtr = toNodeRef(target);
    siblings = graph.getChildren(target, NodeType.BLOCK);
    orderKey = generateOrderKey(siblings[siblings.length - 1]?.orderKey ?? null, null);
  } else {
    if (isNode(target, NodeType.PACKAGE)) throw new Error(`unexpected target node type: ${describeNode(target)}`);
    parentPtr = target.parentPtr!;
    siblings = graph.getChildren(target.parentPtr!, NodeType.BLOCK);
    orderKey = getOrderKey({ position: options.anchor, reference: target, nodes: siblings });
  }
  if (options.block.subnode == null) options.block.subnode = {};

  // add value type if not given
  if (options.block.type == BlockType.VALUE && (options.block.subnode as ValueBlockData).valueType == null) {
    (options.block.subnode as ValueBlockData).valueType = makeTypeInfo({
      kind: TypeKind.STRUCT,
      benchType: BenchType.TEXT,
    });
  }

  // create
  const block = tx.create({
    metatype: NodeType.BLOCK,
    parentPtr,
    packagePtr,
    ...options.block,
    orderKey,
    name: makeNodeName(graph, { metatype: ObjectType.BLOCK, type: options.block.type, parentPtr }),
  });
  return block;
}

/** Gets the (primary) type represented by the Block. */
export function blockToType(block: BlockData): TypeInfoData {
  // NOTE: technically there is more than one possible mapping from node to type identity
  //  (for instance Signal blocks could map to both Signal nodes based in that block or Values of that Signal type)
  let kind: TypeKind;
  let benchType: BenchType | undefined;
  let baseFieldType: FieldType | undefined;
  if (block.type == BlockType.CHOICE) {
    benchType = BenchType.FIELD;
    kind = TypeKind.BASED_NODE;
    baseFieldType = FieldType.OPTION;
  } else if (block.type == BlockType.MESSAGE) {
    benchType = BenchType.MESSAGE;
    kind = TypeKind.BASED_NODE;
  } else if (block.type == BlockType.DATABASE) {
    benchType = BenchType.RECORD;
    kind = TypeKind.BASED_NODE;
  } else {
    throw new Error(`unsupported block type: ${block.type}`);
  }
  const type = makeTypeInfo({ kind, benchType, baseFieldType });
  type.baseTypePtr = toNodeRef(block);
  return type;
}
