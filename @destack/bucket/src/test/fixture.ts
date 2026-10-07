import { mkdtemp, rm } from "node:fs/promises";
import { AuditHistory } from "@destack/audit/history";
import { Scope, type Subject } from "@destack/sync";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
    accessRelationship,
    accessRole,
    accessRolePermission,
    principal,
    Relationship,
} from "@destack/access";
import * as accountObject from "@destack/account/object";
import { AuditCall } from "@destack/audit";
import { Journal } from "@destack/audit/server";
import { LocalBucket } from "../local/index.ts";
import * as bucketObject from "../object/index.ts";
import { bucketService } from "../service/index.ts";
import type { Client } from "@destack/service";
import type { ObjectProcedures } from "@destack/object";
import { type BucketReference, S3Server, S3Signature } from "../s3/index.ts";
import { and, eq, type DatabaseConnection, type Dialect } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { ResourceContext } from "@destack/resource/context";
import { schema, type Identifier } from "@destack/schema";
import { Authentication, type AuthenticationClaims } from "@destack/service/authentication";
import { ServiceError } from "@destack/service/error";
import { Health } from "@destack/service/health";
import { Server } from "@destack/service/server";
import * as spaceObject from "@destack/space/object";
import { space } from "@destack/space/object";
import { v7 } from "uuid";
import { ObjectServer } from "@destack/object/server";
import { serveBuckets, LEASE_LIFETIME } from "../server/index.ts";
import { bucketDatabase } from "../stack/index.ts";
import { DirectoryStore, directoryTables } from "@destack/directory";
import type { Lease } from "@destack/resource";
import type {} from "@destack/package/import-meta";
import { testCallKey } from "@destack/service/test";

/** The bucket package, declaring the buckets and their permissions. */
const PACKAGE = import.meta.destack.package;

/** The origin the fixture serves S3 under. */
const S3_ENDPOINT = new URL("https://s3.test");
/** The S3 name of the fixture's bucket. */
const BUCKET_NAME = "files";
/** The region the fixture's S3 server signs for, R2's region. */
const REGION = "auto";

/** The access key presigning transfers. */
const ACCESS_KEY_ID = "AKIDBUCKET";
/** The secret presigning transfers, fixed so presigned URLs repeat. */
const SECRET_ACCESS_KEY = "fixture-secret-access-key";

/** The time scenarios run at, fixed so presigned URLs repeat: 2026-10-01 00:00 UTC. */
export const NOW = Date.UTC(2026, 9, 1);

/** The query parameters presigning a transfer at the scenarios' time, before its signed headers. */
export const PRESIGNED =
    "X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=AKIDBUCKET%2F20261001%2Fauto%2Fs3%2Faws4_request&X-Amz-Date=20261001T000000Z&X-Amz-Expires=900";

/** A space with a local bucket served over S3, a member role on it, and an object server presigning. */
export class BucketFixture implements AsyncDisposable {
    /** The account of the space with members for the role to bind. */
    readonly accountId = schema.identifier("account").parse(`account-${v7()}`);
    /** The space of the bucket. */
    readonly spaceId = schema.identifier("space").parse(`space-${v7()}`);
    /** The bucket resource. */
    readonly bucketId = schema.identifier("bucket").parse(`bucket-${v7()}`);
    /** The member calling the bucket's methods. */
    readonly userId = schema.identifier("user").parse(`user-${v7()}`);
    /** The role granting the bucket permissions in the space. */
    readonly roleId = schema.identifier("role").parse(`role-${v7()}`);
    /** The migrated spaces database. */
    readonly database: DatabaseConnection;
    /** The bucket's files. */
    readonly files: LocalBucket;
    /** The credentials presigning transfers. */
    readonly credentials = { accessKeyId: ACCESS_KEY_ID, secretAccessKey: SECRET_ACCESS_KEY };
    /** The S3 server over the bucket for presigned transfers. */
    readonly s3: S3Server;
    /** The hosted object server. */
    readonly server: Server;
    /** The member's client. */
    readonly client: Client<ObjectProcedures<typeof bucketObject.bucket>>;
    /** The authenticated caller, replaced by scenarios acting as others. */
    caller: Authentication;
    /** The audit history the journal delivers calls to. */
    readonly #history: AuditCall[] = [];
    /** Release the database and bucket directory. */
    readonly #close: () => Promise<void>;

    /** Retain the provisioned state. */
    private constructor(
        database: DatabaseConnection,
        files: LocalBucket,
        directory: DatabaseConnection,
        close: () => Promise<void>,
    ) {
        // retain the storage and the member's identity
        this.database = database;
        this.files = files;
        this.#close = close;
        this.caller = this.member();

        // serve the bucket over S3 under the presigning credentials
        this.s3 = new S3Server({
            region: REGION,
            credentials: async (accessKeyId) =>
                accessKeyId === this.credentials.accessKeyId ? this.credentials : undefined,
            open: async (name) => (name === BUCKET_NAME ? this.files : undefined),
        });

        // serve the bucket's files under the member's bearer identity
        const objects = this.#serveObjects(database, directory);
        this.server = Server.start({
            ...objects.implement(bucketService),
            audience: bucketService.package.id,
            resources: new ResourceContext(),
            health: new Health("files"),
            authenticate: async (request) => {
                if (request.headers.get("authorization") !== `Bearer ${this.userId}`) {
                    throw new ServiceError("UNAUTHORIZED", {
                        message: "invalid bearer credential",
                    });
                }

                return this.caller;
            },
            authorizeMachine: async () => {},
            drainTimeout: 1000,
        });
        this.client = bucketObject.bucket.connect(bucketService, {
            url: "https://files.test",
            headers: { authorization: `Bearer ${this.userId}` },
            fetch: (request) => this.server.fetch(request),
        });
    }

    /** Serve the bucket object over the spaces and directory databases. */
    #serveObjects(database: DatabaseConnection, directory: DatabaseConnection): ObjectServer {
        return new ObjectServer({
            objects: {
                bucket: serveBuckets({
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
            },
            policies: [space],
            directory: new DirectoryStore(directory),
            database,
            callKey: testCallKey,
            origin: {
                package: bucketService.package,
                service: bucketService.name,
            },
            history: { ingest: async (batch) => this.#history.push(...batch.calls) },
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
            { ...accountObject.account.reference(this.userId, this.accountId), relation: "editor" },
        ]);
    }

    /** Authenticate a subject with the given subjects and claims. */
    authenticate(
        subject: Subject,
        subjects: Subject[],
        claims: Partial<AuthenticationClaims> = {},
    ): Authentication {
        return new Authentication({
            credential: { kind: "fixture", id: "fixture-1" },
            audience: bucketService.package.id,
            verifiedAt: Date.now(),
            expiresAt: Date.now() + 60_000,
            subject,
            subjects,
            ...claims,
        });
    }

    /** Read the audited calls of the fixture's member and installations, oldest first, as the history keeps them. */
    async audits(): Promise<AuditCall[]> {
        // join the delivered and waiting calls once
        const calls = new Map(
            [...this.#history, ...(await new Journal(this.database, testCallKey).read())].map(
                (call) => [call.execution.id, AuditHistory.kept(call)],
            ),
        );

        // keep the fixture's own callers, ordered by their time-ordered identifiers
        return [...calls.values()]
            .filter(({ execution }) => {
                const caller = execution.context.caller;
                const subject = caller.type === "subject" ? caller.subject : undefined;

                return subject?.id === this.userId || subject?.scope === this.spaceId;
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
        const signature = new S3Signature({ region: REGION });
        const presigned = await signature.presign(
            request,
            this.credentials,
            LEASE_LIFETIME,
            Date.now(),
        );

        // read the signature from the query
        const signed = new URL(presigned.url).searchParams.get("X-Amz-Signature");
        if (signed === null) {
            throw new TypeError("a presigned url carries no signature");
        }

        return signed;
    }

    /** Open the fixture's bucket, the only one its host serves. */
    async #open(reference: BucketReference): Promise<LocalBucket> {
        if (reference.scope !== this.spaceId || reference.bucketId !== this.bucketId) {
            throw new ServiceError("NOT_FOUND", { message: `no bucket ${reference.bucketId}` });
        }

        return this.files;
    }

    /** Grant the role a bucket permission in the space. */
    async grant(name: string): Promise<void> {
        await this.database.insert(accessRolePermission).values({
            id: schema.identifier("role-permission").parse(`role-permission-${v7()}`),
            roleId: this.roleId,
            scope: this.spaceId,
            packageId: bucketObject.bucket.policy.definition.packageId,
            type: bucketObject.bucket.name,
            name,
        });
    }

    /** Withdraw a bucket permission from the role. */
    async withdraw(name: string): Promise<void> {
        await this.database
            .delete(accessRolePermission)
            .where(
                and(
                    eq(accessRolePermission.roleId, this.roleId),
                    eq(accessRolePermission.type, bucketObject.bucket.name),
                    eq(accessRolePermission.name, name),
                ),
            );
    }

    /** Bind the role to a subject in the space, or to a delegate acting for a principal. */
    async bind(subject: Subject, onBehalfOf?: Subject): Promise<void> {
        await this.database.insert(accessRelationship).values(
            Relationship.encode(
                {
                    id: `relationship-${v7()}`,
                    object: spaceObject.space.reference(this.accountId, this.spaceId),
                    role: this.roleId,
                    subject,
                    createdAt: Date.now(),
                    expiresAt: null,
                    ...(onBehalfOf === undefined ? {} : { conditions: { onBehalfOf } }),
                },
                this.spaceId,
            ),
        );
    }

    /** Relate an installation of the package to the bucket as a consumer, as its consumption does. */
    async install() {
        // relate a new installation to the bucket as a consumer
        const now = Date.now();
        const installationId = schema.identifier("installation").parse(`installation-${v7()}`);
        await this.database.insert(accessRelationship).values(
            Relationship.encode(
                {
                    id: `relationship-${v7()}`,
                    object: bucketObject.bucket.reference(this.spaceId, this.bucketId),
                    relation: "consumer",
                    subject: principal.installation.reference(this.spaceId, installationId),
                    createdAt: now,
                    expiresAt: null,
                },
                this.spaceId,
            ),
        );

        return installationId;
    }

    /** Open the migrated bucket service database of each dialect for a file's scenarios. */
    static async databases(): Promise<Map<Dialect, TestDatabase>> {
        const opened = new Map<Dialect, TestDatabase>();
        for (const dialect of TEST_DIALECTS) {
            opened.set(
                dialect,
                await TestDatabase.create(dialect, bucketDatabase, {
                    isMigrated: true,
                }),
            );
        }

        return opened;
    }

    /** Provision a new space, its bucket and a member role with every bucket permission. */
    static async open(database: DatabaseConnection): Promise<BucketFixture> {
        // open the bucket's directory, and the universe's directory keeping its name claims
        const directory = await mkdtemp(join(tmpdir(), "destack-bucket-files-"));
        const files = await LocalBucket.open(directory, "space-test");
        const claims = await TestDatabase.create("sqlite", directoryTables, { isMigrated: true });
        const release = async () => {
            await files[Symbol.asyncDispose]();
            await claims.close();
            await rm(directory, { recursive: true });
        };

        // serve the files, releasing the bucket when it fails to start
        let fixture: BucketFixture;
        try {
            fixture = new BucketFixture(database, files, claims.database, release);
        } catch (error) {
            await release();
            throw error;
        }

        // provision the space, closing everything when that fails
        try {
            await fixture.#provision();

            return fixture;
        } catch (error) {
            await fixture.close();
            throw error;
        }
    }

    /** Record the space, its bucket and the member's role as the space controller would. */
    async #provision(): Promise<void> {
        // record the space and its bucket
        const now = Date.now();
        await this.#record(this.spaceId, this.bucketId, "bucket-fixture");

        // grant every bucket permission to the account's members through a role in the space
        await this.database.insert(accessRole).values({
            id: this.roleId,
            scope: this.spaceId,
            name: "bucket-owner",
            description: "use the fixture bucket",
            createdAt: now,
            updatedAt: now,
        });
        await this.bind({
            ...accountObject.account.reference(this.userId, this.accountId),
            relation: "editor",
        });
        for (const name of Object.keys(bucketObject.bucket.policy.definition.permissions)) {
            await this.grant(name);
        }
    }

    /** Record another space of the account with its own bucket and no role, and select it. */
    async neighbour() {
        // record the space and its bucket under new identifiers
        const spaceId = schema.identifier("space").parse(`space-${v7()}`);
        const bucketId = schema.identifier("bucket").parse(`bucket-${v7()}`);
        await this.#record(spaceId, bucketId, "neighbour");

        return { spaceId, id: bucketId };
    }

    /** Record a space of the account below its scope, with one bucket resource. */
    async #record(
        spaceId: Identifier<"space">,
        bucketId: Identifier<"bucket">,
        name: string,
    ): Promise<void> {
        // record the space below the account
        const now = Date.now();
        const record = { createdAt: now, updatedAt: now };
        await this.database
            .insert(space.table)
            .values({ ...record, id: spaceId, scope: this.accountId, name });
        await this.database.insert(Scope.table).values({
            scope: spaceId,
            parent: this.accountId,
            packageId: spaceObject.space.policy.definition.packageId,
            type: spaceObject.space.name,
            ancestors: [this.accountId],
        });

        // record its bucket
        await this.database.insert(bucketObject.bucket.table).values({
            ...record,
            id: bucketId,
            scope: spaceId,
            name: "files",
            definitionPackageId: bucketObject.bucket.policy.definition.packageId,
            definitionVersion: PACKAGE.version,
            definitionName: "files",
            spec: {},
        });
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
