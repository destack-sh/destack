import type * as Cloudflare from "@cloudflare/workers-types";
import { R2BlobStorage, R2Bucket } from "@destack/bucket/cloudflare";
import { BuildBucket } from "@destack/build/store";
import type { DurableObjectNamespace } from "@destack/db/cloudflare";
import { PostgresConnector } from "@destack/db/postgres";
import { Envelope, EnvelopeConfiguration, UNIVERSE_OWNER } from "@destack/identity/aws";
import { SesClient, SesTransport } from "@destack/mail/ses";
import { type DurableObjectState, DurableObjectWorkload } from "@destack/object/cloudflare";
import type { Alarm } from "@destack/db";
import { CallKey } from "@destack/object";
import { TelemetryScope } from "@destack/telemetry";
import { DistributionManifest, Package } from "@destack/package";
import { WorkerdDistribution } from "@destack/package/workerd";
import { UNIVERSE_SCHEMA, Universe, UniverseDefinition } from "@destack/universe";
import universePackage from "@destack/universe/package";
import { Environment } from "@destack/environment";
import { PlatformUniverse } from "../universe.ts";

/** The name of the one object running the universe. */
const OBJECT_NAME = "universe";

/** The prefix of the universe's blobs in its history bucket, beside its call events' segments. */
const BLOBS_PREFIX = "blobs/";

/** The bindings of the universe's Worker. */
export interface UniverseBindings {
    /** The namespace of the one object running the universe. */
    readonly INSTANCE: DurableObjectNamespace;
    /** The account database through Hyperdrive. */
    readonly DATABASE: { readonly connectionString: string };
    /** The bucket with the segments of the universe's call events. */
    readonly EVENTS: Cloudflare.R2Bucket;
    /** The platform residency's build bucket, where the deploy wrote the releases it shipped. */
    readonly BUILDS: Cloudflare.R2Bucket;
    /** The universe's issuer, its origin, such as https://destack.app. */
    readonly DESTACK_ISSUER: string;
    /** The location hint starting the object near its database, such as weur. */
    readonly DESTACK_LOCATION: string;
    /** How the universe's root keys are sealed, as `EnvelopeConfiguration` JSON. */
    readonly DESTACK_ENVELOPE: string;
    /** The universe's root keys, sealed as its envelope says. */
    readonly DESTACK_ROOT_KEY: string;
    /** The access key decrypting the universe's root keys, under a KMS envelope. */
    readonly DESTACK_KMS_ACCESS_KEY_ID?: string;
    /** The secret of the access key decrypting the universe's root keys, under a KMS envelope. */
    readonly DESTACK_KMS_SECRET_ACCESS_KEY?: string;
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
        super(state, (alarm) => startUniverse(bindings, alarm), Environment.system.clock);
    }
}

/** Start the universe over the Worker's bindings, waking its controllers by the object's alarm. */
async function startUniverse(bindings: UniverseBindings, alarm: Alarm) {
    // open the migrated database, the root keys through their envelope and the universe's own SES credentials
    const environment = Environment.system;
    const connector = new PostgresConnector(new URL(bindings.DATABASE.connectionString));
    const database = await connector.connect(
        { reference: UNIVERSE_SCHEMA },
        Universe.database,
        environment,
    );
    const envelope = new Envelope(
        EnvelopeConfiguration.parse(JSON.parse(bindings.DESTACK_ENVELOPE)),
        UNIVERSE_OWNER,
        Envelope.credentials(
            bindings.DESTACK_KMS_ACCESS_KEY_ID,
            bindings.DESTACK_KMS_SECRET_ACCESS_KEY,
        ),
        environment,
    );
    const keyring = await envelope.keyring(bindings.DESTACK_ROOT_KEY, environment);
    const key = CallKey.of(keyring, environment.crypto);
    const credentials = {
        accessKeyId: bindings.AWS_ACCESS_KEY_ID,
        secretAccessKey: bindings.AWS_SECRET_ACCESS_KEY,
    };
    const ses = new SesClient({
        region: bindings.DESTACK_MAIL_REGION,
        endpoint: SesClient.endpoint(bindings.DESTACK_MAIL_REGION),
        credentials: async () => credentials,
        environment,
    });

    // read the declaration the deploy supplied, then start the universe, calling the machines and forges,
    //  and capture the failures no request answers on the environment's telemetry
    const instruments = TelemetryScope.of(Package.of(universePackage), environment.telemetry);
    const declared: { readonly default: unknown } = await import("destack:universe");
    const universe = await Universe.start({
        ...PlatformUniverse.options(environment, {
            releases: DistributionManifest.shipped(await WorkerdDistribution.read()),
            open: (manifest) =>
                new BuildBucket(new R2Bucket(bindings.BUILDS), environment.crypto).read(manifest),
        }),
        definition: UniverseDefinition.parse(declared.default),
        origin: bindings.DESTACK_ISSUER,
        database,
        keyring,
        callKey: () => key,
        mail: new SesTransport(ses, { from: bindings.DESTACK_MAIL_FROM }),
        events: new R2Bucket(bindings.EVENTS),
        blobs: new R2BlobStorage(new R2Bucket(bindings.EVENTS), BLOBS_PREFIX, environment.random),
        ipAddress: { ipAddressHeaders: ["cf-connecting-ip"] },
        environment,
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
