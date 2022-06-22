export type Artifact = {
  id: string;
  type: string;
  name: string;
  description?: string;
  versions?: ArtifactVersion[];
};

export type ArtifactVersion = {
  id: string;
  version: string;
  name: string;
  description?: string;
};
