import type {} from "@destack/package/import-meta";
import { TelemetryScope } from "@destack/telemetry";
import { type SpaceProvider, gitStorage } from "@destack/stack/object";
import type { CatalogueBucketHost } from "@destack/bucket/provider";
import type { Connector } from "@destack/resource";
import { capRetention } from "../space/entitlement.ts";
import { SettingValue } from "@destack/setting/object";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { LOOPBACK_DOMAIN, S3Origin } from "@destack/account/address";
import { LocalBlobStorage, LocalBucket, LocalBucketHost } from "@destack/bucket/bun";
import type { S3Credentials } from "@destack/bucket/s3";
import {
    type DatabaseCluster,
    type DatabaseConnection,
    databaseProvider,
    Tracker,
} from "@destack/db/database";
import { BunSqliteDatabaseHost, connectBunSqlite } from "@destack/db/bun";
import { PostgresDatabaseHost } from "@destack/db/postgres";
import type { Environment } from "@destack/environment";
import { DatabasePersonalKeyring, EventStore } from "@destack/event/store";
import { LocalGitStorage, localGitProvider } from "@destack/git/local";
import { mailProvider, type MailTransport } from "@destack/mail/transport";
import type { Keyring } from "@destack/identity";
import { type Identifier, schema } from "@destack/schema";
import { BunGraphStorage } from "./graph.ts";
import * as stackObject from "@destack/stack/object";
import type { BuildStore } from "@destack/build/store";
import { SocketRegistry } from "../space/socket.ts";
import { Traffic } from "../gateway/index.ts";
import { Reading } from "../meter/index.ts";
import { SpaceDatabaseHost, SpaceStorage, type Volume } from "../space/index.ts";

/** The memory a local instance's run time counts in GB-seconds at: 128 MB. */
const INSTANCE_GIGABYTES = 0.125;

/** The name prefix of a space's identifier, which its schema in a cluster is named by. */
const SPACE_PREFIX = "space-";

/** The region a machine's S3 requests are signed for. */
const S3_REGION = "local";

/** What a computer stores its spaces with. */
export interface LocalSpaceStorageOptions {
    /** The directory of the spaces' databases, buckets, events and snapshots. */
    readonly directory: string;
    /** The URL of the machine's listener, whose port the spaces' S3 origins answer at. */
    readonly endpoint: string;
    /** The machine's root keys, wrapping the spaces' personal keys. */
    readonly keyring: Keyring;
    /** The host the spaces run on: its clock, randomness, crypto, network, files and git. */
    readonly environment: Environment<"files" | "process" | "host" | "server">;
    /** The transport the spaces' mail goes out through, such as printing it. */
    readonly mail: Connector<MailTransport>;
    /** Where the spaces' databases live: SQLite files below the directory, or a cluster. */
    readonly databases: "files" | URL;
}

/** The spaces' storage on a computer: SQLite files or schemas, local buckets and Git. */
export class LocalSpaceStorage implements SpaceStorage, AsyncDisposable {
    /** The main database every space stores. */
    readonly main: SpaceStorage["main"];
    /** The tables every space replicates of packages a space can install. */
    readonly replicated: SpaceStorage["replicated"];
    /** The event kinds every space records. */
    readonly kinds: SpaceStorage["kinds"];
    /** The servers' ephemeral object types. */
    readonly ephemeral: SpaceStorage["ephemeral"];
    /** What the host stores the spaces with. */
    readonly #options: LocalSpaceStorageOptions;
    /** The databases of the spaces and their installations, as SQLite files or schemas. */
    readonly #databases: DatabaseCluster;
    /** The live sockets of the spaces, in memory. */
    readonly #sockets: SocketRegistry;
    /** The providers of the spaces' resources: SQLite files, Git and mail. */
    readonly #providers: readonly SpaceProvider[];

    /** Take the stores the host opened for the spaces. */
    private constructor(
        options: LocalSpaceStorageOptions,
        opened: Pick<SpaceStorage, "main" | "replicated" | "kinds" | "ephemeral"> & {
            readonly databases: DatabaseCluster;
            readonly sockets: SocketRegistry;
            readonly providers: readonly SpaceProvider[];
        },
    ) {
        // set the options, what every space stores and the stores the host opened
        this.#options = options;
        this.main = opened.main;
        this.replicated = opened.replicated;
        this.kinds = opened.kinds;
        this.ephemeral = opened.ephemeral;
        this.#databases = opened.databases;
        this.#sockets = opened.sockets;
        this.#providers = opened.providers;
    }

    /** Open the spaces' stores under a directory. */
    static async open(options: LocalSpaceStorageOptions): Promise<LocalSpaceStorage> {
        // provide the spaces' databases as SQLite files below the directory or a cluster's schemas, Git and mail
        const databases: DatabaseCluster =
            options.databases === "files"
                ? new BunSqliteDatabaseHost(
                      pathToFileURL(`${options.directory}/`),
                      options.environment,
                  )
                : new PostgresDatabaseHost(options.databases, options.environment);
        const providers = [
            databaseProvider(databases, stackObject.database),
            databaseProvider(
                new SpaceDatabaseHost(databases, (spaceId) =>
                    databases.reference(spaceId, spaceId),
                ),
                stackObject.database,
            ),
            localGitProvider(join(options.directory, "git"), gitStorage, options.environment),
            mailProvider(options.mail.code, stackObject.mail),
        ];
        const declared = SpaceStorage.declare();

        // store the spaces' live sockets and their other ephemeral objects in memory
        const sockets = await SocketRegistry.open({
            objects: declared.ephemeral,
            environment: options.environment,
            tracker: new Tracker(options.environment, undefined),
            memory: (tables) =>
                connectBunSqlite(":memory:", tables, { environment: options.environment }),
        });

        return new LocalSpaceStorage(options, { ...declared, databases, sockets, providers });
    }

    /** List the spaces whose own databases the host keeps. */
    async list(): Promise<readonly Identifier<"space">[]> {
        const scopes = await this.#databases.scopes(SPACE_PREFIX);

        return scopes.map((scope) => schema.identifier("space").parse(scope));
    }

    /** Open a space's volume: its database migrated to its tables, its events and blobs. */
    open(space: Identifier<"space">): Promise<LocalVolume> {
        // open the space's volume in its span, timed by the provider
        return TelemetryScope.of(
            import.meta.destack.package,
            this.#options.environment.telemetry,
        ).span(
            "destack.volume.open",
            { "destack.space.id": space, "destack.volume.provider": "local" },
            async () => {
                // open and migrate the space's database, closed last
                const { environment, keyring } = this.#options;
                const lifetime = new AsyncDisposableStack();
                try {
                    const definition = this.main;
                    const database = await this.#connect(space, definition);
                    lifetime.defer(() => database.close());
                    await database.migrate(definition.tables);
                    await database.declareApplied(this.replicated);

                    // open its events, their segments in a bucket of the space's own
                    const segments = lifetime.use(
                        await LocalBucket.open(
                            join(this.#directory("events"), space),
                            space,
                            environment,
                        ),
                    );
                    const events = new EventStore({
                        environment,
                        database,
                        kinds: this.kinds,
                        files: () => segments,
                        personal: new DatabasePersonalKeyring(database, keyring, environment),
                        settings: capRetention(
                            database,
                            SettingValue.retention(database),
                            environment.clock,
                        ),
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
                            { synchronous: "full" },
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
            },
            { duration: { attributes: { "destack.volume.provider": "local" } } },
        );
    }

    /** Delete a closed space's own database, events and blobs. */
    async delete(space: Identifier<"space">): Promise<void> {
        // drop the space's own database and its folder, its other databases dropped by their providers before
        const { files } = this.#options.environment;
        await this.#databases.drop(this.#databases.reference(space, space));
        await files.remove(join(this.#options.directory, space), { recursive: true, force: true });

        // delete its events and blobs
        await files.remove(join(this.#directory("events"), space), {
            recursive: true,
            force: true,
        });
        await files.remove(join(this.#options.directory, "blobs", space), {
            recursive: true,
            force: true,
        });
    }

    /** Close the spaces' sockets and their databases' host. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#sockets[Symbol.asyncDispose]();
        await this.#databases[Symbol.asyncDispose]();
    }

    /** Open a space's own database, created empty once, its log started by its migration. */
    async #connect(
        space: Identifier<"space">,
        definition: SpaceStorage["main"],
    ): Promise<DatabaseConnection & { readonly close: () => Promise<void> }> {
        const reference = this.#databases.reference(space, space);
        await this.#databases.create(reference);

        return await this.#databases.connect(reference, definition);
    }

    /** Locate one of the host's directories. */
    #directory(name: "events" | "buckets" | "git"): string {
        return join(this.#options.directory, name);
    }
}

/** A space's volume on a computer: its database, events, blobs, buckets and graphs. */
export class LocalVolume implements Volume {
    /** The space's database: its SQLite file, or its schema in a cluster. */
    readonly database: DatabaseConnection & { readonly close: () => Promise<void> };
    /** The space's events, their segments in a bucket of its own. */
    readonly events: EventStore;
    /** The space's blobs, in a directory of its own. */
    readonly blobs: LocalBlobStorage;
    /** The machine's one registry of live sockets. */
    readonly sockets: SocketRegistry;
    /** The providers of the space's resources: SQLite files, Git and mail. */
    readonly providers: readonly SpaceProvider[];
    /** The connectors of the space's Git storages and its mail. */
    readonly connectors: readonly Connector[];
    /** The space. */
    readonly #space: Identifier<"space">;
    /** What the host stores the spaces with. */
    readonly #options: LocalSpaceStorageOptions;
    /** The databases of the space's installations, measured by their files or schemas. */
    readonly #databases: DatabaseCluster;
    /** What the volume opened, closed once the space stops. */
    readonly #lifetime: AsyncDisposableStack;
    /** The space's buckets, once served. */
    #buckets: LocalBucketHost | undefined;
    /** The graph views of the space's builds, once read. */
    #graph: BunGraphStorage | undefined;

    /** Take what the host opened for a space. */
    constructor(
        fields: Pick<
            LocalVolume,
            "database" | "events" | "blobs" | "sockets" | "providers" | "connectors"
        > & {
            readonly space: Identifier<"space">;
            readonly options: LocalSpaceStorageOptions;
            readonly databases: DatabaseCluster;
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

    /** Open the space's buckets once at its own S3 origin. */
    buckets(credentials: (scope: string) => Promise<S3Credentials | undefined>): LocalBucketHost {
        // answer S3 at the listener's port below the space's own host name
        const endpoint = new URL(this.#options.endpoint);
        endpoint.hostname = S3Origin.format(this.#space, LOOPBACK_DOMAIN);
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
        builds: Pick<
            BuildStore,
            "storeOf" | "contains" | "contents" | "image" | "attachment" | "graph" | "file"
        >,
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
    close(): Promise<void> {
        // close the space's volume in its span, timed by the provider
        return TelemetryScope.of(
            import.meta.destack.package,
            this.database.state.environment.telemetry,
        ).span(
            "destack.volume.close",
            { "destack.volume.provider": "local" },
            async () => {
                await this.#lifetime.disposeAsync();
            },
            { duration: { attributes: { "destack.volume.provider": "local" } } },
        );
    }
}
