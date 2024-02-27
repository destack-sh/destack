import type { AnyNodeData } from "@/proto/wire";


export class NodeDataGraph {
  nodesById: { [id: string]: AnyNodeData } = {};
  nodesByParentIdAndType: { [parentId: string]: { [type: string]: AnyNodeData[] } } = {};

  constructor() {
  }

  get(id: string) {
    return this.nodesById[id];
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
      this.nodesByParentIdAndType[parentId][node.metatype].push(node);
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
      const nodeIdx = this.nodesByParentIdAndType[parentId][node.metatype].indexOf(node);
      if (nodeIdx === -1) {
        throw new Error(`node with id ${node.id} not found in parent ${parentId}`);
      }
      this.nodesByParentIdAndType[parentId][node.metatype].splice(nodeIdx, 1);
    }
  }
}