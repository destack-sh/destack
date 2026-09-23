import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { connect } from "@destack/db/turso";
import * as postgres from "@destack/db/postgres";
import { orderSchemas } from "@destack/db";
import { migrate } from "@destack/db/migration";
import { AuditRecorder } from "@destack/audit";
import { AuditOutbox } from "@destack/audit/outbox";
import { settingSchema } from "../../src/stack/index.ts";
import { SettingStore, type SettingWrite } from "../../src/database/index.ts";
import { notes } from "./settings/index.ts";
import { Subject } from "@destack/access";
import { identifier } from "@destack/schema";

/** Real settings persistence using the package's committed migrations. */
export class Storage {
    /** Isolated database directory removed after the scenario. */
    readonly directory: string;
    /** Embedded asynchronous SQL connection. */
    readonly database:
        | Awaited<ReturnType<typeof connect>>
        | Awaited<ReturnType<typeof postgres.connect>>;
    /** Isolated PostgreSQL database name, when exercising the hosted dialect. */
    readonly name?: string;
    /** Store under examination. */
    readonly store: SettingStore;
    /** Verified local attribution and transactional audit writer. */
    readonly context: SettingWrite;

    /** Bind persistence and audit to the same database. */
    constructor(directory: string, database: Storage["database"], name?: string) {
        this.directory = directory;
        this.database = database;
        this.name = name;
        this.store = new SettingStore(database);
        this.context = {
            subject: Subject.parse({
                kind: "user",
                authority: "global",
                id: "user-019f5530-8000-7000-8000-000000000003",
            }),
            audit: new AuditRecorder(
                {
                    actor: {
                        type: "user",
                        id: identifier("user").parse("user-019f5530-8000-7000-8000-000000000003"),
                    },
                    delegation: [],
                    package: notes,
                    service: "setting",
                },
                new AuditOutbox(database),
            ),
        };
    }

    /** Open a migrated database without handwritten test DDL. */
    static async open(): Promise<Storage> {
        const directory = await mkdtemp(join(tmpdir(), "destack-setting-"));
        let database: Storage["database"];
        let name: string | undefined;
        if (process.env.DESTACK_TEST_POSTGRES) {
            const administration = await postgres.connect(process.env.DESTACK_TEST_POSTGRES);
            name = `setting_${crypto.randomUUID().replaceAll("-", "")}`;
            try {
                await administration.$client.unsafe(`CREATE DATABASE "${name}"`);
            } finally {
                await administration.close();
            }
            const url = new URL(process.env.DESTACK_TEST_POSTGRES);
            url.pathname = `/${name}`;
            database = await postgres.connect(url.href, settingSchema);
        } else {
            database = await connect(join(directory, "setting.db"), settingSchema);
        }
        for (const definition of orderSchemas([settingSchema])) {
            await migrate(database, definition);
        }

        return new Storage(directory, database, name);
    }

    /** Release database handles before removing the scenario's files. */
    async close(): Promise<void> {
        await this.database.close();
        if (this.name) {
            const administration = await postgres.connect(process.env.DESTACK_TEST_POSTGRES!);
            try {
                await administration.$client.unsafe(`DROP DATABASE "${this.name}"`);
            } finally {
                await administration.close();
            }
        }
        await rm(this.directory, { recursive: true });
    }
}
