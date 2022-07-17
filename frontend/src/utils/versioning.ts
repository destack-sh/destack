import type { ArtifactVersion, FlowVersion } from "@/types";

export function splitNameVersion(artifact: string): [string, string?] {
  // assumes schema artifactName[@version]
  const parts = artifact.split("@");
  if (parts.length == 2) {
    return [parts[0], parts[1]];
  } else {
    return [parts[0], undefined];
  }
}

export function toNameVersion(obj: ArtifactVersion | FlowVersion) {
  const name = (obj as ArtifactVersion).artifact || (obj as FlowVersion).flow;
  return `${name}@${obj.version}`;
}

export function mapNameVersion(artifact: string, defaultVersion = "HEAD"): [string, string] {
  const [name, version] = splitNameVersion(artifact);
  return [name, version || defaultVersion];
}
