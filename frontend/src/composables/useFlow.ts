import { api } from "@/api";
import { useFlowsStore } from "@/stores";
import type { ArtifactConnection, FlowArtifactEdge, FlowNode, FlowNodeEdge, FlowVersion } from "@/types";
import { computed, type Ref } from "vue";

export const CURRENT_USE_FLOW_KEY = Symbol();

// TODO @Cleanup @Architecture: unify state management for editable objects between store and composables
//  like flow store and useFlow, or records and artifact store, ...
//  It's probably okay to have an editable wrapper on top for different sessions
//  Will also want to consider eventual multiplayer/collaborative features (if feasible)
export function useFlow(flow: Ref<FlowVersion | null>) {
  const flowStore = useFlowsStore();

  const _flow: Ref<FlowVersion> = computed(() => {
    if (flow.value == null) {
      throw new Error("flow not yet initialized");
    }
    return flow.value;
  });

  const _flowUrl = computed(() => `/flows/${_flow.value.flow}/versions/${_flow.value.version}`);

  async function connectFlowNodes(
    dependencyNode: FlowNode,
    dependentNodes: FlowNode[],
    connectionType: "input" | "argument" = "input",
    connectionNameDependency = "*",
    connectionNameDependent = "*"
  ) {
    const nodeEdges = await Promise.all(
      dependentNodes.map((dependentNodes) =>
        flowStore.createFlowNodeEdge(_flow.value.flow, _flow.value.version, {
          dependency: dependencyNode.id,
          dependent: dependentNodes.id,
          connection_name_dependency: connectionNameDependency,
          connection_name_dependent: connectionNameDependent,
          connection_type: connectionType,
        })
      )
    );
    nodeEdges.forEach(_addFlowNodeEdge);
  }

  async function connectFlowNodeArtifact(flowNode: FlowNode, connection: ArtifactConnection) {
    return flowStore
      .createFlowArtifactEdge(_flow.value.flow, _flow.value.version, {
        dependent: flowNode.id,
        ...connection,
      })
      .then(_addFlowArtifactEdge);
  }

  async function setFlowNodeArtifactConnections(flowNode: FlowNode, connections: ArtifactConnection[], remove = true) {
    const existingConnections = artifactEdges(flowNode, null);

    function sameConnection(c1: ArtifactConnection, c2: ArtifactConnection) {
      return (
        c1.dependency == c2.dependency &&
        c1.connection_type == c2.connection_type &&
        c1.connection_name == c2.connection_name
      );
    }

    function getExistingConnection(connection: ArtifactConnection): FlowArtifactEdge | undefined {
      return existingConnections.find((conn) => sameConnection(conn, connection));
    }

    // create new connections & update existing connections
    for (const connection of connections) {
      const existingConnection = getExistingConnection(connection);
      if (existingConnection == null) {
        // create new
        await connectFlowNodeArtifact(flowNode, connection).then(_addFlowArtifactEdge);
      } else if (existingConnection != connection) {
        // update existing if changed
        await flowStore
          .updateFlowArtifactEdge(_flow.value.flow, _flow.value.version, {
            ...existingConnection,
            ...connection,
          })
          .then(_updateFlowArtifactEdge);
      }
    }

    // remove no longer needed connections
    if (remove) {
      const redundantConnections = existingConnections.filter(
        (existingEdge) => !connections.find((e) => sameConnection(e, existingEdge))
      );
      await Promise.all(
        redundantConnections.map((edge) =>
          flowStore.deleteFlowArtifactEdge(_flow.value.flow, _flow.value.version, edge.id)
        )
      ).then(() => redundantConnections.forEach(_deleteFlowArtifactEdge));
    }
  }

  async function createFlowNode(flowNode: Pick<FlowNode, "name" | "function_id" | "config_arguments">) {
    return flowStore.createFlowNode(_flow.value.flow, _flow.value.version, flowNode).then(_addFlowNode);
  }

  async function updateFlowNode(flowNode: Partial<FlowNode> & Pick<FlowNode, "id">) {
    return api
      .patch<FlowNode>(`${_flowUrl.value}/nodes/${flowNode.id}`, flowNode)
      .then((response) => response.data)
      .then(_updateFlowNode);
  }

  async function deleteFlowNode(flowNode: FlowNode) {
    await api.delete(`${_flowUrl.value}/nodes/${flowNode.id}`);
    _flow.value.nodes = _flow.value.nodes?.filter((node) => node.id != flowNode.id) || [];
  }

  function artifactEdges(node: FlowNode, type: "input" | "argument" | null): FlowArtifactEdge[] {
    return (
      flow.value?.artifact_edges?.filter(
        (edge) => edge.dependent == node.id && (type == null || edge.connection_type == type)
      ) || []
    );
  }

  function getFlowNode(id: string): FlowNode | undefined {
    return flow.value?.nodes?.find((node) => node.id == id);
  }

  function _addFlowNode(node: FlowNode) {
    if (_flow.value.nodes == null) {
      _flow.value.nodes = [];
    }
    _flow.value.nodes.push(node);
    return node;
  }

  function _updateFlowNode(node: FlowNode) {
    const existingNode = _flow.value.nodes?.find((n) => n.id == node.id);
    if (existingNode != null) {
      Object.assign(existingNode, node);
    }
  }

  function _addFlowNodeEdge(nodeEdge: FlowNodeEdge) {
    if (_flow.value.node_edges == null) {
      _flow.value.node_edges = [];
    }
    _flow.value.node_edges.push(nodeEdge);
    return nodeEdge;
  }

  function _addFlowArtifactEdge(artifactEdge: FlowArtifactEdge) {
    if (_flow.value.artifact_edges == null) {
      _flow.value.artifact_edges = [];
    }
    _flow.value.artifact_edges.push(artifactEdge);
    return artifactEdge;
  }

  function _updateFlowArtifactEdge(artifactEdge: FlowArtifactEdge) {
    if (_flow.value.artifact_edges == null) {
      _addFlowArtifactEdge(artifactEdge);
    } else {
      const existingEdge = _flow.value.artifact_edges.find((e) => e.id == artifactEdge.id);
      if (existingEdge == null) {
        _addFlowArtifactEdge(artifactEdge);
      } else {
        Object.assign(existingEdge, artifactEdge);
      }
    }
  }

  function _deleteFlowArtifactEdge(artifactEdge: FlowArtifactEdge) {
    if (_flow.value.artifact_edges == null) {
      return;
    }
    _flow.value.artifact_edges = _flow.value.artifact_edges.filter((e) => e.id != artifactEdge.id);
  }

  return {
    createFlowNode,
    updateFlowNode,
    deleteFlowNode,
    connectFlowNodes,
    connectFlowNodeArtifact,
    setFlowNodeArtifactConnections,
    artifactEdges,
    getFlowNode,
  };
}
