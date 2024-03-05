import {
  EditType,
  NodeType,
  type AnyNodeData,
  type EditData,
  NODE_PROPERTY_ENUM_BY_TYPE,
  BenchType,
  type NodeTypeMapping,
  MESSAGE_TYPE_BY_BENCH_TYPE,
  type AnyPropertyType,
} from "@/proto/wire";
import { newStructId, unwrapSomeNode, wrapSomeNode } from "@/proto/wiring";
import { type ReadNodeGraph, type WriteNodeGraph } from "@/system/graph";
import { v4 } from "uuid";

/** A transaction on the Bench state graph. */
export class Transaction {
  id: string;
  edits: EditData[] = [];

  constructor(id: string | undefined = undefined) {
    this.id = id ?? v4();
  }

  _makeEdit(type: EditType, node: AnyNodeData, properties?: number[]): EditData {
    const edit: EditData = {
      id: newStructId(),
      type,
      node: wrapSomeNode(node),
      nodeType: node.metatype as unknown as NodeType,
      properties: properties ?? [],
      scope: {
        benchId: "packagePtr" in node ? node.packagePtr?.benchId : undefined,
        packageId: "packagePtr" in node ? node.packagePtr?.id : undefined,
        transactionId: this.id,
      },
    };
    return edit;
  }

  _addEdit(type: EditType, node: AnyNodeData, properties?: number[]) {
    const edit = this._makeEdit(type, node, properties);
    this.edits.push(edit);
  }

  /** Create a new node */
  create(node: AnyNodeData) {
    this._addEdit(EditType.CREATE, node);
  }

  /** Create or update all properties in the node */
  upsert(node: AnyNodeData) {
    this._addEdit(EditType.UPSERT, node);
  }

  /** Update regular properties in this node */
  update<T extends NodeType>(
    update: Partial<Omit<NodeTypeMapping[T], "id" | "metatype">> & { metatype: T; id: string },
  ) {
    const allProperties: AnyPropertyType = NODE_PROPERTY_ENUM_BY_TYPE[update.metatype as unknown as NodeType]!;
    const messageType = MESSAGE_TYPE_BY_BENCH_TYPE[update.metatype as unknown as BenchType]!;
    const properties: number[] = [];
    const patchedNode = { ...update };
    let ord = 0;
    for (const propName in Object.keys(allProperties)) {
      if (!Number.isNaN(Number(propName))) continue; // skip numeric keys
      if (propName === "id" || propName === "metatype") {
        // keep as is (but not part of the 'update')
      } else if ((update as any)[propName] !== undefined) {
        // update the assigned property
        properties.push((allProperties as any)[propName]);
      } else {
        // init unset fields with an allowed default value
        //  (will be ignored anyway since its not in 'properties', but required for protobuf validation)
        const field = messageType.fields[ord];
        (patchedNode as any)[propName] = field.repeat ? [] : undefined;
      }
      ord += 1;
    }
    this._addEdit(EditType.UPDATE, patchedNode as unknown as NodeTypeMapping[T], properties);
  }

  /** Move node between parents */
  move(node: AnyNodeData) {
    this._addEdit(EditType.MOVE, node);
  }

  /** Archive node (incl. descendants) */
  archive(node: AnyNodeData) {
    this._addEdit(EditType.ARCHIVE, node);
  }

  /** Restore node from archive */
  unarchive(node: AnyNodeData) {
    this._addEdit(EditType.UNARCHIVE, node);
  }

  /** Soft delete node (incl.descendants), marked for later deletion after retention period */
  softDelete(node: AnyNodeData) {
    this._addEdit(EditType.SOFT_DELETE, node);
  }

  /** Restore node from soft delete */
  restore(node: AnyNodeData) {
    this._addEdit(EditType.RESTORE, node);
  }

  /**
   * @deprecated use softDelete by default (not really deprecated, just to make it clear this should be used deliberately)
   */
  delete(node: AnyNodeData) {
    this._addEdit(EditType.DELETE, node);
  }
}

export function editGraph(graph: ReadNodeGraph & WriteNodeGraph, edits: EditData[]) {
  /** Applies the edits to the graph (in place!). */

  for (const edit of edits) {
    if (edit.node == null) throw new Error(`missing node in edit: ${edit}`);
    const nodeData = unwrapSomeNode(edit.node);
    const editType = edit.type;
    if (editType == EditType.CREATE || (editType == EditType.UPSERT && !graph.get({ id: nodeData.id }))) {
      graph.add(nodeData);
    } else if (editType == EditType.DELETE) {
      graph.remove(nodeData);
    } else {
      let properties: number[];
      const nodeProperties = NODE_PROPERTY_ENUM_BY_TYPE[nodeData.metatype]!;
      if (editType == EditType.UPDATE || editType == EditType.MOVE) {
        properties = edit.properties;
      } else if (editType == EditType.ARCHIVE || editType == EditType.UNARCHIVE) {
        properties = [nodeProperties.archivedAt];
      } else if (editType == EditType.SOFT_DELETE || editType == EditType.RESTORE) {
        properties = [nodeProperties.deletedAt];
      } else {
        throw new Error(`unexpected edit type: ${editType}`);
      }
      const existingNode = graph.get({ id: nodeData.id });
      if (!existingNode) throw new Error(`missing node for update: ${edit}`);
      for (const propId of properties) {
        const propName = nodeProperties[propId];
        (existingNode as any)[propName] = (nodeData as any)[propName];
      }
      graph.update(existingNode);
    }
  }
}
