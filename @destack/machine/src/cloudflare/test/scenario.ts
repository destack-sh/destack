import { Random } from "@destack/environment";
import { WorkerdEnvironment } from "@destack/environment/workerd";
import { principal } from "@destack/access";
import { defineDatabase, defineTable, text } from "@destack/db";
import {
    CONTENT_PATH,
    type DurableObjectDatabaseBindings,
    DurableObjectDatabaseHost,
    type DurableObjectStorage,
} from "@destack/db/cloudflare";
import { DurableObjectDatabase } from "@destack/db/cloudflare/durable";
import { ModuleMetadata } from "@destack/package";
import { Plan, ResourceId } from "@destack/resource";
import { MemoryContentStore } from "@destack/resource";
import { schema } from "@destack/schema";
import { refusal } from "@destack/object";
import { type DurableObjectNamespace, PartitionName } from "../runtime/index.ts";
import { database } from "@destack/stack/object";
import { ObjectReference } from "@destack/sync";
import { SpaceStorage } from "../../space/storage.ts";
import { DurableObjectSpaceDatabaseHost } from "../database.ts";

/** The space's main database on a cloud machine. */
const MAIN = SpaceStorage.declare().main;

/** The space whose object stores the installations' databases' copies. */
const SPACE = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-00000000e001");

/** The installation whose object the space measures and copies a database from. */
const INSTALLED = {
    scope: SPACE,
    installationId: schema
        .identifier("installation")
        .parse("installation-01996ab0-0000-7000-8000-00000000e002"),
};

/** The installation whose object the space restores the copy into. */
const RESTORED = {
    scope: SPACE,
    installationId: schema
        .identifier("installation")
        .parse("installation-01996ab0-0000-7000-8000-00000000e003"),
};

/** The database the first installation's object stores. */
const SOURCE = "database-01996ab0-0000-7000-8000-00000000e005";

/** The database the second installation's object restores the first's copy into. */
const TARGET = "database-01996ab0-0000-7000-8000-00000000e006";

/** The bytes an installation's object writes: well above a space object's empty storage. */
const WRITTEN_BYTES = 256 * 1024;

/** The module the scenario's tables belong to. */
const MODULE = ModuleMetadata.parse({
    package: {
        id: "package-01996ab0-0000-7000-8000-00000000e004",
        name: "@destack/copy-scenario",
        version: "2026.10.0",
    },
});

/** Entries in an installation's database. */
const entry = defineTable(
    "entry",
    { id: text("id").primaryKey(), title: text("title").notNull() },
    { log: {} },
    MODULE,
);

/** The database of entries the installations declare. */
export const entries = defineDatabase({ name: "entries", tables: [entry] }, MODULE);

/** A count of bytes an object answers. */
const Counted = schema.object({ bytes: schema.number().int().nonnegative() });

/** The entries an object answers. */
const Entries = schema.array(schema.object({ id: schema.string(), title: schema.string() }));

/** An installation's partition that writes bytes and entries of its own. */
class Instance {
    /** The object's storage. */
    readonly storage: DurableObjectStorage;
    /** The databases' operations, copying through the space's object. */
    readonly databases: DurableObjectDatabase;

    /** Take the object's storage and answer database operations as the Worker's export does. */
    constructor(
        state: { readonly storage: DurableObjectStorage },
        bindings: DurableObjectDatabaseBindings,
    ) {
        this.storage = state.storage;
        this.databases = new DurableObjectDatabase(state, bindings);
    }

    /** The databases in the object's storage. */
    get host() {
        return this.databases.host;
    }

    /** Write bytes, write or read a database's entries, or answer an operation of the space. */
    async fetch(request: Request): Promise<Response> {
        // read the path and its database
        const url = new URL(request.url);
        const namespace = url.searchParams.get("database");

        // write a blob of the installation's own and count the object's bytes
        if (url.pathname === "/write") {
            this.storage.sql.exec("CREATE TABLE IF NOT EXISTS blob (bytes BLOB)");
            this.storage.sql.exec("INSERT INTO blob VALUES (?)", new Uint8Array(WRITTEN_BYTES));

            return Response.json({ bytes: this.storage.sql.databaseSize });
        }
        // write entries into a database as its workload does
        else if (url.pathname === "/entries" && request.method === "POST" && namespace !== null) {
            await using written = this.host.connect([entry], { namespace });
            const rows = Entries.parse(await request.json());
            await written.transaction(
                async (transaction) => await transaction.insert(entry).values(rows),
            );

            return Response.json({});
        }
        // read a database's entries
        else if (url.pathname === "/entries" && namespace !== null) {
            await using read = this.host.connect([entry], { namespace });

            return Response.json(await read.select().from(entry).orderBy(entry.id));
        }

        return this.databases.fetch(request);
    }
}

/** A space's partition measuring and copying its installations' databases. */
class Space {
    /** The object's storage. */
    readonly storage: DurableObjectStorage;
    /** The installations' objects. */
    readonly instances: DurableObjectNamespace;
    /** The space's database, recording each database's owning installation. */
    readonly records: ReturnType<DurableObjectDatabaseHost["connect"]>;
    /** The space's databases: its own in its storage, its installations' in their objects. */
    readonly databases: DurableObjectSpaceDatabaseHost;

    /** Take the object's storage, the installations' objects and the space's databases. */
    constructor(
        state: { readonly storage: DurableObjectStorage },
        bindings: { readonly PARTITION: DurableObjectNamespace },
    ) {
        // set the storage, the installations' objects and the space's records of its databases
        this.storage = state.storage;
        this.instances = bindings.PARTITION;
        const host = new DurableObjectDatabaseHost(state.storage, WorkerdEnvironment.system);
        this.records = host.connect(MAIN);
        this.databases = new DurableObjectSpaceDatabaseHost(
            this.records,
            bindings.PARTITION,
            host,
            Random.system,
        );
    }

    /** Serve a copy's page, copy a database between installations, or measure the objects. */
    async fetch(request: Request): Promise<Response> {
        const { pathname } = new URL(request.url);
        try {
            // serve a page to the installation's object running a copy
            if (pathname.startsWith(`${CONTENT_PATH}/`)) {
                return await this.databases.content(request);
            }
            // copy a database from one installation's object into another's
            else if (pathname === "/copy") {
                return await this.copy();
            }

            return await this.measure();
        } catch (error) {
            return refusal(error, WorkerdEnvironment.system.telemetry);
        }
    }

    /** Snapshot an installation's database into the space's store. */
    async copy(): Promise<Response> {
        // record a database owned by each installation in the space
        await this.records.migrate(MAIN.tables);
        const source = await this.#provision(INSTALLED, "source", SOURCE);
        const target = await this.#provision(RESTORED, "target", TARGET);
        const desired = [entries.state()];

        // write entries in the first installation's object
        const written = [
            { id: "a", title: "Kept" },
            { id: "b", title: "Moved" },
        ];
        await this.#instance(INSTALLED).fetch(
            new Request(`https://instance/entries?database=${source.id}`, {
                method: "POST",
                body: JSON.stringify(written),
            }),
        );

        // snapshot it through the space's store and restore the copy into the second installation's object
        const store = new MemoryContentStore(WorkerdEnvironment.system.crypto);
        const digest = await this.databases.snapshot(source, desired, store);
        await this.databases.restore(target, desired, digest, store);
        const read = await this.#instance(RESTORED).fetch(
            new Request(`https://instance/entries?database=${target.id}`),
        );

        // refuse a page outside any copy in flight
        const stray = await this.fetch(
            new Request(`https://space${CONTENT_PATH}/${digest}`, {
                headers: { authorization: "Bearer stray" },
            }),
        );

        return Response.json({
            pages: store.size,
            entries: Entries.parse(await read.json()),
            stray: stray.status,
        });
    }

    /** Measure the space's object and the installation's object through the machine's databases. */
    async measure(): Promise<Response> {
        // store a row of the stack service in the space's own storage
        this.storage.sql.exec("CREATE TABLE IF NOT EXISTS space (id TEXT)");
        this.storage.sql.exec("INSERT INTO space VALUES (?)", INSTALLED.scope);

        // write a blob in the installation's object
        const written = await this.#instance(INSTALLED).fetch(
            new Request("https://instance/write"),
        );
        const installation = Counted.parse(await written.json()).bytes;

        return Response.json({
            space: {
                measured: (await this.databases.measure()).bytes,
                own: this.storage.sql.databaseSize,
            },
            installation: {
                measured: (await this.databases.measure(INSTALLED)).bytes,
                own: installation,
            },
        });
    }

    /** Record a database of an installation in the space. */
    async #provision(installed: typeof INSTALLED, name: string, id: string) {
        // record the database with its owner
        const owner = principal.installation.reference(installed.scope, installed.installationId);
        const record = {
            id: ResourceId.parse(id),
            scope: installed.scope,
            spec: { replicated: [] },
            reference: null,
        };
        await this.records.transaction(
            async (transaction) =>
                await transaction.insert(database.table).values({
                    id: database.identifier(id),
                    scope: record.scope,
                    name,
                    definitionPackageId: MODULE.package.id,
                    definitionVersion: MODULE.package.version,
                    definitionName: name,
                    spec: record.spec,
                    ...ObjectReference.columns("owner", owner),
                    createdAt: 0,
                    updatedAt: 0,
                }),
        );

        // provision it and apply its tables in the installation's object
        const provisioned = { ...record, ...(await this.databases.provision(record)) };
        const desired = [entries.state()];
        const plan = await this.databases.plan(provisioned, desired);
        await this.databases.apply(
            provisioned,
            desired,
            await Plan.digest(plan, WorkerdEnvironment.system.crypto),
        );

        return provisioned;
    }

    /** Address an installation's object. */
    #instance(installed: typeof INSTALLED) {
        return this.instances.get(
            this.instances.idFromName(
                PartitionName.of({
                    space: installed.scope,
                    installation: installed.installationId,
                }),
            ),
        );
    }
}

/** A partition's object: the space's own, named by the space, or an installation's. */
export class Partition {
    /** The role the object's name gives it. */
    readonly #role: Space | Instance;

    /** Take the space's role under the space's name, else an installation's. */
    constructor(
        state: { readonly id: { readonly name?: string }; readonly storage: DurableObjectStorage },
        bindings: DurableObjectDatabaseBindings & {
            readonly PARTITION: DurableObjectNamespace;
        },
    ) {
        this.#role =
            state.id.name === PartitionName.of({ space: SPACE })
                ? new Space(state, bindings)
                : new Instance(state, bindings);
    }

    /** Serve a request in the object's role. */
    fetch(request: Request): Promise<Response> {
        return this.#role.fetch(request);
    }
}
