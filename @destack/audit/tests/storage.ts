import { schema } from "@destack/schema";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import type { DatabaseConnection } from "@destack/db";
import { AuditRecorder, defineAuditAction } from "../src/index.ts";
import { AuditOutbox, auditOutboxTables } from "../src/outbox/index.ts";
import { AuditHistory, auditTables } from "../src/history/index.ts";
import { accountRecord, document } from "./stack/index.ts";
import { PackageId } from "@destack/package";
import { ACCESS_TABLES } from "@destack/access";

/** A declared change with explicit historical details. */
export const renameDocument = defineAuditAction(
    {
        name: "Document.rename",
        version: 1,
        targets: schema.object({
            document: schema.object({ type: schema.literal("document"), id: schema.string() }),
        }),
        details: schema.object({ name: schema.string() }),
    },
    {
        package: {
            id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000004"),
            name: "@example/document",
            version: "1.0.0",
        },
    },
);

/** The tables the audit scenarios use. */
const TABLES = [document, accountRecord, ...auditOutboxTables, ...auditTables, ...ACCESS_TABLES];

/** Persistent storage shared by recording and delivery scenarios. */
export class AuditStorage {
    /** The isolated database, which outlives restarts of its connection. */
    readonly test: TestDatabase;
    /** Connection replaced when a scenario simulates a restart. */
    database: DatabaseConnection & { close(): Promise<void> };
    /** The pending delivery queue. */
    outbox: AuditOutbox;
    /** The accepted history. */
    history: AuditHistory;
    /** The recorder writing into the outbox. */
    recorder: AuditRecorder<DatabaseConnection>;

    /** Bind the same authority after each connection restart. */
    constructor(test: TestDatabase, database: AuditStorage["database"]) {
        this.test = test;
        this.database = database;
        this.outbox = new AuditOutbox(database);
        this.history = new AuditHistory(database);
        this.recorder = new AuditRecorder(
            {
                actor: { type: "system", name: "integration" },
                delegation: [],
                package: renameDocument.package,
                service: "document",
                scope: "global",
            },
            this.outbox,
        );
    }

    /** Create migrated storage in a file, or a database of the configured dialect. */
    static async open(): Promise<AuditStorage> {
        const test = await TestDatabase.create(TEST_DIALECTS.at(-1)!, TABLES, { isMigrated: true });

        return new AuditStorage(test, test.database);
    }

    /** Reopen persistent state without retaining delivery objects. */
    async reopen(): Promise<AuditStorage> {
        await this.database.close();

        return new AuditStorage(this.test, await this.test.connect(TABLES));
    }

    /** Open an independent connection to exercise database locking. */
    connection(): Promise<AuditStorage["database"]> {
        return this.test.connect(TABLES);
    }

    /** Release the connection and remove the database. */
    async close(): Promise<void> {
        await this.database.close();
        await this.test.close();
    }
}

/** The values used by the rename scenarios. */
export const rename = {
    targets: { document: { type: "document" as const, id: "one" } },
    details: { name: "renamed" },
};
