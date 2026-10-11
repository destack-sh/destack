import { Clock } from "@destack/schema";
import { outbox } from "@destack/service/outbox";
import { AccessFixture } from "@destack/access/test";
import { Scope, type ObjectReference, Subject } from "@destack/sync";
import { TestDatabase } from "@destack/db/test";
import { RequestId } from "@destack/service/request";
import { accessTables, principal, universe } from "@destack/access";
import { account, machine, organisation, user, space } from "@destack/account/object";

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

import { settingTables } from "../../src/stack/index.ts";
import { setting } from "../../src/object/index.ts";
import { serveSettings } from "../../src/server/index.ts";
import type { BuildReader } from "@destack/package/manifest";
import { defineService } from "@destack/service";
import { type Setting, SettingCatalog } from "../../src/setting/index.ts";
import { editor, lineNumbers, release } from "./setting/index.ts";
import { alice } from "./value.ts";
import { subjectContext, testCallKey } from "@destack/service/test";

/** The fixture's service of setting values. */
export const settingService = defineService("setting", { objects: { setting } });

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
    constructor(test: TestDatabase, declarations: readonly Setting[]) {
        this.test = test;
        this.database = test.database;
        this.subject = Subject.parse(principal.user.reference(Scope.universe.id, alice));
        this.reader = release(declarations);
        this.catalog = SettingCatalog.cached(() => this.reader);
        this.objects = new ObjectStore({
            clock: Clock.system,
            principal: universe.reference(Scope.universe.id, Scope.universe.id),
            objects: serveSettings(this.catalog, []),
            // decide access in the scopes the values live in
            policies: [user, space, account, organisation, machine].map((type) => type.policy),
            database: test.database,
            callKey: testCallKey,
            origin: { package: settingService.package, service: settingService.name },
        });
    }

    /** Let Alice own a scope. */
    async own(scope: ObjectReference): Promise<void> {
        const copies = new AccessFixture(this.database);
        await copies.copyScope(scope);
        await copies.copyOwner(scope, this.subject);
    }

    /** Send a call as an untyped client does, as Alice in a scope, its input unchecked until the store parses it. */
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
                  .mutate(caller, { id: RequestId.create(), calls: [call] })
                  .then(([result]) => result)
            : this.objects.query(caller, call);
    }

    /** Call a method as Alice in the scope its input names. */
    async call<Type extends ObjectType, Name extends CallableName<Type>>(
        object: Type,
        name: Name,
        input: NoInfer<ObjectCallInput<Type, Name>> & { readonly scope: string },
    ): Promise<CallOutput<Type, Name>> {
        // act in the scope the call names
        const context = subjectContext(this.subject, input.scope);
        const call = object.call(name, input);
        const caller = context.caller();

        // read through a query, or write through a one-call mutation
        return object.method(name).mutates
            ? this.objects
                  .mutate(caller, { id: RequestId.create(), calls: [call] })
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
        const storage = new Storage(test, declarations);
        await storage.own(principal.user.reference(Scope.universe.id, alice));

        return storage;
    }

    /** Close the connection and remove the database. */
    close(): Promise<void> {
        return this.test.close();
    }
}
