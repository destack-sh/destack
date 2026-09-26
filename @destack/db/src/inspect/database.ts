import { defineSchema, type schema } from "@destack/schema";
import { DatabaseDescription, type Database } from "../declare/database.ts";
import { DatabaseState } from "../migration/state.ts";

/** A declared database as the package manifest records it: its resource and the tables it requires. */
export const DatabaseDeclaration = defineSchema(DatabaseDescription.extend(DatabaseState.shape));
/** A declared database as the package manifest records it: its resource and the tables it requires. */
export type DatabaseDeclaration = schema.Infer<typeof DatabaseDeclaration>;

/** Describe a declared database and its tables for the package manifest. */
export function describeDatabase(database: Database): DatabaseDeclaration {
    return DatabaseDeclaration.parse({
        name: database.name,
        kind: database.kind,
        version: database.version,
        spec: database.spec,
        ...database.state(),
    });
}
