import type { FieldValuePrimitive, RecordSpec } from "@/types/spec";
import type { Taggable } from "@/types/tags";

export type Artifact = Taggable & {
  id: string;
  type: string;
  name: string;
  description?: string;
  versions?: string[]; // fk to ArtifactVersion.version
  head?: ArtifactVersion;
  created_at: string;
};

export type ArtifactVersion = Taggable & {
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

export type ArtifactView = Taggable & {
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
  metadata_spec?: RecordSpec;
};

export type DatasetRecord = {
  data: Record<string, any>;
  metadata?: Record<string, any>;
};

export type ModelMetadata = {
  handler_id: string;
  config_arguments: Record<string, FieldValuePrimitive>;
  input_spec?: RecordSpec;
  output_spec?: RecordSpec;
};

export type ModelSpecEditable = Pick<ModelMetadata, "input_spec" | "output_spec">;
