import {
  EditType,
  GraphScope,
  type AnyNodeData,
  type BenchType,
  type EditData,
  type NodeType,
  type NodeTypeMapping,
PropertyEnumByType,
NodePropertyEnumByType,
} from "@/proto/wire";
import { unwrapSomeNode } from "@/proto/wiring";
import { ref, type Ref } from "vue";

export class NodeDataGraph {
  public readonly scope: GraphScope = {};
  private nodesById: { [id: string]: AnyNodeData } = {};
  private nodesByParentIdAndType: { [parentId: string]: { [type: string]: string[] } } = {};
  private roots: string[] = [];

  constructor(scope: GraphScope = {}) {
    this.scope = scope;
  }

  get(id: string) {
    return this.nodesById[id];
  }

  findRoots<T extends NodeType>(metatype: NodeType): NodeTypeMapping[T][] {
    return this.roots
      .filter((id) => this.nodesById[id].metatype == (metatype as unknown as BenchType))
      .map((id) => this.nodesById[id] as NodeTypeMapping[T]);
  }

  findRoot<T extends NodeType>(metatype: T): NodeTypeMapping[T] {
    const roots = this.findRoots(metatype);
    if (roots.length > 1) throw new Error(`multiple roots of type ${metatype}`);
    return roots[0] as NodeTypeMapping[T];
  }

  getChildren<T extends NodeType>(parent: AnyNodeData, metatype: T): NodeTypeMapping[T][] {
    const children = this.nodesByParentIdAndType[parent.id]?.[metatype];
    if (!children) return [];
    return children.map((id) => this.nodesById[id]) as NodeTypeMapping[T][];
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
      const nodeIdx = this.nodesByParentIdAndType[parentId][node.metatype].findIndex((n) => n == node.id);
      if (nodeIdx == -1) throw new Error(`node with id ${node.id} not found in parent ${parentId}`);
      this.nodesByParentIdAndType[parentId][node.metatype].splice(nodeIdx, 1);
    } else {
      const rootIdx = this.roots.findIndex((n) => n == node.id);
      if (rootIdx == -1) throw new Error(`node with id ${node.id} not found in roots`);
      this.roots.splice(rootIdx, 1);
    }
    // remove any children
    for (const childId of this.nodesByParentIdAndType[node.id]?.children || []) {
      const child = this.nodesById[childId];
      this.remove(child);
    }
  }

  getRef(id: string | Ref<string>): Ref<AnyNodeData | null> {
    throw new Error("not yet implemented");
  }

  findRootRef<T extends NodeType>(metatype: T): Ref<NodeTypeMapping[T] | null> {
    return ref(null); // nocheckin
  }

  getChildrenRef<T extends NodeType>(parent: Ref<AnyNodeData | null>, metatype: T): Ref<NodeTypeMapping[T][]> {
    return ref([]); // nocheckin
  }
}

function editGraph(graph: NodeDataGraph, edits: EditData[]) {
  /** Applies the edits to the graph (in place!). */

  for (const edit of edits) {
    if (edit.node == null) throw new Error(`missing node in edit: ${edit}`);
    const nodeData = unwrapSomeNode(edit.node);
    const editType = edit.type;
    if (editType == EditType.CREATE || (editType == EditType.UPSERT && !graph.get(nodeData.id))) {
      graph.add(nodeData);
    } else if (editType == EditType.DELETE) {
      graph.remove(nodeData);
    } else {
      let properties: number[];
      const nodeProperties = NodePropertyEnumByType[nodeData.metatype]!;
      if (editType == EditType.UPDATE || editType == EditType.MOVE) {
        properties = edit.properties;
      } else if (editType == EditType.ARCHIVE || editType == EditType.UNARCHIVE) {
        properties = [nodeProperties.archivedAt];
      } else if (editType == EditType.SOFT_DELETE || editType == EditType.RESTORE) {
        properties = [nodeProperties.deletedAt];
      } else {
        throw new Error(`unexpected edit type: ${editType}`);
      }
      const existingNode = graph.get(nodeData.id);
      if (!existingNode) throw new Error(`missing node for update: ${edit}`);
      for (const propId of properties) {
        const propName = nodeProperties[propId];
        (existingNode as any)[propName] = (nodeData as any)[propName];
      }
      graph.update(existingNode);
    }
  }
}
