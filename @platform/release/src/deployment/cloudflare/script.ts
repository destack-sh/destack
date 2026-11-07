import { present } from "@destack/schema";
import { DeclarationReference } from "@destack/package/declare";
import { FORGE_INSTALLATION } from "@destack/forge/package";
import { WORKERD_COMPATIBILITY_DATE } from "@destack/package/runtime";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { type Plugin, rolldown } from "rolldown";
import { DOMAINS } from "@destack/account/address";
import accountPackage, { OAUTH_PATH } from "@destack/account/package";
import { PackageMount } from "@destack/object";
import { ModuleTransformer } from "@destack/package/transform";
import { modulePlugin } from "@destack/package/vite";
import { DURABLE_OBJECT_GIGABYTES } from "@platform/pricing/cloudflare";
import { UNIT_SKUS } from "@platform/pricing";
import { S3Client } from "@destack/bucket/s3";
import { BuildBucket } from "@destack/build/store";
import type { Custody } from "../custody.ts";
import {
    Root,
    ROOT,
    type ResidencyOutputs,
    type AccessKey,
    type SharedOutputs,
    type UniverseOutputs,
    WRANGLER_VARIABLES,
} from "../root.ts";
import type { Deployment } from "../deployment.ts";
import { PLATFORM, type PlatformResidency, RESIDENCIES } from "@platform/stack";
import { CloudflareCredentials } from "./credential.ts";

/** The package conditions selecting each dependency's workerd build. */
const CONDITIONS = ["workerd", "worker", "browser"];

/** The AWS region SES sends Destack's mail from, where the shared root verifies destack.app. */
const MAIL_REGION = "eu-central-1";

/** The mailbox Destack's mail comes from, on the domain SES verifies. */
const MAIL_FROM = "Destack <notices@destack.app>";

/** The universe's object, near its database in eu-central. */
const UNIVERSE_LOCATION = "weur";

/** The class of the universe's one object. */
const UNIVERSE_CLASS = "DurableObjectUniverse";

/** The region R2 signs S3 requests for. */
const R2_REGION = "auto";

/** The module each kind of script runs, as its engine package ships it. */
const SCRIPT_ENTRIES = {
    universe: "@platform/release/deployment/universe",
    machine: "@destack/machine/cloudflare/worker",
    tail: "@destack/machine/cloudflare/tail",
} as const;

/** The classes of a cloud machine's objects. */
const MACHINE_CLASSES = {
    partition: "PartitionObject",
    database: "DurableObjectDatabase",
    graph: "GraphIndexObject",
} as const;

/** A Worker's wrangler configuration, as `wrangler.json` takes it. */
export type WranglerConfiguration = Readonly<Record<string, unknown>>;

/** The data modules a deploy supplies to a script's bundle, each a JSON value by specifier. */
export type SuppliedModules = Readonly<Record<string, unknown>>;

/** The module the universe's bundle reads its declaration from, supplied by its deploy. */
export const UNIVERSE_MODULE = "destack:universe";

/** What a Worker script deploys: the universe, or a residency's machine or its tail. */
export type ScriptTarget =
    | {
          /** The universe's Worker. */
          readonly kind: "universe";
      }
    | {
          /** A cloud machine's Worker, or the Tail Worker metering its traces. */
          readonly kind: "machine" | "tail";
          /** The residency the machine serves. */
          readonly residency: PlatformResidency;
      };

/** One of the Worker scripts a stage deploys. */
export class WorkerScript {
    /** The stage the script deploys to. */
    readonly deployment: Deployment;
    /** What the script deploys. */
    readonly target: ScriptTarget;

    /** Describe the script deploying a target to a stage. */
    constructor(deployment: Deployment, target: ScriptTarget) {
        this.deployment = deployment;
        this.target = target;
    }

    /** The script's name. */
    get name(): string {
        const target = this.target;

        return target.kind === "universe"
            ? `destack-${this.deployment.stage.name}-universe`
            : `destack-${this.deployment.stage.name}-${target.kind}-${target.residency.code}`;
    }

    /** The module the script runs, as its engine package ships it. */
    get entry(): string {
        return WorkerScript.entry(this.target.kind);
    }

    /** Resolve the module a kind of script runs. */
    static entry(kind: ScriptTarget["kind"]): string {
        return fileURLToPath(import.meta.resolve(SCRIPT_ENTRIES[kind]));
    }

    /** Write the script's secrets by binding: its sealed keys, its KMS key and the mail key. */
    secrets(
        custody: Custody,
        kms: AccessKey | undefined,
        shared: SharedOutputs,
    ): Readonly<Record<string, string>> {
        // keep no secrets in a tail
        const target = this.target;
        if (target.kind === "tail") {
            return {};
        }

        // bind the sealed keys, a cloud machine's own key among them
        const keys =
            target.kind === "universe"
                ? custody.record.universe
                : custody.machine(target.residency.machine.id);
        const machineKey =
            target.kind === "universe"
                ? {}
                : { DESTACK_MACHINE_KEY: custody.machine(target.residency.machine.id).key };
        const sealed = {
            DESTACK_ENVELOPE: JSON.stringify(keys.envelope),
            DESTACK_ROOT_KEY: keys.rootKey,
            ...machineKey,
        };

        // decrypt them with the Worker's KMS key under a KMS envelope, refusing one without it
        let decrypting = {};
        if (keys.envelope.kind === "aws-kms") {
            if (kms === undefined) {
                throw new Error(
                    `${this.name} needs its KMS access key to open its aws-kms envelope`,
                );
            }
            decrypting = {
                DESTACK_KMS_ACCESS_KEY_ID: kms.accessKeyId,
                DESTACK_KMS_SECRET_ACCESS_KEY: kms.secretAccessKey,
            };
        }

        // send mail with the shared root's key
        return {
            ...sealed,
            ...decrypting,
            AWS_ACCESS_KEY_ID: shared.mail.accessKeyId,
            AWS_SECRET_ACCESS_KEY: shared.mail.secretAccessKey,
        };
    }

    /** The origin the script answers at, refused for a tail. */
    get origin(): string {
        const target = this.target;
        if (target.kind === "tail") {
            throw new Error(`${this.name} answers at no origin`);
        }

        return target.kind === "universe"
            ? this.deployment.issuer
            : this.deployment.machineOrigin(target.residency);
    }

    /** Write the universe's configuration over its outputs and the platform residency's builds. */
    universe(outputs: UniverseOutputs, platform: ResidencyOutputs): WranglerConfiguration {
        // read the shipped releases from the platform residency's build bucket, in its jurisdiction
        const residency = present(
            RESIDENCIES.find((each) => each.code === PLATFORM.residencyId),
            "the platform residency",
        );
        const kept = this.deployment.jurisdiction(residency);
        const jurisdiction = kept === "local" ? {} : { jurisdiction: kept };

        return {
            ...this.#shared(),
            durable_objects: { bindings: [{ name: "INSTANCE", class_name: UNIVERSE_CLASS }] },
            migrations: [{ tag: "v1", new_sqlite_classes: [UNIVERSE_CLASS] }],
            hyperdrive: [{ binding: "DATABASE", id: outputs.hyperdrive }],
            r2_buckets: [
                { binding: "EVENTS", bucket_name: outputs.history },
                { binding: "BUILDS", bucket_name: platform.packages, ...jurisdiction },
            ],
            routes: [{ pattern: new URL(this.origin).host, custom_domain: true }],
            vars: {
                DESTACK_ISSUER: this.deployment.issuer,
                DESTACK_LOCATION: UNIVERSE_LOCATION,
                DESTACK_MAIL_REGION: MAIL_REGION,
                DESTACK_MAIL_FROM: MAIL_FROM,
            },
        };
    }

    /** Write a cloud machine's configuration over its residency's outputs and shipped releases. */
    machine(outputs: ResidencyOutputs, residency: PlatformResidency): WranglerConfiguration {
        // keep the objects, the files and the builds in the residency's jurisdiction
        const kept = this.deployment.jurisdiction(residency);
        const jurisdiction = kept === "local" ? {} : { jurisdiction: kept };
        const universe = new WorkerScript(this.deployment, { kind: "universe" });
        const tail = new WorkerScript(this.deployment, { kind: "tail", residency });

        // reach every cloud machine and its relay through a service binding, never their public hosts
        const peers = RESIDENCIES.map((peer) => ({
            binding: `MACHINE_${peer.code.toUpperCase()}`,
            script: new WorkerScript(this.deployment, { kind: "machine", residency: peer }),
            hosts: [
                new URL(this.deployment.machineOrigin(peer)).host,
                new URL(this.deployment.relayOrigin(peer)).host,
            ],
        }));

        return {
            ...this.#shared(),
            durable_objects: {
                bindings: [
                    { name: "PARTITION", class_name: MACHINE_CLASSES.partition },
                    { name: "GRAPH", class_name: MACHINE_CLASSES.graph },
                ],
            },
            tail_consumers: [{ service: tail.name }],
            migrations: [{ tag: "v1", new_sqlite_classes: Object.values(MACHINE_CLASSES) }],
            worker_loaders: [{ binding: "LOADER" }],
            r2_buckets: [
                { binding: "FILES", bucket_name: outputs.files, ...jurisdiction },
                { binding: "BUILDS", bucket_name: outputs.packages, ...jurisdiction },
            ],
            artifacts: [
                {
                    binding: "ARTIFACTS",
                    namespace: `destack-${this.deployment.stage.name}-repositories-${residency.code}`,
                },
            ],
            services: [
                { binding: "UNIVERSE", service: universe.name },
                ...peers.map(({ binding, script }) => ({ binding, service: script.name })),
            ],
            routes: this.#routes(outputs, residency),
            vars: {
                DESTACK_MACHINE_ID: residency.machine.id,
                DESTACK_ISSUER: this.deployment.issuer,
                DESTACK_JURISDICTION: kept,
                DESTACK_SPACE_DOMAIN: DOMAINS.space,
                DESTACK_HOST: outputs.host,
                DESTACK_FILES: outputs.files,
                DESTACK_S3: `https://${outputs.s3}`,
                DESTACK_REGISTRY: FORGE_INSTALLATION,
                DESTACK_RATES: JSON.stringify({
                    skus: Object.fromEntries(
                        UNIT_SKUS.map((sku) => [
                            DeclarationReference.key(sku.definition.meter),
                            DeclarationReference.key(sku.reference),
                        ]),
                    ),
                    gigabytes: DURABLE_OBJECT_GIGABYTES,
                }),
                DESTACK_MAIL_REGION: MAIL_REGION,
                DESTACK_MAIL_FROM: MAIL_FROM,
                DESTACK_PEERS: JSON.stringify(
                    Object.fromEntries(
                        peers.flatMap(({ binding, hosts }) => hosts.map((host) => [host, binding])),
                    ),
                ),
            },
        };
    }

    /** Route a machine's own names and relay host, and the edge's names at the platform's. */
    #routes(outputs: ResidencyOutputs, residency: PlatformResidency) {
        // route the machine's host, its buckets' S3 names and its relay's host
        const relay = new URL(this.deployment.relayOrigin(residency)).host;
        const own = [outputs.host, outputs.s3, `*.${outputs.s3}`, relay].map((name) => ({
            pattern: `${name}/*`,
            zone_name: DOMAINS.machine,
        }));
        if (residency.code !== PLATFORM.residencyId) {
            return own;
        }

        // take every other space and machine name at the platform residency's machine
        return [
            ...own,
            { pattern: `*.${DOMAINS.space}/*`, zone_name: DOMAINS.space },
            { pattern: `*.${DOMAINS.machine}/*`, zone_name: DOMAINS.machine },
        ];
    }

    /** Open a cloud machine's build bucket, signed with its R2 key. */
    bucket(outputs: ResidencyOutputs, residency: PlatformResidency): BuildBucket {
        // sign with the R2 key the residency root gives the machine
        const credentials = outputs.r2;
        const location = {
            endpoint: this.#r2(outputs, residency),
            bucket: outputs.packages,
            region: R2_REGION,
        };
        const { environment } = this.deployment;

        return new BuildBucket(
            S3Client.of({ location, credentials }, environment),
            environment.crypto,
        );
    }

    /** Write a cloud machine's Tail Worker configuration over the machine's objects. */
    tail(residency: PlatformResidency): WranglerConfiguration {
        const machine = new WorkerScript(this.deployment, { kind: "machine", residency });

        return {
            ...this.#shared(),
            durable_objects: {
                bindings: [
                    {
                        name: "PARTITION",
                        class_name: MACHINE_CLASSES.partition,
                        script_name: machine.name,
                    },
                ],
            },
            vars: { DESTACK_JURISDICTION: this.deployment.jurisdiction(residency) },
        };
    }

    /** Bundle the script's entry and its supplied modules for workerd into one module. */
    async build(directory: string, modules: SuppliedModules): Promise<void> {
        // bundle with module metadata, leaving workerd's own modules to the runtime
        const { files } = this.deployment.environment;
        await using bundle = await rolldown({
            input: this.entry,
            platform: "browser",
            resolve: { conditionNames: CONDITIONS },
            // define the production mode as wrangler's bundler does
            transform: { define: { "process.env.NODE_ENV": JSON.stringify("production") } },
            external: [/^cloudflare:/u, /^node:/u, /^bun(?::|$)/u],
            plugins: [modulePlugin(new ModuleTransformer(files)), supplyPlugin(modules)],
        });
        const { output } = await bundle.generate({
            format: "esm",
            minify: true,
            codeSplitting: false,
        });

        // require one module, refusing assets or chunks a single script cannot carry
        const [chunk, ...others] = output;
        if (chunk?.type !== "chunk" || others.length > 0) {
            throw new Error(
                `${this.name} bundles into ${output.length} outputs instead of one module`,
            );
        }

        // refuse a bundle still importing a Bun module, which workerd lacks
        const imported = /\bfrom\s*"(bun(?::[a-z]+)?)"/u.exec(chunk.code)?.[1];
        if (imported !== undefined) {
            throw new Error(`${this.name} imports ${imported}, which workerd lacks`);
        }

        // write the module
        await files.mkdir(directory, { recursive: true });
        await files.writeTextFile(resolve(directory, "index.js"), chunk.code);
    }

    /** Bundle the script and write its wrangler configuration beside it, answering its path. */
    async write(configuration: WranglerConfiguration, modules: SuppliedModules): Promise<string> {
        // bundle the script into a fresh directory with its configuration beside it
        const { files } = this.deployment.environment;
        const directory = resolve(ROOT, ".wrangler", this.deployment.stage.name, this.name);
        await files.remove(directory, { recursive: true, force: true });
        await this.build(directory, modules);
        const path = resolve(directory, "wrangler.json");
        await files.writeTextFile(path, `${JSON.stringify(configuration, null, 4)}\n`);

        return path;
    }

    /** Require the script to answer at its origin without a server failure. */
    async smoke(): Promise<void> {
        // request the issuer's discovery document from the universe, and the origin from a cloud machine
        const isUniverse = this.target.kind === "universe";
        const issuer = `${PackageMount.url(this.origin, accountPackage.id)}${OAUTH_PATH}`;
        const url = isUniverse
            ? new URL(`${issuer}/.well-known/openid-configuration`)
            : new URL("/", this.origin);
        const response = await this.deployment.environment.network.fetch(
            new Request(url, { redirect: "manual" }),
        );
        if (response.status >= 500 || (isUniverse && !response.ok)) {
            const body = await response.text();
            throw new Error(`${this.name} answered ${url.href} with ${response.status}: ${body}`);
        }
    }

    /** Deploy the script with its configuration and its secrets. */
    async deploy(
        configuration: WranglerConfiguration,
        secrets: Readonly<Record<string, string>>,
        modules: SuppliedModules,
    ): Promise<void> {
        // write the configuration beside the bundle
        const path = await this.write(configuration, modules);

        // upload the secrets with the version from a private file, removed after the upload
        const { environment } = this.deployment;
        const directory = await environment.files.makeTemporaryDirectory("destack-secrets-");
        try {
            const file = resolve(directory, "secrets.json");
            await environment.files.writeTextFile(file, JSON.stringify(secrets), { mode: 0o600 });
            await Root.execute(
                resolve(ROOT, "node_modules/.bin/wrangler"),
                ["deploy", "--config", path, "--secrets-file", file],
                {
                    ...Root.inherited(environment.process.variables, WRANGLER_VARIABLES),
                    ...(await CloudflareCredentials.resolve(environment)).environment,
                    WRANGLER_SEND_METRICS: "false",
                },
                environment,
            );
        } finally {
            await environment.files.remove(directory, { recursive: true, force: true });
        }
    }

    /** Write what every script shares: its name, bundled module, runtime and observability. */
    #shared(): WranglerConfiguration {
        return {
            name: this.name,
            main: "index.js",
            no_bundle: true,
            compatibility_date: WORKERD_COMPATIBILITY_DATE,
            compatibility_flags: ["nodejs_compat"],
            workers_dev: false,
            preview_urls: false,
            observability: { enabled: true },
        };
    }

    /** Address R2's S3 API in a residency's jurisdiction. */
    #r2(outputs: ResidencyOutputs, residency: PlatformResidency): URL {
        const kept = this.deployment.jurisdiction(residency);
        const r2 = kept === "local" ? "r2" : `${kept}.r2`;

        return new URL(`https://${outputs.account}.${r2}.cloudflarestorage.com`);
    }
}

/** Resolve the specifiers a deploy supplies to their data, as modules exporting it. */
function supplyPlugin(modules: SuppliedModules): Plugin {
    return {
        name: "destack:supply",
        // mark a supplied specifier as a virtual module
        resolveId: (specifier) => (Object.hasOwn(modules, specifier) ? `\0${specifier}` : null),
        // export the supplied value of a virtual module
        load: (id) => {
            const specifier = id.slice(1);

            return id.startsWith("\0") && Object.hasOwn(modules, specifier)
                ? `export default ${JSON.stringify(modules[specifier])};`
                : null;
        },
    };
}
