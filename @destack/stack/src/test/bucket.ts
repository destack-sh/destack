import { space, account } from "@destack/account/object";
import { and, type DatabaseConnection, defineDatabase, eq, Transaction } from "@destack/db";
import { MemoryBlobStorage } from "@destack/resource";
import { Scope, Subject } from "@destack/sync";
import {
    AccessContext,
    accessRelationship,
    accessRole,
    accessRolePermission,
    Authentication,
    Authorization,
    Caller,
    principal,
    Relationship,
} from "@destack/access";
import { accountDatabase } from "@destack/account/stack";
import { STACKS, TestDirectory } from "@destack/account/test";
import { CallEvent, call, journal, CallError, Installation, installation } from "@destack/object";
import { Health, Journal, Server, type ClaimPrimary, ObjectStore } from "@destack/object/server";
import type { BaseServerContext } from "@destack/object/workload";
import { testEvents, serversOf, testCallKey } from "@destack/object/test";
import type { EventStore } from "@destack/event";
import { LocalBucket } from "@destack/bucket/bun";
import stackPackage from "../package.ts";
import type { Client, ObjectProcedures } from "@destack/object";
import {
    type BucketReference,
    S3Server,
    S3Signature,
    LEASE_LIFETIME_SECONDS,
} from "@destack/bucket/s3";
import { TestDatabase } from "@destack/db/test";
import { ResourceContext } from "@destack/resource/context";
import { Identifier } from "@destack/schema";
import { createClient } from "@destack/object/client";
import { bucket } from "../object/index.ts";
import { implementBucket } from "../server/resource/bucket.ts";
import type { Lease } from "@destack/resource";
import type { Clock, Environment } from "@destack/environment";

/** A database with buckets, a journal and calls, replicating the spaces they live in. */
export const bucketDatabase = defineDatabase({
    name: "bucket",
    tables: [...bucket.tables, journal, ...call.tables, ...installation.tables],
    replicated: [space.table],
});

/** The space package, declaring the buckets and their permissions. */
const PACKAGE = import.meta.destack.package;

/** The origin the fixture serves S3 under. */
const S3_ENDPOINT = new URL("https://s3.test");

/** The S3 name of the fixture's bucket. */
const BUCKET_NAME = "files";

/** The signing region of the fixture's S3 server, in S3's terms. */
const REGION = "auto";

/** The access key presigning transfers. */
const ACCESS_KEY_ID = "AKIDBUCKET";

/** The secret presigning transfers, fixed so presigned URLs repeat. */
const SECRET_ACCESS_KEY = "fixture-secret-access-key";

/** The most calls a scenario reads back: a scenario makes tens. */
const AUDITED_CALLS = 1000;

/** The time scenarios run at, fixed so presigned URLs repeat: 2026-10-01 00:00 UTC. */
export const NOW = Date.UTC(2026, 9, 1);

/** The query parameters presigning a transfer at the scenarios' time, before its signed headers. */
export const PRESIGNED =
    "X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=AKIDBUCKET%2F20261001%2Fauto%2Fs3%2Faws4_request&X-Amz-Date=20261001T000000Z&X-Amz-Expires=900";

/** A space with a local bucket served over S3, a member role on it and a presigning store. */
export class TestBucket implements AsyncDisposable {
    /** The account of the space with members for the role to bind. */
    readonly accountId: Identifier<"account">;
    /** The space of the bucket. */
    readonly spaceId: Identifier<"space">;
    /** The bucket resource. */
    readonly bucketId: Identifier<"bucket">;
    /** The member calling the bucket's methods. */
    readonly userId: Identifier<"user">;
    /** The role granting the bucket permissions in the space. */
    readonly roleId: Identifier<"role">;
    /** The migrated spaces database. */
    readonly database: DatabaseConnection;
    /** The bucket's files. */
    readonly files: LocalBucket;
    /** The credentials presigning transfers. */
    readonly credentials = { accessKeyId: ACCESS_KEY_ID, secretAccessKey: SECRET_ACCESS_KEY };
    /** The S3 server over the bucket for presigned transfers. */
    readonly s3: S3Server;
    /** The object store answering the member's calls. */
    readonly server: Server;
    /** The member's client. */
    readonly client: Client<ObjectProcedures<typeof bucket>>;
    /** The authenticated caller with its credential, replaced by scenarios acting as others. */
    authentication: Authentication;
    /** The space's call events. */
    readonly #events: EventStore;
    /** Release the database and bucket directory. */
    readonly #close: () => Promise<void>;
    /** The store serving the bucket as the space's base stack installation. */
    readonly #store: ObjectStore;
    /** The world the bucket's servers and records run in. */
    readonly environment: Environment;

    /** Retain the provisioned state. */
    private constructor(
        database: DatabaseConnection,
        files: LocalBucket,
        claims: ClaimPrimary,
        events: EventStore,
        close: () => Promise<void>,
        environment: Environment,
        ids: { readonly accountId: Identifier<"account">; readonly spaceId: Identifier<"space"> },
        base: BaseServerContext,
    ) {
        // retain the storage, the call events, the world and the member's identity
        this.database = database;
        this.environment = environment;
        this.accountId = ids.accountId;
        this.spaceId = ids.spaceId;
        this.bucketId = Identifier.create("bucket", environment);
        this.userId = Identifier.create("user", environment);
        this.roleId = Identifier.create("role", environment);
        this.#events = events;
        this.files = files;
        this.#close = close;
        this.authentication = this.member();

        // serve the bucket over S3 under the presigning credentials
        this.s3 = new S3Server({
            region: REGION,
            environment: this.environment,
            credentials: async (accessKeyId) =>
                accessKeyId === this.credentials.accessKeyId ? this.credentials : undefined,
            open: async (name) => (name === BUCKET_NAME ? this.files : undefined),
        });

        // serve the bucket's files under the member's bearer identity
        const objects = this.#serveObjects(database, claims, base);
        this.#store = objects;
        this.server = Server.start({
            environment: this.environment,
            ...objects.package(stackPackage),
            resources: new ResourceContext(),
            health: new Health("files"),
            authenticate: async (request) => {
                if (request.headers.get("authorization") !== `Bearer ${this.userId}`) {
                    throw CallError.http("UNAUTHORIZED", {
                        message: "invalid bearer credential",
                    });
                }

                return this.authentication;
            },
            authorizeMachine: async () => {},
            drainTimeout: 1000,
        });
        this.client = createClient(
            { router: bucket.procedures },
            {
                url: "https://files.test",
                headers: { authorization: `Bearer ${this.userId}` },
                fetch: (request) => this.server.fetch(request),
            },
            environment,
        );
    }

    /** The clock the bucket's servers and records run on, its world's. */
    get clock(): Clock {
        return this.environment.clock;
    }

    /** Serve the bucket object over the spaces database and the universe's claims. */
    #serveObjects(
        database: DatabaseConnection,
        claims: ClaimPrimary,
        base: BaseServerContext,
    ): ObjectStore {
        return new ObjectStore({
            blobs: new MemoryBlobStorage(this.environment.crypto),
            chain: async () => undefined,
            environment: this.environment,
            principal: principal.space.reference(Scope.universe.id, this.spaceId),
            servers: serversOf([
                implementBucket({
                    open: (reference) => this.#open(reference),
                    locate: async (reference, mode) => {
                        // refuse a write into the fenced bucket
                        const opened = await this.#open(reference);
                        if (mode === "write") {
                            opened.checkWritable();
                        }

                        return {
                            location: {
                                endpoint: S3_ENDPOINT,
                                bucket: BUCKET_NAME,
                                region: REGION,
                            },
                            credentials: this.credentials,
                        };
                    },
                }),
            ]).map((server) => server.at(base)),
            policies: [space],
            claims,
            database,
            callKey: testCallKey,
            events: this.#events,
            origin: { package: stackPackage },
        });
    }

    /** The fixture's bucket, as its methods target it. */
    get bucket() {
        return { spaceId: this.spaceId, id: this.bucketId };
    }

    /** Authenticate the member of the space's account. */
    member(): Authentication {
        return this.authenticate(principal.user.reference("universe", this.userId), [
            principal.user.reference("universe", this.userId),
            { ...account.reference(this.userId, this.accountId), relation: "editor" },
        ]);
    }

    /** Authenticate a subject with the given subjects and claims. */
    authenticate(
        subject: Subject,
        subjects: Subject[],
        claims: Partial<Caller> = {},
    ): Authentication {
        return {
            caller: {
                ...Caller.of(subject),
                ...claims,
                groups: subjects.filter((each) => !Subject.same(each, subject)),
            },
            credential: {
                kind: "fixture",
                id: "fixture-1",
                audience: stackPackage.id,
                verifiedAt: this.clock.now(),
                expiresAt: this.clock.now() + 60_000,
            },
        };
    }

    /** Read the audited calls of the fixture's member and installations, oldest first. */
    async audits(): Promise<CallEvent[]> {
        // read the space's call events
        const page = await this.#events.query(
            call.eventKind(),
            { scopes: [this.spaceId] },
            { limit: AUDITED_CALLS },
        );
        const calls = page.events.map((event) => Journal.recorded(CallEvent.parse(event.data)));

        // filter to the fixture's own callers, ordered by their time-ordered identifiers
        return calls
            .filter(({ execution }) => {
                const { subject } = execution.context.caller;

                return subject.id === this.userId || subject.scope === this.spaceId;
            })
            .toSorted((left, right) => left.execution.id.localeCompare(right.execution.id));
    }

    /** Send a request through a presigned lease, as its holder would: GET to read, PUT to write. */
    async transfer(lease: Lease, body?: string | Uint8Array<ArrayBuffer>): Promise<Response> {
        const request = new Request(lease.url, {
            method: lease.mode === "read" ? "GET" : "PUT",
            headers: lease.headers,
            ...(body === undefined ? {} : { body }),
        });

        return await this.s3.fetch(request);
    }

    /** Presign a request at the scenarios' time as S3 verifies it, returning the signature. */
    async sign(
        method: "GET" | "PUT",
        url: string,
        headers: Record<string, string>,
    ): Promise<string> {
        // presign the request with the fixture's credentials
        const request = new Request(url, { method, headers });
        const signature = new S3Signature({ region: REGION, crypto: this.environment.crypto });
        const presigned = await signature.presign(
            request,
            this.credentials,
            LEASE_LIFETIME_SECONDS,
            this.clock.now(),
        );

        // read the signature from the query
        const signed = new URL(presigned.url).searchParams.get("X-Amz-Signature");
        if (signed === null) {
            throw new TypeError("a presigned url carries no signature");
        }

        return signed;
    }

    /** Open the fixture's bucket, the only one it serves. */
    async #open(reference: BucketReference): Promise<LocalBucket> {
        if (reference.scope !== this.spaceId || reference.bucketId !== this.bucketId) {
            throw CallError.http("NOT_FOUND", {
                message: `bucket ${reference.bucketId} not found`,
            });
        }

        return this.files;
    }

    /** Grant the role a bucket permission in the space. */
    async grant(name: string): Promise<void> {
        await this.database.transaction(
            async (transaction) =>
                await transaction.insert(accessRolePermission).values({
                    id: Identifier.create("role-permission", this.environment),
                    roleId: this.roleId,
                    scope: this.spaceId,
                    packageId: bucket.policy.definition.packageId,
                    type: bucket.name,
                    name,
                }),
        );
    }

    /** Withdraw a bucket permission from the role. */
    async withdraw(name: string): Promise<void> {
        await this.database.transaction(
            async (transaction) =>
                await transaction
                    .delete(accessRolePermission)
                    .where(
                        and(
                            eq(accessRolePermission.roleId, this.roleId),
                            eq(accessRolePermission.type, bucket.name),
                            eq(accessRolePermission.name, name),
                        ),
                    ),
        );
    }

    /** Bind the role to a subject in the space. */
    async bind(subject: Subject): Promise<void> {
        await this.database.transaction(
            async (transaction) =>
                await transaction.insert(accessRelationship).values(
                    Relationship.encode({
                        id: Identifier.create("relationship", this.environment),
                        object: space.reference(this.accountId, this.spaceId),
                        role: this.roleId,
                        subject,
                        createdAt: this.clock.now(),
                        expiresAt: null,
                    }),
                ),
        );
    }

    /** Relate an installation of the package to the bucket as a consumer, as consumption does. */
    async install() {
        // relate a new installation to the bucket as a consumer
        const now = this.clock.now();
        const installationId = Identifier.create("installation", this.environment);
        await this.database.transaction(
            async (transaction) =>
                await transaction.insert(accessRelationship).values(
                    Relationship.encode({
                        id: Identifier.create("relationship", this.environment),
                        object: bucket.reference(this.spaceId, this.bucketId),
                        relation: "consumer",
                        subject: principal.installation.reference(this.spaceId, installationId),
                        createdAt: now,
                        expiresAt: null,
                    }),
                ),
        );

        return installationId;
    }

    /** Open the migrated bucket database for a file's scenarios in their world. */
    static async database(environment: Environment): Promise<TestDatabase> {
        return await TestDatabase.create(bucketDatabase, {
            environment: environment,
            isMigrated: true,
        });
    }

    /** Provision a space, its bucket and a member role with every bucket permission, in a world. */
    static async open(
        database: DatabaseConnection,
        environment: Environment<"files">,
    ): Promise<TestBucket> {
        // open the bucket's directory, and the universe's database with its name claims
        const directory = await environment.files.makeTemporaryDirectory("destack-bucket-files-");
        const files = await LocalBucket.open(directory, "space-test", environment);
        const universe = await TestDatabase.create(accountDatabase, {
            environment,
            isMigrated: true,
        });
        const claims = await TestDirectory.claims(universe.database, environment);
        const events = await testEvents(database, environment);
        const release = async () => {
            await files[Symbol.asyncDispose]();
            await universe.close();
            await environment.files.remove(directory, { recursive: true });
        };

        // serve the files as the space's base stack installation, releasing the bucket when it fails to start
        const ids = {
            accountId: Identifier.create("account", environment),
            spaceId: Identifier.create("space", environment),
        };
        const scope = space.reference(ids.accountId, ids.spaceId);
        const base = await Installation.base(scope, stackPackage, environment.crypto);
        let fixture: TestBucket;
        try {
            fixture = new TestBucket(
                database,
                files,
                claims,
                events,
                release,
                environment,
                ids,
                base,
            );
        } catch (error) {
            await release();
            throw error;
        }

        // provision the space, closing everything when that fails
        try {
            await fixture.#provision();

            return fixture;
        } catch (error) {
            await fixture[Symbol.asyncDispose]();
            throw error;
        }
    }

    /** Record the space, its bucket and the member's role as the space controller would. */
    async #provision(): Promise<void> {
        // record the space and its bucket
        const now = this.clock.now();
        await this.#record(this.spaceId, this.bucketId, "bucket-fixture", "home");

        // grant every bucket permission to the account's members through a role in the space
        await this.database.transaction(
            async (transaction) =>
                await transaction.insert(accessRole).values({
                    id: this.roleId,
                    scope: this.spaceId,
                    name: "bucket-owner",
                    description: "use the fixture bucket",
                    createdAt: now,
                    updatedAt: now,
                }),
        );
        await this.bind({
            ...account.reference(this.userId, this.accountId),
            relation: "editor",
        });
        for (const name of Object.keys(bucket.policy.definition.permissions)) {
            await this.grant(name);
        }
    }

    /** Record another space of the account with its own bucket and no role, and select it. */
    async neighbour() {
        // record the space and its bucket under new identifiers
        const spaceId = Identifier.create("space", this.environment);
        const bucketId = Identifier.create("bucket", this.environment);
        await this.#record(spaceId, bucketId, "neighbour", "replica");

        return { spaceId, id: bucketId };
    }

    /** Receive a space at its home with its base installations, as its placement does. */
    async #receive(spaceId: Identifier<"space">, now: number): Promise<void> {
        await Transaction.run(this.database, async (transaction) => {
            const authorization = new Authorization(
                this.#store.authorizer,
                transaction,
                () => AccessContext.of(Caller.of(this.#store.principal), now),
                this.environment,
            );
            await Installation.receive(
                authorization,
                space.reference(this.accountId, spaceId),
                [this.accountId],
                [stackPackage],
            );
        });
    }

    /** Record a space of the account below its scope, with one bucket resource. */
    async #record(
        spaceId: Identifier<"space">,
        bucketId: Identifier<"bucket">,
        name: string,
        place: "home" | "replica",
    ): Promise<void> {
        // record the space below the account
        const now = this.clock.now();
        const record = { createdAt: now, updatedAt: now };
        await this.database.transaction(
            async (transaction) =>
                await transaction.insert(space.table).values({
                    ...record,
                    id: spaceId,
                    scope: this.accountId,
                    name,
                    stack: STACKS.space,
                }),
        );
        // receive a homed scope with its base installations, as its placement does
        if (place === "home") {
            await this.#receive(spaceId, now);
        }
        // insert another space's scope row as its replica does
        else {
            await this.database.transaction(
                async (transaction) =>
                    await transaction.insert(Scope.table).values({
                        scope: spaceId,
                        parent: this.accountId,
                        partition: spaceId,
                        packageId: space.policy.definition.packageId,
                        type: space.name,
                        ancestors: [this.accountId],
                    }),
            );
        }

        // record its bucket
        await this.database.transaction(
            async (transaction) =>
                await transaction.insert(bucket.table).values({
                    ...record,
                    id: bucketId,
                    scope: spaceId,
                    name: "files",
                    definitionPackageId: bucket.policy.definition.packageId,
                    definitionVersion: PACKAGE.version,
                    definitionName: "files",
                    spec: {},
                }),
        );
    }

    /** Drain the server, then release the bucket and database. */
    async close(): Promise<void> {
        await this.server.close();
        await this.#close();
    }

    /** Close the fixture after a scenario. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }
}
