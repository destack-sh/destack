import type { Environment } from "@destack/environment";
import { join } from "node:path";
import { LocalBlobStorage, LocalBucket } from "@destack/bucket/bun";
import type { DatabaseConnection } from "@destack/db/database";
import { connectPostgres } from "@destack/db/postgres";
import { LocalKeyring } from "@destack/identity";
import type { MailTransport } from "@destack/mail/transport";
import type { UniverseOrigin } from "@destack/account/address";
import type { ProviderClients } from "@destack/account/better-auth";
import { CallKey } from "@destack/object";
import type { HostListening } from "@destack/object/workload";
import type { PackageRelease } from "@destack/package";
import type { BuildLayer } from "@destack/package/manifest";
import { UNIVERSE_SCHEMA, Universe, type UniverseDefinition } from "../universe/index.ts";

/** What a Bun process runs a universe with. */
export interface BunUniverseOptions {
    /** The universe's definition. */
    readonly definition: UniverseDefinition;
    /** The environment the universe runs in. */
    readonly environment: Environment<"files">;
    /** The universe's address, such as http://destack.localhost:4100 with its domains. */
    readonly origin: UniverseOrigin;
    /** The PostgreSQL URL of the migrated account database. */
    readonly database: string;
    /** The universe's root keys, as `LocalKeyring.generate` writes them. */
    readonly rootKey: string;
    /** The transport mailing sign-in codes. */
    readonly mail: MailTransport;
    /** The applications at the sign-in providers whose secrets the operator's environment holds. */
    readonly providers: ProviderClients;
    /** The directory of the bucket of the universe's call events. */
    readonly directory: string;
    /** Report a failure the universe answers to no one. */
    readonly report: (error: unknown) => void;
    /** The releases the universe's distribution has, which it opens before any forge's. */
    readonly distribution: readonly PackageRelease[];
    /** The builds of the distribution's releases. */
    readonly layer: BuildLayer;
}

/** A universe in a Bun process, served by the listener of the machine at its origin. */
export class BunUniverse implements AsyncDisposable {
    /** The universe's requests and the sockets it keeps, which the machine's listener hands it. */
    readonly listening: HostListening;
    /** The universe's database, whose directory names the accounts. */
    readonly database: DatabaseConnection;
    /** Close what the universe opened, last opened first. */
    readonly #stack: AsyncDisposableStack;

    /** Create a started universe with what closes it. */
    private constructor(
        listening: HostListening,
        database: DatabaseConnection,
        stack: AsyncDisposableStack,
    ) {
        this.listening = listening;
        this.database = database;
        this.#stack = stack;
    }

    /** Start the universe over its database and call events, its controllers looping in process. */
    static async start(options: BunUniverseOptions): Promise<BunUniverse> {
        // open the database, the call events' bucket, the blobs and the root keys
        await using stack = new AsyncDisposableStack();
        const database = stack.use(
            await connectPostgres(options.database, Universe.database, {
                environment: options.environment,
                schema: UNIVERSE_SCHEMA,
            }),
        );
        const events = stack.use(
            await LocalBucket.open(options.directory, "universe", options.environment),
        );
        const blobs = await LocalBlobStorage.open(
            join(options.directory, "universe-blobs"),
            options.environment,
            { synchronous: "full" },
        );
        const keyring = await LocalKeyring.read(options.rootKey, options.environment);
        const key = CallKey.of(keyring, options.environment.crypto);

        // start the universe's packages, sending to other origins over the network and calling the forges through it
        const universe = await Universe.start({
            distribution: options.distribution,
            layer: options.layer,
            definition: options.definition,
            origin: options.origin,
            database,
            keyring,
            callKey: () => key,
            mail: options.mail,
            providers: options.providers,
            events,
            blobs,
            ipAddress: { disableIpTracking: true },
            environment: options.environment,
            report: (error) => options.report(error),
        });
        stack.use(universe);

        // return once every server serves, the definition applied
        await Promise.all([...universe.servers.values()].map((server) => server.ready()));
        const listening = universe.connections.listen((request) => universe.fetch(request));

        return new BunUniverse(listening, database, stack.move());
    }

    /** Drain the packages and close the database and the call events. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#stack.disposeAsync();
    }
}
