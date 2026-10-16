import { defineBucket } from "@destack/bucket";
import { defineGitStorage } from "../storage/index.ts";

/** The bucket of the builds and npm archives of published releases. */
export const packageBucket = defineBucket({ name: "packages", spec: { write: "package" } });

/** The storage of the platform repositories the forge hosts. */
export const repositoryStorage = defineGitStorage({ name: "repositories", spec: {} });
