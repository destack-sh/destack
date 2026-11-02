import type { ResourceDeclaration } from "@destack/resource";
import { type GitStorageDescription, GitStorageKind } from "../storage/index.ts";

/** Describe a declared Git storage for the package manifest. */
export function describeGitStorage(
    storage: ResourceDeclaration<unknown, GitStorageDescription>,
): GitStorageDescription {
    return GitStorageKind.description.parse({
        name: storage.name,
        kind: storage.kind,
        spec: storage.spec,
    });
}
