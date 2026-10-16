import { TestTelemetry } from "@destack/telemetry/test";
import { defineTable, text } from "@destack/db";
import { Clock, type Duration, schema } from "@destack/schema";
import type { JsonCondition } from "@destack/db";
import { testCallKey } from "@destack/runtime/test";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import type { DatabaseConnection } from "@destack/db";
import { CallEvent, defineAuditAction } from "../../src/index.ts";
import { AuditRecorder, Journal } from "../../src/server/index.ts";
import { call, storeTables } from "../../src/index.ts";
import { MemoryBucket } from "@destack/bucket/test";
import { DatabasePersonalKeyring, EventStore } from "@destack/event";
import { LocalKeyring } from "@destack/identity";

import { Scope } from "@destack/sync";
import { PackageId } from "@destack/package";
import { principal } from "@destack/access";

/** The documents the scenarios change. */
export const document = defineTable("document", { name: text("name").primaryKey().notNull() });

/** The accounts whose history the scenarios read. */
export const accountRecord = defineTable("account_record", {
    id: text("id").primaryKey().notNull(),
    scope: text("scope").notNull(),
});

/** The machine the fixture's own calls run as, made by its integration component. */
export const INTEGRATION = {
    caller: { subject: principal.machine.reference(Scope.universe.id, "machine-integration") },
    component: "integration",
};

/** A document rename action. */
export const documentRename = defineAuditAction(
    {
        name: "document.rename",
        target: schema.object({ type: schema.literal("document"), id: schema.string() }),
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
const TABLES = [document, accountRecord, ...storeTables, ...call.tables];

/** The bucket and keyring the call events of every connection share. */
interface CallFiles {
    /** The bucket with the segments. */
    readonly bucket: MemoryBucket;
    /** The keyring sealing personal values. */
    readonly keyring: LocalKeyring;
}

/** The storage of the audit scenarios. */
export class AuditStorage {
    /** The test database. */
    readonly test: TestDatabase;
    /** The current connection. */
    database: DatabaseConnection & { readonly close: () => Promise<void> };
    /** The event log with the call events over the current connection. */
    readonly events: EventStore;
    /** The journal recording calls, each audited one as a call event. */
    journal: Journal;
    /** The recorder writing into the journal. */
    recorder: AuditRecorder;
    /** The telemetry with the exceptions the storage captures, refused at close unless taken. */
    readonly telemetry: TestTelemetry;
    /** The bucket and keyring every connection's event log shares. */
    readonly #files: CallFiles;

    /** Bind the storage to a connection, journaling calls for a lifetime. */
    constructor(
        test: TestDatabase,
        database: AuditStorage["database"],
        files: CallFiles,
        telemetry: TestTelemetry,
        lifetime?: Duration,
    ) {
        // open the connection's event log and its journal
        this.test = test;
        this.telemetry = telemetry;
        this.database = database;
        this.#files = files;
        this.events = AuditStorage.#events(database, files);
        this.journal = new Journal(database, testCallKey, {
            events: this.events,
            clock: Clock.system,
            ...(lifetime === undefined ? {} : { lifetime }),
        });

        // record the fixture's own calls into the journal
        this.recorder = new AuditRecorder(
            {
                ...INTEGRATION,
                package: documentRename.package,
                scope: "universe",
            },
            this.journal,
        );
    }

    /** Create migrated storage, journaling calls for a lifetime. */
    static async open(lifetime?: Duration): Promise<AuditStorage> {
        const dialect = TEST_DIALECTS.at(-1);
        if (dialect === undefined) {
            throw new TypeError("no test dialect");
        }
        const test = await TestDatabase.create(dialect, TABLES, { isMigrated: true });
        const files = {
            bucket: new MemoryBucket(Clock.system),
            keyring: await LocalKeyring.read(LocalKeyring.generate()),
        };

        return new AuditStorage(test, test.database, files, TestTelemetry.register(), lifetime);
    }

    /** Open the call events over a connection. */
    static #events(database: DatabaseConnection, files: CallFiles): EventStore {
        const kind = call.eventKind();

        return new EventStore({
            database,
            kinds: [kind],
            files: () => files.bucket,
            personal: new DatabasePersonalKeyring(database, files.keyring),
            clock: Clock.system,
        });
    }

    /** Reopen the storage on a new connection. */
    async reopen(): Promise<AuditStorage> {
        await this.database.close();

        const database = await this.test.connect(TABLES);

        return new AuditStorage(this.test, database, this.#files, this.telemetry);
    }

    /** Open another connection. */
    connection(): Promise<AuditStorage["database"]> {
        return this.test.connect(TABLES);
    }

    /** Read a scope's calls in start order, narrowed by a condition over the call type's keys. */
    async calls(scope: string, where?: JsonCondition): Promise<CallEvent[]> {
        const page = await this.events.query(
            call.eventKind(),
            { scopes: [scope], ...(where === undefined ? {} : { where }) },
            { limit: 100 },
        );

        return page.events.map((event) => CallEvent.parse(event.data));
    }

    /** Release the connection and remove the databases, refusing exceptions no test took. */
    async close(): Promise<void> {
        await this.database.close();
        await this.test.close();
        this.telemetry[Symbol.dispose]();
    }
}

/** The values of the rename scenarios. */
export const rename = {
    target: { type: "document" as const, id: "one" },
    details: { name: "renamed" },
};
