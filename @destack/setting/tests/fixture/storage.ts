import { outbox } from "@destack/service/outbox";
import { AccessFixture } from "@destack/access/test";
import { Scope, type ObjectReference, Subject } from "@destack/sync";
import { TestDatabase } from "@destack/db/test";
import { RequestId } from "@destack/service/request";
import { accessTables, principal } from "@destack/access";
import { device, type Device } from "@destack/account/object";
import type { Dialect } from "@destack/db";
import type { JsonObject } from "@destack/schema";
import type { CallableName, CallOutput, ObjectType } from "@destack/object";
import { ObjectServer } from "@destack/object/server";

import { settingTables } from "../../src/stack/index.ts";
import { setting } from "../../src/object/index.ts";
import { type OpenRelease, servedObjects } from "../../src/server/index.ts";
import { defineService } from "@destack/service";
import type { Setting } from "../../src/setting/index.ts";
import { editor, lineNumbers, release } from "./setting/index.ts";
import { alice } from "./value.ts";
import { subjectContext, testCallKey } from "@destack/service/test";

/** The fixture's service of setting values and Alice's devices. */
export const settingService = defineService("setting", { objects: { setting, device } });

/** Setting values and Alice's devices served from a migrated test database. */
export class Storage {
    /** The isolated database. */
    readonly test: TestDatabase;
    /** The database's connection. */
    readonly database: TestDatabase["database"];
    /** Alice, who owns the scopes she writes in. */
    readonly subject: Subject;
    /** Open the fixture release declaring the settings. */
    readonly release: OpenRelease;
    /** The served setting values and devices. */
    readonly objects: ObjectServer<ReturnType<typeof servedObjects> & { device: typeof device }>;

    /** Serve values checked against a release declaring some settings. */
    constructor(test: TestDatabase, declarations: readonly Setting[]) {
        this.test = test;
        this.database = test.database;
        this.subject = Subject.parse(principal.user.reference(Scope.universe.id, alice));
        const reader = release(declarations);
        this.release = () => Promise.resolve(reader);
        this.objects = new ObjectServer({
            objects: { ...servedObjects(this.release), device },
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

    /** Register a device of Alice's. */
    async register(name: string): Promise<Device> {
        return await this.call(device, "create", alice, { name, kind: "desktop" });
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
