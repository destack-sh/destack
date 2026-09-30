import type { PackageId } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import type { ServerRuntime } from "@destack/package/runtime";
import type { ResourceRecord } from "@destack/resource";
import type { Identifier } from "@destack/schema";
import type { Caller } from "@destack/service/authentication";

/** The instances a host runs on one server runtime: it starts, stops and serves them as its holders assign them. */
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
        caller: Caller,
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
    /** The build output holding the workload. */
    readonly output: string;
    /** The package-local workload. */
    readonly workload: string;
    /** The resources the workload's declarations are bound to. */
    readonly resources: readonly WorkloadResource[];
}

/** A resource one of a workload's declarations is bound to. */
export interface WorkloadResource extends ResourceRecord {
    /** The package declaring the binding. */
    readonly packageId: PackageId;
    /** The declaration's name within its package. */
    readonly name: string;
    /** The provider holding the resource. */
    readonly providerCode: string;
}
