import { ModuleMetadata } from "@destack/package";
import { type Connector, defineResourceKind, ResourceDeclaration } from "@destack/resource";
import { connectors } from "#git";
import { schema } from "@destack/schema";
import type { GitStorage } from "./storage.ts";

/** The specification of a Git storage: none beyond its name. */
export const GitStorageSpec = schema.object({});

/** The Git storage resource kind: repositories a machine stores. */
export const GitStorageKind = defineResourceKind("git", { spec: GitStorageSpec });
/** A declared Git storage. */
export type GitStorageDescription = schema.Infer<typeof GitStorageKind.description>;

/** A declared Git storage, opened through the connector of the runtime running the workload. */
export class GitStorageDeclaration extends ResourceDeclaration<GitStorage, GitStorageDescription> {
    /** The connectors opening Git storages on the running runtime, by provider code. */
    override get connectors(): Readonly<
        Record<string, Connector<GitStorage, GitStorageDeclaration>>
    > {
        return connectors;
    }
}

/** Declare the storage of a package's Git repositories. */
export function defineGitStorage(
    declaration: Omit<GitStorageDescription, "kind">,
    module?: ModuleMetadata,
): GitStorageDeclaration {
    const owner = ModuleMetadata.require(module, "defineGitStorage").package;
    const description = GitStorageKind.description.parse({
        ...declaration,
        kind: GitStorageKind.name,
    });

    return new GitStorageDeclaration(owner, description);
}
