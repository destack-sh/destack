import { EditType, NodeType, type AnyNodeData, type EditData, NodePropertyEnumByType, BenchType } from "@/proto/wire";
import { newStructId, wrapSomeNode } from "@/proto/wiring";
import { v4 } from "uuid";

/** A transaction on the Bench state graph. */
export class Transaction {
  id: string;
  edits: EditData[] = [];

  constructor(id: string | undefined) {
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
  update(node: Partial<AnyNodeData> & { metatype: BenchType, id: string }) {
    const nodeProperties = NodePropertyEnumByType[node.metatype as unknown as NodeType];
    const properties: number[] = [];
    const patchedNode: AnyNodeData = {...node};
    for (const propName in nodeProperties) {
      if ((node as any)[propName] !== undefined) {
        properties.push(nodeProperties[propName]);
      } else {
        patchedNode[propName] = undefined;
      }
    }
    this._addEdit(EditType.UPDATE, patchedNode, properties);
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
