import { Package } from "@destack/package";
import { Clock } from "@destack/schema";
import { outbox } from "@destack/runtime/outbox";
import { AccessFixture } from "@destack/access/test";
import { Scope, type ObjectReference, Subject } from "@destack/sync";
import { TestDatabase } from "@destack/db/test";
import { RequestId } from "@destack/runtime/request";
import { accessTables, principal, universe } from "@destack/access";
import { space } from "./space.ts";

import type { Dialect } from "@destack/db";
import type { JsonObject } from "@destack/schema";
import {
    Call,
    type CallableName,
    type CallOutput,
    type ObjectCallInput,
    type ObjectType,
} from "@destack/object";
import { ObjectStore } from "@destack/object/server";
import { testEvents } from "@destack/object/test";

import { settingTables } from "../../src/stack/index.ts";
import { implementSetting } from "../../src/server/index.ts";
import settingPackage from "../../src/package.ts";
import type { BuildReader } from "@destack/package/manifest";
import { type Setting, SettingCatalog } from "../../src/setting/index.ts";
import { editor, lineNumbers, release } from "./setting/index.ts";
import { alice } from "./value.ts";
import { subjectContext, testCallKey } from "@destack/runtime/test";

/** Setting values served from a migrated test database. */
export class Storage {
    /** The isolated database. */
    readonly test: TestDatabase;
    /** The database's connection. */
    readonly database: TestDatabase["database"];
    /** Alice, who owns the scopes she writes in. */
    readonly subject: Subject;
    /** The fixture release declaring the settings. */
    readonly reader: Promise<BuildReader>;
    /** Open the fixture release's settings for every written value. */
    readonly catalog: ReturnType<typeof SettingCatalog.cached>;
    /** The served setting values. */
    readonly objects: ObjectStore;

    /** Serve values checked against a release declaring some settings. */
    constructor(
        test: TestDatabase,
        declarations: readonly Setting[],
        events: Awaited<ReturnType<typeof testEvents>>,
    ) {
        this.test = test;
        this.database = test.database;
        this.subject = Subject.parse(principal.user.reference(Scope.universe.id, alice));
        this.reader = release(declarations);
        this.catalog = SettingCatalog.cached(() => this.reader);
        this.objects = new ObjectStore({
            chain: async () => undefined,
            clock: Clock.system,
            principal: universe.reference(Scope.universe.id, Scope.universe.id),
            objects: [implementSetting({ declared: this.catalog, after: [] })],
            // decide access in the scopes the values live in
            policies: [principal.user, space.policy],
            database: test.database,
            callKey: testCallKey,
            events,
            origin: { package: Package.of(settingPackage) },
        });
    }

    /** Let Alice own a scope. */
    async own(scope: ObjectReference): Promise<void> {
        const access = new AccessFixture(this.database);
        await access.replicateScope(scope);
        await access.bindOwner(scope, this.subject);
    }

    /** Send a call as Alice in a scope, its input unchecked until the store parses it. */
    async send(
        object: ObjectType,
        name: string,
        scope: string,
        input: JsonObject,
    ): Promise<unknown> {
        const caller = subjectContext(this.subject, scope).caller();
        const call = Call.record(object, name, { scope, ...input });

        return object.method(name).mutates
            ? this.objects
                  .mutate(caller, { id: RequestId.create(Date.now()), calls: [call] })
                  .then(([result]) => result)
            : this.objects.query(caller, call);
    }

    /** Call a method as Alice in its input's scope. */
    async call<Type extends ObjectType, Name extends CallableName<Type>>(
        object: Type,
        name: Name,
        input: NoInfer<ObjectCallInput<Type, Name>> & { readonly scope: string },
    ): Promise<CallOutput<Type, Name>> {
        // act in the call's scope
        const context = subjectContext(this.subject, input.scope);
        const call = object.call(name, input);
        const caller = context.caller();

        // read through a query, or write through a one-call mutation
        return object.method(name).mutates
            ? this.objects
                  .mutate(caller, { id: RequestId.create(Date.now()), calls: [call] })
                  .then(([result]) => result)
            : this.objects.query(caller, call);
    }

    /** Open a database of a dialect with Alice's own personal scope. */
    static async open(
        dialect: Dialect,
        declarations: readonly Setting[] = [editor, lineNumbers],
    ): Promise<Storage> {
        const tables = [...accessTables, ...settingTables, outbox];
        const test = await TestDatabase.create(dialect, tables, { isMigrated: true });
        const storage = new Storage(test, declarations, await testEvents(test.database));
        await storage.own(principal.user.reference(Scope.universe.id, alice));

        return storage;
    }

    /** Close the connection and remove the database. */
    close(): Promise<void> {
        return this.test.close();
    }
}
