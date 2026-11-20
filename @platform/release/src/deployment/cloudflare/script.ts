import type { ProviderClients } from "@destack/account/better-auth";
import { WORKERD_COMPATIBILITY_DATE } from "@destack/package/runtime";
import { builtinModules } from "node:module";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { type Plugin, rolldown } from "rolldown";
import { DOMAINS, InstallationOrigin, UniverseOrigin } from "@destack/account/address";
import { ModuleTransformer } from "@destack/package/transform";
import { modulePlugin } from "@destack/package/vite";
import type { Custody } from "../custody.ts";
import {
    Root,
    ROOT,
    type ResidencyOutputs,
    type AccessKey,
    type UniverseOutputs,
    WRANGLER_VARIABLES,
} from "../root.ts";
import type { Deployment } from "../deployment.ts";
import { PLATFORM, type PlatformResidency, RESIDENCIES } from "@platform/stack";
import { CloudflareCredentials } from "./credential.ts";
import { present, schema } from "@destack/schema";
import { FileSystemError } from "@destack/environment";

/** The pause before asking again when nothing listens at a Worker's origin yet: 0.2 s. */
const CONNECT_RETRY_MILLISECONDS = 200;

/** How long a Worker may take to answer: four times the slowest local start measured, 15 s. */
const ANSWER_MILLISECONDS = 60_000;

/** The file beside a bundled script with the digests of every input it was bundled from. */
const INPUTS_FILE = "inputs.json";

/** The manifests the module transformer reads of each bundled module's package. */
const MANIFESTS = ["package.json", "destack.json"];

/** The digests of a bundle's inputs: its supplied modules, and each file by path. */
const BundleInputs = schema.object({
    supplied: schema.string(),
    files: schema.record(schema.string(), schema.string()),
});
/** The digests of a bundle's inputs. */
type BundleInputs = schema.Infer<typeof BundleInputs>;

/** What a starting server answers: its reason and the pause before asking again. */
const Starting = schema.looseObject({
    reason: schema.literal("SERVER_UNAVAILABLE"),
    data: schema.looseObject({ retryAfter: schema.number().int().positive() }),
});

/** The package conditions selecting each dependency's workerd build. */
const CONDITIONS = ["workerd", "worker", "browser"];

/** The universe's object, near its database in eu-central. */
const UNIVERSE_LOCATION = "weur";

/** The class of the universe's one object. */
const UNIVERSE_CLASS = "DurableObjectUniverse";

/** The universe Worker's Durable Object migrations, which each deploy applies up to the last. */
const UNIVERSE_MIGRATIONS = [
    { tag: "v1", new_sqlite_classes: [UNIVERSE_CLASS] },
    // reset 2026-11-20: drop every object's storage, then serve the class again empty
    { tag: "v2", deleted_classes: [UNIVERSE_CLASS] },
    { tag: "v3", new_sqlite_classes: [UNIVERSE_CLASS] },
];

/** The binding of the releases a Worker ships as its static assets. */
const RELEASES_BINDING = "RELEASES";

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

/** The machine Workers' Durable Object migrations, which each deploy applies up to the last. */
const MACHINE_MIGRATIONS = [
    { tag: "v1", new_sqlite_classes: Object.values(MACHINE_CLASSES) },
    // reset 2026-11-20: drop every space's storage, then serve the classes again empty
    { tag: "v2", deleted_classes: Object.values(MACHINE_CLASSES) },
    { tag: "v3", new_sqlite_classes: Object.values(MACHINE_CLASSES) },
];

/** A Worker's wrangler configuration, as `wrangler.json` takes it. */
export type WranglerConfiguration = Readonly<Record<string, unknown>>;

/** A module a deploy supplies to a bundle: a JSON value, or a table of imported defaults. */
export type SuppliedModule =
    | {
          /** A module exporting a JSON value. */
          readonly kind: "data";
          /** The value. */
          readonly value: unknown;
      }
    | {
          /** A module exporting the defaults of other modules, by key. */
          readonly kind: "imports";
          /** The specifier of each imported module, by key. */
          readonly specifiers: Readonly<Record<string, string>>;
      };

/** The modules a deploy supplies to a script's bundle, by specifier. */
export type SuppliedModules = Readonly<Record<string, SuppliedModule>>;

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

/** One of the Worker scripts a universe deploys. */
export class WorkerScript {
    /** The universe the script deploys to. */
    readonly deployment: Deployment;
    /** What the script deploys. */
    readonly target: ScriptTarget;

    /** Describe the script deploying a target to a universe. */
    constructor(deployment: Deployment, target: ScriptTarget) {
        this.deployment = deployment;
        this.target = target;
    }

    /** The service a universe prints the script's lines under: universe, or a machine's name. */
    get service(): string {
        // name the universe, a residency's machine, or the Tail Worker beside it
        const { target } = this;
        if (target.kind === "universe") {
            return "universe";
        } else if (target.kind === "machine") {
            return target.residency.machine.name;
        } else {
            return `${target.residency.machine.name}-tail`;
        }
    }

    /** The script's name. */
    get name(): string {
        const target = this.target;

        return target.kind === "universe"
            ? `destack-${this.deployment.universe.name}-universe`
            : `destack-${this.deployment.universe.name}-${target.kind}-${target.residency.code}`;
    }

    /** The module the script runs, as its engine package ships it. */
    get entry(): string {
        return WorkerScript.entry(this.target.kind);
    }

    /** Resolve the module a kind of script runs. */
    static entry(kind: ScriptTarget["kind"]): string {
        return fileURLToPath(import.meta.resolve(SCRIPT_ENTRIES[kind]));
    }

    /** Write the script's secrets by binding: its sealed keys, its AWS access key and providers. */
    secrets(
        custody: Custody,
        access: AccessKey,
        providers: ProviderClients,
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

        // sign people in through the providers at the universe
        const signIn =
            target.kind === "universe" ? { DESTACK_PROVIDERS: JSON.stringify(providers) } : {};

        // bind the Worker's AWS access key
        return {
            ...sealed,
            ...signIn,
            AWS_ACCESS_KEY_ID: access.accessKeyId,
            AWS_SECRET_ACCESS_KEY: access.secretAccessKey,
        };
    }

    /** The origin the script answers at, refused for a tail. */
    get origin(): string {
        const target = this.target;
        if (target.kind === "tail") {
            throw new Error(`'${this.name}' answers at no origin`);
        }

        return target.kind === "universe"
            ? this.deployment.universeWorkerOrigin
            : this.deployment.machineOrigin(target.residency);
    }

    /** Write the universe's configuration over its outputs, shipping a releases layer. */
    universe(outputs: UniverseOutputs, releases: string): WranglerConfiguration {
        return {
            ...this.#shared(),
            assets: WorkerScript.#releases(releases),
            durable_objects: { bindings: [{ name: "INSTANCE", class_name: UNIVERSE_CLASS }] },
            migrations: UNIVERSE_MIGRATIONS,
            hyperdrive: [{ binding: "DATABASE", id: outputs.hyperdrive }],
            vars: {
                DESTACK_LOG_FORMAT: this.deployment.logFormat,
                DESTACK_UNIVERSE_ORIGIN: this.deployment.universe.origin,
                DESTACK_LOCATION: UNIVERSE_LOCATION,
                DESTACK_BUCKET: JSON.stringify(outputs.files),
                DESTACK_MAIL: JSON.stringify(this.deployment.mail),
            },
        };
    }

    /** Write a cloud machine's configuration over its residency's outputs and shipped releases. */
    machine(
        outputs: ResidencyOutputs,
        residency: PlatformResidency,
        releases: string,
    ): WranglerConfiguration {
        // keep the objects, the files and the builds in the residency's jurisdiction
        const kept = this.deployment.jurisdiction(residency);
        const jurisdiction = kept === "none" ? {} : { jurisdiction: kept };
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
            assets: WorkerScript.#releases(releases),
            durable_objects: {
                bindings: [
                    { name: "PARTITION", class_name: MACHINE_CLASSES.partition },
                    { name: "GRAPH", class_name: MACHINE_CLASSES.graph },
                ],
            },
            tail_consumers: [{ service: tail.name }],
            migrations: MACHINE_MIGRATIONS,
            worker_loaders: [{ binding: "LOADER" }],
            r2_buckets: [{ binding: "BUCKET", bucket_name: outputs.bucket, ...jurisdiction }],
            artifacts: [
                {
                    binding: "ARTIFACTS",
                    namespace: `destack-${this.deployment.universe.name}-repositories-${residency.code}`,
                },
            ],
            services: [
                { binding: "UNIVERSE", service: universe.name },
                ...peers.map(({ binding, script }) => ({ binding, service: script.name })),
            ],
            routes: this.#routes(outputs, residency),
            vars: {
                DESTACK_LOG_FORMAT: this.deployment.logFormat,
                DESTACK_UNIVERSE_ORIGIN: this.deployment.universe.origin,
                DESTACK_JURISDICTION: kept,
                DESTACK_BUCKET: outputs.bucket,
                DESTACK_MAIL: JSON.stringify(this.deployment.mail),
                DESTACK_MACHINE_ID: residency.machine.id,
                DESTACK_ORIGIN: this.deployment.machineOrigin(residency),
                DESTACK_SPACE_DOMAIN: InstallationOrigin.domain(
                    this.deployment.machineOrigin(residency),
                ),
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
        // route the machine's host and its relay's host
        const relay = new URL(this.deployment.relayOrigin(residency)).host;
        const own = [outputs.host, relay].map((name) => ({
            pattern: `${name}/*`,
            zone_name: DOMAINS.machine,
        }));
        if (residency.code !== PLATFORM.residencyId) {
            return own;
        }

        // take every other space and machine name, and the universe's, at the platform residency's machine
        return [
            ...own,
            { pattern: `*.${DOMAINS.space}/*`, zone_name: DOMAINS.space },
            { pattern: `*.${DOMAINS.machine}/*`, zone_name: DOMAINS.machine },
            { pattern: `${DOMAINS.universe}/*`, zone_name: DOMAINS.universe },
        ];
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
            vars: {
                DESTACK_LOG_FORMAT: this.deployment.logFormat,
                DESTACK_JURISDICTION: this.deployment.jurisdiction(residency),
            },
        };
    }

    /** Bundle the script for workerd into one module, answering the files it read. */
    async build(directory: string, modules: SuppliedModules): Promise<readonly string[]> {
        // bundle with module metadata, leaving workerd's own modules to the runtime
        const { environment } = this.deployment;
        const { files } = environment;
        await using bundle = await rolldown({
            input: this.entry,
            platform: "browser",
            resolve: { conditionNames: CONDITIONS },
            // define the production mode as wrangler's bundler does
            transform: { define: { "process.env.NODE_ENV": JSON.stringify("production") } },
            // leave the runtime's modules as imports, Node's with and without their prefix
            external: [/^cloudflare:/u, /^node:/u, /^bun(?::|$)/u, ...builtinModules],
            plugins: [modulePlugin(new ModuleTransformer(environment)), supplyPlugin(modules)],
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
                `'${this.name}' bundles into ${output.length} outputs instead of one module`,
            );
        }

        // refuse a bundle still importing a Bun module, which workerd lacks
        const imported = /\bfrom\s*"(bun(?::[a-z]+)?)"/u.exec(chunk.code)?.[1];
        if (imported !== undefined) {
            throw new Error(`'${this.name}' imports '${imported}', which workerd lacks`);
        }

        // write the module
        await files.mkdir(directory, { recursive: true });
        await files.writeTextFile(resolve(directory, "index.js"), chunk.code);

        // answer the bundled files, leaving out the supplied virtual modules
        return chunk.moduleIds.filter((id) => !id.startsWith("\0"));
    }

    /** Bundle the script when an input changed, write its configuration, and answer its path. */
    async write(configuration: WranglerConfiguration, modules: SuppliedModules): Promise<string> {
        // bundle the script into a fresh directory unless its last bundle read the same inputs
        const { files } = this.deployment.environment;
        const directory = resolve(ROOT, ".wrangler", this.deployment.universe.name, this.name);
        const inputs = resolve(directory, INPUTS_FILE);
        if (!(await this.#isBundled(directory, modules))) {
            await this.deployment.scope.span(
                "platform.worker.bundle",
                { "platform.worker.name": this.service },
                async () => {
                    // bundle into a fresh directory, recording the inputs it read
                    await files.remove(directory, { recursive: true, force: true });
                    const read = await this.build(directory, modules);
                    const digests = await this.#inputs(read, modules);
                    await files.writeTextFile(inputs, JSON.stringify(digests));
                },
                { level: "info" },
            );
        }

        // write the configuration, which changes without the bundle
        const path = resolve(directory, "wrangler.json");
        await files.writeTextFile(path, `${JSON.stringify(configuration, null, 4)}\n`);

        return path;
    }

    /** Decide whether the script's last bundle read the same inputs as now. */
    async #isBundled(directory: string, modules: SuppliedModules): Promise<boolean> {
        // read the last bundle's inputs, none before a first bundle
        const { files } = this.deployment.environment;
        const text = await files
            .readTextFile(resolve(directory, INPUTS_FILE))
            .catch((error: unknown) => {
                if (FileSystemError.is(error, "FILE_NOT_FOUND")) {
                    return undefined;
                }
                throw error;
            });
        if (text === undefined) {
            return false;
        }

        // compare them with the same files' digests now
        const recorded = BundleInputs.parse(JSON.parse(text));
        const current = await this.#inputs(Object.keys(recorded.files), modules);

        return (
            current.supplied === recorded.supplied &&
            Object.entries(recorded.files).every(([path, digest]) => current.files[path] === digest)
        );
    }

    /** Digest a bundle's files, their manifests, the bundler and the supplied modules. */
    async #inputs(read: readonly string[], modules: SuppliedModules): Promise<BundleInputs> {
        // add the manifests of each read file's package and the bundler's own code
        const { files, crypto } = this.deployment.environment;
        const roots = new Map<string, string | undefined>();
        const packages = new Set(
            (await Promise.all(read.map((path) => this.#root(dirname(path), roots)))).flatMap(
                (root) => root ?? [],
            ),
        );
        const paths = [
            ...new Set([
                ...read,
                ...[...packages].flatMap((root) => MANIFESTS.map((name) => resolve(root, name))),
                fileURLToPath(import.meta.url),
                fileURLToPath(import.meta.resolve("rolldown/package.json")),
            ]),
        ];

        // digest each file, a missing one as absent
        const digest = async (bytes: Uint8Array<ArrayBuffer>) =>
            new Uint8Array(await crypto.digest("SHA-256", bytes)).toHex();
        const entries = await Promise.all(
            paths.map(async (path) => {
                const bytes = await files.readFile(path).catch((error: unknown) => {
                    if (FileSystemError.is(error, "FILE_NOT_FOUND")) {
                        return undefined;
                    }
                    throw error;
                });

                return [path, bytes === undefined ? "" : await digest(bytes)] as const;
            }),
        );
        const supplied = await digest(new TextEncoder().encode(JSON.stringify(modules)));

        return { supplied, files: Object.fromEntries(entries) };
    }

    /** Find the package directory a directory lies in, the nearest one with a package.json. */
    async #root(
        directory: string,
        roots: Map<string, string | undefined>,
    ): Promise<string | undefined> {
        // answer a directory looked up before, else look in it, then in its parent
        if (roots.has(directory)) {
            return roots.get(directory);
        }
        const parent = dirname(directory);
        const root = (await this.deployment.environment.files.exists(
            resolve(directory, "package.json"),
        ))
            ? directory
            : parent === directory
              ? undefined
              : await this.#root(parent, roots);
        roots.set(directory, root);

        return root;
    }

    /** Wait until the script answers at its origin without a server failure, in milliseconds. */
    async waitReady(): Promise<number> {
        // request the issuer's discovery document from the universe, and the origin from a cloud machine
        const isUniverse = this.target.kind === "universe";
        const issuer = UniverseOrigin.issuer(this.origin);
        const url = isUniverse
            ? new URL(`${issuer}/.well-known/openid-configuration`)
            : new URL("/", this.origin);
        const { network, clock } = this.deployment.environment;
        const asked = clock.now();

        // ask again while nothing listens yet or the script answers that it is starting, within the deadline
        const ask = async (): Promise<{
            readonly answered?: Response;
            readonly retryAfter?: number;
        }> => {
            // request the address, a refused connection meaning workerd does not listen yet
            const answered = await network
                .fetch(new Request(url, { redirect: "manual" }))
                .catch(() => undefined);
            if (answered === undefined) {
                return { retryAfter: CONNECT_RETRY_MILLISECONDS };
            }

            // read a failure's body when it is JSON, as a starting server's is
            const type = answered.headers.get("content-type") ?? "";
            const isJson = !answered.ok && type.startsWith("application/json");
            const body: unknown = isJson ? await answered.clone().json() : undefined;

            // ask again after the pause a starting server names
            const starting = Starting.safeParse(body).data;

            return starting === undefined
                ? { answered }
                : { answered, retryAfter: starting.data.retryAfter };
        };
        let reply = await ask();
        while (reply.retryAfter !== undefined) {
            if (clock.now() - asked > ANSWER_MILLISECONDS) {
                throw new Error(
                    `'${this.name}' did not answer '${url.href}' within ${ANSWER_MILLISECONDS} ms`,
                );
            }
            await clock.sleep(reply.retryAfter);
            reply = await ask();
        }
        const response = present(reply.answered, `the answer of '${this.name}'`);

        // refuse a server failure, and any failure of the universe
        if (response.status >= 500 || (isUniverse && !response.ok)) {
            const body = await response.text();
            throw new Error(
                `'${this.name}' answered '${url.href}' with ${response.status}: '${body}'`,
            );
        }

        return clock.now() - asked;
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

    /** Ship a releases layer as static assets, which the Worker reads through their binding. */
    static #releases(directory: string): WranglerConfiguration {
        return {
            directory,
            binding: RELEASES_BINDING,
            run_worker_first: true,
        };
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
}

/** Resolve the specifiers a deploy supplies to their data, as modules exporting it. */
function supplyPlugin(modules: SuppliedModules): Plugin {
    return {
        name: "destack:supply",
        // mark a supplied specifier as a virtual module
        resolveId: (specifier) => (Object.hasOwn(modules, specifier) ? `\0${specifier}` : null),
        // export the supplied value of a virtual module, or its imported defaults by key
        load: (id) => {
            // take a supplied virtual module, leaving every other one to the bundler
            const supplied = id.startsWith("\0") ? modules[id.slice(1)] : undefined;
            if (supplied === undefined) {
                return null;
            } else if (supplied.kind === "data") {
                return `export default ${JSON.stringify(supplied.value)};`;
            }
            const entries = Object.entries(supplied.specifiers);
            const imports = entries.map(
                ([, specifier], index) => `import m${index} from ${JSON.stringify(specifier)};`,
            );
            const table = entries.map(([key], index) => `${JSON.stringify(key)}: m${index}`);

            return `${imports.join("\n")}\nexport default { ${table.join(", ")} };`;
        },
    };
}
