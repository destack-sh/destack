import type { ResourceDeclaration } from "@destack/resource";
import {
    type GitStorageDescription,
    GitStorageKind,
    type GitStorageSpec,
} from "../storage/index.ts";

/** Describe a declared Git storage for the package manifest. */
export function describeGitStorage(
    storage: Pick<ResourceDeclaration<unknown, typeof GitStorageSpec>, "name" | "spec">,
): GitStorageDescription {
    return GitStorageKind.description.parse({
        name: storage.name,
        spec: storage.spec,
    });
}
