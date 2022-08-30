import RecordSpecDisplayVue from "@/components/RecordSpecDisplay.vue";
import type {
  ArtifactConnection,
  FlowArtifactEdge,
  FlowNode,
  FlowNodeEdge,
  FlowNodePort,
  FlowNodePortType,
  FlowVersion,
  FunctionHandlerSpec,
  RecordSpec,
} from "@/types";

export function getFlowNode(flow: FlowVersion, id: string): FlowNode | undefined {
  return flow.nodes?.find((n) => n.id == id);
}

export function flowNode(flow: FlowVersion, id: string): FlowNode {
  const node = flow.nodes?.find((n) => n.id == id);
  if (node == null) {
    throw new Error(`could not find node ${id} in flow ${flow}`);
  }
  return node;
}

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
      (where.type == null || edge.connection_type == where.type) &&
      (where.sourceName == null || edge.connection_name_dependency == where.sourceName) &&
      (where.targetName == null || edge.connection_name_dependent == where.targetName)
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
  functionHandlersById: Record<string, FunctionHandlerSpec>,
  ofType?: FlowNodePortType
): FlowNodePort[] {
  if (flow.nodes == null) {
    return [];
  }
  return flow.nodes.flatMap((node) => nodePorts(node, functionHandlersById, ofType));
}

export function getFlowNodePortId(port: Omit<FlowNodePort, "id">): string {
  if (port.name != "*") {
    return `${port.node.id}.${port.type}s.${port.name}`;
  } else {
    return `${port.node.id}.${port.type}s`;
  }
}

export function makeFlowNodePort(name: string, node: FlowNode, type: FlowNodePortType): FlowNodePort {
  return {
    id: getFlowNodePortId({ name, node, type }),
    name,
    node,
    type,
  };
}

export function nodeEdgePorts(flow: FlowVersion, edge: FlowNodeEdge): { source: FlowNodePort; target: FlowNodePort } {
  const sourceNode = flowNode(flow, edge.dependency);
  const targetNode = flowNode(flow, edge.dependent);

  return {
    source: makeFlowNodePort(edge.connection_name_dependency, sourceNode, "output"),
    target: makeFlowNodePort(edge.connection_name_dependent, targetNode, "input"),
  };
}

export function nodePorts(
  node: FlowNode,
  functionHandlersById: Record<string, FunctionHandlerSpec>,
  ofType?: FlowNodePortType
): FlowNodePort[] {
  const functionSpec = functionHandlersById[node.function_id];

  let types: FlowNodePortType[] = ["input", "output", "argument"] as FlowNodePortType[];
  if (ofType != null) {
    types = [ofType];
  }

  const specs: Record<FlowNodePortType, Record<string, any>> = {
    input: functionSpec.base_spec.input_spec,
    output: functionSpec.base_spec.output_spec,
    argument: functionSpec.config_spec,
  };
  return types.flatMap((type) => Object.keys(specs[type]).map((name) => makeFlowNodePort(name, node, type)));
}

export function getPortSpec(
  port: FlowNodePort,
  functionHandlersById: Record<string, FunctionHandlerSpec>,
  allowBase = false
): RecordSpec | null {
  const functionSpec = functionHandlersById[port.node.function_id];
  const nodeMetadata = port.node.metadata;

  let portSpec: RecordSpec | undefined;
  if (port.type == "input") {
    portSpec = nodeMetadata?.input_spec?.[port.name];
    if (portSpec == null && allowBase) {
      portSpec = functionSpec.base_spec.input_spec[port.name];
    }
  } else {
    throw new Error(`get port spec for port type ${port.type} not supported: ${JSON.stringify(port)}`);
  }

  return portSpec || null;
}

export function portSpec(
  port: FlowNodePort,
  functionHandlersById: Record<string, FunctionHandlerSpec>,
  allowBase = false
): RecordSpec {
  const portSpec = getPortSpec(port, functionHandlersById, allowBase);
  if (portSpec == null) {
    throw new Error(`port spec not configured for port: ${JSON.stringify(port)} (allowBase=${allowBase})`);
  }
  return portSpec;
}

export function artifactConnections(
  flow: FlowVersion,
  node: FlowNode,
  type: FlowNodePortType
): Record<string, ArtifactConnection> {
  const connections = {} as Record<string, ArtifactConnection>;
  artifactEdges(flow, { dependency: node, type }).forEach(
    (connection) => (connections[connection.connection_name] = connection)
  );
  return connections;
}

export function isValidEdge(flow: FlowVersion, edge: Omit<FlowNodeEdge, "id">): { valid: boolean; reason?: string } {
  function _invalid(reason: string) {
    return { valid: false, reason };
  }

  // edge already exists
  const existingEdge = (flow.node_edges || []).find(
    (e) =>
      e.connection_type == edge.connection_type &&
      e.dependency == edge.dependency &&
      e.dependent == edge.dependent &&
      e.connection_name_dependency == edge.connection_name_dependency &&
      e.connection_name_dependent == edge.connection_name_dependent
  );
  if (existingEdge != null) {
    return _invalid("edge already exists");
  }

  // nodes are already connected the other way
  const opposingEdge = (flow.node_edges || []).find(
    (e) => e.connection_type == edge.connection_type && e.dependency == edge.dependent && e.dependent == edge.dependency
  );
  if (opposingEdge != null) {
    return _invalid("already connected the other way");
  }

  // simple loop
  if (edge.dependency == edge.dependent) {
    return _invalid("cannot loop");
  }

  // TODO @Feature: detect indirect loop in node edge

  // TODO @Feature: detect incompatible node types

  return { valid: true };
}

export function getOrderedNodes(flow: FlowVersion): FlowNode[] {
  if (flow.nodes == null) {
    return [];
  }

  // naive order by created date
  // TODO @Feature: sort nodes properly - but in what order?
  const orderedNodes: FlowNode[] = [];
  orderedNodes.push(...flow.nodes);
  orderedNodes.sort((a, b) => a.created_at.localeCompare(b.created_at));
  return orderedNodes;
}
