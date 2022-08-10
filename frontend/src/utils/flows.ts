import type {
  FlowArtifactEdge,
  FlowNode,
  FlowNodeEdge,
  FlowNodePort,
  FlowNodePortType,
  FlowVersion,
  FunctionHandlerSpec,
} from "@/types";

export function artifactEdges(
  flow: FlowVersion,
  where: {
    dependency?: FlowNode;
    name?: string;
    type?: "input" | "argument" | "output";
  }
): FlowArtifactEdge[] {
  if (flow.artifact_edges == null) {
    return [];
  }

  if (where == {}) {
    return flow.artifact_edges;
  }

  return flow.artifact_edges.filter(
    (edge) =>
      (where.dependency == null || edge.dependent == where.dependency.id) &&
      (where.name == null || edge.connection_name == where.name) &&
      (where.type == null || edge.connection_type == where.type)
  );
}

export function nodeEdges(
  flow: FlowVersion,
  where: {
    dependent?: FlowNode;
    dependency?: FlowNode;
    sourceName?: string;
    targetName?: string;
    type?: "input" | "argument";
  }
): FlowNodeEdge[] {
  if (flow.node_edges == null) {
    return [];
  }
  if (where == {}) {
    return flow.node_edges;
  }

  return flow.node_edges.filter(
    (edge) =>
      (where.dependent == null || edge.dependent == where.dependent.id) &&
      (where.dependency == null || edge.dependency == where.dependency.id) &&
      (where.type == null || edge.connection_type == where.type)
  );
}

export function nodeEdgesAtPort(flow: FlowVersion, port: FlowNodePort): FlowNodeEdge[] {
  if (port.type == "output") {
    // invert output search (since nodes are symmetric and we only store the input edge)
    return nodeEdges(flow, { dependency: port.node, type: "input", sourceName: port.name });
  } else {
    return nodeEdges(flow, { dependent: port.node, type: port.type, targetName: port.name });
  }
}

export function artifactEdgesAtPort(flow: FlowVersion, port: FlowNodePort): FlowArtifactEdge[] {
  return artifactEdges(flow, { dependency: port.node, type: port.type, name: port.name });
}

export function isPortSatisfied(flow: FlowVersion, port: FlowNodePort): boolean {
  return nodeEdgesAtPort(flow, port).length > 0;
}

export function flowPorts(
  flow: FlowVersion,
  functionHandlersByName: Record<string, FunctionHandlerSpec>,
  ofType?: FlowNodePortType
): FlowNodePort[] {
  if (flow.nodes == null) {
    return [];
  }
  return flow.nodes.flatMap((node) => nodePorts(node, functionHandlersByName, ofType));
}

export function nodePorts(
  node: FlowNode,
  functionHandlersByName: Record<string, FunctionHandlerSpec>,
  ofType?: FlowNodePortType
): FlowNodePort[] {
  const functionSpec = functionHandlersByName[node.function_id];

  let types: FlowNodePortType[] = ["input", "output", "argument"] as FlowNodePortType[];
  if (ofType != null) {
    types = [ofType];
  }

  const specs: Record<FlowNodePortType, Record<string, any>> = {
    input: functionSpec.base_spec.input_spec,
    output: functionSpec.base_spec.output_spec,
    argument: functionSpec.config_spec.type,
  };
  return types.flatMap((type) =>
    Object.keys(specs[type]).map((name) => ({
      name,
      node,
      type,
    }))
  );
}

export function isValidEdge(flow: FlowVersion, edge: Omit<FlowNodeEdge, "id">): { valid: boolean; reason?: string } {
  function _invalid(reason: string) {
    return { valid: false, reason };
  }

  // edge already exists
  // TODO @Broken: isValidEdge doesn't accept multi-edges between same nodes via different ports
  const matchingEdge = (flow.node_edges || [])
    .filter((node) => node.connection_type == edge.connection_type)
    .find(
      (node) =>
        (node.dependency == edge.dependency && node.dependent == edge.dependent) ||
        (node.dependency == edge.dependent && node.dependent == edge.dependency)
    );
  if (matchingEdge != null) {
    return _invalid("already exists");
  }

  // simple loop
  if (edge.dependency == edge.dependent) {
    return _invalid("cannot loop");
  }

  // TODO @Feature: detect indirect loop in node edge

  // TODO @Feature: detect incompatible node types

  return { valid: true };
}
