import type { ArtifactView, ArtifactViewData } from "@/types/artifacts";
import { ref, type Ref } from "vue";

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
  metadata?: Record<string, any>;
};

export type FlowArtifactEdgeType = "input" | "argument" | "output";
export type FlowArtifactEdge = {
  id: string;
  dependent: string; // fk to FlowNode.id
  dependency: string; // fk to Artifact.name@ArtifactVersion.version
  connection_type: FlowArtifactEdgeType;
  connection_name: string;
  view?: ArtifactView;
  view_inline?: ArtifactViewData;
};

// like FlowArtifactEdge but untethered
export type ArtifactConnection = Omit<FlowArtifactEdge, "id" | "dependent">;

export type FlowNodeEdgeType = "input" | "argument"; // output is unnecessary because symmetry
export type FlowNodeEdge = {
  id: string;
  dependent: string; // fk to FlowNode.id
  dependency: string; // fk to FlowNode.id
  connection_type: FlowNodeEdgeType;
  connection_name_dependent: string;
  connection_name_dependency: string;
};

// like FlowNodeEdge but untethered
export type FlowNodeConnection = Omit<FlowNodeEdge, "id" | "dependent">;

export type FlowNodePortType = "input" | "output" | "argument";
export type FlowNodePort = {
  id: string;
  name: string;
  node: FlowNode;
  type: FlowNodePortType;
};

export type FlowNodeExecutionArgument = {
  type: "input" | "argument";
  node: string; // fk to FlowNode.id;
  name: string;
  artifact?: string; // fk to Artifact.name@ArtifactVersion.version
  records?: any;
};

export type FlowRuntimeData = {
  // inputs by node name
  inputs?: Record<string, Record<string, any>>;
};

export type FlowRuntimeValidation = "off" | "lazy" | "full";

export type FlowExecutionOptions = {
  blocking: boolean;
  validate: FlowRuntimeValidation;
};

export type FlowExecutionPlan = {
  arguments?: Record<string, FlowNodeExecutionArgument[]>; // key is FlowNode.id
  options?: FlowExecutionOptions;
};

export type FlowInteractionData = {
  selectedNode: FlowNode | null;
};

export function makeInteractionData(): FlowInteractionData {
  return { selectedNode: null };
}
