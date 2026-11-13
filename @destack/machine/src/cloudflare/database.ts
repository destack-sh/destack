import { Installation } from "@destack/object";
import type { Random } from "@destack/environment";
import {
    Snapshot,
    and,
    type DatabaseConnection,
    type DatabaseHandle,
    type DatabaseHost,
    type DatabaseKind,
    type DatabaseState,
    gathered,
} from "@destack/db";
import {
    CONTENT_PATH,
    DATABASE_PATH,
    DURABLE_OBJECT_PROVIDER,
    DurableObjectDatabaseHost,
    DurableObjectMeasurement,
} from "@destack/db/cloudflare";
import { PartitionName } from "./runtime/index.ts";
import { type ContentStore, Plan, type ResourceRecord } from "@destack/resource";
import { Digest, schema } from "@destack/schema";
import { Bearer } from "@destack/access/token";
import { StackError } from "@destack/stack/error";
import { database } from "@destack/stack/object";
import { MachineError } from "../error/index.ts";

/** The namespace of the space's database in the space object's storage. */
export const SPACE_NAMESPACE = "space";

/** The namespace of the machine's builds in the space object's storage. */
export const CACHE_NAMESPACE = "cache";

/** The bytes of the credential of a copy's pages: 256 bits, unguessable for the copy's lifetime. */
const COPY_TOKEN_BYTES = 32;

/** The databases a cloud machine stores for a space. */
export class DurableObjectSpaceDatabaseHost implements DatabaseHost {
    /** The provider code of databases in Durable Objects. */
    readonly provider = DURABLE_OBJECT_PROVIDER;
    /** The space's database, recording each database's owning installation. */
    readonly #records: DatabaseConnection;
    /** The facet of an installation in the space's object, which stores its databases. */
    readonly #facet: (installation: string) => {
        readonly fetch: (request: Request) => Promise<Response>;
    };
    /** The databases the space's object stores itself. */
    readonly #space: DurableObjectDatabaseHost;
    /** The stores of the copies in flight in installations' facets. */
    readonly #copies = new Map<string, Pick<ContentStore, "read" | "write">>();
    /** The randomness the copies' credentials are drawn from. */
    readonly #random: Random;

    /** Store databases in the facets of their installations, read from the space's database. */
    constructor(
        records: DatabaseConnection,
        facet: (installation: string) => {
            readonly fetch: (request: Request) => Promise<Response>;
        },
        space: DurableObjectDatabaseHost,
        random: Random,
    ) {
        // set the records, the installations' facets, the space's databases and the randomness
        this.#records = records;
        this.#facet = facet;
        this.#space = space;
        this.#random = random;
    }

    /** Create the database in its object, answering its reference. */
    async provision(record: ResourceRecord<typeof DatabaseKind>): Promise<{ reference: string }> {
        const object = await this.#object(record);

        return object === undefined
            ? await this.#space.provision(record)
            : schema.object({ reference: schema.string().min(1) }).parse(
                  await DurableObjectSpaceDatabaseHost.#send(object, {
                      operation: "provision",
                      record,
                  }),
              );
    }

    /** Drop the database in its object. */
    async destroy(record: ResourceRecord<typeof DatabaseKind>): Promise<void> {
        const object = await this.#object(record);
        await (object === undefined
            ? this.#space.destroy(record)
            : DurableObjectSpaceDatabaseHost.#send(object, { operation: "destroy", record }));
    }

    /** Plan the database's tables in its object. */
    async plan(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly DatabaseState[],
    ): Promise<Plan> {
        const object = await this.#object(record);

        return object === undefined
            ? await this.#space.plan(record, desired)
            : Plan.parse(
                  await DurableObjectSpaceDatabaseHost.#send(object, {
                      operation: "plan",
                      record,
                      desired,
                  }),
              );
    }

    /** Apply the reviewed plan in the database's object. */
    async apply(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly DatabaseState[],
        digest: Digest,
    ): Promise<void> {
        const object = await this.#object(record);
        await (object === undefined
            ? this.#space.apply(record, desired, digest)
            : DurableObjectSpaceDatabaseHost.#send(object, {
                  operation: "apply",
                  record,
                  desired,
                  digest,
              }));
    }

    /** Open a space's own database. */
    async open(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly DatabaseState[],
    ): Promise<DatabaseHandle> {
        if ((await this.#object(record)) !== undefined) {
            throw StackError.of(
                "DATABASE_DENIED",
                `database '${record.id}' is forbidden: only the workload of its installation's facet can open it`,
            );
        }

        return this.#space.open(record, desired);
    }

    /** Copy a database's rows into a store in its object. */
    async snapshot(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly DatabaseState[],
        store: Pick<ContentStore, "read" | "batch" | "write" | "fetch">,
    ): Promise<Digest> {
        // copy the space's own database here
        const object = await this.#object(record);
        if (object === undefined) {
            return this.#space.snapshot(record, desired, store);
        }

        // have the installation's facet copy it through the store
        const copied = await this.#copy(object, store, { operation: "snapshot", record, desired });

        return schema.object({ digest: Digest }).parse(copied).digest;
    }

    /** Restore a copy from a store into a database in its object. */
    async restore(...copy: Parameters<DatabaseHost["restore"]>): Promise<void> {
        // restore the space's own database here
        const [record, desired, digest, store, recipient, base] = copy;
        const object = await this.#object(record);
        if (object === undefined) {
            await this.#space.restore(record, desired, digest, store, recipient, base);

            return;
        }

        // have the installation's facet restore it through the store
        const restored = { operation: "restore", record, desired, digest };
        await this.#copy(object, store, base === undefined ? restored : { ...restored, base });
    }

    /** Serve a page of a copy in flight to the installation's facet that runs it. */
    async content(request: Request): Promise<Response> {
        // require the store of a copy in flight
        const token = Bearer.read(request.headers);
        const store = token === undefined ? undefined : this.#copies.get(token);
        if (store === undefined) {
            throw MachineError.of(
                "COPY_CREDENTIAL_UNAUTHENTICATED",
                "authentication required: no copy in progress has this credential",
            );
        }

        // read the page of the path
        const { pathname } = new URL(request.url);
        const digest = Digest.parse(pathname.slice(CONTENT_PATH.length + 1));
        if (request.method === "GET") {
            return new Response(await gathered(store.read(digest)));
        }
        // write it
        else if (request.method === "PUT" && request.body !== null) {
            return Response.json({ digest: await store.write(request.body, digest) });
        }
        // refuse any other method
        else {
            throw MachineError.of(
                "METHOD_UNSUPPORTED",
                `content method '${request.method}' is not supported`,
            );
        }
    }

    /** Send a copy's operation to an installation's facet. */
    async #copy(
        object: InstanceObject,
        store: Pick<ContentStore, "read" | "write">,
        operation: object,
    ): Promise<unknown> {
        // serve the store under a fresh credential while the object copies
        const token = this.#random.bytes(COPY_TOKEN_BYTES).toHex();
        this.#copies.set(token, store);
        try {
            return await DurableObjectSpaceDatabaseHost.#send(object, operation, `Bearer ${token}`);
        } finally {
            this.#copies.delete(token);
        }
    }

    /** Measure an object's databases. */
    async measure(installed?: {
        readonly scope: string;
        readonly installationId: string;
    }): Promise<DurableObjectMeasurement> {
        // measure the space's own object here
        if (installed === undefined) {
            return this.#space.measure();
        }

        // ask the installation's facet
        const measured = await DurableObjectSpaceDatabaseHost.#send(this.#instance(installed), {
            operation: "measure",
        });

        return DurableObjectMeasurement.parse(measured);
    }

    /** Find the object of the installation a database belongs to. */
    async #object(
        record: ResourceRecord<typeof DatabaseKind>,
    ): Promise<InstanceObject | undefined> {
        // find the application installation managing the database, none for the space's own
        const scope = schema.identifier("space").parse(record.scope);
        const [row] = await this.#records
            .select({ manager: database.table.managerInstallationId })
            .from(database.table)
            .where(and(database.inScope(scope), database.withIds([record.id])));
        const manager = row?.manager ?? null;
        const applications =
            manager === null
                ? new Set<string>()
                : await Installation.applications(Snapshot.live(this.#records), [manager]);
        if (manager === null || !applications.has(manager)) {
            return undefined;
        }

        // address the installation's facet
        return this.#instance({ scope, installationId: manager });
    }

    /** Address an installation's facet in the space's object. */
    #instance(installed: {
        readonly scope: string;
        readonly installationId: string;
    }): InstanceObject {
        const name = PartitionName.of({
            space: installed.scope,
            installation: installed.installationId,
        });

        return { name, fetch: (request) => this.#facet(installed.installationId).fetch(request) };
    }

    /** Send an operation on a database to its object. */
    static async #send(
        object: InstanceObject,
        operation: object,
        authorization?: string,
    ): Promise<unknown> {
        // post the operation and require the object's answer
        const response = await object.fetch(
            new Request(`https://instance${DATABASE_PATH}`, {
                method: "POST",
                body: JSON.stringify(operation),
                ...(authorization === undefined ? {} : { headers: { authorization } }),
            }),
        );
        const text = await response.text();
        if (!response.ok) {
            throw MachineError.of(
                "OBJECT_UNREACHABLE",
                `could not run an operation on the object of ${object.name}: ${response.status} '${text}'`,
            );
        }

        return JSON.parse(text);
    }
}

/** An installation's facet, named for failures. */
interface InstanceObject {
    /** The object's name. */
    readonly name: string;
    /** Send a request to the object. */
    readonly fetch: (request: Request) => Promise<Response>;
}
