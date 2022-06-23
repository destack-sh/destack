export type Artifact = {
  id: string;
  type: string;
  name: string;
  description?: string;
  versions?: string[];
  created_at: string;
};

export type ArtifactVersion = {
  id: string;
  version: string;
  name: string;
  description?: string;
  created_at: string;
};
