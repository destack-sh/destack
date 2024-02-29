import { EditType, type AnyNodeData, type EditData } from "@/proto/wire";
import { v4 } from "uuid";

export class Transaction {
  id: string;
  edits: EditData[] = [];

  constructor(id: string | undefined) {
    this.id = id ?? v4();
  }

  _makeEdit(type: EditType, node: AnyNodeData, properties?: number[]): EditData {
    throw new Error("not yet implemented");
  }

  _addEdit(type: EditType, node: AnyNodeData, properties?: number[]) {
    const edit = this._makeEdit(type, node, properties);
    this.edits.push(edit);
  }

  create(node: AnyNodeData) {
    this._addEdit(EditType.CREATE, node);
  }

  upsert(node: AnyNodeData) {
    this._addEdit(EditType.UPSERT, node);
  }

  update(node: Partial<AnyNodeData> & { id: string}) {
    throw new Error("not yet implemented");
  }

  move(node: AnyNodeData) {
    this._addEdit(EditType.MOVE, node);
  }

  archive(node: AnyNodeData) {
    this._addEdit(EditType.ARCHIVE, node);
  }

  unarchive(node: AnyNodeData) {
    this._addEdit(EditType.UNARCHIVE, node);
  }

  softDelete(node: AnyNodeData) {
    this._addEdit(EditType.SOFT_DELETE, node);
  }

  restore(node: AnyNodeData) {
    this._addEdit(EditType.RESTORE, node);
  }

  delete(node: AnyNodeData) {
    this._addEdit(EditType.DELETE, node);
  }
}
