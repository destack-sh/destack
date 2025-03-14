import { isInlineNode, RUNNABLE_NODE_TYPES, TYPE_NODE_TYPES } from "@/language/core/const";
import type { ReadNodeGraph } from "@/language/core/graph";
import { cloneNode, moveNode } from "@/language/core/node";
import { getOrderKey } from "@/language/core/order";
import { getTypeName, nodeToType, TypeIdentity } from "@/language/core/type";
import { newChangeId, type Transaction } from "@/language/runtime/transaction";
import {
  ActionData,
  BenchType,
  FieldData,
  FieldType,
  InlineNodeData,
  NodeReferenceData,
  NodeType,
  TypeKind,
} from "@/proto/wire";
import { describeNode, isNode, toNodeRef } from "@/proto/wiring";
import { DragContent, MultiAnchor } from "@/ui/drag";
import { Ref } from "vue";

/** Create a Field relative to some Field-containing node. */
export function createField(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: {
    field?: Partial<FieldData>;
    anchor: "before" | "above" | "after" | "below" | "inside" | "start" | "end" | "center";
    target: FieldData | ActionData | InlineNodeData;
  },
): FieldData {
  // eslint-disable-next-line prefer-const
  let { anchor, field: fieldIn } = options;
  const target = isNode(options.target) ? options.target : graph.getOrError(options.target);

  // position in graph
  let parentPtr: NodeReferenceData;
  let orderKey: string;
  let type: FieldType;
  let kind: TypeKind | null = fieldIn?.kind ?? null;
  let siblings: FieldData[];
  if (isInlineNode(target)) {
    if (anchor != "inside" && anchor != "center") throw new Error(`unexpected anchor for block: ${anchor}`);
    siblings = graph.getChildren(target, NodeType.FIELD);
    parentPtr = toNodeRef(target);
    orderKey = getOrderKey({ position: "after", reference: siblings[siblings.length - 1], nodes: siblings });
    // figure out field kind based on block type
    const nodeAsType = nodeToType(target, "instance");
    if (TYPE_NODE_TYPES.includes(target.metatype as unknown as NodeType)) {
      type = FieldType.MEMBER;
    } else if (RUNNABLE_NODE_TYPES.includes(target.metatype as unknown as NodeType)) {
      type = FieldType.INPUT;
    } else {
      throw new Error(`unexpected target node type: ${describeNode(target)}`);
    }
  } else if (isNode(target, NodeType.ACTION)) {
    if (fieldIn?.type == null) throw new Error(`missing type for action field: ${describeNode(target)}`);
    siblings = graph.getChildren(target, NodeType.FIELD);
    parentPtr = toNodeRef(target);
    if (anchor == "start") {
      orderKey = getOrderKey({ position: "before", reference: siblings[0], nodes: siblings });
    } else {
      orderKey = getOrderKey({ position: "after", reference: siblings[siblings.length - 1], nodes: siblings });
    }
    type = fieldIn.type;
  } else if (isNode(target, NodeType.FIELD)) {
    if (anchor == "inside" || anchor == "center") throw new Error(`unexpected anchor for field: ${anchor}`);
    siblings = graph.getChildren(target.parentPtr!, NodeType.FIELD);
    parentPtr = target.parentPtr!;
    orderKey = getOrderKey({ position: anchor == "start" ? "before" : "after", reference: target, nodes: siblings });
    type = target.type;
    // copy kind if none given
    kind = fieldIn?.kind ?? (target as FieldData).kind;
  } else {
    throw new Error(`unexpected target node type: ${describeNode(target)}`);
  }

  // type
  if (fieldIn?.kind == null) {
    // default to Text if no type given
    fieldIn = { ...fieldIn, kind: TypeKind.STRUCT, benchType: BenchType.TEXT };
  } else if (kind != null) {
    // override kind if forced
    fieldIn = { ...fieldIn, kind };
  }

  // name
  let name: string;
  if (fieldIn?.name != null) {
    name = fieldIn.name;
  } else {
    // make name unique (bumping number if needed)
    name = getTypeName(fieldIn!);
    const siblings = graph.getChildren(parentPtr, NodeType.FIELD);
    let i = 2;
    while (siblings.some((s) => s.name == name)) {
      name = `${name}${i++}`;
    }
  }

  const field = tx.create({
    name,
    type: type,
    ...fieldIn,
    // overwrite non-required properties
    id: undefined,
    ck: undefined,
    metatype: NodeType.FIELD,
    parentPtr,
    packagePtr: target.packagePtr,
    orderKey,
  });
  return field;
}

/** Gets the update that would be applied to a field to update its type. */
export function getFieldTypeUpdate(
  graph: ReadNodeGraph,
  field: FieldData,
  type: TypeIdentity | null,
): Partial<FieldData> {
  const update: Partial<FieldData> = {};
  // update changed properties
  for (const key of [
    "kind",
    "primitiveType",
    "benchType",
    "baseTypePtr",
    "format",
    "condition",
    "constraint",
    "isList",
    "isRequired",
  ] as (keyof TypeIdentity)[]) {
    if (field[key] != type?.[key]) {
      update[key] = type?.[key];
    }
  }

  if (type != null) {
    // update name if it was generated
    const oldName = getTypeName(field);
    if (field.name?.startsWith(oldName)) {
      // make it unique (bumping number if needed)
      update.name = getTypeName(type);
      const siblings = graph.getChildren(field.parentPtr!, NodeType.FIELD);
      let i = 2;
      while (siblings.some((s) => s.name == update.name)) {
        update.name = `${update.name}${i++}`;
      }
    }
  }

  return update;
}

/** Updates the field type to a new type identity. */
export function updateFieldType(tx: Transaction, graph: ReadNodeGraph, field: FieldData, type: TypeIdentity | null) {
  const update = getFieldTypeUpdate(graph, field, type);
  tx.update(field, update, { debounce: "tick" });
}

/** Handle drag/drop for Fields. */
export function useFieldList(options: {
  graph: ReadNodeGraph;
  txFactory: () => Transaction;
  fieldType: Ref<FieldType>;
  base: Ref<InlineNodeData | ActionData | null>;
}) {
  const { graph, txFactory, fieldType, base } = options;

  /** Whether the given drag content is allowed to be dropped on the target. */
  function allowDrop(dragged: DragContent, anchor: MultiAnchor, targetId: string | null, event?: DragEvent): boolean {
    if (dragged.kind != "node" && dragged.kind != "selection") return false;
    return dragged.nodes.every((node) => {
      node = graph.getOrError(node);
      if (isNode(node, NodeType.FIELD)) {
        return true;
      } else if (isInlineNode(node) && TYPE_NODE_TYPES.includes(node.metatype as unknown as NodeType)) {
        return true;
      } else {
        return false;
      }
    });
  }

  /** Handle drop of a dragged node onto a target. */
  function onDrop(dragged: DragContent, anchor: MultiAnchor, targetId: string | null, event: DragEvent) {
    if (dragged.kind != "node" && dragged.kind != "selection") return;
    const tx = txFactory().with({ change: { key: newChangeId(), title: "Move" } });
    const target = targetId != null ? graph.get({ id: targetId }) : null;
    for (let i = 0; i < dragged.nodes.length; i++) {
      let node = graph.getOrError(dragged.nodes[i]);
      if (isNode(node, NodeType.FIELD)) {
        if (event.altKey) {
          // duplicate field before moving
          node = cloneNode(tx, graph, node, { keepProperties: true }) as FieldData;
        }
        // move field
        if (target != null) {
          if (!isNode(target, NodeType.FIELD)) throw new Error(`unexpected target node: ${describeNode(target)}`);
          moveNode(tx, graph, node, {
            anchor: i == 0 ? anchor : "after",
            target: i == 0 ? target : graph.getOrError(dragged.nodes[i - 1]),
          });
        } else {
          moveNode(tx, graph, node, { anchor: "center", target: base.value! });
        }
        if ((node as FieldData).type != fieldType.value) {
          tx.update(node, { type: fieldType.value ?? undefined }, { debounce: "tick" });
        }
      } else {
        // add field with any node type
        const type = nodeToType(node, "instance");
        const fieldIn = { ...type, type: fieldType.value! };
        if (target != null) {
          if (!isNode(target, NodeType.FIELD)) throw new Error(`unexpected target node: ${describeNode(target)}`);
          createField(tx, graph, {
            field: fieldIn,
            anchor: i == 0 ? anchor : "after",
            target: i == 0 ? target : (graph.getOrError(dragged.nodes[i - 1]) as FieldData),
          });
        } else {
          createField(tx, graph, { field: fieldIn, anchor: "inside", target: base.value! });
        }
      }
    }
  }

  return { allowDrop, onDrop };
}
