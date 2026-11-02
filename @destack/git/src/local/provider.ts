import { join } from "node:path";
import type { Provider, ResourceRecord } from "@destack/resource";
import { GitStorageKind } from "../storage/index.ts";
import { LOCAL_PROVIDER, type LocalGitEnvironment } from "./storage.ts";

/** Store each Git storage resource as a directory of bare repositories below a host directory. */
export function localGitProvider<Object>(
    directory: string,
    object: Object,
    environment: Pick<LocalGitEnvironment, "files">,
): Provider<typeof GitStorageKind, Object> {
    // locate a resource's directory
    const path = (resource: Pick<ResourceRecord, "id">) => join(directory, resource.id);

    return {
        kind: GitStorageKind,
        code: LOCAL_PROVIDER,
        object,
        provision: {
            provision: async (resource) => {
                // create the directory once, referenced below the provider's directory
                await environment.files.mkdir(path(resource), { recursive: true });

                return { reference: resource.id };
            },
            destroy: (resource) =>
                environment.files.remove(path(resource), { recursive: true, force: true }),
        },
    };
}
