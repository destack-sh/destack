import type * as Cloudflare from "@cloudflare/workers-types";
import { MachineKey } from "@destack/account/client";
import * as accountObject from "@destack/account/object";
import accountPackage from "@destack/account/package";
import { UniverseOrigin } from "@destack/account/address";
import type { ArtifactsNamespace } from "@destack/git/cloudflare";
import type { DatabaseConnection, Snapshot, Alarm } from "@destack/db/database";
import {
    DurableObjectReader,
    Jurisdiction,
    DurableObjectDatabaseHost,
    type DurableObjectStorage,
} from "@destack/db/cloudflare";
import {
    HttpTransport,
    copyRequest,
    type Environment,
    type FetchOptions,
} from "@destack/environment";
import { WorkerdEnvironment } from "@destack/environment/workerd";
import { type LocalKeyring, MemoryKeychain } from "@destack/identity";
import { Envelope, EnvelopeConfiguration } from "@destack/identity/aws";
import { MailSender } from "@destack/mail/sender";
import { mailConnector } from "@destack/mail/transport";
import { CallKey, type Fetch, PackageMount, tablesOf } from "@destack/object";
import { principal } from "@destack/access";
import { RelayHost } from "@destack/relay/server";
import { Scope } from "@destack/sync";
import { type Identifier, present, schema } from "@destack/schema";
import type { DurableObjectNamespace, PartitionBindings } from "./runtime/index.ts";
import { defineDatabase } from "@destack/db";
import { DIRECTORY_PREFIX, directoryConfiguration, directoryWorkload } from "./directory.ts";
import { R2Bucket } from "@destack/bucket/cloudflare";
import { BucketBlobStorage } from "@destack/bucket/blob";
import { BuildLayer, type AssetBinding } from "@destack/package/manifest";
import type { DurableObjectWorkloadInstance } from "@destack/object/cloudflare";
import { ObjectConnection } from "@destack/object/client";
import { WorkloadInstance } from "@destack/object/workload";
import { ResourceContext } from "@destack/resource/context";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../error/index.ts";
import type { MachineEnrollment } from "../computer/device/object/index.ts";
import { DRAIN_TIMEOUT_MILLISECONDS, Machine } from "../machine/index.ts";

/** The runtimes a Cloudflare machine serves: its instances' workerd and its views' browser. */
export const CLOUD_RUNTIMES = ["workerd", "browser"] as const;

/** The header Cloudflare writes a public request's client address into, which marks it public. */
export const CONNECTING_IP_HEADER = "cf-connecting-ip";

/** The name of the object running the region's relay's root, and below it each tunnel's. */
const RELAY_OBJECT = "relay";

/** The bindings of a cloud machine's spaces' storage. */
export interface CloudflareStorageBindings {
    /** The region's R2 bucket, by owner: `spaces/<space>/` and `directory/`. */
    readonly BUCKET: Cloudflare.R2Bucket;
    /** The name of the region's R2 bucket. */
    readonly DESTACK_BUCKET: string;
    /** The region's Artifacts namespace of the spaces' Git storages. */
    readonly ARTIFACTS: ArtifactsNamespace;
    /** The releases the Worker version ships, a build layer in its static assets. */
    readonly RELEASES: AssetBinding;
    /** Where the spaces' mail goes, as `MailSender` JSON. */
    readonly DESTACK_MAIL: string;
    /** The Worker's AWS access key: decrypting its sealed keys and sending the spaces' mail. */
    readonly AWS_ACCESS_KEY_ID: string;
    /** The secret of the Worker's AWS access key. */
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
    /** The universe's address, as `UniverseOrigin` JSON. */
    readonly DESTACK_UNIVERSE: string;
    /** The objects of the builds' graph views, one per manifest. */
    readonly GRAPH: DurableObjectNamespace;
    /** The jurisdiction of the machine's objects, empty for none. */
    readonly DESTACK_JURISDICTION: Jurisdiction;
    /** The domain of the installation origins the machine answers, such as destack.space. */
    readonly DESTACK_SPACE_DOMAIN: string;
    /** The machine's own origin, whose paths address its spaces directly. */
    readonly DESTACK_ORIGIN: string;
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
                    "CONFIGURATION_BROKEN",
                    `no service binding '${name}' for '${host}'`,
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
    readonly identity: MachineKey;
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

    /** Read the universe's address from a Worker's bindings. */
    static universe(bindings: Pick<MachineBindings, "DESTACK_UNIVERSE">): UniverseOrigin {
        return UniverseOrigin.of(
            UniverseOrigin.schema.parse(JSON.parse(bindings.DESTACK_UNIVERSE)),
        );
    }

    /** Read the cloud machine of a Worker's bindings in the isolate's system environment. */
    static async read(bindings: MachineBindings, system: Environment): Promise<CloudMachine> {
        // send the universe's and peers' requests through their bindings, others out
        const universe = CloudMachine.universe(bindings);
        const peers = peersOf(bindings);
        const source = HttpTransport.sourceOf(system.network);
        const environment: Environment = {
            ...system,
            network: new HttpTransport(
                {
                    ...source,
                    fetch: (request: Request, options?: FetchOptions) => {
                        // call the universe, and a peer machine or its relay, through their bindings
                        const peer = peers.get(new URL(request.url).host);
                        if (universe.serves(request.url) !== undefined) {
                            return bindings.UNIVERSE.fetch(request);
                        } else if (peer !== undefined) {
                            return peer.fetch(internal(request));
                        }

                        return source.fetch(request, options);
                    },
                },
                system.telemetry,
            ),
        };

        // unseal the machine's root keys and its key
        const id = schema.identifier("machine").parse(bindings.DESTACK_MACHINE_ID);
        const envelope = new Envelope(
            EnvelopeConfiguration.parse(JSON.parse(bindings.DESTACK_ENVELOPE)),
            Envelope.machine(id),
            {
                accessKeyId: bindings.AWS_ACCESS_KEY_ID,
                secretAccessKey: bindings.AWS_SECRET_ACCESS_KEY,
            },
            environment,
        );
        const keyring = await envelope.keyring(bindings.DESTACK_ROOT_KEY, environment);
        const keychain = new MemoryKeychain();
        await keychain.save(id, await envelope.open(bindings.DESTACK_MACHINE_KEY));
        const identity = new MachineKey(id, keychain, environment);

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
            issuer: universe.issuer,
        });
    }

    /** Open the releases the deploy bundled with the Worker version, read in place. */
    releases(): Promise<BuildLayer> {
        return BuildLayer.assets(this.bindings.RELEASES, this.environment.crypto);
    }

    /** Read the machine's enrollment from its directory row, absent before it is declared. */
    async enrollment(view: Pick<Snapshot, "read">): Promise<MachineEnrollment | undefined> {
        // read the machine's row in the directory replica
        const row = await view
            .read(accountObject.machine.table)
            .findFirst({ where: { id: this.id } });
        if (row === undefined) {
            return undefined;
        }

        return {
            kind: "managed",
            accountId: row.scope,
            universe: CloudMachine.universe(this.bindings).toJSON(),
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

    /** Store the machine's replicas in its object's storage, starting each space placed here. */
    async startDirectory(
        storage: DurableObjectStorage,
        alarm: Alarm,
        start: (space: Identifier<"space">) => Promise<void>,
    ): Promise<DurableObjectWorkloadInstance> {
        // store the replicas in the object's own storage, in the isolate's environment
        const host = new DurableObjectDatabaseHost(storage, this.environment);
        const database = host.connect(CloudMachine.database);
        await database.migrate(CloudMachine.database.tables);

        // follow them as the machine through the universe's account package
        const publisher = this.publisher();
        const instance = await WorkloadInstance.start(directoryWorkload, {
            environment: this.environment,
            resources: new ResourceContext().bind(directoryConfiguration, {
                database,
                blobs: new BucketBlobStorage(
                    new R2Bucket(this.bindings.BUCKET),
                    DIRECTORY_PREFIX,
                    this.environment.random,
                ),
                machine: this.identity,
                publisher,
                start,
            }),
            callKey: () => this.callKey(),
            alarm,
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

    /** Open the machine's connection to the universe's account package, followed as itself. */
    publisher(): ObjectConnection {
        return ObjectConnection.open({
            url: ObjectConnection.url(
                PackageMount.url(CloudMachine.universe(this.bindings).origin, accountPackage.id),
            ),
            authorize: (request) =>
                this.identity
                    .at(this.issuer, (sent) => this.environment.network.fetch(sent))
                    .authorize(request, accountPackage.id, {}),
            environment: this.environment,
        });
    }

    /** Host the region's relay, at its region's relay origin, calling its instances' objects. */
    async relay(): Promise<RelayHost> {
        // read the machine's region and its relay's origin from the directory replica
        const snapshot = this.replica.snapshot();
        const row = present(
            await snapshot.read(accountObject.machine.table).findFirst({ where: { id: this.id } }),
            `the machine ${this.id}`,
        );
        const region = await accountObject.Region.require(
            snapshot,
            present(row.regionId, `the region of the managed machine ${this.id}`),
        );

        // follow the universe as the machine, and call each instance in its own object
        const publisher = this.publisher();

        return new RelayHost({
            origin: region.relay,
            region: region.code,
            universe: CloudMachine.universe(this.bindings),
            principal: principal.machine.reference(Scope.universe.id, this.id),
            publisher: () => publisher,
            tunnels: (request) => this.relayObject(RelayHost.partition(request)).fetch(request),
            environment: this.environment,
        });
    }

    /** Read the object running the relay's root, or a tunnel's partition below it. */
    relayObject(partition: string | undefined): { readonly fetch: Fetch } {
        const objects = Jurisdiction.narrow(
            this.bindings.PARTITION,
            this.bindings.DESTACK_JURISDICTION,
        );
        const name = partition === undefined ? RELAY_OBJECT : `${partition}--${RELAY_OBJECT}`;

        return objects.get(objects.idFromName(name));
    }

    /** Read the partition a relay object runs, the root's as absent, nothing for another object. */
    static relayPartition(name: string | undefined): { readonly partition?: string } | undefined {
        const suffix = `--${RELAY_OBJECT}`;

        // read the root's object
        if (name === RELAY_OBJECT) {
            return {};
        }
        // read a tunnel's, below the root's name
        else if (name?.endsWith(suffix) === true) {
            return { partition: name.slice(0, -suffix.length) };
        }
        // read any other object as none
        else {
            return undefined;
        }
    }

    /** Refuse a call to the replicas' packages, which serve nothing. */
    static #refusal() {
        return MachineError.of(
            "CALLS_NOT_FOUND",
            "call not found: a cloud machine object does not accept calls",
        );
    }
}
