import type { DatabaseConnection } from "@destack/db";
import type { ObjectType } from "../object/object.ts";

/** The files external objects' rows live in, each scope's opened as a read-only database built from them. */
export interface ExternalStorage {
    /** The external object types kept. */
    readonly objects: readonly ObjectType[];
    /** Open the database with a scope's rows of the external types. */
    open(scope: string): Promise<DatabaseConnection>;
}
