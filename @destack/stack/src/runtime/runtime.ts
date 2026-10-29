import type { Random } from "@destack/environment";
import { type Authentication, CALLER_HEADER, Caller } from "@destack/access";
import type { DirectoryGrant, PackageId, Sandbox } from "@destack/package";
import { type BuildReader, PackageManifest } from "@destack/package/manifest";
import { BucketKind, S3_PROVIDER } from "@destack/bucket";
import { type ResourceId, type ResourceRecord } from "@destack/resource";
import type { Digest, Identifier, JsonValue } from "@destack/schema";
import {
    Egress,
    PackageMount,
    INSTALLATION_PROVIDER,
    InstallationKind,
    STORE_PATHS,
} from "@destack/object";
import type { Deriver } from "@destack/identity";
import { InstallationRoot, type WorkloadStart } from "@destack/object/workload";
import type {} from "@destack/package/import-meta";
import { StackError } from "../error/index.ts";
import type { DeploymentRuntime } from "../object/deployment.ts";

/** The bytes of an instance's or a call's secret: 256 bits, beyond guessing. */
export const SECRET_BYTES = 32;

/** What each runtime grants its workloads. */
export const RUNTIME_SUPPORT: Readonly<
    Record<
        DeploymentRuntime,
        { readonly hasFilesystem: boolean; readonly hasCertificates: boolean }
    >
> = {
    bun: { hasFilesystem: true, hasCertificates: true },
    workerd: { hasFilesystem: false, hasCertificates: false },
    process: { hasFilesystem: false, hasCertificates: true },
};

/** How an instance's workload ended unasked: its exit code, its signal or its runtime's failure. */
export type InstanceExit =
    | { readonly code: number }
    | { readonly signal: string }
    | { readonly failure: string };

/** Describe an instance's exit, as a condition states it. */
export const InstanceExit = {
    /** Describe an exit in one line. */
    describe(exit: InstanceExit): string {
        // describe a code, a signal, or a failure
        if ("code" in exit) {
            return `the workload exited with code ${exit.code}`;
        }
        // describe a signal
        else if ("signal" in exit) {
            return `the workload was killed by ${exit.signal}`;
        }
        // describe a failure
        else {
            return `the workload's runtime failed: ${exit.failure}`;
        }
    },
};

/** The instances a machine runs on one server runtime. */
export interface Runtime {
    /** The runtime, such as bun, or the machine's process. */
    readonly name: DeploymentRuntime;
    /** Start an instance's workload unless it serves, and resolve once it serves. */
    readonly start: (
        spec: InstanceSpec,
        exited: (exit: InstanceExit) => Promise<void>,
    ) => Promise<void>;
    /** Stop an instance, draining it first. */
    readonly stop: (instanceId: Identifier<"instance">) => Promise<void>;
    /** Report whether an instance serves now. */
    readonly isRunning: (instanceId: Identifier<"instance">) => boolean;
    /** Find the run of the instance a secret proves, absent for any other secret. */
    readonly identify: (secret: string) => InstanceSpec | undefined;
    /** Forward a verified caller's request below an instance's installation. */
    readonly fetch: (
        instanceId: Identifier<"instance">,
        path: string,
        request: Request,
        authentication: Authentication,
        partition?: string,
    ) => Promise<Response>;
    /** Call an export of a build's module once with JSON arguments, as it runs package code. */
    readonly call: (
        build: BuildReader,
        path: string,
        name: string,
        input: readonly JsonValue[],
    ) => Promise<JsonValue>;
    /** Hand an instance a webhook delivery or socket event from its machine. */
    readonly deliver: (
        instanceId: Identifier<"instance">,
        path: string,
        request: Request,
    ) => Promise<Response>;
}

/** What a runtime runs for one instance. */
export interface InstanceSpec {
    /** The instance. */
    readonly instanceId: Identifier<"instance">;
    /** The installation the workload runs for. */
    readonly installationId: Identifier<"installation">;
    /** The installation's name in its space, its address there. */
    readonly name: string;
    /** The space the installation serves. */
    readonly scope: Identifier<"space">;
    /** The deployment the instance runs. */
    readonly deploymentId: Identifier<"deployment">;
    /** The build the deployment runs. */
    readonly build: BuildReader;
    /** The digest of the build's manifest, absent for a release no machine resolved. */
    readonly manifest?: Digest;
    /** The build output with the workload. */
    readonly output: string;
    /** The package-local workload. */
    readonly workload: string;
    /** What the workload may use beyond the bare sandbox. */
    readonly sandbox: Sandbox;
    /** The package's capabilities the installation runs with. */
    readonly enabled: readonly string[];
    /** The directories its installation grants the `fs` capability. */
    readonly directories: readonly DirectoryGrant[];
    /** The resources the workload's declarations are bound to. */
    readonly resources: readonly WorkloadResource[];
    /** The served objects, such as secret versions, bound to the workload's declarations. */
    readonly served: readonly WorkloadServed[];
    /** The installations of other spaces the workload's installation declarations are bound to. */
    readonly installations: readonly WorkloadInstallation[];
    /** The installation's value of each setting its build declares, by `<package>/<name>`. */
    readonly settings: Readonly<Record<string, JsonValue>>;
}

/** An installation calling out through its machine's egress, from an instance or its store. */
export interface EgressCaller {
    /** The space of the installation. */
    readonly scope: Identifier<"space">;
    /** The installation calling. */
    readonly installationId: Identifier<"installation">;
    /** The resources its declarations are bound to. */
    readonly resources: readonly WorkloadResource[];
    /** The installations of other spaces its declarations are bound to. */
    readonly installations: readonly WorkloadInstallation[];
    /** Where its code runs: an instance of a deployment, or the space's store. */
    readonly runs:
        | {
              readonly kind: "instance";
              /** The instance. */
              readonly instanceId: Identifier<"instance">;
              /** The deployment the instance runs. */
              readonly deploymentId: Identifier<"deployment">;
          }
        | { readonly kind: "store" };
}

/** The installations calling out through a machine's egress. */
export const EgressCaller = {
    /** Read an instance's workload as the caller of its egress. */
    of(spec: InstanceSpec): EgressCaller {
        const { scope, installationId, resources, installations, instanceId, deploymentId } = spec;

        return {
            scope,
            installationId,
            resources,
            installations,
            runs: { kind: "instance", instanceId, deploymentId },
        };
    },

    /** Report whether a caller may call a peer's path: a store procedure, else once bound. */
    binds(
        caller: Pick<EgressCaller, "installations">,
        peer: { readonly name: string; readonly spaceId: string },
        path: string,
    ): boolean {
        const isBound = caller.installations.some(
            (bound) => bound.installation === peer.name && bound.scope === peer.spaceId,
        );

        return isBound || STORE_PATHS.has(path);
    },
};

/** The starts of instances' workloads. */
export const InstanceSpec = {
    /** Describe the start message of an instance's runner. */
    async start(
        spec: InstanceSpec,
        options: {
            /** The machine's egress, below which the instance calls addresses. */
            readonly egress: string;
            /** The URL callers call the installation at, with its packages mounted below. */
            readonly url: string;
            /** The share of traces the installation samples. */
            readonly sampling: number;
            /** The identity of the instance's space, which the installation's root derives from. */
            readonly identity: Pick<Deriver, "derive">;
            /** The randomness the instance's secret draws from. */
            readonly random: Random;
        },
    ): Promise<WorkloadStart> {
        const secret = options.random.bytes(SECRET_BYTES).toHex();

        return {
            instance: spec.instanceId,
            scope: spec.scope,
            installation: spec.installationId,
            name: spec.name,
            bindings: bindings(spec, options.egress, secret),
            settings: spec.settings,
            capabilities: [...spec.enabled],
            secret,
            egress: options.egress,
            url: options.url,
            sampling: options.sampling,
            root: (await InstallationRoot.derive(options.identity, spec.installationId)).toHex(),
        };
    },

    /** Find the output of an instance's workload and the path of its entry. */
    entry(spec: InstanceSpec): {
        readonly output: PackageManifest["outputs"][string];
        readonly path: string;
    } {
        // find the workload's entry among the output's exports
        const output = spec.build.manifest.outputs[spec.output];
        const entrypoint = output?.workloads[spec.workload]?.entrypoint;
        const path = entrypoint === undefined ? undefined : output?.exports[entrypoint]?.path;
        if (output === undefined || path === undefined) {
            throw StackError.of(
                "OUTPUT_ENTRY_MISSING",
                `output ${spec.output} has no entry for workload ${spec.workload}`,
            );
        }

        return { output, path };
    },
};

/** The requests a runtime forwards to an instance's workload at its origin. */
export const WorkloadRequest = {
    /** Forward a request at its path below the workload's origin. */
    forward(
        origin: string,
        path: string,
        request: Request,
        secret: string,
        authentication?: Authentication,
    ): Request {
        // replace the caller's credentials with the secret and the verified caller
        const headers = new Headers(request.headers);
        headers.set("authorization", `Bearer ${secret}`);
        headers.delete(CALLER_HEADER);
        headers.delete("cookie");
        if (authentication !== undefined) {
            Caller.forward(authentication, headers);
        }

        // pass the query on and leave redirects to the caller
        const url = `${origin}${path}${new URL(request.url).search}`;

        return new Request(url, new Request(request, { headers, redirect: "manual" }));
    },
};

/** The instances a runtime runs: the starts in progress, the running ones and their secrets. */
export class InstanceRegistry<
    Running extends { readonly spec: Pick<InstanceSpec, "instanceId">; readonly secret: string },
> {
    /** The starts in progress, by instance. */
    readonly #starts = new Map<string, Promise<void>>();
    /** The running instances, by instance. */
    readonly #running = new Map<string, Running>();
    /** The specs of the starting and running instances, by the secret their requests prove. */
    readonly #secrets = new Map<string, Running["spec"]>();

    /** Start an instance unless it runs, joining a start in progress. */
    start(
        spec: Running["spec"],
        launch: (identify: (secret: string) => void) => Promise<Running>,
    ): Promise<void> {
        // join a start in progress
        const pending = this.#starts.get(spec.instanceId);
        if (pending !== undefined) {
            return pending;
        }
        // leave a running instance as it is
        else if (this.#running.has(spec.instanceId)) {
            return Promise.resolve();
        }

        // launch once, caching the secret while it starts and dropping it when the start fails
        const secrets = new Set<string>();
        const identify = (secret: string) => {
            secrets.add(secret);
            this.#secrets.set(secret, spec);
        };
        const starting = launch(identify)
            .then(
                (running) => {
                    this.#running.set(spec.instanceId, running);
                },
                (error: unknown) => {
                    secrets.forEach((secret) => this.#secrets.delete(secret));
                    throw error;
                },
            )
            .finally(() => this.#starts.delete(spec.instanceId));
        this.#starts.set(spec.instanceId, starting);

        return starting;
    }

    /** Wait for an instance's start in progress to settle, whose own caller sees its failure. */
    async settle(instanceId: Identifier<"instance">): Promise<void> {
        await Promise.allSettled([this.#starts.get(instanceId)]);
    }

    /** Stop an instance after any start in progress, with its secret valid until it drains. */
    async stop(
        instanceId: Identifier<"instance">,
        drain: (running: Running) => Promise<void>,
    ): Promise<void> {
        // take the instance off the running ones after any start in progress
        await this.settle(instanceId);
        const running = this.#running.get(instanceId);
        if (running === undefined) {
            return;
        }
        this.#running.delete(instanceId);

        // drop its secret once it drains
        try {
            await drain(running);
        } finally {
            this.#secrets.delete(running.secret);
        }
    }

    /** Forget an exited instance so its secret proves nothing. */
    forget(running: Running): boolean {
        // leave an instance a later start replaced
        if (this.#running.get(running.spec.instanceId) !== running) {
            return false;
        }

        // drop it and its secret
        this.#running.delete(running.spec.instanceId);
        this.#secrets.delete(running.secret);

        return true;
    }

    /** Report whether an instance runs. */
    isRunning(instanceId: Identifier<"instance">): boolean {
        return this.#running.has(instanceId);
    }

    /** Find the spec of the starting or running instance whose requests a secret proves. */
    identify(secret: string): Running["spec"] | undefined {
        return this.#secrets.get(secret);
    }

    /** Read a running instance, refusing one that does not run. */
    require(instanceId: Identifier<"instance">): Running {
        const running = this.#running.get(instanceId);
        if (running === undefined) {
            throw StackError.of("INSTANCE_NOT_RUNNING", `instance ${instanceId} is not running`);
        }

        return running;
    }

    /** List the running instances. */
    list(): Running[] {
        return [...this.#running.values()];
    }
}

/** A resource one of a workload's declarations is bound to. */
export interface WorkloadResource extends ResourceRecord {
    /** The package declaring the binding. */
    readonly packageId: PackageId;
    /** The declaration's name within its package. */
    readonly name: string;
    /** The resource kind, such as database. */
    readonly kind: string;
    /** The provider of the resource. */
    readonly provider: string;
    /** The provider's reference of the provisioned resource. */
    readonly reference: string;
    /** The connection in the space lending its credential to the resource's calls. */
    readonly connection?: Identifier<"connection">;
}

/** An object a package serves, such as a secret, bound to one of a workload's declarations. */
export interface WorkloadServed {
    /** The package declaring the binding. */
    readonly packageId: PackageId;
    /** The declaration's name within its package. */
    readonly name: string;
    /** The declaration kind, such as secret. */
    readonly kind: string;
    /** The provider workloads read the object through, such as vault. */
    readonly provider: string;
    /** The bound object. */
    readonly target: ResourceId;
    /** The version the deployment captured. */
    readonly version: number;
    /** The address of the package serving the object. */
    readonly reference: string;
}

/** An installation of another space bound to one of a workload's installation declarations. */
export interface WorkloadInstallation {
    /** The package declaring the binding. */
    readonly packageId: PackageId;
    /** The declaration's name within its package. */
    readonly name: string;
    /** The package the declaration calls, which the installation mounts. */
    readonly package: PackageId;
    /** The bound installation's name in its space, its identity and address there. */
    readonly installation: string;
    /** The space of the installation. */
    readonly scope: Identifier<"space">;
}

/** Bind the running package's resources, secrets and other spaces' installations by their names. */
function bindings(spec: InstanceSpec, egress: string, secret: string): WorkloadStart["bindings"] {
    // bind each resource of the running package under its declared name
    const packageId = PackageManifest.package(spec.build.manifest).id;
    const bound: WorkloadStart["bindings"] = {};
    for (const resource of spec.resources) {
        // refuse a resource of another package
        if (resource.packageId !== packageId) {
            throw StackError.of(
                "FOREIGN_RESOURCE",
                `resource ${resource.id} does not bind a resource of ${packageId}`,
            );
        }

        // bind a bucket over S3 at the egress, storing the host's reference on the machine
        if (resource.kind === BucketKind.name) {
            bound[resource.name] = {
                resource: resource.id,
                kind: resource.kind,
                provider: S3_PROVIDER,
                reference: Egress.url(egress, resource.id),
                credential: secret,
                scope: spec.scope,
            };
        }
        // bind an address at the machine's egress with the secret and a URL as it is
        else {
            const isAddress = !URL.canParse(resource.reference);
            bound[resource.name] = {
                resource: resource.id,
                kind: resource.kind,
                provider: resource.provider,
                reference: isAddress ? Egress.url(egress, resource.reference) : resource.reference,
                ...(isAddress ? { credential: secret, scope: spec.scope } : {}),
            };
        }
    }

    // bind each served object's captured version to the package serving it at the egress
    for (const entry of spec.served) {
        bound[entry.name] = {
            resource: entry.target,
            kind: entry.kind,
            provider: entry.provider,
            reference: Egress.url(egress, entry.reference),
            credential: secret,
            scope: spec.scope,
            version: entry.version,
        };
    }

    // bind each installation of another space at its address at the egress, below the called package's mount
    for (const entry of spec.installations) {
        const address = Egress.address(entry.installation, entry.scope);
        bound[entry.name] = {
            resource: entry.installation,
            kind: InstallationKind.name,
            provider: INSTALLATION_PROVIDER,
            reference: `${Egress.url(egress, address)}${PackageMount.path(entry.package)}`,
            credential: secret,
            scope: entry.scope,
        };
    }

    return bound;
}
