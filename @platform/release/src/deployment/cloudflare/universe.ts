// load the Reflect metadata polyfill the passkey library requires, before any module of the bundle
import "reflect-metadata";
import { BucketBlobStorage } from "@destack/bucket/blob";
import { S3Client, S3Location } from "@destack/bucket/s3";
import { BuildLayer, type LayerAssets } from "@destack/build/store";
import type { DurableObjectNamespace } from "@destack/db/cloudflare";
import { connectPostgres } from "@destack/db/postgres";
import { Envelope, EnvelopeConfiguration, UNIVERSE_OWNER } from "@destack/identity/aws";
import { MailSender } from "@destack/mail/sender";
import { ProviderClients } from "@destack/account/better-auth";
import { type DurableObjectState, DurableObjectWorkload } from "@destack/object/cloudflare";
import type { Alarm } from "@destack/db";
import { CallKey } from "@destack/object";
import { TelemetryScope } from "@destack/telemetry";
import { DistributionManifest, Package } from "@destack/package";
import { WorkerdDistribution } from "@destack/package/workerd";
import { UNIVERSE_SCHEMA, Control, UniverseDefinition } from "@destack/control";
import controlPackage from "@destack/control/package";
import { WorkerdEnvironment } from "@destack/environment/workerd";
import { present } from "@destack/schema";
import { PlatformUniverse } from "../universe.ts";

/** The name of the one object running the universe. */
const OBJECT_NAME = "universe";

/** The prefix of the universe's blobs in its files bucket, beside its call events' segments. */
const BLOBS_PREFIX = "blobs/";

/** The bindings of the universe's Worker. */
export interface UniverseBindings {
    /** The namespace of the one object running the universe. */
    readonly INSTANCE: DurableObjectNamespace;
    /** The account database through Hyperdrive. */
    readonly DATABASE: { readonly connectionString: string };
    /** The releases the Worker version ships, a build layer in its static assets. */
    readonly RELEASES: LayerAssets;
    /** The S3 bucket keeping the universe's files, as `S3Location` JSON. */
    readonly DESTACK_FILES: string;
    /** The universe's issuer, its origin, such as https://destack.app. */
    readonly DESTACK_UNIVERSE: string;
    /** The location hint starting the object near its database, such as weur. */
    readonly DESTACK_LOCATION: string;
    /** How the universe's root keys are sealed, as `EnvelopeConfiguration` JSON. */
    readonly DESTACK_ENVELOPE: string;
    /** The universe's root keys, sealed as its envelope says. */
    readonly DESTACK_ROOT_KEY: string;
    /** Where sign-in mail goes, as `MailSender` JSON. */
    readonly DESTACK_MAIL: string;
    /** The applications at the sign-in providers, as `ProviderClients` JSON. */
    readonly DESTACK_PROVIDERS: string;
    /** The Worker's AWS access key: decrypting its root keys, sending mail, keeping its files. */
    readonly AWS_ACCESS_KEY_ID: string;
    /** The secret of the Worker's AWS access key. */
    readonly AWS_SECRET_ACCESS_KEY: string;
}

/** The one object running the universe, its controllers woken by the object's alarm. */
export class DurableObjectUniverse extends DurableObjectWorkload {
    /** Start the universe before the object takes any event. */
    constructor(state: DurableObjectState, bindings: UniverseBindings) {
        super(state, (alarm) => startUniverse(bindings, alarm), WorkerdEnvironment.system.clock);
    }
}

/** Start the universe over the Worker's bindings, waking its controllers by the object's alarm. */
async function startUniverse(bindings: UniverseBindings, alarm: Alarm) {
    // open the migrated database, the root keys through their envelope and the universe's own mail sender
    const environment = WorkerdEnvironment.system;
    const database = await connectPostgres(bindings.DATABASE.connectionString, Control.database, {
        writers: "sole",
        environment,
        schema: UNIVERSE_SCHEMA,
    });
    const credentials = {
        accessKeyId: bindings.AWS_ACCESS_KEY_ID,
        secretAccessKey: bindings.AWS_SECRET_ACCESS_KEY,
    };
    const envelope = new Envelope(
        EnvelopeConfiguration.parse(JSON.parse(bindings.DESTACK_ENVELOPE)),
        UNIVERSE_OWNER,
        credentials,
        environment,
    );
    const keyring = await envelope.keyring(bindings.DESTACK_ROOT_KEY, environment);
    const key = CallKey.of(keyring, environment.crypto);
    const mail = MailSender.transport(MailSender.parse(JSON.parse(bindings.DESTACK_MAIL)), {
        credentials: async () => credentials,
        print: (text) => WorkerdEnvironment.print(text),
        environment,
    });

    // open the universe's files over S3
    const files = S3Client.of(
        { location: S3Location.parse(JSON.parse(bindings.DESTACK_FILES)), credentials },
        environment,
    );

    // read the declaration the deploy supplied, then start the universe, calling the machines and forges,
    //  and capture the failures no request answers on the environment's telemetry
    const instruments = TelemetryScope.of(Package.of(controlPackage), environment.telemetry);
    const declared: { readonly default: unknown } = await import("destack:universe");
    const releases = await BuildLayer.assets(bindings.RELEASES, environment.crypto);
    const universe = await Control.start({
        ...PlatformUniverse.options(environment, {
            releases: DistributionManifest.releases(await WorkerdDistribution.read()),
            open: (manifest) =>
                present(releases.build(manifest), `the shipped release ${manifest}`),
        }),
        definition: UniverseDefinition.parse(declared.default),
        origin: bindings.DESTACK_UNIVERSE,
        database,
        keyring,
        callKey: () => key,
        mail,
        providers: ProviderClients.parse(JSON.parse(bindings.DESTACK_PROVIDERS)),
        events: files,
        blobs: new BucketBlobStorage(files, BLOBS_PREFIX, environment.random),
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
