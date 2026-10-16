import { type Provider, ResourceRecord } from "@destack/resource";
import type { Fetch } from "@destack/runtime";
import { GitStorageKind } from "../storage/index.ts";
import { ARTIFACTS_PROVIDER, type ArtifactsNamespace, ArtifactsStorage } from "./storage.ts";

/** Store each Git storage resource's repositories in a namespace, named below the resource. */
export function artifactsProvider<Object>(
    namespace: ArtifactsNamespace,
    object: Object,
    fetch: Fetch,
): Provider<typeof GitStorageKind, Object> {
    return {
        kind: GitStorageKind,
        code: ARTIFACTS_PROVIDER,
        object,
        provision: {
            // name the resource's repositories below its identifier
            provision: async (resource) => ({ reference: resource.id }),
            // delete the resource's repositories
            destroy: (resource) =>
                new ArtifactsStorage(
                    namespace,
                    ResourceRecord.reference(resource),
                    fetch,
                ).deleteAll(),
        },
    };
}
