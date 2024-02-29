import type { AnyNodeData, BenchType, EditData, NodeType } from "@/proto/wire";

export class NodeDataGraph {
  private nodesById: { [id: string]: AnyNodeData } = {};
  private nodesByParentIdAndType: { [parentId: string]: { [type: string]: string[] } } = {};
  private roots: string[] = [];

  constructor() {}

  get(id: string) {
    return this.nodesById[id];
  }

  getRef(id: string) {
    throw new Error("not implemented");
  }

  clear() {
    this.nodesById = {};
    this.nodesByParentIdAndType = {};
  }

  add(node: AnyNodeData) {
    if (this.nodesById[node.id]) {
      throw new Error(`node with id ${node.id} already exists`);
    }
    this.nodesById[node.id] = node;

    // add to parent
    if (node.parentPtr?.id) {
      const parentId: string = node.parentPtr.id;
      if (!this.nodesByParentIdAndType[parentId]) {
        this.nodesByParentIdAndType[parentId] = {};
      }
      if (!this.nodesByParentIdAndType[parentId][node.metatype]) {
        this.nodesByParentIdAndType[parentId][node.metatype] = [];
      }
      this.nodesByParentIdAndType[parentId][node.metatype].push(node.id);
    } else {
      this.roots.push(node.id);
    }
  }

  update(node: AnyNodeData) {
    const existing = this.nodesById[node.id];
    if (!existing) {
      throw new Error(`node with id ${node.id} does not exist`);
    }

    // remove/re-add to update with parent if needed, otherwise just update in place
    if (existing.parentPtr?.id != node.parentPtr?.id) {
      this.remove(existing);
      this.add(node);
    } else {
      this.nodesById[node.id] = node;
    }
  }

  remove(node: AnyNodeData) {
    delete this.nodesById[node.id];

    // remove from parent
    if (node.parentPtr?.id) {
      const parentId: string = node.parentPtr.id;
      const nodeIdx = this.nodesByParentIdAndType[parentId][node.metatype].findIndex(n => n == node.id);
      if (nodeIdx === -1) throw new Error(`node with id ${node.id} not found in parent ${parentId}`);
      this.nodesByParentIdAndType[parentId][node.metatype].splice(nodeIdx, 1);
    } else {
      const rootIdx = this.roots.findIndex(n => n == node.id);
      if (rootIdx === -1) throw new Error(`node with id ${node.id} not found in roots`);
      this.roots.splice(rootIdx, 1);
    }
    // remove any children
    for (const childId of this.nodesByParentIdAndType[node.id]?.children || []) {
      const child = this.nodesById[childId];
      this.remove(child);
    }
  }
}

function editGraph(graph: NodeDataGraph, edits: EditData[]) {
  throw new Error("not yet implemented");
}