import type * as Cloudflare from "@cloudflare/workers-types";
import { R2Bucket } from "@destack/bucket/cloudflare";
import type { DurableObjectNamespace } from "@destack/db/cloudflare";
import { PostgresConnector } from "@destack/db/postgres";
import { LocalKeyring } from "@destack/identity";
import { SesClient, SesTransport } from "@destack/message/aws";
import { type DurableObjectState, DurableObjectWorkload } from "@destack/runtime/cloudflare";
import type { Alarm } from "@destack/runtime/control";
import { CallKey } from "@destack/runtime/request";
import { telemetry } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import { Universe, UniverseDeclaration, universeDatabase } from "../universe/index.ts";
import { Environment } from "@destack/environment";

/** The instruments capturing the failures no request answers. */
const instruments = telemetry.scope(import.meta.destack.package);

/** The name of the one object running the universe. */
const OBJECT_NAME = "universe";

/** The schema the universe's tables live in, PostgreSQL's default. */
const UNIVERSE_SCHEMA = "public";

/** The bindings of the universe's Worker. */
export interface UniverseBindings {
    /** The namespace of the one object running the universe. */
    readonly INSTANCE: DurableObjectNamespace;
    /** The universe's declaration, as `release` writes it. */
    readonly UNIVERSE: unknown;
    /** The account database through Hyperdrive. */
    readonly DATABASE: { readonly connectionString: string };
    /** The bucket with the segments of the universe's call events. */
    readonly EVENTS: Cloudflare.R2Bucket;
    /** The universe's issuer, its origin, such as https://destack.app. */
    readonly DESTACK_ISSUER: string;
    /** The location hint starting the object near its database, such as weur. */
    readonly DESTACK_LOCATION: string;
    /** The universe's root keys, as `LocalKeyring.generate` writes them. */
    readonly DESTACK_ROOT_KEY: string;
    /** The AWS region SES sends sign-in mail from, such as eu-central-1. */
    readonly DESTACK_MAIL_REGION: string;
    /** The address sign-in mail comes from. */
    readonly DESTACK_MAIL_FROM: string;
    /** The access key of the universe's own SES credentials. */
    readonly AWS_ACCESS_KEY_ID: string;
    /** The secret key of the universe's own SES credentials. */
    readonly AWS_SECRET_ACCESS_KEY: string;
}

/** The one object running the universe, its controllers woken by the object's alarm. */
export class DurableObjectUniverse extends DurableObjectWorkload {
    /** Start the universe before the object takes any event. */
    constructor(state: DurableObjectState, bindings: UniverseBindings) {
        super(state, (alarm) => startUniverse(bindings, alarm));
    }
}

/** Start the universe over the Worker's bindings, waking its controllers by the object's alarm. */
async function startUniverse(bindings: UniverseBindings, alarm: Alarm) {
    // open the migrated database, the root keys and the universe's own SES credentials
    const connector = new PostgresConnector(new URL(bindings.DATABASE.connectionString));
    const database = await connector.connect({ reference: UNIVERSE_SCHEMA }, universeDatabase);
    const keyring = await LocalKeyring.read(bindings.DESTACK_ROOT_KEY);
    const key = CallKey.of(keyring);
    const credentials = {
        accessKeyId: bindings.AWS_ACCESS_KEY_ID,
        secretAccessKey: bindings.AWS_SECRET_ACCESS_KEY,
    };
    const ses = new SesClient({
        region: bindings.DESTACK_MAIL_REGION,
        endpoint: SesClient.endpoint(bindings.DESTACK_MAIL_REGION),
        credentials: async () => credentials,
    });

    // start the universe, calling the machine Workers at their origins
    const universe = await Universe.start({
        declaration: UniverseDeclaration.parse(bindings.UNIVERSE),
        origin: bindings.DESTACK_ISSUER,
        database,
        keyring,
        callKey: () => key,
        mail: new SesTransport(ses, { from: bindings.DESTACK_MAIL_FROM }),
        events: new R2Bucket(bindings.EVENTS),
        ipAddress: { ipAddressHeaders: ["cf-connecting-ip"] },
        environment: Environment.system,
        alarm,
        report: (error) => instruments.captureException(error, { isEscaped: true }),
    });

    return {
        fetch: (request: Request) => universe.fetch(request),
        alarm: (deadline: number) => universe.idle(deadline),
    };
}

/** The universe Worker's handler, handing every request to the one object running the universe. */
export default {
    /** Answer a request in the universe's object, started near its database. */
    fetch(request: Request, bindings: UniverseBindings): Promise<Response> {
        const id = bindings.INSTANCE.idFromName(OBJECT_NAME);

        return bindings.INSTANCE.get(id, {
            locationHint: bindings.DESTACK_LOCATION,
        }).fetch(request);
    },
};
