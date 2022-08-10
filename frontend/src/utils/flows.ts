import type { FlowNode, FlowNodeEdge, FlowNodePortType, FlowVersion, FunctionHandlerSpec } from "@/types";

export function flowPorts(
  flow: FlowVersion,
  functionHandlersByName: Record<string, FunctionHandlerSpec>,
  ofType?: FlowNodePortType
) {
  if (flow.nodes == null) {
    return [];
  }
  return flow.nodes.flatMap((node) => nodePorts(node, functionHandlersByName, ofType));
}

export function nodePorts(
  node: FlowNode,
  functionHandlersByName: Record<string, FunctionHandlerSpec>,
  ofType?: FlowNodePortType
) {
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
