import type { FieldValuePrimitive, RecordSpec } from "@/types/spec";

export type Artifact = {
  id: string;
  type: string;
  name: string;
  description?: string;
  versions?: string[]; // fk to ArtifactVersion.version
  latest_version?: ArtifactVersion;
  created_at: string;
};

export type ArtifactVersion = {
  id: string;
  artifact: string; // fk to Artifact.name
  version: string;
  parents: string[]; // fk to ArtifactVersion.version
  name?: string;
  description?: string;
  created_at: string;
  storage_uri?: string;
  metadata: Record<string, any>;
  committed: boolean;
};

export type ArtifactViewData = Record<string, any>;
export type DatasetIndexSliceView = ArtifactViewData & {
  start: number;
  end: number;
};

export type ArtifactView = {
  id: string;
  type: string;
  artifact: string; // fk to Artifact.name
  compatible_versions?: string[]; // fk to ArtifactVersion.version
  data: ArtifactViewData;
};

export type DatasetMetadata = {
  handler_id: string;
  config_arguments: Record<string, any>;
  record_spec?: RecordSpec;
};

export type ModelMetadata = {
  handler_id: string;
  config_arguments: Record<string, FieldValuePrimitive>;
  input_spec?: RecordSpec;
  output_spec?: RecordSpec;
};

export type ModelSpecEditable = Pick<ModelMetadata, "input_spec" | "output_spec">;
