import type { DatabaseSchema } from "../schema/schema.ts";
import { DatabaseDescription, type Database } from "../declare/database.ts";
import { DatabaseSchemaDescription } from "./schema.ts";
import { describeSchema } from "./describe.ts";

/** A database schema described in every supported dialect. */
export interface DatabaseSchemaDialects {
    /** The schema name. */
    readonly name: string;
    /** The SQLite description. */
    readonly sqlite: DatabaseSchemaDescription;
    /** The PostgreSQL description. */
    readonly postgresql: DatabaseSchemaDescription;
}

/** Describe a declared database for the package manifest. */
export function describeDatabase(database: Database): DatabaseDescription {
    return DatabaseDescription.parse({
        name: database.name,
        kind: database.kind,
        version: database.version,
        spec: database.spec,
    });
}

/** Describe a database schema in every supported dialect. */
export function describeDatabaseSchema(database: DatabaseSchema): DatabaseSchemaDialects {
    return {
        name: database.name,
        sqlite: DatabaseSchemaDescription.parse(describeSchema(database, "sqlite")),
        postgresql: DatabaseSchemaDescription.parse(describeSchema(database, "postgresql")),
    };
}
