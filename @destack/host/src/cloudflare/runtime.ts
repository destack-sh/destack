import type { Identifier } from "@destack/schema";
import { ServiceMount } from "@destack/service";
import { AUTHENTICATION_HEADER, type Authentication } from "@destack/service/authentication";
import { START_PATH, STOP_PATH } from "@destack/service/cloudflare";
import { ServiceError } from "@destack/service/error";
import type { CallKey } from "@destack/service/request";
import { WEBHOOK_PATH } from "@destack/service/workload";
import { InstanceSpec, InstanceStarts, type Runtime } from "../runtime/runtime.ts";
import type { DurableObjectInstanceStart, DurableObjectNamespace } from "./instance.ts";

/** The bytes of the secret a zone and an instance prove each other's requests with: 256 bits. */
const SECRET_BYTES = 32;

/** The media type of the JavaScript modules a workerd output distributes. */
const MODULE_MEDIA_TYPE = "text/javascript";

/** An instance this runtime started, with its object and secret. */
interface Running {
    /** The instance's spec. */
    readonly spec: InstanceSpec;
    /** The instance's object. */
    readonly object: { fetch(request: Request): Promise<Response> };
    /** The secret the zone and the instance prove each other's requests with. */
    readonly secret: string;
}

/** Run a zone's instances on workerd, each in the Durable Object of its installation in its space, which loads its workload's code. */
export class CloudflareRuntime implements Runtime {
    /** The server runtime. */
    readonly name = "workerd";
    /** The instances' objects, in the zone's jurisdiction. */
    readonly #instances: DurableObjectNamespace;
    /** The zone whose object serves the instances' egress and build files. */
    readonly #zone: string;
    /** The zone's egress, below which the instances call addresses. */
    readonly #egress: string;
    /** Resolve the share of traces an installation keeps. */
    readonly #sampling: (
        scope: Identifier<"space">,
        installation: Identifier<"installation">,
    ) => Promise<number>;
    /** Read the zone's journal key. */
    readonly #callKey: CallKey;
    /** The running instances, by instance. */
    readonly #running = new Map<string, Running>();
    /** The starts in progress, which concurrent starts of one instance join. */
    readonly #starts = new InstanceStarts();

    /** Run a zone's instances in their objects, calling addresses through the zone's egress. */
    constructor(options: {
        /** The instances' objects, in the zone's jurisdiction. */
        readonly instances: DurableObjectNamespace;
        /** The zone whose object serves the instances' egress and build files. */
        readonly zone: string;
        /** The zone's egress, below which the instances call addresses. */
        readonly egress: string;
        /** Resolve the share of traces an installation keeps, as its space's settings place it. */
        readonly sampling: (
            scope: Identifier<"space">,
            installation: Identifier<"installation">,
        ) => Promise<number>;
        /** Read the zone's journal key, which each installation's key derives from. */
        readonly callKey: CallKey;
    }) {
        // keep the objects, the zone, its egress, the sampling and the journal key
        this.#instances = options.instances;
        this.#zone = options.zone;
        this.#egress = options.egress;
        this.#sampling = options.sampling;
        this.#callKey = options.callKey;
    }

    /** Name the object of an installation in a space. */
    static object(spec: Pick<InstanceSpec, "scope" | "installationId">): string {
        return `${spec.scope}.${spec.installationId}`;
    }

    /** Start an instance in its installation's object and resolve once it serves, replacing the deployment the object ran before. */
    async start(spec: InstanceSpec): Promise<void> {
        return this.#starts.join(spec.instanceId, async () => {
            if (!this.#running.has(spec.instanceId)) {
                await this.#start(spec);
            }
        });
    }

    /** Report whether an instance this runtime started serves now. */
    isRunning(instanceId: Identifier<"instance">): boolean {
        return this.#running.has(instanceId);
    }

    /** Find the spec of the instance whose requests a secret proves. */
    identify(secret: string): InstanceSpec | undefined {
        return [...this.#running.values()].find((running) => running.secret === secret)?.spec;
    }

    /** Stop an instance after any start in progress, draining its workload in its object. */
    async stop(instanceId: Identifier<"instance">): Promise<void> {
        // wait for a start in progress, whose caller sees its failure
        await this.#starts.settle(instanceId);

        // forget the instance and drain it
        const running = this.#running.get(instanceId);
        if (running === undefined) {
            return;
        }
        this.#running.delete(instanceId);
        await requireAnswer(
            await running.object.fetch(
                new Request(`https://instance${STOP_PATH}`, { method: "POST" }),
            ),
            `stop ${instanceId}`,
        );
    }

    /** Read a file of the build of the instance a secret proves, as its object loads its code and its workload reads its build. */
    async build(secret: string, path: string): Promise<Uint8Array<ArrayBuffer>> {
        const spec = this.identify(secret);
        if (spec === undefined) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid instance secret" });
        }

        return spec.build.load(path);
    }

    /** Forward a request below an instance's service to its object, as a verified caller. */
    async fetch(
        instanceId: Identifier<"instance">,
        path: string,
        request: Request,
        authentication: Authentication,
    ): Promise<Response> {
        // forward the request below the package's mount with the verified caller and the secret
        const running = this.#require(instanceId);
        const packageId = running.spec.build.manifest.package.id;
        const url = new URL(request.url);
        const target = `https://instance${ServiceMount.path(packageId)}${path}${url.search}`;
        const headers = new Headers(request.headers);
        headers.set("authorization", `Bearer ${running.secret}`);
        authentication.forward(headers);
        headers.delete("cookie");

        return running.object.fetch(
            new Request(target, new Request(request, { headers, redirect: "manual" })),
        );
    }

    /** Forward a webhook request below an instance's webhooks to its object, which verifies it. */
    async receive(
        instanceId: Identifier<"instance">,
        path: string,
        request: Request,
    ): Promise<Response> {
        // forward the request below the webhook path with the secret
        const running = this.#require(instanceId);
        const url = new URL(request.url);
        const target = `https://instance${WEBHOOK_PATH}${path}${url.search}`;
        const headers = new Headers(request.headers);
        headers.set("authorization", `Bearer ${running.secret}`);
        headers.delete(AUTHENTICATION_HEADER);
        headers.delete("cookie");

        return running.object.fetch(
            new Request(target, new Request(request, { headers, redirect: "manual" })),
        );
    }

    /** Hand an instance's object its start, its output's modules and the zone it calls out through. */
    async #start(spec: InstanceSpec): Promise<void> {
        // describe the start with a fresh secret, keeping the instance identifiable while its code loads
        const start = await InstanceSpec.start(spec, {
            egress: this.#egress,
            secret: crypto.getRandomValues(new Uint8Array(SECRET_BYTES)).toHex(),
            sampling: await this.#sampling(spec.scope, spec.installationId),
            callKey: this.#callKey,
        });
        const object = this.#instances.get(
            this.#instances.idFromName(CloudflareRuntime.object(spec)),
        );
        const running = { spec, object, secret: start.secret };
        this.#running.set(spec.instanceId, running);

        // load the output's modules and run the workload's entry, forgetting a failed start
        try {
            const body: DurableObjectInstanceStart = {
                workload: { start, manifest: spec.build.manifest },
                zone: this.#zone,
                ...(await CloudflareRuntime.#code(spec)),
            };
            const response = await object.fetch(
                new Request(`https://instance${START_PATH}`, {
                    method: "POST",
                    body: JSON.stringify(body),
                }),
            );
            await requireAnswer(response, `start ${spec.instanceId}`);
        } catch (error) {
            this.#running.delete(spec.instanceId);
            throw error;
        }
    }

    /** Find the output's directory, its modules and the workload's entry among them, refusing an output without its entry. */
    static async #code(
        spec: InstanceSpec,
    ): Promise<Pick<DurableObjectInstanceStart, "directory" | "main" | "modules">> {
        // find the workload's entry among the output's exports
        const output = spec.build.manifest.outputs[spec.output];
        const entrypoint = output?.workloads[spec.workload]?.entrypoint;
        const main = entrypoint === undefined ? undefined : output?.exports[entrypoint];
        if (output === undefined || main === undefined) {
            throw new ServiceError("PRECONDITION_FAILED", {
                message: `output ${spec.output} carries no entry of workload ${spec.workload}`,
            });
        }

        // name the output's modules relative to its directory
        const prefix = `${output.directory}/`;
        const modules = (await spec.build.files())
            .filter((file) => file.path.startsWith(prefix) && file.mediaType === MODULE_MEDIA_TYPE)
            .map((file) => file.path.slice(prefix.length));

        return { directory: output.directory, main: main.slice(prefix.length), modules };
    }

    /** Read a running instance, refusing one that does not run. */
    #require(instanceId: Identifier<"instance">): Running {
        const running = this.#running.get(instanceId);
        if (running === undefined) {
            throw new ServiceError("SERVICE_UNAVAILABLE", {
                message: `instance ${instanceId} is not running`,
            });
        }

        return running;
    }
}

/** Read an object's answer to the end, refusing a failed one. */
async function requireAnswer(response: Response, action: string): Promise<void> {
    const text = await response.text();
    if (!response.ok) {
        throw new ServiceError("SERVICE_UNAVAILABLE", {
            message: `${action} failed with ${response.status}: ${text}`,
        });
    }
}
