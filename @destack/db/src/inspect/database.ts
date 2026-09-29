import { defineSchema, type schema } from "@destack/schema";
import { DatabaseKind, type Database } from "../declare/database.ts";
import { DatabaseState } from "../migration/state.ts";

/** A declared database and its required tables, as the manifest records it. */
export const DatabaseDeclaration = defineSchema(
    DatabaseKind.description.extend(DatabaseState.shape),
);
/** A declared database and its required tables, as the manifest records it. */
export type DatabaseDeclaration = schema.Infer<typeof DatabaseDeclaration>;

/** Describe a declared database. */
export function describeDatabase(database: Database): DatabaseDeclaration {
    return DatabaseDeclaration.parse({
        name: database.name,
        kind: database.kind,
        spec: database.spec,
        ...database.state(),
    });
}
