import { CLOUD_RUNTIMES } from "./cloud.ts";
import { MachineError } from "../error/index.ts";
import type { CatalogueBucketHost } from "@destack/bucket/provider";
import { BuildStore } from "@destack/build/store";
import type { BuildLayer } from "@destack/package/manifest";
import type { Connector } from "@destack/resource";
import type * as Cloudflare from "@cloudflare/workers-types";
import { Entitlement } from "@destack/account/object";
import { SettingValue } from "@destack/setting/object";
import { DatabasePersonalKeyring, EventStore } from "@destack/event";
import { bucketOperation, databaseOperation } from "@destack/telemetry/meter";
import { DurableObjectCatalogueStore, R2Bucket, R2BucketHost } from "@destack/bucket/cloudflare";
import { BucketBlobStorage } from "@destack/bucket/blob";
import { S3Origin } from "@destack/account/address";
import type { S3Credentials } from "@destack/bucket/s3";
import {
    BlobStore,
    channelHub,
    databaseProvider,
    type DatabaseConnection,
    Tracker,
} from "@destack/db";
import {
    DurableObjectDatabaseHost,
    type DurableObjectStorage,
    Jurisdiction,
} from "@destack/db/cloudflare";
import {
    type ArtifactsNamespace,
    ArtifactsStorage,
    artifactsProvider,
} from "@destack/git/cloudflare";
import { gitStorage } from "@destack/stack/object";
import { mailProvider, type MailTransport } from "@destack/mail/transport";
import type { Keyring } from "@destack/identity";
import type { Identifier } from "@destack/schema";
import { DurableObjectGraphStorage } from "@destack/stack/cloudflare";
import * as stackObject from "@destack/stack/object";
import { BuildCache, SocketRegistry, type SpaceOptions } from "@destack/stack/server";
import type { Environment } from "@destack/environment";
import { SpaceDatabaseHost, SpaceStorage, type Volume } from "../space/index.ts";
import { type Count, Reading } from "../meter/index.ts";
import { Traffic } from "../gateway/index.ts";
import { CACHE_NAMESPACE, DurableObjectSpaceDatabaseHost, SPACE_NAMESPACE } from "./database.ts";
import { DistributionManifest } from "@destack/package";
import type { DurableObjectNamespace } from "./runtime/index.ts";

/** The prefix of the machines' build caches in the residency's build bucket, a space's below it. */
const CACHE_PREFIX = "cache/";
/** The namespace of the space's live sockets, stored only while the object runs. */
const SOCKETS_NAMESPACE = "sockets";

/** The region R2 and a space's buckets sign their S3 requests for. */
const S3_REGION = "auto";

/** The prefix of the spaces' blobs in the residency's files, each below its space. */
const BLOBS_PREFIX = "blobs";

/** What a space's object stores the space with. */
export interface DurableObjectSpaceStorageOptions {
    /** The space the object serves. */
    readonly space: Identifier<"space">;
    /** The object's storage, with a namespace per database it stores. */
    readonly storage: DurableObjectStorage;
    /** The residency's R2 bucket of the spaces' buckets, snapshots and event segments. */
    readonly files: Cloudflare.R2Bucket;
    /** The name of the residency's R2 bucket. */
    readonly filesName: string;
    /** The domain of the installation origins, which each space's S3 origin answers below. */
    readonly domain: string;
    /** The residency's Artifacts namespace of the spaces' Git storages. */
    readonly artifacts: ArtifactsNamespace;
    /** The facet of an installation in the space's object, which stores its databases. */
    readonly facet: (installation: string) => {
        readonly fetch: (request: Request) => Promise<Response>;
    };
    /** The builds' graph index objects, one per manifest. */
    readonly graphs: DurableObjectNamespace;
    /** The jurisdiction of the objects, empty for none. */
    readonly jurisdiction: Jurisdiction;
    /** The rate of a gigabyte-second of the space's instances' memory. */
    readonly gigabytes: Parameters<typeof Reading.instances>[3];
    /** The machine's root keys, sealing personal event data. */
    readonly keyring: Keyring;
    /** Send a request as the machine, as Artifacts calls go out. */
    readonly send: (request: Request) => Promise<Response>;
    /** The transport the space's mail goes out through. */
    readonly mail: Connector<MailTransport>;
    /** Report a failure of the space's background work. */
    readonly report: (error: unknown) => void;
    /** The environment the spaces run in: its clock, randomness, crypto and network. */
    readonly environment: Environment;
}

/** A space's storage in its Durable Object, opening the space's volume while it runs. */
export class DurableObjectSpaceStorage implements SpaceStorage {
    /** The main database the space stores. */
    readonly main: SpaceStorage["main"];
    /** The event kinds the space records. */
    readonly kinds: SpaceStorage["kinds"];
    /** The servers' ephemeral object types. */
    readonly ephemeral: SpaceStorage["ephemeral"];
    /** What the object stores the space with. */
    readonly #options: DurableObjectSpaceStorageOptions;
    /** The databases in the object's one storage. */
    readonly #host: DurableObjectDatabaseHost;
    /** The space's database, in the object's storage across the space's starts. */
    readonly database: DatabaseConnection;
    /** The installations' databases, each in its partition's object. */
    readonly databases: DurableObjectSpaceDatabaseHost;
    /** The residency's bucket of the space's buckets, snapshots and event segments. */
    readonly #files: R2Bucket;

    /** Take a space over its object's migrated database. */
    private constructor(
        options: DurableObjectSpaceStorageOptions,
        declared: Pick<SpaceStorage, "main" | "kinds" | "ephemeral">,
        host: DurableObjectDatabaseHost,
        database: DatabaseConnection,
    ) {
        // set the space, what it stores, the object's storage and the installations' objects
        this.#options = options;
        this.main = declared.main;
        this.kinds = declared.kinds;
        this.ephemeral = declared.ephemeral;
        this.#host = host;
        this.database = database;
        this.#files = new R2Bucket(options.files);
        this.databases = new DurableObjectSpaceDatabaseHost(
            database,
            options.facet,
            host,
            options.environment.random,
        );
    }

    /** Open a space's storage in its object, migrating the space's database. */
    static async open(
        options: DurableObjectSpaceStorageOptions,
    ): Promise<DurableObjectSpaceStorage> {
        // open the space's database in its namespace of the object's one storage host
        const declared = SpaceStorage.declare();
        const host = new DurableObjectDatabaseHost(options.storage, options.environment);
        const database = host.connect(declared.main, {
            namespace: SPACE_NAMESPACE,
            openChannel: channelHub(),
        });
        await database.migrate(declared.main.tables, { others: "retain" });

        return new DurableObjectSpaceStorage(options, declared, host, database);
    }

    /** Open the machine's build cache in the object's storage, over the shipped releases. */
    async cache(
        builds: Cloudflare.R2Bucket,
        releases: BuildLayer,
        distribution: DistributionManifest,
    ): Promise<BuildCache> {
        // index the pulled builds in the object's storage, their blobs in the build bucket, the shipped ones read in place
        const { environment, space } = this.#options;
        const database = this.#host.connect(BuildStore.database.tables, {
            namespace: CACHE_NAMESPACE,
        });
        await database.migrate(BuildStore.database.tables, { others: "retain" });
        const storage = new BucketBlobStorage(
            new R2Bucket(builds),
            `${CACHE_PREFIX}${space}/`,
            environment.random,
        );
        const blobs = new BlobStore({ storage, database, environment });

        return new BuildCache(
            new BuildStore({ blobs, environment, layer: releases }),
            CLOUD_RUNTIMES,
            DistributionManifest.builtins(distribution, "workerd"),
        );
    }

    /** List the one space the object stores. */
    async list(): Promise<readonly Identifier<"space">[]> {
        return [this.#options.space];
    }

    /** Open the volume of the space the object stores, refusing any other space. */
    async open(space: Identifier<"space">): Promise<DurableObjectVolume> {
        // refuse a space other than the object's
        if (space !== this.#options.space) {
            throw MachineError.of(
                "SPACE_NOT_FOUND",
                `space '${space}' in the object of '${this.#options.space}' not found`,
            );
        }

        // open the registry of the space's sockets, its memory fresh in the object's storage
        const host = this.#host;
        const sockets = await SocketRegistry.open({
            objects: this.ephemeral,
            environment: this.#options.environment,
            tracker: new Tracker(this.#options.environment, undefined),
            memory: async (tables) => {
                await host.drop(SOCKETS_NAMESPACE);

                return host.connect(tables, { namespace: SOCKETS_NAMESPACE });
            },
            report: (error) => this.#options.report(error),
        });

        return new DurableObjectVolume({
            options: this.#options,
            kinds: this.kinds,
            host,
            database: this.database,
            databases: this.databases,
            files: this.#files,
            sockets,
        });
    }

    /** Serve a page of a database replica an installation's object copies. */
    content(request: Request): Promise<Response> {
        return this.databases.content(request);
    }
}

/** A space's volume in its Durable Object: its database, events, blobs, buckets and sockets. */
export class DurableObjectVolume implements Volume {
    /** The space's database. */
    readonly database: DatabaseConnection;
    /** The space's events, their segments in the residency's files. */
    readonly events: EventStore;
    /** The space's blobs, below its prefix of the residency's files. */
    readonly blobs: BucketBlobStorage;
    /** The registry of the space's sockets and presence in the object's storage. */
    readonly sockets: SocketRegistry;
    /** The providers of the space's databases in objects, Git in Artifacts and mail. */
    readonly providers: SpaceOptions["providers"];
    /** The connectors of the space's Git storages in Artifacts, and its mail. */
    readonly connectors: readonly Connector[];
    /** What the object stores the space with. */
    readonly #options: DurableObjectSpaceStorageOptions;
    /** The databases in the object's one storage. */
    readonly #host: DurableObjectDatabaseHost;
    /** The installations' databases, each in its partition's object. */
    readonly #databases: DurableObjectSpaceDatabaseHost;
    /** The residency's bucket of the space's buckets, snapshots and event segments. */
    readonly #files: R2Bucket;
    /** The space's buckets, once served. */
    #buckets: R2BucketHost | undefined;
    /** The graph indexes of the space's builds, once read. */
    #graph: DurableObjectGraphStorage | undefined;

    /** Take what the object opened for its space. */
    constructor(fields: {
        readonly options: DurableObjectSpaceStorageOptions;
        readonly kinds: SpaceStorage["kinds"];
        readonly host: DurableObjectDatabaseHost;
        readonly database: DatabaseConnection;
        readonly databases: DurableObjectSpaceDatabaseHost;
        readonly files: R2Bucket;
        readonly sockets: SocketRegistry;
    }) {
        // set the object's stores
        const { options, database } = fields;
        this.#options = options;
        this.#host = fields.host;
        this.#databases = fields.databases;
        this.#files = fields.files;
        this.database = database;
        this.sockets = fields.sockets;

        // store the space's events and blobs in the residency's files
        const { environment, space } = options;
        this.events = new EventStore({
            clock: environment.clock,
            database,
            kinds: fields.kinds,
            files: () => fields.files,
            personal: new DatabasePersonalKeyring(database, options.keyring, environment),
            settings: Entitlement.capRetention(database, SettingValue.retention(database)),
        });
        this.blobs = new BucketBlobStorage(
            fields.files,
            `${BLOBS_PREFIX}/${space}/`,
            environment.random,
        );

        // provide databases in objects, Git in Artifacts and mail
        const { artifacts, send, mail: transport } = options;
        this.providers = [
            databaseProvider(fields.databases, stackObject.database),
            databaseProvider(
                new SpaceDatabaseHost(fields.host, () =>
                    DurableObjectDatabaseHost.reference(SPACE_NAMESPACE),
                ),
                stackObject.database,
            ),
            artifactsProvider(artifacts, gitStorage, send),
            mailProvider(transport.code, stackObject.mail),
        ];
        this.connectors = [ArtifactsStorage.connector(artifacts, send), transport];
    }

    /** Open the space's buckets once in the residency's bucket, catalogued in its object. */
    buckets(credentials: (scope: string) => Promise<S3Credentials | undefined>): R2BucketHost {
        // answer S3 at the space's own host name
        const endpoint = new URL(
            `https://${S3Origin.format(this.#options.space, this.#options.domain)}`,
        );

        // store the buckets' files in the residency's bucket, counting its operations for the meters
        this.#buckets ??= new R2BucketHost({
            files: this.#files,
            name: this.#options.filesName,
            catalogues: new DurableObjectCatalogueStore(this.#host),
            endpoint,
            region: S3_REGION,
            credentials,
            environment: this.#options.environment,
        });

        return this.#buckets;
    }

    /** Open the graphs of the space's builds once, one object per manifest. */
    graphs(
        builds: Pick<BuildStore, "contains" | "contents" | "lock" | "referrer" | "file">,
    ): DurableObjectGraphStorage {
        // index each manifest in an object of the jurisdiction
        const indexes = Jurisdiction.narrow(this.#options.graphs, this.#options.jurisdiction);
        this.#graph ??= new DurableObjectGraphStorage(this.database, builds, indexes);

        return this.#graph;
    }

    /** Read what the space used over an interval, each operation on its meter. */
    async measure(
        buckets: Pick<CatalogueBucketHost, "bytes">,
        from: number,
        to: number,
        traffic: Traffic,
    ): Promise<Reading[]> {
        // measure the space's object and each installation's object once, with the rows they read and wrote
        const { database } = this;
        const { space } = this.#options;
        const reads: Count[] = [];
        const writes: Count[] = [];
        const objectBytes = async (
            counted: Omit<Count, "value">,
            installed?: Parameters<DurableObjectSpaceDatabaseHost["measure"]>[0],
        ) => {
            // measure the object and add the rows it read and wrote
            const object = await this.#databases.measure(installed);
            reads.push({ ...counted, value: object.rowsRead });
            writes.push({ ...counted, value: object.rowsWritten });

            return object.bytes;
        };

        // read the run time and the bytes stored
        const readings = [
            ...(await Reading.instances(database, from, to, this.#options.gigabytes)),
            ...(await Reading.space(space, () => objectBytes({ scope: space }))),
            ...(await Reading.installations(database, (installed) =>
                objectBytes(
                    { scope: installed.scope, installation: installed.installationId },
                    installed,
                ),
            )),
            ...(await Reading.buckets(database, (bucketId) => buckets.bytes({ bucketId }))),
        ];

        // take the counts in memory once every read that may fail has answered
        const operations = this.#files.takeOperations();
        const taken = traffic.take();

        return [
            ...readings,
            ...Reading.counted(databaseOperation, { direction: "read" }, reads),
            ...Reading.counted(databaseOperation, { direction: "write" }, writes),
            ...Reading.counted(bucketOperation, { direction: "write" }, [
                { scope: space, value: operations.classA },
            ]),
            ...Reading.counted(bucketOperation, { direction: "read" }, [
                { scope: space, value: operations.classB },
            ]),
            ...Traffic.readings(taken),
        ];
    }

    /** Close the space's socket registry once it stops, as the object's storage outlives it. */
    async close(): Promise<void> {
        await this.sockets[Symbol.asyncDispose]();
    }
}
