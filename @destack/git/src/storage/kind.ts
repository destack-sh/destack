import { defineKind, ModuleMetadata } from "@destack/package";
import { ResourceDeclaration } from "@destack/resource";
import { connectors } from "./connector.ts";
import { schema } from "@destack/schema";
import type { GitStorage } from "./storage.ts";

/** The specification of a Git storage: none beyond its name. */
export const GitStorageSpec = schema.object({});

/** The Git storage resource kind: repositories a machine stores. */
export const GitStorageKind = defineKind({ name: "git", provisioned: { spec: GitStorageSpec } });
/** A declared Git storage. */
export type GitStorageDescription = schema.Infer<typeof GitStorageKind.description>;

/** Declare the storage of a package's Git repositories. */
export function defineGitStorage(
    options: { readonly name: string },
    module?: ModuleMetadata,
): ResourceDeclaration<GitStorage, typeof GitStorageSpec> {
    const owner = ModuleMetadata.require(module, "defineGitStorage").package;

    return new ResourceDeclaration<GitStorage, typeof GitStorageSpec>(owner, GitStorageKind, {
        name: options.name,
        spec: {},
        connectors,
    });
}
