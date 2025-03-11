import { supergraph } from "@/globals";
import { INLINE_NODE_TYPES, isInlineNode, toCamelName } from "@/language/core/const";
import { type ReadNodeGraph } from "@/language/core/graph";
import { NodeIn } from "@/language/core/node";
import { getOrderKey } from "@/language/core/order";
import { createTask } from "@/language/runtime/task";
import { newChangeId, type Transaction } from "@/language/runtime/transaction";
import { choiceToType, createChoice } from "@/language/source/choice";
import { classToType, createClass } from "@/language/source/class";
import { createDatabase, databaseToType } from "@/language/source/database";
import { createFlow, flowToType } from "@/language/source/flow";
import { createKit } from "@/language/source/kit";
import { createPage } from "@/language/source/page";
import {
  AnyNodeData,
  BlockData,
  BlockType,
  FieldType,
  InlineNodeData,
  NodeReferenceData,
  NodeType,
  PageData,
  StructType,
  TypeData,
} from "@/proto/wire";
import { describeNode, isNode, makeStruct, toNodeRef } from "@/proto/wiring";
import { generateOrderKey } from "@/utils/fractional";

/** Create a Block. */
export function createBlock(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: {
    block: Partial<NodeIn<NodeType.BLOCK>>;
    node?: InlineNodeData | Partial<NodeIn<any>>;
    anchor: "before" | "after" | "inside";
    target: BlockData | PageData;
  },
): BlockData {
  const { target, anchor } = options;
  if (tx.change?.key == null) {
    tx = tx.with({ change: { key: newChangeId(), title: "Create" } });
  }
  const type = options.block.type;
  if (type == null) {
    throw new Error(`block type is required`);
  }

  // position
  let parentPtr: NodeReferenceData;
  let orderKey: string;
  let siblings: BlockData[];
  if (anchor == "inside") {
    parentPtr = toNodeRef(target);
    siblings = graph.getChildren(target, NodeType.BLOCK);
    orderKey = generateOrderKey(siblings[siblings.length - 1]?.orderKey ?? null, null);
  } else {
    if (isNode(target, NodeType.PACKAGE)) throw new Error(`unexpected target node type: ${describeNode(target)}`);
    parentPtr = target.parentPtr!;
    siblings = graph.getChildren(target.parentPtr!, NodeType.BLOCK);
    orderKey = getOrderKey({ position: anchor, reference: target, nodes: siblings });
  }

  // text
  let line = options.block.line;
  if (type >= BlockType.PARAGRAPH) {
    if (line == null) {
      line = makeStruct({ metatype: StructType.TEXT_LINE, type: type - 10_000 });
    }
    if (line.type != type - 10_000) {
      throw new Error(`text type mismatch: ${line.type} != ${type - 10_000}`);
    }
  }

  // create
  const block = tx.make({
    metatype: NodeType.BLOCK,
    parentPtr,
    packagePtr: target.packagePtr,
    ...options.block,
    line,
    orderKey,
  });
  if (INLINE_NODE_TYPES.includes(options.block.type as unknown as NodeType) && options.block.nodePtr == null) {
    if (options.node?.id != null) {
      block.nodePtr = toNodeRef(options.node as AnyNodeData);
    } else {
      // new inline node
      const node = createInlineNode(tx, graph, {
        node: {
          metatype: block.type,
          ...options.node,
          parentPtr,
          packagePtr: target.packagePtr,
          definitionPtr: toNodeRef(block),
        },
      });
      block.nodePtr = toNodeRef(node);
    }
  }
  tx.create(block);
  return block;
}

/** Create an InlineNode for a Block. */
export function createInlineNode(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: { node: Partial<NodeIn<any>> },
): InlineNodeData {
  if (tx.change?.key == null) {
    tx = tx.with({ change: { key: newChangeId(), title: "Create" } });
  }

  if (options.node.metatype == NodeType.PAGE) {
    const parent = graph.getOrError(options.node.parentPtr);
    if (!isNode(parent, NodeType.PAGE)) {
      throw new Error(`cannot create Page inside: ${describeNode(parent)}`);
    }
    return createPage(tx, graph, { page: options.node, anchor: "inside", target: parent });
  } else if (options.node.metatype == NodeType.DATABASE) {
    return createDatabase(tx, graph, { database: options.node });
  } else if (options.node.metatype == NodeType.FLOW) {
    return createFlow(tx, graph, { flow: options.node });
  } else if (options.node.metatype == NodeType.CLASS) {
    return createClass(tx, graph, { class: options.node });
  } else if (options.node.metatype == NodeType.CHOICE) {
    return createChoice(tx, graph, { choice: options.node });
  } else if (options.node.metatype == NodeType.KIT) {
    return createKit(tx, graph, { kit: options.node });
  } else if (options.node.metatype == NodeType.TASK) {
    return createTask(tx, graph, { task: options.node });
  } else {
    throw new Error(`unexpected node type: ${toCamelName(NodeType, options.node.metatype)}`);
  }
}

/** Unwrap a Block into its inner source node, if it has one. */
export function unwrapBlockDefinition(block: BlockData): InlineNodeData | undefined {
  if (block.nodePtr == null) return undefined;
  const node = supergraph.get(block.nodePtr);
  if (!isInlineNode(node)) {
    return undefined; // may be other node type
  }
  return node;
}

/** Unwrap an InlineNode into its Block. */
export function unwrapInlineNode(node: InlineNodeData): BlockData | undefined {
  if (node.definitionPtr == null) return undefined;
  const block = supergraph.getOrError(node.definitionPtr);
  return block as BlockData;
}

/** Get the Type for a Block. Unwraps to the type for the inner node (if any). */
export function blockToTypeMaybe(
  block: BlockData,
  of?: "instance" | "value",
  fieldTypes?: FieldType[],
): TypeData | undefined {
  const node = unwrapBlockDefinition(block);
  if (node == null) return undefined;

  if (isNode(node, NodeType.DATABASE)) {
    return databaseToType(node, of, fieldTypes);
  } else if (isNode(node, NodeType.FLOW)) {
    return flowToType(node, of, fieldTypes);
  } else if (isNode(node, NodeType.CLASS)) {
    return classToType(node, of, fieldTypes);
  } else if (isNode(node, NodeType.CHOICE)) {
    return choiceToType(node);
  }

  return undefined;
}

/** Get the Type for a Block. Unwraps to the type for the inner node (if any). */
export function blockToType(block: BlockData, of?: "instance" | "value", fieldTypes?: FieldType[]): TypeData {
  const type = blockToTypeMaybe(block, of, fieldTypes);
  if (type == null) {
    throw new Error(`block has no type: ${describeNode(block)}`);
  }
  return type;
}
