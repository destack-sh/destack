import { principal, sameSubject } from "@destack/access";
import type { PackageId } from "@destack/package";
import type { ServerRuntime } from "@destack/package/runtime";
import { type Identifier, Version } from "@destack/schema";
import { Egress } from "@destack/service";
import {
    Caller,
    CALLER_HEADER,
    CALLER_LIFETIME_MILLISECONDS,
    type Lending,
} from "@destack/service/authentication";
import { ServiceError } from "@destack/service/error";
import { copyRequest, VERSION_HEADER } from "@destack/service/request";
import type { InstanceSpec, Runtime } from "../runtime/index.ts";

/** A running instance serving an installation, with the releases its deployment serves. */
export interface Endpoint {
    /** The instance. */
    readonly instanceId: Identifier<"instance">;
    /** The space the installation serves. */
    readonly scope: Identifier<"space">;
    /** The deployment it runs. */
    readonly deploymentId: Identifier<"deployment">;
    /** The server runtime it runs on. */
    readonly runtime: ServerRuntime;
    /** The package release the deployment runs. */
    readonly release: Version;
    /** The oldest caller release it serves, every earlier one when absent. */
    readonly since?: Version;
}

/** Where an address a workload calls leads: an installation this host serves, a service it mounts for the caller's space, or another origin. */
export type Destination =
    | {
          /** An installation's service served by this host's instances. */
          readonly kind: "installation";
          /** The space of the installation, which the call targets. */
          readonly scope: Identifier<"space">;
          /** The installation. */
          readonly installationId: Identifier<"installation">;
          /** The installation's package, receiving the call. */
          readonly audience: PackageId;
      }
    | {
          /** A service this host mounts itself, such as its cell's audit service. */
          readonly kind: "service";
          /** The service's package, receiving the call. */
          readonly audience: PackageId;
          /** Serve the call as a verified caller. */
          fetch(request: Request, caller: Caller): Promise<Response>;
      }
    | {
          /** A service another origin serves, reached with a signed token. */
          readonly kind: "remote";
          /** The space the call targets. */
          readonly scope: Identifier<"space">;
          /** The service's URL. */
          readonly url: string;
          /** The service's package, receiving the call. */
          readonly audience: PackageId;
      };

/** The routes of the spaces a host serves: their installations' endpoints and where their workloads' addresses lead. */
export interface Routes {
    /** List the running instances serving an installation. */
    endpoints(installationId: Identifier<"installation">): Promise<readonly Endpoint[]>;
    /** Resolve an address an instance's workload calls. */
    resolve(spec: InstanceSpec, address: string): Promise<Destination>;
}

/** A host's ingress and egress: calls to its installations reach the deployment serving the caller's release, and its workloads' calls reach their addresses as their installations. */
export class Router {
    /** The runtimes running the host's instances, by server runtime. */
    readonly #runtimes: ReadonlyMap<string, Runtime>;
    /** The routes of the spaces the host serves. */
    readonly #routes: Routes;
    /** Sign an installation's token for a call leaving the host. */
    readonly #sign: (caller: Caller) => Promise<string>;
    /** The fetch reaching other origins. */
    readonly #fetch: (request: Request) => Promise<Response>;
    /** The lendings of callers' authority to the installations they call, which the called cells verify. */
    readonly #lending?: Pick<Lending, "sign">;

    /** Route through a host's runtimes and its spaces' routes. */
    constructor(options: {
        /** The runtimes running the host's instances. */
        readonly runtimes: readonly Runtime[];
        /** The routes of the spaces the host serves. */
        readonly routes: Routes;
        /** Sign an installation's token for a call leaving the host, as its space's authority. */
        readonly sign: (caller: Caller) => Promise<string>;
        /** The fetch reaching other origins. */
        readonly fetch: (request: Request) => Promise<Response>;
        /** Lend each caller's authority to the installation it calls, for the calls it sends, as the called cells verify. */
        readonly lending?: Pick<Lending, "sign">;
    }) {
        // index the runtimes, and keep the routes, the signer and the fetch
        this.#runtimes = new Map(options.runtimes.map((runtime) => [runtime.name, runtime]));
        this.#routes = options.routes;
        this.#sign = options.sign;
        this.#fetch = options.fetch;
        if (options.lending !== undefined) {
            this.#lending = options.lending;
        }
    }

    /** Serve a call below an installation's service on the newest deployment serving the caller's release. */
    async ingress(
        installationId: Identifier<"installation">,
        path: string,
        request: Request,
        caller: Caller,
    ): Promise<Response> {
        // require the release the caller speaks
        const header = request.headers.get(VERSION_HEADER);
        const version = header === null ? undefined : Version.safeParse(header).data;
        if (version === undefined) {
            const message =
                header === null
                    ? `requires ${VERSION_HEADER}`
                    : `invalid ${VERSION_HEADER}: ${header}`;
            throw new ServiceError("BAD_REQUEST", { message });
        }

        // pick the newest running deployment serving that release
        const endpoints = await this.#routes.endpoints(installationId);
        const [serving] = endpoints
            .filter(
                (endpoint) =>
                    Version.compare(version, endpoint.release) <= 0 &&
                    (endpoint.since === undefined || Version.compare(endpoint.since, version) <= 0),
            )
            .sort((left, right) => Version.compare(right.release, left.release));
        if (serving === undefined) {
            throw new ServiceError("SERVICE_UNAVAILABLE", {
                message: `no running deployment of ${installationId} serves release ${version}`,
            });
        }

        // lend the caller's authority to the installation for the calls it sends, and forward the call
        const delegated = await this.#delegate(caller, installationId, serving.scope);

        return this.#runtime(serving.runtime).fetch(serving.instanceId, path, request, delegated);
    }

    /** Lend a caller's authority to the installation it calls, as a lending the called cell verifies. */
    async #delegate(
        caller: Caller,
        installationId: Identifier<"installation">,
        spaceId: Identifier<"space">,
    ): Promise<Caller> {
        // lend nothing where the host lends no authority, nor an installation to itself
        const installation = principal.installation.reference(spaceId, installationId);
        const lending = this.#lending;
        if (lending === undefined || sameSubject(caller.authentication.subject, installation)) {
            return caller;
        }

        // sign the caller's authority lent to the installation in its space
        const delegation = await lending.sign(caller, installation, spaceId);

        return new Caller({ ...caller.authentication, delegation });
    }

    /** Forward a webhook request to the newest running deployment of an installation, whose workload verifies it. */
    async receive(
        installationId: Identifier<"installation">,
        path: string,
        request: Request,
    ): Promise<Response> {
        // pick the newest running deployment
        const endpoints = await this.#routes.endpoints(installationId);
        const [newest] = [...endpoints].sort((left, right) =>
            Version.compare(right.release, left.release),
        );
        if (newest === undefined) {
            throw new ServiceError("SERVICE_UNAVAILABLE", {
                message: `no running deployment of ${installationId} receives webhooks`,
            });
        }

        return this.#runtime(newest.runtime).receive(newest.instanceId, path, request);
    }

    /** Serve a workload's call to an address below the host's egress, as its installation. */
    async egress(request: Request): Promise<Response> {
        // require an address and the calling instance's secret
        const routed = Egress.route(request);
        if (routed === undefined) {
            throw new ServiceError("NOT_FOUND", { message: "no address below the egress" });
        }
        const secret = request.headers.get("authorization")?.replace(/^Bearer /, "");
        const spec = secret === undefined ? undefined : this.#identify(secret);
        if (spec === undefined) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid instance secret" });
        }

        // resolve the address, and call as the installation without the instance's secret or its own identity claims
        const destination = await this.#routes.resolve(spec, routed.address);
        const caller = Router.#caller(spec, destination);
        const headers = new Headers(routed.request.headers);
        headers.delete("authorization");
        headers.delete(CALLER_HEADER);
        headers.delete("cookie");
        const call = copyRequest(routed.request, { headers });
        const url = new URL(call.url);

        // serve an installation on this host
        if (destination.kind === "installation") {
            return this.ingress(destination.installationId, url.pathname, call, caller);
        }
        // serve a service this host mounts
        else if (destination.kind === "service") {
            return destination.fetch(call, caller);
        }
        // call another origin with the installation's signed token
        else {
            headers.set("authorization", `Bearer ${await this.#sign(caller)}`);
            const target = `${destination.url.replace(/\/$/, "")}${url.pathname}${url.search}`;

            return this.#fetch(copyRequest(call, { headers }, target));
        }
    }

    /** Find the spec of the instance a secret proves, across the runtimes. */
    #identify(secret: string): InstanceSpec | undefined {
        for (const runtime of this.#runtimes.values()) {
            const spec = runtime.identify(secret);
            if (spec !== undefined) {
                return spec;
            }
        }

        return undefined;
    }

    /** Find the runtime of a server runtime, refusing one the host does not spec. */
    #runtime(name: ServerRuntime): Runtime {
        const runtime = this.#runtimes.get(name);
        if (runtime === undefined) {
            throw new ServiceError("SERVICE_UNAVAILABLE", {
                message: `this host runs no ${name} runtime`,
            });
        }

        return runtime;
    }

    /** Build the caller an instance's installation calls a destination as, within its deployment. */
    static #caller(spec: InstanceSpec, destination: Destination): Caller {
        const subject = principal.installation.reference(spec.scope, spec.installationId);
        const now = Date.now();

        return new Caller({
            credential: { kind: "installation", id: spec.instanceId },
            audience: destination.audience,
            scope: destination.kind === "service" ? spec.scope : destination.scope,
            subject,
            subjects: [subject],
            deployments: [{ subject, id: spec.deploymentId }],
            verifiedAt: now,
            expiresAt: now + CALLER_LIFETIME_MILLISECONDS,
        });
    }
}
