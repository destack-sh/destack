import { type Duration, schema } from "@destack/schema";
import { testCallKey } from "@destack/service/test";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import type { DatabaseConnection } from "@destack/db";
import { defineAuditAction } from "../src/index.ts";
import { AuditRecorder, Journal } from "../src/server/index.ts";
import { AuditHistory } from "../src/history/index.ts";
import { auditTables, journal } from "../src/stack/index.ts";
import { accountRecord, document } from "./stack/index.ts";
import { PackageId } from "@destack/package";
import { accessTables } from "@destack/access";

/** A document rename action. */
export const renameDocument = defineAuditAction(
    {
        name: "document.rename",
        targets: schema.object({
            document: schema.object({ type: schema.literal("document"), id: schema.string() }),
        }),
        details: schema.object({ name: schema.string() }),
    },
    {
        package: {
            id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000004"),
            name: "@example/document",
            version: "2026.9.0",
        },
    },
);

/** The tables the audit scenarios use. */
const TABLES = [document, accountRecord, journal, ...auditTables, ...accessTables];

/** The storage of the audit scenarios. */
export class AuditStorage {
    /** The test database. */
    readonly test: TestDatabase;
    /** The current connection. */
    database: DatabaseConnection & { close(): Promise<void> };
    /** The journal recording calls and delivering them. */
    journal: Journal;
    /** The accepted history. */
    history: AuditHistory;
    /** The recorder writing into the journal. */
    recorder: AuditRecorder<DatabaseConnection>;

    /** Bind the storage to a connection, keeping delivered calls for a lifetime. */
    constructor(test: TestDatabase, database: AuditStorage["database"], lifetime?: Duration) {
        this.test = test;
        this.database = database;
        this.journal = new Journal(
            database,
            testCallKey,
            lifetime === undefined ? {} : { lifetime },
        );
        this.history = new AuditHistory(database);
        this.recorder = new AuditRecorder(
            {
                caller: { type: "system", name: "integration" },
                package: renameDocument.package,
                service: "document",
                scope: "universe",
            },
            this.journal,
        );
    }

    /** Create migrated storage, keeping delivered calls for a lifetime. */
    static async open(lifetime?: Duration): Promise<AuditStorage> {
        const dialect = TEST_DIALECTS.at(-1);
        if (dialect === undefined) {
            throw new TypeError("no test dialect");
        }
        const test = await TestDatabase.create(dialect, TABLES, { isMigrated: true });

        return new AuditStorage(test, test.database, lifetime);
    }

    /** Reopen the storage on a new connection. */
    async reopen(): Promise<AuditStorage> {
        await this.database.close();

        return new AuditStorage(this.test, await this.test.connect(TABLES));
    }

    /** Open another connection. */
    connection(): Promise<AuditStorage["database"]> {
        return this.test.connect(TABLES);
    }

    /** Release the connection and remove the database. */
    async close(): Promise<void> {
        await this.database.close();
        await this.test.close();
    }
}

/** The values of the rename scenarios. */
export const rename = {
    targets: { document: { type: "document" as const, id: "one" } },
    details: { name: "renamed" },
};
