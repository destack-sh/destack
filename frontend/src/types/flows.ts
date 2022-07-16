export type Flow = {
  id: string;
  name: string;
  description?: string;
  versions: string[]; // fk to FlowVersion.id
  latest_version?: FlowVersion;
  created_at: string;
};

export type FlowVersion = {
  id: string;
  name: string;
  version: string;
  parents: string[]; // fk to FlowVersion.version
  description?: string;
  flow: string; // fk to Flow.name
  created_at: string;
  committed: boolean;
  // read-only flow
  nodes?: FlowNode[];
  node_edges?: FlowNodeEdge[];
  artifact_edges?: FlowArtifactEdge[];
};

export type FlowNode = {
  id: string;
  name: string;
  flow: string; // fk to FlowVersion.name@FlowVersion.id
  created_at: string;
  function_id: string;
  config_arguments?: Record<string, any>;
};

export type FlowArtifactEdgeType = "input" | "argument" | "output";
export type FlowArtifactEdge = {
  id: string;
  dependent: string; // fk to FlowNode.id
  dependency: string; // fk to ArtifactVersion.artifact.name@ArtifactVersion.version
  connection_type: FlowArtifactEdgeType;
  connection_name: string;
};

export type FlowNodeEdgeType = "input" | "argument"; // output is unnecessary because symmetry
export type FlowNodeEdge = {
  id: string;
  dependent_node: string; // fk to FlowNode.id
  dependency_node: string; // fk to FlowNode.id
  connection_type: FlowNodeEdgeType;
  connection_name_dependent: string;
  connection_name_dependency: string;
};
