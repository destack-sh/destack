import type { Capabilities, DirectoryGrant, PackageId } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import type { ServerRuntime } from "@destack/package/runtime";
import { type ResourceRecord, SECRET_KIND, SECRET_PROVIDER } from "@destack/resource";
import type { Digest, Identifier } from "@destack/schema";
import type { Authentication } from "@destack/service/authentication";
import { Egress } from "@destack/service";
import { ServiceError } from "@destack/service/error";
import { CallKey } from "@destack/service/request";
import type { WorkloadStart } from "@destack/service/workload";

/** The instances a host runs on one server runtime: it starts, stops and serves them as their spaces' cells assign them. */
export interface Runtime {
    /** The server runtime, such as bun. */
    readonly name: ServerRuntime;
    /** Start an instance's workload and resolve when it serves, leaving a serving one as it is, and report an exit nobody asked for. */
    start(spec: InstanceSpec, exited: (code: number) => Promise<void>): Promise<void>;
    /** Stop an instance, draining it first. */
    stop(instanceId: Identifier<"instance">): Promise<void>;
    /** Report whether an instance serves now. */
    isRunning(instanceId: Identifier<"instance">): boolean;
    /** Find the run of the instance a secret proves, absent for any other secret. */
    identify(secret: string): InstanceSpec | undefined;
    /** Forward a request below an instance's service to it, as a verified caller. */
    fetch(
        instanceId: Identifier<"instance">,
        path: string,
        request: Request,
        authentication: Authentication,
    ): Promise<Response>;
    /** Forward a webhook request below an instance's webhooks to it, verified by the workload itself. */
    receive(instanceId: Identifier<"instance">, path: string, request: Request): Promise<Response>;
}

/** What a runtime runs for one instance: its deployment's build, output and workload, and the resources it runs with. */
export interface InstanceSpec {
    /** The instance. */
    readonly instanceId: Identifier<"instance">;
    /** The installation the workload runs for. */
    readonly installationId: Identifier<"installation">;
    /** The space the installation serves. */
    readonly scope: Identifier<"space">;
    /** The deployment the instance runs. */
    readonly deploymentId: Identifier<"deployment">;
    /** The build the deployment runs. */
    readonly build: BuildReader;
    /** The digest of the build's manifest, absent for a release no host resolved. */
    readonly manifest?: Digest;
    /** The build output with the workload. */
    readonly output: string;
    /** The package-local workload. */
    readonly workload: string;
    /** What the workload may use beyond its sandbox: its required capabilities and the optional ones its installation allows. */
    readonly capabilities: Capabilities;
    /** The directories its installation grants the `fs` capability. */
    readonly directories: readonly DirectoryGrant[];
    /** The resources the workload's declarations are bound to. */
    readonly resources: readonly WorkloadResource[];
    /** The secret versions the workload's secret declarations are bound to. */
    readonly secrets: readonly WorkloadSecret[];
}

/** The starts of instances' workloads. */
export const InstanceSpec = {
    /** Describe the start message of an instance's runner. */
    async start(
        spec: InstanceSpec,
        options: {
            /** The host's egress, below which the instance calls addresses. */
            readonly egress: string;
            /** The secret the host and the runner prove each other's requests with. */
            readonly secret: string;
            /** The share of traces the installation keeps. */
            readonly sampling: number;
            /** Read the host's journal key, which the installation's key derives from. */
            readonly callKey: CallKey;
        },
    ): Promise<WorkloadStart> {
        return {
            instance: spec.instanceId,
            scope: spec.scope,
            installation: spec.installationId,
            ...(spec.manifest === undefined ? {} : { manifest: spec.manifest }),
            bindings: bindings(spec, options.egress, options.secret),
            secret: options.secret,
            egress: options.egress,
            sampling: options.sampling,
            callKey: (await CallKey.derive(await options.callKey(), spec.installationId)).toHex(),
        };
    },
};

/** The starts in progress on one runtime, which a concurrent start of the same instance joins. */
export class InstanceStarts {
    /** The starts in progress, by instance. */
    readonly #pending = new Map<string, Promise<void>>();

    /** Run an instance's start unless one is in progress, and resolve when the start in progress settles. */
    join(instanceId: Identifier<"instance">, start: () => Promise<void>): Promise<void> {
        // join a start in progress
        const pending = this.#pending.get(instanceId);
        if (pending !== undefined) {
            return pending;
        }

        // run the start once, forgetting it when it settles
        const starting = start().finally(() => this.#pending.delete(instanceId));
        this.#pending.set(instanceId, starting);

        return starting;
    }

    /** Wait for an instance's start in progress to settle, whose own caller sees its failure. */
    async settle(instanceId: Identifier<"instance">): Promise<void> {
        await Promise.allSettled([this.#pending.get(instanceId)]);
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
}

/** A secret version one of a workload's secret declarations is bound to. */
export interface WorkloadSecret {
    /** The package declaring the secret. */
    readonly packageId: PackageId;
    /** The declaration's name within its package. */
    readonly name: string;
    /** The bound secret. */
    readonly secret: Identifier<"secret">;
    /** The version the deployment captured. */
    readonly version: number;
    /** The address of the service serving the secret. */
    readonly reference: string;
}

/** Bind the running package's resources and secrets by their declared names. */
function bindings(spec: InstanceSpec, egress: string, secret: string): WorkloadStart["bindings"] {
    // bind each resource of the running package under its declared name
    const packageId = spec.build.manifest.package.id;
    const bound: WorkloadStart["bindings"] = {};
    for (const resource of spec.resources) {
        // refuse a resource of another package or one not provisioned
        if (resource.packageId !== packageId || resource.reference === null) {
            throw new ServiceError("PRECONDITION_FAILED", {
                message: `resource ${resource.id} binds no provisioned resource of ${packageId}`,
            });
        }

        // bind an address at the host's egress with the secret, and a URL as it is
        const isAddress = !URL.canParse(resource.reference);
        bound[resource.name] = {
            resource: resource.id,
            kind: resource.kind,
            provider: resource.provider,
            reference: isAddress ? Egress.url(egress, resource.reference) : resource.reference,
            ...(isAddress ? { credential: secret, scope: spec.scope } : {}),
        };
    }

    // bind each secret version to the service serving it at the egress
    for (const entry of spec.secrets) {
        bound[entry.name] = {
            resource: entry.secret,
            kind: SECRET_KIND,
            provider: SECRET_PROVIDER,
            reference: Egress.url(egress, entry.reference),
            credential: secret,
            scope: spec.scope,
            version: entry.version,
        };
    }

    return bound;
}
