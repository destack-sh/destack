import { outbox } from "@destack/service/outbox";
import { copyOwner, copyScope } from "@destack/access/test";
import { Scope, type ObjectReference } from "@destack/sync";
import { TestDatabase } from "@destack/db/test";
import { AuditRecorder } from "@destack/audit";
import { AuditOutbox } from "@destack/audit/outbox";
import { Bookmark } from "@destack/service/bookmark";
import { RequestId } from "@destack/service/request";
import type { ServiceContext } from "@destack/service/server";
import { accessTables, principal, Subject } from "@destack/access";
import { device, type Device } from "@destack/account/object";
import type { DatabaseConnection, Dialect } from "@destack/db";
import type { ObjectType } from "@destack/object";
import { ObjectServer } from "@destack/object/server";
import { Journal } from "@destack/service/database";
import { settingJournal, settingTables } from "../../src/stack/index.ts";
import { servedObjects, type SettingServiceOptions } from "../../src/server/index.ts";
import type { Setting } from "../../src/setting/index.ts";
import { editor, lineNumbers, notes, release } from "./settings/index.ts";
import { alice } from "./value.ts";

/** Setting values and Alice's devices served from a migrated test database. */
export class Storage {
    /** The isolated database. */
    readonly test: TestDatabase;
    /** The database's connection. */
    readonly database: TestDatabase["database"];
    /** Alice, who owns the scopes she writes in. */
    readonly subject: Subject;
    /** The database, releases and audit serving values. */
    readonly options: SettingServiceOptions;
    /** The served setting values and devices. */
    readonly objects: ObjectServer<ReturnType<typeof servedObjects> & { device: typeof device }>;

    /** Serve values checked against a release declaring some settings. */
    constructor(test: TestDatabase, declarations: readonly Setting[]) {
        this.test = test;
        this.database = test.database;
        this.subject = Subject.parse(principal.user.reference(Scope.universe.id, alice));
        const reader = release(declarations);
        this.options = {
            database: test.database,
            release: () => reader,
            audit: (scope) => this.audit(scope),
        };
        this.objects = new ObjectServer({
            objects: { ...servedObjects(this.options.release), device },
            database: test.database,
            context: (context, scope) => context.access(scope),
            journal: new Journal(settingJournal),
            audit: this.options.audit,
        });
    }

    /** Record Alice's audit events in a scope. */
    audit(scope: string): AuditRecorder<DatabaseConnection> {
        return new AuditRecorder(
            {
                actor: { type: "subject", subject: this.subject },
                delegation: [],
                scope,
                package: notes,
                service: "setting",
            },
            new AuditOutbox(this.test.database),
        );
    }

    /** Let Alice own a scope. */
    async own(scope: ObjectReference): Promise<void> {
        await copyScope(this.database, scope);
        await copyOwner(this.database, scope, this.subject);
    }

    /** Call a method as Alice in a scope. */
    call(
        object: ObjectType,
        name: string,
        scope: string,
        input: Readonly<Record<string, unknown>>,
    ): Promise<unknown> {
        const context = {
            scope,
            requestId: RequestId.create(),
            requireCaller: () => ({ id: alice }),
            access: () => ({ subjects: [this.subject], now: Date.now(), attributes: {} }),
            bookmark: new Bookmark(),
            observed: new Bookmark(),
            signal: new AbortController().signal,
        } as unknown as ServiceContext;

        return this.objects.call(
            object,
            name,
            { [object.route.field!]: scope, requestId: RequestId.create(), ...input },
            context,
        );
    }

    /** Register a device of Alice's. */
    register(name: string): Promise<Device> {
        return this.call(device, "create", alice, { name }) as Promise<Device>;
    }

    /** Open a database of a dialect with Alice's own personal scope. */
    static async open(
        dialect: Dialect,
        declarations: readonly Setting[] = [editor, lineNumbers],
    ): Promise<Storage> {
        const tables = [...accessTables, ...settingTables, device.table, outbox];
        const test = await TestDatabase.create(dialect, tables, { isMigrated: true });
        const storage = new Storage(test, declarations);
        await storage.own(principal.user.reference(Scope.universe.id, alice));

        return storage;
    }

    /** Close the connection and remove the database. */
    close(): Promise<void> {
        return this.test.close();
    }
}
