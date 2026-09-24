import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { schema } from "@destack/schema";
import { connect } from "@destack/db/turso";
import * as postgres from "@destack/db/postgres";
import { migrate } from "@destack/db/migration";
import type { DatabaseConnection } from "@destack/db";
import { AuditRecorder, defineAuditAction } from "../src/index.ts";
import { AuditOutbox, auditOutboxSchema } from "../src/outbox/index.ts";
import { AuditHistory, auditSchema } from "../src/history/index.ts";
import { document, testSchema } from "./stack/index.ts";
import { PackageId } from "@destack/package";

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

/** Persistent local storage shared by recording and delivery scenarios. */
export class AuditStorage {
    /** Temporary directory containing the real embedded database. */
    readonly directory: string;
    /** Connection replaced when a scenario simulates a restart. */
    database: Awaited<ReturnType<typeof connect>> | Awaited<ReturnType<typeof postgres.connect>>;
    /** Isolated PostgreSQL database URL, when explicitly requested by the runner. */
    readonly url?: string;
    /** The pending delivery queue. */
    outbox: AuditOutbox;
    /** The accepted history. */
    history: AuditHistory;
    /** The recorder writing into the outbox. */
    recorder: AuditRecorder<DatabaseConnection>;

    /** Bind the same authority after each connection restart. */
    constructor(directory: string, database: AuditStorage["database"], url?: string) {
        this.directory = directory;
        this.database = database;
        this.url = url;
        this.outbox = new AuditOutbox(database);
        this.history = new AuditHistory(database);
        this.recorder = new AuditRecorder(
            {
                actor: { type: "system", name: "integration" },
                delegation: [],
                package: renameDocument.package,
                service: "document",
            },
            this.outbox,
        );
    }

    /** Create real storage using the package's committed migrations. */
    static async open(): Promise<AuditStorage> {
        const directory = await mkdtemp(join(tmpdir(), "destack-audit-"));
        const tables = [
            document,
            ...Object.values(auditOutboxSchema.tables),
            ...Object.values(auditSchema.tables),
        ];

        // explicitly select PostgreSQL and allocate an isolated database for each scenario
        let url: string | undefined;
        if (process.env.DESTACK_TEST_POSTGRES) {
            const administration = await postgres.connect(process.env.DESTACK_TEST_POSTGRES);
            const name = `audit_${crypto.randomUUID().replaceAll("-", "")}`;
            try {
                await administration.$client.unsafe(`CREATE DATABASE "${name}"`);
                const address = new URL(process.env.DESTACK_TEST_POSTGRES);
                address.pathname = `/${name}`;
                url = address.href;
            } finally {
                await administration.close();
            }
        }

        // run the same committed migrations and persistence scenarios on the selected engine
        const database = url
            ? await postgres.connect(url, tables)
            : await connect(join(directory, "audit.db"), tables);
        await migrate(database, auditOutboxSchema);
        await migrate(database, auditSchema);
        await migrate(database, testSchema);

        return new AuditStorage(directory, database, url);
    }

    /** Reopen persistent state without retaining delivery objects. */
    async reopen(): Promise<AuditStorage> {
        await this.database.close();
        const tables = [
            document,
            ...Object.values(auditOutboxSchema.tables),
            ...Object.values(auditSchema.tables),
        ];
        const database = this.url
            ? await postgres.connect(this.url, tables)
            : await connect(join(this.directory, "audit.db"), tables);

        return new AuditStorage(this.directory, database, this.url);
    }

    /** Open an independent connection to exercise database locking. */
    async connection(): Promise<AuditStorage["database"]> {
        const tables = [
            ...Object.values(auditOutboxSchema.tables),
            ...Object.values(auditSchema.tables),
        ];

        return this.url
            ? postgres.connect(this.url, tables)
            : connect(join(this.directory, "audit.db"), tables);
    }

    /** Release the connection and temporary database. */
    async close(): Promise<void> {
        await this.database.close();
        if (this.url) {
            const administration = await postgres.connect(process.env.DESTACK_TEST_POSTGRES!);
            try {
                const name = new URL(this.url).pathname.slice(1);
                await administration.$client.unsafe(`DROP DATABASE "${name}"`);
            } finally {
                await administration.close();
            }
        }
        await rm(this.directory, { recursive: true });
    }
}

/** The values used by the rename scenarios. */
export const rename = {
    targets: { document: { type: "document" as const, id: "one" } },
    details: { name: "renamed" },
};
