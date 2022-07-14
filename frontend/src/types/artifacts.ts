import type { FieldSpec, ModelHandlerSpec } from "@/types/spec";

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

export type DatasetMetadata = {
  handler_id: string;
  config_arguments: Record<string, any>;
  record_spec?: Record<string, FieldSpec>;
};

export type ModelMetadata = {
  handler_id: string;
  config_arguments: Record<string, any>;
  input_spec?: Record<string, FieldSpec>;
  output_spec?: Record<string, FieldSpec>;
};

export type ModelTemplate = {
  name: string;
  handler: ModelHandlerSpec;
};

export type EmptyTemplate = {
  name: string;
  empty: boolean;
};

export function splitArtifactNameVersion(artifact: string): [string, string?] {
  // assumes schema artifactName[@version]
  const parts = artifact.split("@");
  if (parts.length == 2) {
    return [parts[0], parts[1]];
  } else {
    return [parts[0], undefined];
  }
}

export function mapArtifactNameVersion(
  artifact: string,
  defaultVersion = "HEAD"
): [string, string] {
  const [name, version] = splitArtifactNameVersion(artifact);
  return [name, version || defaultVersion];
}
