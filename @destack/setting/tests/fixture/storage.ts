import { outbox } from "@destack/service/outbox";
import { AccessFixture } from "@destack/access/test";
import { Scope, type ObjectReference, Subject } from "@destack/sync";
import { TestDatabase } from "@destack/db/test";
import { RequestId } from "@destack/service/request";
import { accessTables, principal } from "@destack/access";
import {
    account,
    client,
    key,
    machine,
    organisation,
    user,
    type Client,
} from "@destack/account/object";
import { space } from "@destack/space/object";
import type { Dialect } from "@destack/db";
import type { JsonObject } from "@destack/schema";
import type { CallableName, CallOutput, ObjectType } from "@destack/object";
import { ObjectServer } from "@destack/object/server";

import { settingTables } from "../../src/stack/index.ts";
import { setting } from "../../src/object/index.ts";
import { serveSettings } from "../../src/server/index.ts";
import type { BuildReader } from "@destack/package/manifest";
import { defineService } from "@destack/service";
import { type Setting, SettingCatalog } from "../../src/setting/index.ts";
import { editor, lineNumbers, release } from "./setting/index.ts";
import { alice } from "./value.ts";
import { subjectContext, testCallKey } from "@destack/service/test";

/** The fixture's service of setting values and Alice's clients. */
export const settingService = defineService("setting", { objects: { setting, client } });

/** Setting values and Alice's clients served from a migrated test database. */
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
    /** The served setting values and clients. */
    readonly objects: ObjectServer<ReturnType<typeof serveSettings> & { client: typeof client }>;

    /** Serve values checked against a release declaring some settings. */
    constructor(test: TestDatabase, declarations: readonly Setting[]) {
        this.test = test;
        this.database = test.database;
        this.subject = Subject.parse(principal.user.reference(Scope.universe.id, alice));
        this.reader = release(declarations);
        this.catalog = SettingCatalog.cached(() => this.reader);
        this.objects = new ObjectServer({
            objects: { ...serveSettings(this.catalog), client },
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

    /** Call a method as Alice in a scope. */
    async call<Type extends ObjectType, Name extends CallableName<Type>>(
        object: Type,
        name: Name,
        scope: string,
        input: JsonObject,
    ): Promise<CallOutput<Type, Name>> {
        // place the call in the scope through the object's route field
        const field = object.route.field;
        if (field === undefined) {
            throw new TypeError("a scoped call needs an object routed by a field");
        }
        const context = subjectContext(this.subject, scope);

        return await this.objects.call(
            object,
            name,
            { [field]: scope, requestId: RequestId.create(), ...input },
            context,
        );
    }

    /** Register a client of Alice's. */
    async register(name: string): Promise<Omit<Client, "push">> {
        return await this.call(client, "create", alice, { name, kind: "desktop" });
    }

    /** Open a database of a dialect with Alice's own personal scope. */
    static async open(
        dialect: Dialect,
        declarations: readonly Setting[] = [editor, lineNumbers],
    ): Promise<Storage> {
        const tables = [...accessTables, ...settingTables, client.table, key.table, outbox];
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
