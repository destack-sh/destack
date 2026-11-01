import type { CatalogueBucketHost } from "@destack/bucket/provider";
import type { Connector } from "@destack/resource";
import { Entitlement } from "@destack/account/object";
import { SettingValue } from "@destack/setting/object";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { LocalBlobStorage, LocalBucket, LocalBucketHost } from "@destack/bucket/bun";
import type { S3Credentials } from "@destack/bucket/s3";
import { databaseProvider, Tracker } from "@destack/db";
import { BunSqliteDatabaseHost, connectBunSqlite } from "@destack/db/bun";
import type { SqliteDatabase } from "@destack/db/sqlite";
import type { Environment } from "@destack/environment";
import { DatabasePersonalKeyring, EventStore } from "@destack/event";
import { LocalGitStorage, localGitProvider } from "@destack/forge/local";
import { gitStorage } from "@destack/forge/storage";
import { type MailDeclaration, mailProvider, type MailTransport } from "@destack/message/email";
import { mail } from "@destack/message/object";
import type { PackageImplementation } from "@destack/object/server";
import type { Keyring } from "@destack/identity";
import { type Identifier, schema } from "@destack/schema";
import { BunGraphStorage } from "@destack/stack/bun";
import * as stackObject from "@destack/stack/object";
import { type SpaceBuildStore, SocketRegistry, type SpaceOptions } from "@destack/stack/server";
import { Traffic } from "../gateway/index.ts";
import { Reading } from "../meter/index.ts";
import { S3Host, SpaceDatabaseHost, SpaceStorage, type Volume } from "../space/index.ts";

/** The memory a local instance's run time counts in GB-seconds at: 128 MB. */
const INSTANCE_GIGABYTES = 0.125;

/** The host name serving a machine's buckets over S3 (RFC 6761 6.3). */
const S3_HOST = "s3.localhost";

/** The region a machine's S3 requests are signed for. */
const S3_REGION = "local";

/** What a computer stores its spaces with. */
export interface LocalSpaceStorageOptions {
    /** The directory of the spaces' databases, buckets, events and snapshots. */
    readonly directory: string;
    /** The URL of the machine's listener, whose port the spaces' S3 host names answer at. */
    readonly endpoint: string;
    /** The machine's root keys, wrapping the spaces' personal keys. */
    readonly keyring: Keyring;
    /** The host the spaces run on: its clock, randomness, crypto, network, files and git. */
    readonly environment: Environment<"files" | "process" | "host" | "server">;
    /** The servers the machine runs in process, whose types the spaces store. */
    readonly servers: readonly PackageImplementation[];
    /** The transport the spaces' mail goes out through, such as printing it. */
    readonly mail: Connector<MailTransport, MailDeclaration>;
    /** Report a failure of the spaces' sockets. */
    readonly report: (error: unknown) => void;
}

/** The spaces' storage on a computer: SQLite files, local buckets and Git, under one directory. */
export class LocalSpaceStorage implements SpaceStorage, AsyncDisposable {
    /** The main database every space stores. */
    readonly main: SpaceStorage["main"];
    /** The event kinds every space records. */
    readonly kinds: SpaceStorage["kinds"];
    /** The servers' ephemeral object types. */
    readonly ephemeral: SpaceStorage["ephemeral"];
    /** What the host stores the spaces with. */
    readonly #options: LocalSpaceStorageOptions;
    /** The databases of the spaces and their installations, as SQLite files. */
    readonly #databases: BunSqliteDatabaseHost;
    /** The live sockets of the spaces, in memory. */
    readonly #sockets: SocketRegistry;
    /** The providers of the spaces' resources: SQLite files, Git and mail. */
    readonly #providers: SpaceOptions["providers"];

    /** Take the stores the host opened for the spaces. */
    private constructor(
        options: LocalSpaceStorageOptions,
        opened: Pick<SpaceStorage, "main" | "kinds" | "ephemeral"> & {
            readonly databases: BunSqliteDatabaseHost;
            readonly sockets: SocketRegistry;
            readonly providers: SpaceOptions["providers"];
        },
    ) {
        // set the options, what every space stores and the stores the host opened
        this.#options = options;
        this.main = opened.main;
        this.kinds = opened.kinds;
        this.ephemeral = opened.ephemeral;
        this.#databases = opened.databases;
        this.#sockets = opened.sockets;
        this.#providers = opened.providers;
    }

    /** Open the spaces' stores under a directory. */
    static async open(options: LocalSpaceStorageOptions): Promise<LocalSpaceStorage> {
        // provide the spaces' databases as SQLite files below the directory, Git and mail
        const databases = new BunSqliteDatabaseHost(
            pathToFileURL(`${options.directory}/`),
            options.environment,
        );
        const providers = [
            databaseProvider(databases, stackObject.database),
            databaseProvider(
                new SpaceDatabaseHost(databases, (spaceId) => LocalSpaceStorage.#reference(spaceId)),
                stackObject.database,
            ),
            localGitProvider(join(options.directory, "git"), gitStorage, options.environment),
            mailProvider(options.mail.code, mail),
        ];
        const declared = SpaceStorage.declare(options.servers);

        // store the spaces' live sockets and their other ephemeral objects in memory
        const sockets = await SocketRegistry.open({
            objects: declared.ephemeral,
            environment: options.environment,
            tracker: new Tracker(options.environment, undefined),
            memory: (tables) =>
                connectBunSqlite(":memory:", tables, { environment: options.environment }),
            report: options.report,
        });

        return new LocalSpaceStorage(options, { ...declared, databases, sockets, providers });
    }

    /** List the spaces whose database files the host stores. */
    async list(): Promise<readonly Identifier<"space">[]> {
        // list none before the first space's file
        const { files } = this.#options.environment;
        const directory = this.#directory("databases");
        const entries = (await files.exists(directory)) ? await files.readDirectory(directory) : [];

        return entries
            .map((entry) => entry.name)
            .filter((file) => file.endsWith(".db"))
            .map((file) => schema.identifier("space").parse(file.slice(0, -3)));
    }

    /** Open a space's volume: its database file migrated to its tables, its events and blobs. */
    async open(space: Identifier<"space">): Promise<LocalVolume> {
        // open and migrate the space's database file, closed last
        const { environment, keyring } = this.#options;
        const lifetime = new AsyncDisposableStack();
        try {
            await environment.files.mkdir(this.#directory("databases"), { recursive: true });
            const definition = this.main;
            const file = fileURLToPath(this.#databases.resolve(LocalSpaceStorage.#reference(space)));
            const database = await connectBunSqlite(file, definition, { environment });
            lifetime.defer(() => database.close());
            await database.migrate(definition.tables, { others: "retain" });

            // open its events, their segments in a bucket of the space's own
            const segments = lifetime.use(
                await LocalBucket.open(join(this.#directory("events"), space), space, environment),
            );
            const events = new EventStore({
                clock: environment.clock,
                database,
                kinds: this.kinds,
                files: () => segments,
                personal: new DatabasePersonalKeyring(database, keyring, environment),
                settings: Entitlement.capRetention(database, SettingValue.retention(database)),
            });

            return new LocalVolume({
                space,
                options: this.#options,
                databases: this.#databases,
                lifetime,
                database,
                events,
                blobs: await LocalBlobStorage.open(
                    join(this.#options.directory, "blobs", space),
                    environment,
                ),
                sockets: this.#sockets,
                providers: this.#providers,
                connectors: [
                    LocalGitStorage.connector(this.#directory("git"), environment),
                    this.#options.mail,
                ],
            });
        }
        // close what the open opened when it fails
        catch (error) {
            await lifetime.disposeAsync();
            throw error;
        }
    }

    /** Close the spaces' sockets. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#sockets[Symbol.asyncDispose]();
    }

    /** Refer to a space's own database file below the host's root. */
    static #reference(space: Identifier<"space">): string {
        return `databases/${space}.db`;
    }

    /** Locate one of the host's directories. */
    #directory(name: "databases" | "events" | "buckets" | "git"): string {
        return join(this.#options.directory, name);
    }
}

/** A space's volume on a computer: its SQLite file, events, blobs, buckets and graphs. */
export class LocalVolume implements Volume {
    /** The space's database file. */
    readonly database: SqliteDatabase;
    /** The space's events, their segments in a bucket of its own. */
    readonly events: EventStore;
    /** The space's blobs, in a directory of its own. */
    readonly blobs: LocalBlobStorage;
    /** The machine's one registry of live sockets. */
    readonly sockets: SpaceOptions["sockets"];
    /** The providers of the space's resources: SQLite files, Git and mail. */
    readonly providers: SpaceOptions["providers"];
    /** The connectors of the space's Git storages and its mail. */
    readonly connectors: readonly Connector[];
    /** The space. */
    readonly #space: Identifier<"space">;
    /** What the host stores the spaces with. */
    readonly #options: LocalSpaceStorageOptions;
    /** The databases of the space's installations, measured by their files. */
    readonly #databases: BunSqliteDatabaseHost;
    /** What the volume opened, closed once the space stops. */
    readonly #lifetime: AsyncDisposableStack;
    /** The space's buckets, once served. */
    #buckets: LocalBucketHost | undefined;
    /** The graph indexes of the space's builds, once read. */
    #graph: BunGraphStorage | undefined;

    /** Take what the host opened for a space. */
    constructor(
        fields: Pick<
            LocalVolume,
            "database" | "events" | "blobs" | "sockets" | "providers" | "connectors"
        > & {
            readonly space: Identifier<"space">;
            readonly options: LocalSpaceStorageOptions;
            readonly databases: BunSqliteDatabaseHost;
            readonly lifetime: AsyncDisposableStack;
        },
    ) {
        // set the space's stores, then what measures and closes them
        this.database = fields.database;
        this.events = fields.events;
        this.blobs = fields.blobs;
        this.sockets = fields.sockets;
        this.providers = fields.providers;
        this.connectors = fields.connectors;
        this.#space = fields.space;
        this.#options = fields.options;
        this.#databases = fields.databases;
        this.#lifetime = fields.lifetime;
    }

    /** Open the space's buckets once below its own S3 host name. */
    buckets(credentials: (scope: string) => Promise<S3Credentials | undefined>): LocalBucketHost {
        // answer S3 at the listener's port below the space's own host name
        const endpoint = new URL(this.#options.endpoint);
        endpoint.hostname = S3Host.of(this.#space, S3_HOST);
        this.#buckets ??= this.#lifetime.use(
            new LocalBucketHost({
                directory: join(this.#options.directory, "buckets"),
                endpoint,
                region: S3_REGION,
                credentials,
                environment: this.#options.environment,
            }),
        );

        return this.#buckets;
    }

    /** Open the graphs of the space's builds once, each attached to its build. */
    graphs(
        builds: Pick<SpaceBuildStore, "store" | "contains" | "contents" | "referrer" | "file">,
    ): BunGraphStorage {
        this.#graph ??= this.#lifetime.use(
            new BunGraphStorage(this.database, builds, this.#options.environment.files),
        );

        return this.#graph;
    }

    /** Read what the space used: its run time, databases, buckets and traffic. */
    async measure(
        buckets: Pick<CatalogueBucketHost, "bytes">,
        from: number,
        to: number,
        traffic: Traffic,
    ): Promise<Reading[]> {
        // read the run time and each provisioned database's and bucket's bytes
        const { database } = this;

        return [
            ...(await Reading.instances(database, from, to, INSTANCE_GIGABYTES)),
            ...(await Reading.databases(database, (reference) => this.#databases.bytes(reference))),
            ...(await Reading.buckets(database, (bucketId) => buckets.bytes({ bucketId }))),
            ...Traffic.readings(traffic.take()),
        ];
    }

    /** Close the space's graphs, buckets, events and database. */
    async close(): Promise<void> {
        await this.#lifetime.disposeAsync();
    }
}
