import type { Environment } from "@destack/environment";
import { join } from "node:path";
import { LocalBlobStorage, LocalBucket } from "@destack/bucket/bun";
import { connectPostgres } from "@destack/db/postgres";
import { LocalKeyring } from "@destack/identity";
import type { MailTransport } from "@destack/mail/transport";
import type { ProviderClients } from "@destack/account/better-auth";
import { CallKey } from "@destack/object";
import { UNIVERSE_SCHEMA, Control, type UniverseDefinition } from "@destack/control";
import { UniverseOrigin } from "@destack/account/address";
import { PlatformUniverse, type ShippedReleases } from "../deployment/universe.ts";

/** What a Bun process runs a universe with. */
export interface BunUniverseOptions {
    /** The universe's definition. */
    readonly definition: UniverseDefinition;
    /** The environment the universe runs in, and the listener it serves its origin on. */
    readonly environment: Environment<"server" | "files">;
    /** The universe's origin on loopback, its issuer, such as http://127.0.0.1:4100. */
    readonly origin: string;
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
    /** The releases the dev build shipped, which the universe opens before any forge's. */
    readonly shipped: ShippedReleases;
    /** Answer what the origin serves besides the universe, as production's edge does. */
    readonly edge: (request: Request) => Promise<Response>;
}

/** A universe served by a Bun process at its origin, its controllers looping in process. */
export class BunUniverse implements AsyncDisposable {
    /** The URL the universe answers at. */
    readonly url: URL;
    /** Close what the universe opened, last opened first. */
    readonly #stack: AsyncDisposableStack;

    /** Create a started universe with what closes it. */
    private constructor(url: URL, stack: AsyncDisposableStack) {
        this.url = url;
        this.#stack = stack;
    }

    /** Start the universe over its database and call events, and listen at its origin. */
    static async start(options: BunUniverseOptions): Promise<BunUniverse> {
        // open the database, the call events' bucket, the blobs and the root keys
        await using stack = new AsyncDisposableStack();
        const database = stack.use(
            await connectPostgres(options.database, Control.database, {
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
        );
        const keyring = await LocalKeyring.read(options.rootKey, options.environment);
        const key = CallKey.of(keyring, options.environment.crypto);

        // start the universe's packages, sending to other origins over the network and calling the forges through it
        const universe = await Control.start({
            ...PlatformUniverse.options(options.environment, options.shipped),
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

        // listen at the origin
        const origin = new URL(options.origin);
        const listener = options.environment.server.http({
            hostname: origin.hostname,
            port: Number(origin.port),
            // hand the universe its issuer and package mounts, and the edge every other path
            fetch: (request) =>
                UniverseOrigin.path(new URL(request.url).pathname) === undefined
                    ? options.edge(request)
                    : universe.fetch(request),
        });
        stack.defer(async () => listener.stop(true));

        return new BunUniverse(origin, stack.move());
    }

    /** Stop listening, drain the packages and close the database and the call events. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#stack.disposeAsync();
    }
}
