import { mkdir, rm } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import type { Provider, ResourceRecord } from "@destack/resource";
import { GitStorageKind } from "../storage/index.ts";
import { LOCAL_PROVIDER } from "./storage.ts";

/** Store each Git storage resource as a directory of bare repositories below a host directory. */
export function localGitProvider<Object>(
    directory: string,
    object: Object,
): Provider<typeof GitStorageKind, Object> {
    // locate a resource's directory
    const path = (resource: Pick<ResourceRecord, "id">) => join(directory, resource.id);

    return {
        kind: GitStorageKind,
        code: LOCAL_PROVIDER,
        object,
        provision: {
            provision: async (resource) => {
                // create the directory once
                await mkdir(path(resource), { recursive: true });

                return { reference: pathToFileURL(path(resource)).href };
            },
            destroy: (resource) => rm(path(resource), { recursive: true, force: true }),
        },
    };
}
