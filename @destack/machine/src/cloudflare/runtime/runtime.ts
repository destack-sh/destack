import type { Random } from "@destack/environment";
import { Authentication } from "@destack/access";
import { Digest, type Identifier, JsonValue } from "@destack/schema";
import { START_PATH, STOP_PATH } from "@destack/object/cloudflare";
import type { Deriver } from "@destack/identity";
import {
    InstanceSpec,
    InstanceRegistry,
    type Runtime,
    SECRET_BYTES,
    WorkloadRequest,
} from "@destack/stack/runtime";
import {
    type DurableObjectInstanceStart,
    type DurableObjectNamespace,
    PartitionName,
    type WorkerLoader,
} from "./instance.ts";
import { WORKERD_COMPATIBILITY_DATE } from "@destack/package/runtime";
import { type BuildReader, ImportMap } from "@destack/package/manifest";
import type { SpaceBuildStore } from "@destack/stack/server";
import type { ImportClosure } from "@destack/build/store";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../../error/index.ts";

/** The origin of the requests an instance's object serves, which its stub alone routes. */
const INSTANCE_ORIGIN = "https://instance";

/** The runtime the Cloudflare machine's instances run on. */
const WORKERD_RUNTIME = "workerd";

/** The media type of the JavaScript modules a workerd output distributes. */
const MODULE_MEDIA_TYPE = "text/javascript";

/** The base of the module names in an isolate, which name each build's modules below its digest. */
const MODULE_BASE = "/";

/** The module a call's isolate starts from, which calls the export a request names. */
const CALL_MODULE = "destack-call.js";

/** An instance this runtime started, with its facet and secret. */
interface Runner {
    /** The instance's spec. */
    readonly spec: InstanceSpec;
    /** The instance's facet in the space's object. */
    readonly object: { readonly fetch: (request: Request) => Promise<Response> };
    /** The secret the space and the instance prove each other's requests with. */
    readonly secret: string;
    /** The start its objects take: its root's, and each partition's with the partition. */
    readonly start: DurableObjectInstanceStart;
    /** The objects of the partitions below its root, by partition. */
    readonly partitions: Map<
        string,
        Promise<{ readonly fetch: (request: Request) => Promise<Response> }>
    >;
}

/** Run a space's instances on workerd, each in a facet of the space's object. */
export class WorkerdRuntime implements Runtime {
    /** The server runtime. */
    readonly name = WORKERD_RUNTIME;
    /** The partitions' objects, in the space's jurisdiction. */
    readonly #instances: DurableObjectNamespace;
    /** The facet of an installation in the space's object, beside the space. */
    readonly #facet: (installation: string) => {
        readonly fetch: (request: Request) => Promise<Response>;
    };
    /** The loader of the calls' isolates. */
    readonly #loader: WorkerLoader;
    /** The space whose object serves the instances' egress and build files. */
    readonly #space: string;
    /** The space's egress, below which the instances call addresses. */
    readonly #egress: string;
    /** Resolve the share of traces an installation samples. */
    readonly #sampling: (
        scope: Identifier<"space">,
        installation: Identifier<"installation">,
    ) => Promise<number>;
    /** Resolve the public URL an installation's instances serve at. */
    readonly #url: (
        scope: Identifier<"space">,
        installation: Identifier<"installation">,
    ) => Promise<string>;
    /** Read the identity of an instance's space, which its installation's root derives from. */
    readonly #identity: (scope: string) => Promise<Pick<Deriver, "derive">>;
    /** The randomness the instances' secrets draw from. */
    readonly #random: Random;
    /** Read the space's builds, which map a build's imports and open its dependencies. */
    readonly #builds: () => Pick<SpaceBuildStore, "imports" | "contents">;
    /** The instances starting and running. */
    readonly #registry = new InstanceRegistry<Runner>();

    /** Run a space's instances in its object's facets, calling out through the space's egress. */
    constructor(options: {
        /** The partitions' objects, in the space's jurisdiction. */
        readonly instances: DurableObjectNamespace;
        /** The facet of an installation in the space's object. */
        readonly facet: (installation: string) => {
            readonly fetch: (request: Request) => Promise<Response>;
        };
        /** The loader of the calls' isolates. */
        readonly loader: WorkerLoader;
        /** The space whose object serves the instances' egress and build files. */
        readonly space: string;
        /** The space's egress, below which the instances call addresses. */
        readonly egress: string;
        /** Resolve the share of traces an installation samples, as its space's settings set it. */
        readonly sampling: (
            scope: Identifier<"space">,
            installation: Identifier<"installation">,
        ) => Promise<number>;
        /** Resolve the public URL an installation's instances serve at. */
        readonly url: (
            scope: Identifier<"space">,
            installation: Identifier<"installation">,
        ) => Promise<string>;
        /** Read the identity of an instance's space, which its installation's root derives from. */
        readonly identity: (scope: string) => Promise<Pick<Deriver, "derive">>;
        /** The randomness the instances' secrets draw from. */
        readonly random: Random;
        /** Read the space's builds, which map a build's imports and open its dependencies. */
        readonly builds: () => Pick<SpaceBuildStore, "imports" | "contents">;
    }) {
        // set the objects and facets, the space, its egress, the sampling and the spaces' identities
        this.#instances = options.instances;
        this.#facet = options.facet;
        this.#loader = options.loader;
        this.#space = options.space;
        this.#egress = options.egress;
        this.#sampling = options.sampling;
        this.#url = options.url;
        this.#identity = options.identity;
        this.#random = options.random;
        this.#builds = options.builds;
    }

    /** Start an instance in its installation's facet, replacing its previous deployment. */
    async start(spec: InstanceSpec): Promise<void> {
        return this.#registry.start(spec, (identify) => this.#start(spec, identify));
    }

    /** Report whether an instance this runtime started serves now. */
    isRunning(instanceId: Identifier<"instance">): boolean {
        return this.#registry.isRunning(instanceId);
    }

    /** Find the spec of the instance whose requests a secret proves. */
    identify(secret: string): InstanceSpec | undefined {
        return this.#registry.identify(secret);
    }

    /** Stop an instance after any start in progress, draining its workload in its object. */
    async stop(instanceId: Identifier<"instance">): Promise<void> {
        await this.#registry.stop(instanceId, (running) => WorkerdRuntime.#drain(running));
    }

    /** Drain a running instance's facet and each started partition's object. */
    static async #drain(running: Runner): Promise<void> {
        // read the root's object and each partition that started, skipping a failed start
        const started = await Promise.allSettled(running.partitions.values());
        const objects = [
            running.object,
            ...started.flatMap((result) => (result.status === "fulfilled" ? [result.value] : [])),
        ];

        // stop the root, then each started partition
        for (const object of objects) {
            await WorkerdRuntime.#requireAnswer(
                await object.fetch(
                    new Request(`${INSTANCE_ORIGIN}${STOP_PATH}`, { method: "POST" }),
                ),
                `stop ${running.spec.instanceId}`,
            );
        }
    }

    /** Read every module of the build a secret proves and of its dependencies, imports resolved. */
    async modules(secret: string): Promise<Record<string, string>> {
        // require the build a secret proves
        const build = this.identify(secret)?.build;
        if (build === undefined) {
            throw MachineError.of(
                "INSTANCE_SECRET_UNAUTHENTICATED",
                "authentication required: the instance secret is invalid",
            );
        }

        return this.#modules(build);
    }

    /** Read each module a build's imports reach, resolved through its import map. */
    async #modules(build: BuildReader): Promise<Record<string, string>> {
        // read each module the build's imports reach, its specifiers resolved through the closure's import map
        const closure = await this.#closure(build);
        const own = await build.digest();
        const read = await Promise.all(
            (await this.#names(build, closure)).map(async (name) => {
                // load the module from its build, the instance's own or a dependency the space keeps
                const [digest = "", ...path] = name.split("/");
                const reader =
                    digest === own ? build : await this.#builds().contents(Digest.parse(digest));
                const code = new TextDecoder().decode(await reader.load(path.join("/")));
                const resolved = await ImportMap.apply(closure.map, code, `${MODULE_BASE}${name}`);

                return [name, resolved] as const;
            }),
        );

        return Object.fromEntries(read);
    }

    /** Call an export of a build's module once, in an isolate of its own beside the space. */
    async call(
        build: BuildReader,
        path: string,
        name: string,
        input: readonly JsonValue[],
    ): Promise<JsonValue> {
        // load the build's modules beside a module calling the export, in a fresh isolate
        const modules = await this.#modules(build);
        const main = JSON.stringify(`./${await build.digest()}/${path}`);
        const isolate = this.#loader.get(this.#random.bytes(SECRET_BYTES).toHex(), async () => ({
            compatibilityDate: WORKERD_COMPATIBILITY_DATE,
            compatibilityFlags: ["nodejs_compat"],
            mainModule: CALL_MODULE,
            modules: {
                ...Object.fromEntries(
                    Object.entries(modules).map(([module, js]) => [module, { js }]),
                ),
                [CALL_MODULE]: {
                    js: `import * as module from ${main};\nexport default {\n    async fetch(request) {\n        const { name, input } = await request.json();\n        return Response.json((await module[name](...input)) ?? null);\n    },\n};\n`,
                },
            },
            env: {},
            globalOutbound: this.#instances.get(
                this.#instances.idFromName(PartitionName.of({ space: this.#space })),
            ),
        }));

        // call the export with the call's input, refusing a failed call
        const response = await isolate.getEntrypoint().fetch(
            new Request("https://call", {
                method: "POST",
                body: JSON.stringify({ name, input }),
            }),
        );
        const text = await response.text();
        if (!response.ok) {
            throw MachineError.of(
                "RUNTIME_ACTION_UNREACHABLE",
                `could not call '${name}' of '${path}': ${response.status} '${text}'`,
            );
        }

        return JsonValue.of(JSON.parse(text));
    }

    /** Forward a request at its path below an instance's installation to its facet or partition. */
    async fetch(
        instanceId: Identifier<"instance">,
        path: string,
        request: Request,
        authentication: Authentication | undefined,
        partition?: string,
    ): Promise<Response> {
        // forward the request with its caller and secret to the partition's object
        const running = this.#registry.require(instanceId);
        const object =
            partition === undefined || partition === running.spec.scope
                ? running.object
                : await this.#partition(running, partition);

        return object.fetch(
            WorkloadRequest.forward(INSTANCE_ORIGIN, path, request, running.secret, authentication),
        );
    }

    /** Hand an instance's facet a request the machine makes at a path below its origin. */
    async deliver(
        instanceId: Identifier<"instance">,
        path: string,
        request: Request,
    ): Promise<Response> {
        // hand the request over at its path with the secret
        const { secret, object } = this.#registry.require(instanceId);

        return object.fetch(WorkloadRequest.forward(INSTANCE_ORIGIN, path, request, secret));
    }

    /** Start the object of a partition below an instance's root once. */
    #partition(
        running: Runner,
        partition: string,
    ): Promise<{ readonly fetch: (request: Request) => Promise<Response> }> {
        // join a start in progress or done
        const known = running.partitions.get(partition);
        if (known !== undefined) {
            return known;
        }

        // hand the partition's object the start with its partition
        const { spec, start } = running;
        const object = this.#instances.get(
            this.#instances.idFromName(
                PartitionName.of({
                    space: spec.scope,
                    installation: spec.installationId,
                    partition,
                }),
            ),
        );
        const body: DurableObjectInstanceStart = {
            ...start,
            workload: { ...start.workload, start: { ...start.workload.start, partition } },
        };
        const started = object
            .fetch(
                new Request(`${INSTANCE_ORIGIN}${START_PATH}`, {
                    method: "POST",
                    body: JSON.stringify(body),
                }),
            )
            .then(async (response) => {
                await WorkerdRuntime.#requireAnswer(
                    response,
                    `start partition ${partition} of ${spec.instanceId}`,
                );

                return object;
            });
        running.partitions.set(partition, started);

        // forget a failed start for the next call to retry
        started.catch(() => running.partitions.delete(partition));

        return started;
    }

    /** Hand an instance's facet its start. */
    async #start(spec: InstanceSpec, identify: (secret: string) => void): Promise<Runner> {
        // describe the start with a fresh secret
        const start = await InstanceSpec.start(spec, {
            egress: this.#egress,
            url: await this.#url(spec.scope, spec.installationId),
            sampling: await this.#sampling(spec.scope, spec.installationId),
            identity: await this.#identity(spec.scope),
            random: this.#random,
        });
        const object = this.#facet(spec.installationId);

        // load the output's modules and run the workload's entry
        const body: DurableObjectInstanceStart = {
            workload: { start, manifest: spec.build.manifest },
            space: this.#space,
            main: `${await spec.build.digest()}/${InstanceSpec.entry(spec).path}`,
        };
        const running: Runner = {
            spec,
            object,
            secret: start.secret,
            start: body,
            partitions: new Map(),
        };
        identify(start.secret);
        const response = await object.fetch(
            new Request(`${INSTANCE_ORIGIN}${START_PATH}`, {
                method: "POST",
                body: JSON.stringify(body),
            }),
        );
        await WorkerdRuntime.#requireAnswer(response, `start ${spec.instanceId}`);

        return running;
    }

    /** Name every module of a build and its dependencies on workerd, below their digests. */
    async #names(build: BuildReader, closure: ImportClosure): Promise<string[]> {
        const own = await build.digest();
        const names = await Promise.all(
            [...closure.builds].map(async (digest) => {
                // read the build's workerd output directories
                const reader = digest === own ? build : await this.#builds().contents(digest);
                const directories = Object.values(reader.manifest.outputs)
                    .filter((output) => output.runtime === WORKERD_RUNTIME)
                    .map((output) => `${output.directory}/`);
                const files = await reader.files();

                // name each JavaScript module below them by its build's digest and its path
                return files
                    .filter(
                        (file) =>
                            file.mediaType === MODULE_MEDIA_TYPE &&
                            directories.some((directory) => file.path.startsWith(directory)),
                    )
                    .map((file) => `${digest}/${file.path}`);
            }),
        );

        return names.flat();
    }

    /** Map a build's imports on workerd to its dependencies' modules, below their digests. */
    async #closure(build: BuildReader): Promise<ImportClosure> {
        return this.#builds().imports(await build.digest(), WORKERD_RUNTIME, MODULE_BASE);
    }

    /** Read an object's answer to the end, refusing a failed one. */
    static async #requireAnswer(response: Response, action: string): Promise<void> {
        const text = await response.text();
        if (!response.ok) {
            throw MachineError.of(
                "RUNTIME_ACTION_UNREACHABLE",
                `could not run '${action}': ${response.status} '${text}'`,
            );
        }
    }
}
