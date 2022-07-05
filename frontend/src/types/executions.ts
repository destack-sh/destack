export type ExecutionState =
  | "created"
  | "scheduled"
  | "queued"
  | "running"
  | "aborting"
  | "aborted"
  | "failed"
  | "completed";

export type Execution = {
  id: string;
  type: string;
  created_at: string;
  updated_at: string;
  terminated_at: string;
  state: ExecutionState;
  metadata: Record<string, any>;
  parent: string; // fk to Execution.parent
  flow: string; // fk to FlowVersion.id
  flow_node: string; // fk to FlowNode.id
  model: string; // fk as ModelVersion.name@ModelVersion.version
  connected_artifacts: Array<ExecutionArtifactConnection>;
};

export type ExecutionArtifactConnectionType = "input" | "output" | "argument";

export type ExecutionArtifactConnection = {
  execution: string; // fk to Execution.id
  artifact: string; // fk to ArtifactVersion.name@ArtifactVersion.version
  connection_type: ExecutionArtifactConnectionType;
  connection_name: string;
};

export function splitArtifactVersion(artifact: string) {
  // assumes schema artifactName@version
  const parts = artifact.split("@");
  return parts[0], parts[1];
}
