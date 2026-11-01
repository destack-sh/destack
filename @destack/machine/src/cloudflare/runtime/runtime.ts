import type { Random } from "@destack/environment";
import { Authentication } from "@destack/access";
import { type Identifier, JsonValue, present } from "@destack/schema";
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
    CALL_PATH,
    type DurableObjectCall,
    type DurableObjectInstanceStart,
    type DurableObjectNamespace,
    PartitionName,
} from "./instance.ts";
import type { BuildReader } from "@destack/package/manifest";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../../error/index.ts";

/** The origin of the requests an instance's object serves, which its stub alone routes. */
const INSTANCE_ORIGIN = "https://instance";

/** The runtime the Cloudflare machine's instances run on. */
const WORKERD_RUNTIME = "workerd";

/** The media type of the JavaScript modules a workerd output distributes. */
const MODULE_MEDIA_TYPE = "text/javascript";

/** An instance this runtime started, with its object and secret. */
interface Runner {
    /** The instance's spec. */
    readonly spec: InstanceSpec;
    /** The instance's object. */
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

/** Run a space's instances on workerd. */
export class CloudflareRuntime implements Runtime {
    /** The server runtime. */
    readonly name = WORKERD_RUNTIME;
    /** The instances' objects, in the space's jurisdiction. */
    readonly #instances: DurableObjectNamespace;
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
    /** The instances starting and running. */
    readonly #registry = new InstanceRegistry<Runner>();
    /** The builds of the calls in flight, by the secret each call's isolate reads them with. */
    readonly #calls = new Map<string, BuildReader>();

    /** Run a space's instances in their objects, calling addresses through the space's egress. */
    constructor(options: {
        /** The instances' objects, in the space's jurisdiction. */
        readonly instances: DurableObjectNamespace;
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
    }) {
        // set the objects, the space, its egress, the sampling and the spaces' identities
        this.#instances = options.instances;
        this.#space = options.space;
        this.#egress = options.egress;
        this.#sampling = options.sampling;
        this.#url = options.url;
        this.#identity = options.identity;
        this.#random = options.random;
    }

    /** Start an instance in its installation's object, replacing its previous deployment. */
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
        await this.#registry.stop(instanceId, (running) => CloudflareRuntime.#drain(running));
    }

    /** Drain a running instance's root object and each started partition's. */
    static async #drain(running: Runner): Promise<void> {
        // read the root's object and each partition that started, skipping a failed start
        const started = await Promise.allSettled(running.partitions.values());
        const objects = [
            running.object,
            ...started.flatMap((result) => (result.status === "fulfilled" ? [result.value] : [])),
        ];

        // stop the root, then each started partition
        for (const object of objects) {
            await CloudflareRuntime.#requireAnswer(
                await object.fetch(
                    new Request(`${INSTANCE_ORIGIN}${STOP_PATH}`, { method: "POST" }),
                ),
                `stop ${running.spec.instanceId}`,
            );
        }
    }

    /** Read a file of the build of the instance or the call a secret proves. */
    async build(secret: string, path: string): Promise<Uint8Array<ArrayBuffer>> {
        const build = this.identify(secret)?.build ?? this.#calls.get(secret);
        if (build === undefined) {
            throw MachineError.of("INSTANCE_SECRET_INVALID", "invalid instance secret");
        }

        return build.load(path);
    }

    /** Call an export of a build's module once, in an isolate of a fresh object of its own. */
    async call(
        build: BuildReader,
        path: string,
        name: string,
        input: readonly JsonValue[],
    ): Promise<JsonValue> {
        // find the workerd output with the module, and list its modules
        const output = present(
            Object.values(build.manifest.outputs).find(
                (each) => each.runtime === WORKERD_RUNTIME && path.startsWith(`${each.directory}/`),
            ),
            `the workerd output of ${path}`,
        );
        const prefix = `${output.directory}/`;
        const modules = await CloudflareRuntime.#modules(build, prefix);

        // hand the call to an object of its own with a secret its isolate reads the build with
        const secret = this.#random.bytes(SECRET_BYTES).toHex();
        this.#calls.set(secret, build);
        try {
            const object = this.#instances.get(
                this.#instances.idFromName(
                    PartitionName.of({ space: this.#space, installation: `call-${secret}` }),
                ),
            );
            const call: DurableObjectCall = {
                space: this.#space,
                secret,
                directory: output.directory,
                main: path.slice(prefix.length),
                modules,
                name,
                input: [...input],
            };
            const response = await object.fetch(
                new Request(`${INSTANCE_ORIGIN}${CALL_PATH}`, {
                    method: "POST",
                    body: JSON.stringify(call),
                }),
            );
            const text = await response.text();
            if (!response.ok) {
                throw MachineError.of(
                    "RUNTIME_ACTION_FAILED",
                    `call ${name} of ${path} failed with ${response.status}: ${text}`,
                );
            }

            return JsonValue.of(JSON.parse(text));
        } finally {
            this.#calls.delete(secret);
        }
    }

    /** Forward a request at its path below an instance's installation to its object. */
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

    /** Hand an instance's root object a request the machine makes at a path below its origin. */
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
                await CloudflareRuntime.#requireAnswer(
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

    /** Hand an instance's object its start. */
    async #start(spec: InstanceSpec, identify: (secret: string) => void): Promise<Runner> {
        // describe the start with a fresh secret
        const start = await InstanceSpec.start(spec, {
            egress: this.#egress,
            url: await this.#url(spec.scope, spec.installationId),
            sampling: await this.#sampling(spec.scope, spec.installationId),
            identity: await this.#identity(spec.scope),
            random: this.#random,
        });
        const object = this.#instances.get(
            this.#instances.idFromName(
                PartitionName.of({ space: spec.scope, installation: spec.installationId }),
            ),
        );

        // load the output's modules and run the workload's entry
        const body: DurableObjectInstanceStart = {
            workload: { start, manifest: spec.build.manifest },
            space: this.#space,
            ...(await CloudflareRuntime.#code(spec)),
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
        await CloudflareRuntime.#requireAnswer(response, `start ${spec.instanceId}`);

        return running;
    }

    /** Find the output's directory, its modules and the workload's entry among them. */
    static async #code(
        spec: InstanceSpec,
    ): Promise<Pick<DurableObjectInstanceStart, "directory" | "main" | "modules">> {
        // find the workload's entry among the output's exports
        const { output, path: main } = InstanceSpec.entry(spec);

        // list the output's modules relative to its directory
        const prefix = `${output.directory}/`;
        const modules = await CloudflareRuntime.#modules(spec.build, prefix);

        return { directory: output.directory, main: main.slice(prefix.length), modules };
    }

    /** List a build's JavaScript modules below an output's directory, relative to it. */
    static async #modules(build: BuildReader, prefix: string): Promise<string[]> {
        const files = await build.files();

        return files
            .filter((file) => file.path.startsWith(prefix) && file.mediaType === MODULE_MEDIA_TYPE)
            .map((file) => file.path.slice(prefix.length));
    }

    /** Read an object's answer to the end, refusing a failed one. */
    static async #requireAnswer(response: Response, action: string): Promise<void> {
        const text = await response.text();
        if (!response.ok) {
            throw MachineError.of(
                "RUNTIME_ACTION_FAILED",
                `${action} failed with ${response.status}: ${text}`,
            );
        }
    }
}
