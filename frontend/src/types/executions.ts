import type { ArtifactView, ArtifactViewData } from "@/types/artifacts";
import type { LimitPaginatedResult } from "@/types/utils";

export type ExecutionStatus =
  | "created"
  | "scheduled"
  | "queued"
  | "running"
  | "aborting"
  | "aborted"
  | "failed"
  | "completed";

export function isTerminal(status: ExecutionStatus) {
  return ["aborted", "failed", "completed"].includes(status);
}

export type Execution = {
  id: string;
  type: "flow" | "flow_node" | "model" | "job";
  created_at: string;
  started_at?: string;
  updated_at: string;
  terminated_at?: string;
  status: ExecutionStatus;
  metadata: Record<string, any>;
  parent: string; // fk to Execution.parent
  children: Execution[];
  flow: string; // fk to FlowVersion.id
  flow_node: string; // fk to FlowNode.id
  model: string; // fk as ModelVersion.name@ModelVersion.version
  connected_artifacts: Array<ExecutionArtifactConnection>;
};

export type ExecutionArtifactConnectionType = "input" | "output" | "argument";

export type ExecutionArtifactConnection = {
  execution: string; // fk to Execution.id
  artifact: string; // fk to ArtifactVersion.artifact.name@ArtifactVersion.version
  connection_type: ExecutionArtifactConnectionType;
  connection_name: string;
  dataset_preview?: LimitPaginatedResult<Record<string, any>>;
  view?: ArtifactView;
  view_inline?: ArtifactViewData;
};

export function getAllConnectedArtifacts(execution: Execution): ExecutionArtifactConnection[] {
  const connectedArtifacts = [...execution.connected_artifacts];
  if (execution.children != null) {
    for (const childExecution of execution.children) {
      connectedArtifacts.push(...childExecution.connected_artifacts);
    }
  }
  return connectedArtifacts;
}

export function getAllConnectedDatasets(execution: Execution): ExecutionArtifactConnection[] {
  return getAllConnectedArtifacts(execution).filter((connection) => connection.dataset_preview != null);
}
