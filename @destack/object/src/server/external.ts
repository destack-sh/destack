import type { DatabaseConnection } from "@destack/db";
import { CallError } from "@destack/runtime/error";
import type { ObjectType } from "../object/object.ts";

/** The files of external objects' immutable rows, each scope's opened as a read-only database. */
export interface ExternalStorage {
    /** The external object types stored. */
    readonly objects: readonly ObjectType[];
    /** Open the database with a scope's rows of the external types. */
    readonly open: (scope: string) => Promise<DatabaseConnection>;
}

/** Find the files storing external objects. */
export const ExternalStorage = {
    /** Find the one storage holding every given external type, refusing split or missing types. */
    of(storages: readonly ExternalStorage[], names: readonly string[]): ExternalStorage {
        // find the stores holding every given type
        const found = storages.filter((storage) =>
            names.every((name) =>
                storage.objects.some((object) => object.name === name || object.key === name),
            ),
        );

        // refuse types stored apart or nowhere, and several stores for none
        const [only, ...others] = found;
        if (only === undefined || others.length > 0) {
            throw new CallError("BAD_REQUEST", {
                message: `no single store has the external objects ${names.join(", ")}`,
            });
        }

        return only;
    },
};
