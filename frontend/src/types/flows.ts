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
  description?: string;
  flow: string; // fk to Flow.name
  created_at: string;
  committed: boolean;
};

export type FlowNode = {
  id: string;
  name: string;
  flow: string; // fk to FlowVersion.name@FlowVersion.id
  created_at: string;
  function_id: string;
  config_arguments: Record<string, any>;
  connected_artifacts: FlowArtifactEdge[];
  depends_on_nodes: FlowNodeEdge[];
};

export type FlowArtifactEdgeType = "input" | "argument" | "output";
export type FlowArtifactEdge = {
  id: string;
  artifact: string; // fk to ArtifactVersion.artifact.name@ArtifactVersion.version
  connection_type: FlowArtifactEdgeType;
  connection_name: string;
};

export type FlowNodeEdgeType = "input" | "argument"; // output is unnecessary because symmetry
export type FlowNodeEdge = {
  id: string;
  dependent_nodes: string[]; // fk to FlowNode.id
  dependency_nodes: string[]; // fk to FlowNode.id
  connection_type: FlowNodeEdgeType;
  connection_name_dependent: string;
  connection_name_dependency: string;
};
