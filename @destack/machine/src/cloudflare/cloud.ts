import type * as Cloudflare from "@cloudflare/workers-types";
import { MachineIdentity } from "@destack/account/client";
import * as accountObject from "@destack/account/object";
import accountPackage, { OAUTH_PATH } from "@destack/account/package";
import type { ArtifactsNamespace } from "@destack/git/cloudflare";
import type { DatabaseConnection, Snapshot } from "@destack/db";
import { DurableObjectReader, Jurisdiction } from "@destack/db/cloudflare";
import { copyRequest, type Environment, type FetchOptions } from "@destack/environment";
import { WorkerdEnvironment } from "@destack/environment/workerd";
import { type LocalKeyring, MemoryKeychain } from "@destack/identity";
import { Envelope, EnvelopeConfiguration } from "@destack/identity/aws";
import { MailSender } from "@destack/mail/sender";
import { mailConnector } from "@destack/mail/transport";
import { CallKey, PackageMount, tablesOf } from "@destack/object";
import { type Identifier, schema } from "@destack/schema";
import type { DurableObjectNamespace, PartitionBindings } from "./runtime/index.ts";
import { defineDatabase } from "@destack/db";
import { DIRECTORY_PREFIX, directoryConfiguration, directoryWorkload } from "./directory.ts";
import type { Alarm } from "@destack/db";
import { R2BlobStorage, R2Bucket } from "@destack/bucket/cloudflare";
import { BuildLayer, type LayerAssets } from "@destack/build/store";
import { DurableObjectDatabaseHost, type DurableObjectStorage } from "@destack/db/cloudflare";
import type { DurableObjectWorkloadInstance } from "@destack/object/cloudflare";
import { createClient } from "@destack/object/client";
import { WorkloadInstance } from "@destack/object/workload";
import { ResourceContext } from "@destack/resource/context";
import type { Publisher } from "@destack/sync";
import { TelemetryScope } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../error/index.ts";
import type { MachineEnrollment } from "../computer/device/object/index.ts";
import { DRAIN_TIMEOUT_MILLISECONDS, Machine } from "../machine/index.ts";

/** The runtimes a Cloudflare machine serves: its instances' workerd and its views' browser. */
export const CLOUD_RUNTIMES = ["workerd", "browser"] as const;

/** The header Cloudflare writes a public request's client address into, which marks it public. */
export const CONNECTING_IP_HEADER = "cf-connecting-ip";

/** The bindings of a cloud machine's spaces' storage. */
export interface CloudflareStorageBindings {
    /** The residency's R2 bucket of the spaces' buckets, snapshots and event segments. */
    readonly FILES: Cloudflare.R2Bucket;
    /** The name of the residency's R2 bucket. */
    readonly DESTACK_FILES: string;
    /** The residency's Artifacts namespace of the spaces' Git storages. */
    readonly ARTIFACTS: ArtifactsNamespace;
    /** The residency's R2 bucket of the builds the machines pull from forges. */
    readonly BUILDS: Cloudflare.R2Bucket;
    /** The releases the Worker version ships, a build layer in its static assets. */
    readonly RELEASES: LayerAssets;
    /** The rates of the spaces' use, as `RateCard` JSON. */
    readonly DESTACK_RATES: string;
    /** Where the spaces' mail goes, as `MailSender` JSON. */
    readonly DESTACK_MAIL: string;
    /** The access key sending the spaces' mail. */
    readonly AWS_ACCESS_KEY_ID: string;
    /** The secret of the access key sending the spaces' mail. */
    readonly AWS_SECRET_ACCESS_KEY: string;
}

/** The bindings of a cloud machine's Worker. */
export interface MachineBindings extends PartitionBindings, CloudflareStorageBindings {
    /** The universe's Worker, through a service binding. */
    readonly UNIVERSE: { readonly fetch: (request: Request) => Promise<Response> };
    /** The cloud machine the Worker runs as. */
    readonly DESTACK_MACHINE_ID: string;
    /** How the machine's root keys and key are sealed, as `EnvelopeConfiguration` JSON. */
    readonly DESTACK_ENVELOPE: string;
    /** The machine's root keys, sealed as its envelope says. */
    readonly DESTACK_ROOT_KEY: string;
    /** The machine's private key, as `KeyPair.generate` writes it, sealed as its envelope says. */
    readonly DESTACK_MACHINE_KEY: string;
    /** The access key decrypting the machine's sealed keys, under a KMS envelope. */
    readonly DESTACK_KMS_ACCESS_KEY_ID?: string;
    /** The secret of the access key decrypting the machine's sealed keys under a KMS envelope. */
    readonly DESTACK_KMS_SECRET_ACCESS_KEY?: string;
    /** The universe's origin, serving the account package. */
    readonly DESTACK_ISSUER: string;
    /** The objects of the builds' graph indexes, one per manifest. */
    readonly GRAPH: DurableObjectNamespace;
    /** The jurisdiction of the machine's objects, empty for none. */
    readonly DESTACK_JURISDICTION: Jurisdiction;
    /** The domain of the installation origins the machine answers, such as destack.space. */
    readonly DESTACK_SPACE_DOMAIN: string;
    /** The machine's own host, whose paths address its spaces directly. */
    readonly DESTACK_HOST: string;
    /** The installation residency homes serve the registry protocol at. */
    readonly DESTACK_REGISTRY: string;
    /** The binding of each cloud machine, by the hosts of it and its relay, as JSON. */
    readonly DESTACK_PEERS: string;
}

/** The bindings of the cloud machines and their relays, by host, refusing a missing binding. */
function peersOf(bindings: MachineBindings): ReadonlyMap<string, Fetcher> {
    const declared = schema
        .record(schema.string(), schema.string())
        .parse(JSON.parse(bindings.DESTACK_PEERS));

    return new Map(
        Object.entries(declared).map(([host, name]) => {
            const binding: unknown = Reflect.get(bindings, name);
            if (!isFetcher(binding)) {
                throw MachineError.of(
                    "CONFIGURATION_INVALID",
                    `no service binding ${name} for ${host}`,
                );
            }

            return [host, binding];
        }),
    );
}

/** Copy a request a Worker sends a peer without the client address that marks a public one. */
function internal(request: Request): Request {
    const headers = new Headers(request.headers);
    headers.delete(CONNECTING_IP_HEADER);

    return copyRequest(request, { headers });
}

/** A service binding, which only the account's own Workers answer. */
interface Fetcher {
    /** Send a request to the bound Worker. */
    readonly fetch: (request: Request) => Promise<Response>;
}

/** Report whether a binding is a service binding. */
function isFetcher(value: unknown): value is Fetcher {
    return (
        typeof value === "object" &&
        value !== null &&
        "fetch" in value &&
        typeof value.fetch === "function"
    );
}

/** The cloud machine a Worker runs as: its bindings, keys, identity and directory replica. */
export class CloudMachine {
    /** The database of a cloud machine's object: its directory replica. */
    static readonly database = defineDatabase({
        name: "directory",
        ...tablesOf({ packages: [], follows: [], tables: [] }),
        replicated: Machine.replicated,
    });

    /** The Worker's bindings. */
    readonly bindings: MachineBindings;
    /** The machine. */
    readonly id: Identifier<"machine">;
    /** The machine's root keys. */
    readonly keyring: LocalKeyring;
    /** The machine's identity, proven with its unsealed key. */
    readonly identity: MachineIdentity;
    /** The isolate's environment, sending account package requests through the universe. */
    readonly environment: Environment;
    /** The directory replica in the machine's object. */
    readonly replica: DatabaseConnection;
    /** The universe's OAuth issuer. */
    readonly issuer: string;

    /** Create a cloud machine from its parts. */
    constructor(
        fields: Pick<
            CloudMachine,
            "bindings" | "id" | "keyring" | "identity" | "environment" | "replica" | "issuer"
        >,
    ) {
        // set the bindings, keys and directory replica
        this.bindings = fields.bindings;
        this.id = fields.id;
        this.keyring = fields.keyring;
        this.identity = fields.identity;
        this.environment = fields.environment;
        this.replica = fields.replica;
        this.issuer = fields.issuer;
    }

    /** Read the cloud machine of a Worker's bindings in the isolate's system environment. */
    static async read(bindings: MachineBindings, system: Environment): Promise<CloudMachine> {
        // send the universe's and peers' requests through their bindings, others out
        const accounts = PackageMount.url(bindings.DESTACK_ISSUER, accountPackage.id);
        const peers = peersOf(bindings);
        const environment: Environment = {
            ...system,
            network: {
                ...system.network,
                fetch: (request: Request, options?: FetchOptions) => {
                    // call the universe, and a peer machine or its relay, through their bindings
                    const peer = peers.get(new URL(request.url).host);
                    if (request.url === accounts || request.url.startsWith(`${accounts}/`)) {
                        return bindings.UNIVERSE.fetch(request);
                    } else if (peer !== undefined) {
                        return peer.fetch(internal(request));
                    }

                    return system.network.fetch(request, options);
                },
            },
        };

        // unseal the machine's root keys and its key
        const id = schema.identifier("machine").parse(bindings.DESTACK_MACHINE_ID);
        const envelope = new Envelope(
            EnvelopeConfiguration.parse(JSON.parse(bindings.DESTACK_ENVELOPE)),
            Envelope.machine(id),
            Envelope.credentials(
                bindings.DESTACK_KMS_ACCESS_KEY_ID,
                bindings.DESTACK_KMS_SECRET_ACCESS_KEY,
            ),
            environment,
        );
        const keyring = await envelope.keyring(bindings.DESTACK_ROOT_KEY, environment);
        const keychain = new MemoryKeychain();
        await keychain.save(id, await envelope.open(bindings.DESTACK_MACHINE_KEY));
        const identity = new MachineIdentity(id, keychain, environment);

        // read the directory from the replica in the machine's object, through a stub per request as the isolate outlives each
        const objects = Jurisdiction.narrow(bindings.PARTITION, bindings.DESTACK_JURISDICTION);
        const replica = DurableObjectReader.connect(
            { fetch: (request) => objects.get(objects.idFromName(id)).fetch(request) },
            CloudMachine.database.tables,
            environment,
        );

        return new CloudMachine({
            bindings,
            id,
            keyring,
            identity,
            environment,
            replica,
            issuer: `${accounts}${OAUTH_PATH}`,
        });
    }

    /** Open the releases the deploy shipped with the Worker version, read in place. */
    releases(): Promise<BuildLayer> {
        return BuildLayer.assets(this.bindings.RELEASES, this.environment.crypto);
    }

    /** Read the machine's enrollment from its directory row, absent before it is declared. */
    async enrollment(view: Pick<Snapshot, "read">): Promise<MachineEnrollment | undefined> {
        // read the machine's row in the directory replica
        const row = await view
            .read(accountObject.machine.table)
            .findFirst({ where: { id: this.id } });
        if (row?.residencyId === undefined || row.residencyId === null) {
            return undefined;
        }

        return {
            kind: "cloud",
            residencyId: row.residencyId,
            issuer: this.bindings.DESTACK_ISSUER,
            status: "enrolled",
        };
    }

    /** Open the connector sending the spaces' mail where the deployment directs it. */
    mail() {
        // send through SES signed with the machine's access key, or print to the Worker's log
        const sender = MailSender.parse(JSON.parse(this.bindings.DESTACK_MAIL));
        const credentials = {
            accessKeyId: this.bindings.AWS_ACCESS_KEY_ID,
            secretAccessKey: this.bindings.AWS_SECRET_ACCESS_KEY,
        };
        const transport = MailSender.transport(sender, {
            credentials: async () => credentials,
            print: (text) => WorkerdEnvironment.print(text),
            environment: this.environment,
        });

        return mailConnector(sender.kind, transport);
    }

    /** Derive the key the machine's journals fingerprint sensitive inputs under. */
    callKey(): Promise<CryptoKey> {
        return CallKey.of(this.keyring, this.environment.crypto);
    }

    /** Store the machine's replicas in its object's storage, answering reads of them. */
    async startDirectory(
        storage: DurableObjectStorage,
        alarm: Alarm,
    ): Promise<DurableObjectWorkloadInstance> {
        // store the replicas in the object's own storage, in the isolate's environment
        const host = new DurableObjectDatabaseHost(storage, this.environment);
        const database = host.connect(CloudMachine.database);
        await database.migrate(CloudMachine.database.tables, { others: "retain" });

        // follow them as the machine through the universe's account package
        const accounts = createClient(
            accountPackage,
            {
                url: PackageMount.url(this.bindings.DESTACK_ISSUER, accountPackage.id),
                fetch: this.identity.fetch(
                    (request) => this.environment.network.fetch(request),
                    accountPackage.id,
                    this.issuer,
                ),
            },
            this.environment,
        );
        const publisher: Publisher = {
            async *stream(subscription, signal) {
                yield* await accounts.store.stream(subscription, { signal });
            },
            request: async (subscription) => {
                await accounts.store.request({ subscription });
            },
        };
        const instance = await WorkloadInstance.start(directoryWorkload, {
            environment: this.environment,
            resources: new ResourceContext().bind(directoryConfiguration, {
                database,
                blobs: new R2BlobStorage(
                    new R2Bucket(this.bindings.BUILDS),
                    DIRECTORY_PREFIX,
                    this.environment.random,
                ),
                machine: this.identity,
                publisher,
            }),
            callKey: () => this.callKey(),
            alarm,
            report: (error) =>
                TelemetryScope.of(
                    import.meta.destack.package,
                    this.environment.telemetry,
                ).captureException(error, { isEscaped: true }),
            serverOptions: () => ({
                environment: this.environment,
                authenticate: () => Promise.reject(CloudMachine.#refusal()),
                authorizeMachine: () => Promise.reject(CloudMachine.#refusal()),
                drainTimeout: DRAIN_TIMEOUT_MILLISECONDS,
            }),
        });

        return {
            // answer the reads of the replicas alone
            fetch: (request) => host.read(request),
            alarm: (deadline) => instance.alarm(deadline),
        };
    }

    /** Refuse a call to the replicas' packages, which serve nothing. */
    static #refusal() {
        return MachineError.of("CALLS_MISSING", "a cloud machine object does not accept calls");
    }
}
