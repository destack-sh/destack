import type { DatabaseConnection } from "@destack/db";
import { ServiceError } from "@destack/service/error";
import type { ObjectType } from "../object/object.ts";

/** The files of external objects' rows, each scope's opened as a read-only database. */
export interface ExternalStorage {
    /** The external object types kept. */
    readonly objects: readonly ObjectType[];
    /** Open the database with a scope's rows of the external types. */
    open(scope: string): Promise<DatabaseConnection>;
}

/** Find the files keeping external objects. */
export const ExternalStorage = {
    /** Find the one storage keeping every given external type, refusing split or missing types. */
    of(storages: readonly ExternalStorage[], names: readonly string[]): ExternalStorage {
        // find the stores keeping every given type
        const found = storages.filter((storage) =>
            names.every((name) =>
                storage.objects.some((object) => object.name === name || object.key === name),
            ),
        );

        // refuse types kept apart or nowhere, and several stores for none
        const [only, ...others] = found;
        if (only === undefined || others.length > 0) {
            throw new ServiceError("BAD_REQUEST", {
                message: `no one store keeps the external objects ${names.join(", ")}`,
            });
        }

        return only;
    },
};
