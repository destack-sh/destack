/** Committed SQL that advances one named database schema. */
export interface Migration {
    /** The ordered migration directory name. */
    readonly name: string;
    /** The SHA-256 checksum of the SQL and optional TypeScript transformation. */
    readonly checksum: string;
    /** SQL statements in execution order. */
    readonly statements: readonly string[];
    /** Apply committed row transformations after SQL, inside the migration transaction. */
    readonly apply?: (database: DatabaseConnection) => Promise<void>;
}
import type { DatabaseConnection } from "../database/connection.ts";
