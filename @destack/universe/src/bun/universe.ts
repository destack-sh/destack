import type { Environment } from "@destack/environment";
import { LocalBucket } from "@destack/bucket/bun";
import { connectPostgres } from "@destack/db/postgres";
import { LocalKeyring } from "@destack/identity";
import type { MailTransport } from "@destack/message/email";
import { CallKey } from "@destack/runtime/request";
import { Universe, type UniverseDefinition, universeDatabase } from "../universe/index.ts";

/** What a Bun process runs a universe with. */
export interface BunUniverseOptions {
    /** The universe's definition. */
    readonly definition: UniverseDefinition;
    /** The clock the universe runs at, and the network it listens and sends through. */
    readonly environment: Environment;
    /** The universe's origin on loopback, its issuer, such as http://127.0.0.1:4100. */
    readonly origin: string;
    /** The PostgreSQL URL of the migrated account database. */
    readonly database: string;
    /** The universe's root keys, as `LocalKeyring.generate` writes them. */
    readonly rootKey: string;
    /** The transport mailing sign-in links and codes. */
    readonly mail: MailTransport;
    /** The directory of the bucket of the universe's call events. */
    readonly directory: string;
    /** Report a failure the universe answers to no one. */
    readonly report: (error: unknown) => void;
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
        // open the database, the call events' bucket and the root keys
        await using stack = new AsyncDisposableStack();
        const database = stack.use(await connectPostgres(options.database, universeDatabase));
        const events = stack.use(await LocalBucket.open(options.directory, "universe"));
        const keyring = await LocalKeyring.read(options.rootKey);
        const key = CallKey.of(keyring);

        // start the universe's packages, sending to other origins over the network
        const universe = await Universe.start({
            definition: options.definition,
            origin: options.origin,
            database,
            keyring,
            callKey: () => key,
            mail: options.mail,
            events,
            ipAddress: { disableIpTracking: true },
            environment: options.environment,
            report: (error) => options.report(error),
        });
        stack.use(universe);

        // listen at the origin
        const origin = new URL(options.origin);
        const listener = options.environment.network.serve({
            hostname: origin.hostname,
            port: Number(origin.port),
            fetch: (request) => universe.fetch(request),
        });
        stack.defer(() => listener.stop(true));

        return new BunUniverse(origin, stack.move());
    }

    /** Stop listening, drain the packages and close the database and the call events. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#stack.disposeAsync();
    }
}
