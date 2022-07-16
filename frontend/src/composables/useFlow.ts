import { useFlowsStore } from "@/stores";
import type { FlowNode, FlowVersion, FlowNodeEdge, FlowArtifactEdge } from "@/types";
import { type Ref, computed, ref, watchEffect } from "vue";

export function useFlow(name: Ref<string>) {
  const flowStore = useFlowsStore();

  const flow: Ref<FlowVersion | null> = ref(null);
  const _flow: Ref<FlowVersion> = computed(() => {
    if (flow.value == null) {
      throw new Error("flow not yet initialized");
    }
    return flow.value;
  });

  // ensure flow = latest version of flow by name (create if needed)
  watchEffect(async () => {
    if (flow.value == null || flow.value.name != name.value) {
      // reload or create flow with version
      let flowInstance = flowStore.flow(name.value);
      if (!flowInstance) {
        flowInstance = await flowStore.createFlow({ name: name.value });
      }
      let flowVersion = flowInstance.latest_version;
      if (!flowVersion) {
        flowVersion = await flowStore.createFlowVersion(name.value, {
          name: "Initial commit",
          description: "Auto-generated.",
          parents: [],
        });
      }
      flow.value = flowVersion;
    }
  });

  async function connectFlowNodes(
    dependencyNode: FlowNode,
    dependentNodes: FlowNode[],
    connectionType: "input" | "argument" = "input"
  ) {
    const nodeEdges = await Promise.all(
      dependentNodes.map((dependentNodes) =>
        flowStore.createFlowNodeEdge(_flow.value.name, _flow.value.version, {
          dependency_node: dependencyNode.id,
          dependent_node: dependentNodes.id,
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
    connection_name: "*"
  ) {
    return flowStore
      .createFlowArtifactEdge(_flow.value.name, _flow.value.version, {
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
      .createFlowNode(_flow.value.name, _flow.value.version, flowNode)
      .then(_addFlowNode);
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

  return { flow, createFlowNode, connectFlowNodes, connectFlowNodeArtifact };
}
