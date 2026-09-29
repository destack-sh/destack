import { outbox } from "@destack/service/outbox";
import { schema } from "@destack/schema";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import type { DatabaseConnection } from "@destack/db";
import { AuditRecorder, defineAuditAction } from "../src/index.ts";
import { AuditOutbox } from "../src/outbox/index.ts";
import { AuditHistory, auditTables } from "../src/history/index.ts";
import { accountRecord, document } from "./stack/index.ts";
import { PackageId } from "@destack/package";
import { accessTables } from "@destack/access";

/** A document rename action. */
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
const TABLES = [document, accountRecord, outbox, ...auditTables, ...accessTables];

/** The storage of the audit scenarios. */
export class AuditStorage {
    /** The test database. */
    readonly test: TestDatabase;
    /** The current connection. */
    database: DatabaseConnection & { close(): Promise<void> };
    /** The pending delivery queue. */
    outbox: AuditOutbox;
    /** The accepted history. */
    history: AuditHistory;
    /** The recorder writing into the outbox. */
    recorder: AuditRecorder<DatabaseConnection>;

    /** Bind the storage to a connection. */
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
                scope: "universe",
            },
            this.outbox,
        );
    }

    /** Create migrated storage. */
    static async open(): Promise<AuditStorage> {
        const test = await TestDatabase.create(TEST_DIALECTS.at(-1)!, TABLES, { isMigrated: true });

        return new AuditStorage(test, test.database);
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
