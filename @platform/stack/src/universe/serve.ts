import { mkdir } from "node:fs/promises";
import { resolve } from "node:path";
import { ROOT } from "../deployment/index.ts";
import { signInMail } from "../mail/index.ts";
import { RELAY_ORIGIN, UNIVERSE_ORIGIN } from "./origin.ts";
import { DevelopmentUniverse } from "./development.ts";

/** The local PostgreSQL server, the test server of `just postgres start`. */
const SERVER = "postgres://postgres@127.0.0.1:55432";

/** The database keeping the dev universe's schemas, one per service. */
const DATABASE = "destack_dev";

/** The secret protecting the dev universe's cookies and tokens. */
const SECRET = "destack-dev-universe-secret-for-this-machine-only";

/** The operator of the dev universe, who owns the platform's accounts and signs in with a printed link. */
const OPERATOR = "operator@destack.test";

/** The directory with the dev universe's files, beside the test PostgreSQL's data in the repository's target. */
const DIRECTORY = resolve(ROOT, "../../target/universe");

/** Serve the dev universe until a signal stops it. */
export async function serveUniverse(): Promise<void> {
    // mail sign-in links and codes through SES, or print them
    const mail = signInMail(process.env);
    process.stdout.write(`${mail.description}\n`);

    // start every platform workload in this process
    await mkdir(DIRECTORY, { recursive: true });
    const relay = new URL(RELAY_ORIGIN);
    const universe = await DevelopmentUniverse.start({
        server: SERVER,
        database: DATABASE,
        origin: UNIVERSE_ORIGIN,
        relay: { origin: RELAY_ORIGIN, hostname: relay.hostname, port: Number(relay.port) },
        region: "eu-central",
        directory: DIRECTORY,
        secret: SECRET,
        operator: OPERATOR,
        mail,
        report: (error) => process.stderr.write(`universe: ${String(error)}\n`),
    });
    process.stdout.write(
        `universe answers at ${universe.url.href}, its relay at ${RELAY_ORIGIN} (DESTACK_RELAY=${RELAY_ORIGIN}/tunnel)\n`,
    );

    // close on an interrupt or a termination
    const stopped = Promise.withResolvers<void>();
    process.once("SIGINT", () => stopped.resolve());
    process.once("SIGTERM", () => stopped.resolve());
    await stopped.promise;
    await universe.close();
}
