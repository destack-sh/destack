import { type Duration, schema } from "@destack/schema";
import type { JsonCondition } from "@destack/db";
import { testCallKey } from "@destack/service/test";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import type { DatabaseConnection } from "@destack/db";
import { AuditCall, defineAuditAction } from "../src/index.ts";
import { AuditRecorder, Journal } from "../src/server/index.ts";
import { AuditHistory } from "../src/history/index.ts";
import { journal } from "../src/stack/index.ts";
import { call } from "../src/record/index.ts";
import { EventFixture } from "@destack/event/test";
import { Snapshot } from "@destack/db";
import { Scope } from "@destack/sync";
import { accountRecord, document } from "./stack/index.ts";
import { PackageId } from "@destack/package";
import { accessTables, principal } from "@destack/access";

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
const TABLES = [document, accountRecord, journal, ...accessTables];

/** The storage of the audit scenarios. */
export class AuditStorage {
    /** The test database. */
    readonly test: TestDatabase;
    /** The current connection. */
    database: DatabaseConnection & { close(): Promise<void> };
    /** The journal recording calls and delivering them. */
    journal: Journal;
    /** The call events the history keeps, and the connection reading the scopes they route to. */
    readonly #routing: { readonly events: EventFixture; readonly scopes: AuditStorage["database"] };
    /** The accepted history. */
    history: AuditHistory;
    /** The recorder writing into the journal. */
    recorder: AuditRecorder<DatabaseConnection>;

    /** Bind the storage to a connection, keeping delivered calls for a lifetime. */
    constructor(
        test: TestDatabase,
        database: AuditStorage["database"],
        routing: { readonly events: EventFixture; readonly scopes: AuditStorage["database"] },
        lifetime?: Duration,
    ) {
        this.test = test;
        this.database = database;
        this.#routing = routing;
        this.journal = new Journal(
            database,
            testCallKey,
            lifetime === undefined ? {} : { lifetime },
        );
        this.history = new AuditHistory(routing.events.store);
        this.recorder = new AuditRecorder(
            {
                ...INTEGRATION,
                package: documentRename.package,
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

        // route each scope's calls to the scopes enclosing it as the copies keep them, delivering into the same events
        const scopes = await test.connect(TABLES);
        const events = await EventFixture.open(dialect, [call], {
            targets: async (_route, scope) => {
                const chains = await Scope.chains(Snapshot.live(scopes), [scope]);

                return (chains.get(scope) ?? []).slice(1).map((link) => link.object.id);
            },
            deliver: (_kind, delivered) => events.store.receive(call.key, delivered),
        });

        return new AuditStorage(test, test.database, { events, scopes }, lifetime);
    }

    /** Reopen the storage on a new connection. */
    async reopen(): Promise<AuditStorage> {
        await this.database.close();

        return new AuditStorage(this.test, await this.test.connect(TABLES), this.#routing);
    }

    /** Open another connection. */
    connection(): Promise<AuditStorage["database"]> {
        return this.test.connect(TABLES);
    }

    /** Read a scope's calls in start order, its own or those within it too, narrowed by a condition over the call kind's keys. */
    async calls(
        scope: string,
        options: { readonly within?: boolean; readonly where?: JsonCondition } = {},
    ): Promise<AuditCall[]> {
        const own = options.within === true ? {} : { source: scope };
        const page = await this.#routing.events.store.query(
            call,
            { scope, where: { AND: [own, options.where ?? {}] } },
            { limit: 100 },
        );

        return page.events.map((event) => AuditCall.parse(event.data));
    }

    /** Deliver the history's routed copies, as the machine's controllers do. */
    async route(): Promise<void> {
        await this.#routing.events.settle();
    }

    /** Release the connection and remove the databases. */
    async close(): Promise<void> {
        await this.database.close();
        await this.#routing.scopes.close();
        await this.test.close();
        await this.#routing.events[Symbol.asyncDispose]();
    }
}

/** The values of the rename scenarios. */
export const rename = {
    target: { type: "document" as const, id: "one" },
    details: { name: "renamed" },
};
