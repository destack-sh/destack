import { defineGitStorage } from "@destack/git";

/** The storage of the platform repositories the forge hosts. */
export const repositoryStorage = defineGitStorage({ name: "repositories", spec: {} });
