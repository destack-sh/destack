import type { DatabaseConnection } from "@destack/db";
import { ObjectError } from "../error/error.ts";

import type { ObjectType } from "../object/object.ts";

import type {} from "@destack/package/import-meta";

/** The files of external objects' immutable rows, each scope's opened as a read-only database. */
export interface ExternalStorage {
    /** The external object types stored. */
    readonly objects: readonly ObjectType[];
    /** Open the database with a scope's rows of the external types. */
    readonly open: (scope: string) => Promise<DatabaseConnection>;
}

/** Find the files storing external objects. */
export const ExternalStorage = {
    /** Find the one storage with every given external type, refusing split or missing types. */
    of(storages: readonly ExternalStorage[], names: readonly string[]): ExternalStorage {
        // find the stores with every given type
        const found = storages.filter((storage) =>
            names.every((name) =>
                storage.objects.some((object) => object.name === name || object.key === name),
            ),
        );

        // refuse types stored apart or nowhere, and several stores for none
        const [only, ...others] = found;
        if (only === undefined || others.length > 0) {
            throw new ObjectError("EXTERNAL_STORES_SPLIT", {
                message: `no single store has the external objects ${names.join(", ")}`,
            });
        }

        return only;
    },
};
