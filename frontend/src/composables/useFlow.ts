import { api } from "@/api";
import { useFlowsStore } from "@/stores";
import type { FlowArtifactEdge, FlowNode, FlowNodeEdge, FlowVersion } from "@/types";
import { computed, type Ref } from "vue";

export function useFlow(flow: Ref<FlowVersion | null>) {
  const flowStore = useFlowsStore();

  const _flow: Ref<FlowVersion> = computed(() => {
    if (flow.value == null) {
      throw new Error("flow not yet initialized");
    }
    return flow.value;
  });

  async function connectFlowNodes(
    dependencyNode: FlowNode,
    dependentNodes: FlowNode[],
    connectionType: "input" | "argument" = "input"
  ) {
    const nodeEdges = await Promise.all(
      dependentNodes.map((dependentNodes) =>
        flowStore.createFlowNodeEdge(_flow.value.flow, _flow.value.version, {
          dependency: dependencyNode.id,
          dependent: dependentNodes.id,
          connection_name_dependency: "*",
          connection_name_dependent: "*",
          connection_type: connectionType,
        })
      )
    );
    nodeEdges.forEach(_addFlowNodeEdge);
  }

  async function connectFlowNodeArtifact(
    flowNode: FlowNode,
    artifact: string,
    connection_type: "input" | "argument",
    connection_name: "*" = "*"
  ) {
    return flowStore
      .createFlowArtifactEdge(_flow.value.flow, _flow.value.version, {
        dependent: flowNode.id,
        dependency: artifact,
        connection_type,
        connection_name,
      })
      .then(_addFlowArtifactEdge);
  }

  async function createFlowNode(
    flowNode: Pick<FlowNode, "name" | "function_id" | "config_arguments">
  ) {
    return flowStore
      .createFlowNode(_flow.value.flow, _flow.value.version, flowNode)
      .then(_addFlowNode);
  }

  async function deleteFlowNode(flowNode: FlowNode) {
    await api.delete(
      `/flows/${_flow.value.flow}/versions/${_flow.value.version}/nodes/${flowNode.id}`
    );
    _flow.value.nodes = _flow.value.nodes?.filter((node) => node.id != flowNode.id) || [];
  }

  function artifactEdges(node: FlowNode, type: "input" | "argument" | null): FlowArtifactEdge[] {
    return (
      flow.value?.artifact_edges?.filter(
        (edge) => edge.dependent == node.id && (type == null || edge.connection_type == type)
      ) || []
    );
  }

  function _addFlowNode(node: FlowNode) {
    if (_flow.value.nodes == null) {
      _flow.value.nodes = [];
    }
    _flow.value.nodes.push(node);
    return node;
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

  return {
    createFlowNode,
    deleteFlowNode,
    connectFlowNodes,
    connectFlowNodeArtifact,
    artifactEdges,
  };
}
