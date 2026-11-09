import * as accountObject from "@destack/account/object";
import type { WorkloadInstance } from "@destack/object/workload";
import { Identity } from "@destack/account/object";
import { OtlpExport } from "@destack/telemetry/otlp";
import { DeclarationName, PackageId, PackageName } from "@destack/package";
import { canonicalize, type Digest, type Identifier, schema, Version } from "@destack/schema";
import {
    CallError,
    Egress,
    HandlerAnswer,
    type HandlerRequest,
    installation,
    InstallationBindingSpec,
    InstallationKind,
    InstallationMount,
    Observation,
    type PackageHandle,
    PackageMount,
    refusal,
    RequestBody,
    SpaceMount,
    VERSION_HEADER,
} from "@destack/object";
import {
    PeerCall,
    type Registry,
    type SpaceBuildStore,
    type SpaceHost,
    type SpaceMachine,
} from "@destack/stack/server";
import type { IdentityKeystore } from "@destack/account/identity";
import type { Machine } from "../machine/index.ts";
import { Bearer, Exchange, TOKEN_LIFETIME_MILLISECONDS, TokenCache } from "@destack/access/token";
import {
    HOOK_PATH,
    PARTITION_HEADER,
    ROUTE_HEADER,
    SOCKET_HEADER,
    SOCKET_INSTANCE_HEADER,
    SOCKET_PATH,
} from "@destack/object/workload";
import {
    CodeIndex,
    type DeploymentRuntime,
    type Endpoint,
    Instance,
    type InstallationRollout,
    Rollout,
} from "@destack/stack/object";
import { Bucket, Snapshot } from "@destack/db";
import { Attribution } from "@destack/package/stack";
import { OTLP_ORIGIN_PATH, OtlpSignal } from "@destack/telemetry/otlp";
import { Authentication, Caller, CALLER_HEADER, principal } from "@destack/access";
import { copyRequest, Forwarded } from "@destack/environment";
import { type Space, type SpaceRun } from "../space/space.ts";
import { BucketKind } from "@destack/bucket";
import { S3_PACKAGE } from "@destack/bucket/s3";
import { ConnectionKind, ConnectionSpec } from "@destack/connection/declare";
import type { ConnectionLease } from "@destack/connection/object";
import { ConnectionRequest } from "@destack/connection/client";
import stackPackage from "@destack/stack/package";
import {
    EgressCaller,
    type InstanceSpec,
    type Runtime,
    type WorkloadResource,
} from "@destack/stack/runtime";
import { Subject } from "@destack/sync";
import { ViewOrigin } from "./origin.ts";
import { MachineError } from "../error/index.ts";
import { Failure } from "@destack/error";
import { HOOK_WORKLOAD } from "@destack/stack/workload";
import { DOMAINS, InstallationOrigin, S3Origin, SpaceAddress } from "@destack/account/address";
import { RouteRequest } from "./route.ts";
import type {} from "@destack/package/import-meta";

/** The milliseconds a caller waits for a deploying installation: an instance starts in seconds. */
const DEPLOYMENT_RETRY_MILLISECONDS = 1000;

/** The path below which a person's own tools call through the space's connections. */
const CONNECTION_PATH = "/.destack/connection";

/** The limit of the calls of spaces past a throttling entitlement, keyed by space. */
const EXCEEDED_LIMIT = "gateway:exceeded";

/** The calls a space past its throttling entitlement takes: one each second. */
const EXCEEDED_RATE = { count: 1, every: 1000 } as const;

/** The calls such a space takes at once after a quiet spell, a page load's worth. */
const EXCEEDED_BURST = 20;

/** The largest call body the egress buffers to resend to a moved scope's new machine. */
const MAX_RESENT_BODY_BYTES = 1024 * 1024;

/** The space a gateway serves, with the machine serving it and its host. */
export type GatewaySpace = Pick<
    Space,
    "id" | "database" | "traffic" | "call" | "observe" | "borrow" | "adapters"
> & {
    /** The space's workload, which tells the protocols its packages implement. */
    readonly instance: Pick<WorkloadInstance, "implements">;
    /** The space's builds, read through to the machine's cache. */
    readonly builds: Pick<
        SpaceBuildStore,
        "pull" | "contents" | "referrer" | "file" | "imports" | "get"
    >;
    /** The space's identity and its link to the universe, which the gateway signs with. */
    readonly host: Pick<SpaceHost, "identityOf" | "contain" | "database"> & {
        /** The keystore signing as the space and deriving its secrets. */
        readonly keystore: Pick<IdentityKeystore, "issuer" | "deriver">;
        /** The machine's directory replica, the universe's endpoints and the machine's world. */
        readonly machine: Pick<SpaceMachine, "directory" | "universe" | "environment">;
    };
    /** The machine serving the space: its clock, runtimes, network and registry. */
    readonly machine: Pick<
        Machine,
        "clock" | "environment" | "runtimes" | "hibernation" | "send" | "authenticate" | "domain"
    > & {
        readonly releases: Pick<Registry, "locate">;
    };
};

/** Where an address a workload calls leads. */
export type Destination =
    | {
          /** An installation's package served by this machine's instances. */
          readonly kind: "installation";
          /** The space of the installation, which the call targets. */
          readonly scope: Identifier<"space">;
          /** The installation. */
          readonly installationId: Identifier<"installation">;
          /** The installation's package, receiving the call. */
          readonly audience: PackageId;
          /** The path below the installation, the mount of the package called first. */
          readonly path: string;
      }
    | {
          /** A package this machine mounts itself, such as vault. */
          readonly kind: "package";
          /** The package receiving the call. */
          readonly audience: PackageId;
          /** Serve the call as a verified caller. */
          readonly fetch: (request: Request, authentication: Authentication) => Promise<Response>;
      }
    | {
          /** An external API a connection lends its credential to. */
          readonly kind: "external";
          /** The API's base URL. */
          readonly url: string;
          /** Apply the connection's current credential to the call. */
          readonly lend: (request: Request) => Promise<ConnectionRequest>;
      }
    | {
          /** A package the universe serves, called with a signed token bound to no space. */
          readonly kind: "universe";
          /** The package's URL. */
          readonly url: string;
          /** The package receiving the call. */
          readonly audience: PackageId;
      }
    | {
          /** A package another origin serves, called with a signed token. */
          readonly kind: "remote";
          /** The space the call targets. */
          readonly scope: Identifier<"space">;
          /** The package's URL. */
          readonly url: string;
          /** The package receiving the call. */
          readonly audience: PackageId;
      };

/** Where an exchanged token is bound: a package, in a space or outside any. */
export interface ExchangeTarget {
    /** The receiving package. */
    readonly audience: PackageId;
    /** The space the token is bound to, absent for a package of the universe. */
    readonly scope?: Identifier<"space">;
}

/** Whom an exchanged token acts for: the person under their grants, from their client if given. */
export type ExchangeSubject = Pick<Caller, "subject" | "grants"> & {
    /** The person's client the token lives within, ending with its revocation. */
    readonly client?: Identifier<"client">;
};

/** How a machine acts for the subjects of its installations' calls under their grants. */
export interface TokenExchange {
    /** Exchange an installation's token for its subject's under their grant (RFC 8693). */
    readonly exchange: (
        installation: Subject,
        asked: ExchangeSubject,
        target: ExchangeTarget,
    ) => Promise<string>;
    /** Verify a token the universe exchanged, as its caller. */
    readonly verify: (token: string, target: ExchangeTarget) => Promise<Authentication>;
}

/** A space's gateway: every call into and out of its installations, their origins and telemetry. */
export class Gateway {
    /** The origins of the space's installations, serving their views' pages. */
    readonly #views: ViewOrigin;
    /** The space the gateway serves. */
    readonly #space: GatewaySpace;
    /** What the workload's run serves with: the S3 API over the space's buckets. */
    readonly #serving: Pick<SpaceRun, "s3">;
    /** What the space's store serves: the packages its workloads call by package name. */
    readonly #served: { readonly packages: readonly { readonly package: PackageHandle }[] };
    /** The runtimes running the space's instances, by server runtime. */
    readonly #runtimes: ReadonlyMap<string, Runtime>;
    /** How the space acts for its installations' subjects under their grants. */
    readonly #tokens: TokenExchange;
    /** The attribution of each installation's served build, read at its first exception. */
    readonly #attributions = new Map<
        Identifier<"installation">,
        { readonly manifest: Digest; readonly attribution: Promise<Attribution> }
    >();

    /** Serve a space's installations through its machine's runtimes. */
    constructor(
        space: GatewaySpace,
        serving: Pick<SpaceRun, "s3">,
        store: { readonly packages: readonly { readonly package: PackageHandle }[] },
        tokens: TokenExchange = Gateway.#exchange(space),
    ) {
        // set the space, what its run serves with, its store and the runtimes by name
        this.#space = space;
        this.#serving = serving;
        this.#served = store;
        this.#runtimes = new Map(space.machine.runtimes.map((runtime) => [runtime.name, runtime]));
        this.#tokens = tokens;

        // serve the installations' origins through the gateway
        this.#views = new ViewOrigin(space, this);
    }

    /** List the packages the space's store serves now. */
    #packages(): readonly PackageHandle[] {
        return this.#served.packages.map((served) => served.package);
    }

    /** Exchange tokens for the installations' subjects at the universe, reusing live ones. */
    static #exchange(space: GatewaySpace): TokenExchange {
        // reuse the tokens exchanged for installations' subjects while most of their life remains
        const exchanged = new TokenCache();
        const { directory, universe } = space.host.machine;
        const { clock, environment } = space.machine;

        return {
            exchange: (acting, asked, target) =>
                exchanged.token(canonicalize({ acting, asked, target }), clock.now(), () =>
                    space.host.identityOf(acting).exchange(target.audience, {
                        subject: asked.subject,
                        ...(target.scope === undefined ? {} : { partition: target.scope }),
                        ...(asked.grants.length === 0
                            ? {}
                            : { grants: asked.grants.map((grant) => grant.id) }),
                        ...(asked.client === undefined ? {} : { credential: asked.client }),
                    }),
                ),
            verify: (token, target) =>
                Identity.verify(directory.snapshot, token, {
                    audience: target.audience,
                    universe: universe.issuer,
                    ...(target.scope === undefined ? {} : { partition: target.scope }),
                    now: clock.now(),
                    environment,
                }),
        };
    }

    /** Serve a request to the space: its routes, origins, package mounts and installations. */
    async serve(request: Request, authentication?: Authentication): Promise<Response> {
        try {
            // mount an origin's request below its installation, else a route's below its adapter
            const origin =
                InstallationMount.of(request) === undefined
                    ? (InstallationOrigin.of(request, this.#space.machine.domain) ??
                      InstallationOrigin.of(request, DOMAINS.space))
                    : undefined;
            const routed =
                origin === undefined
                    ? await this.#route(request)
                    : await this.#origin(origin, request);
            if (routed instanceof Response) {
                return routed;
            }

            // serve a package of the space below its mount
            const mounted = routed === undefined ? PackageMount.of(request) : undefined;
            if (mounted !== undefined) {
                return await this.#space.call(
                    PackageId.parse(mounted.packageId),
                    mounted.request,
                    authentication,
                );
            }

            // hand anything else to the installations, else to the stack's own routes
            const forwarded = routed ?? request;

            return (
                (await this.#installations(forwarded, authentication)) ??
                (await this.#space.call(stackPackage.id, forwarded, authentication))
            );
        } catch (error) {
            // answer a request its caller aborted as closed by the client, and refuse any other failure
            const failure = request.signal.aborted
                ? CallError.http("CLIENT_CLOSED_REQUEST", {
                      message: "the caller closed the request",
                      cause: error,
                  })
                : error;

            return refusal(failure, this.#space.machine.environment.telemetry);
        }
    }

    /** Serve a routed request: its redirect or views, else its adapter's request. */
    async #route(request: Request): Promise<Request | Response | undefined> {
        // answer a redirect
        const routed = await RouteRequest.mount(
            this.#space.database,
            this.#space.id,
            request,
            (installationId) => this.#space.adapters(installationId),
        );
        if (routed?.kind === "redirect") {
            return routed.response;
        }
        // serve the views below the route's path, without the machine's headers
        else if (routed?.kind === "view") {
            const found = await Rollout.requireServed(this.#space.database, {
                spaceId: this.#space.id,
                installation: routed.installation,
            });

            return this.#views.serve(RouteRequest.strip(request), found, routed.mount);
        }

        return routed?.request;
    }

    /** Address an installation origin's packages below its mount, else serve the origin. */
    async #origin(origin: InstallationOrigin, request: Request): Promise<Request | Response> {
        // refuse the digest of a name and branch, which names no installation here
        if (origin.digest !== undefined) {
            throw MachineError.of(
                "INSTALLATION_NOT_FOUND",
                `${SpaceAddress.of(origin)} has no installation with digest ${origin.digest}`,
            );
        }

        // read the installation of the origin's name the space serves
        const found = await Rollout.requireServed(this.#space.database, {
            spaceId: this.#space.id,
            installation: origin.name,
        });
        const mount = InstallationMount.url(request.url, this.#space.id, found.installationId);
        const url = new URL(request.url);

        // address its packages below its mount, such as an adapter a service calls
        if (PackageMount.parse(url.pathname) !== undefined) {
            return copyRequest(request, {}, `${mount}${url.pathname}${url.search}`);
        }
        // serve any other path at the origin: its platform paths, build files and views
        else {
            return this.#views.serve(request, found);
        }
    }

    /** Serve a request of the space's installations: calls, webhooks, sockets, telemetry, S3. */
    async #installations(
        request: Request,
        authentication: Authentication | undefined,
    ): Promise<Response | undefined> {
        // take the workloads' calls out, sockets and telemetry
        const url = new URL(request.url);
        const installed = InstallationMount.of(request);
        const isS3 =
            S3Origin.of(request, this.#space.machine.domain) !== undefined ||
            S3Origin.of(request, DOMAINS.space) !== undefined;
        // send an instance's call out
        if (url.pathname.startsWith(`${Egress.path}/`)) {
            return await this.egress(request);
        }
        // answer an instance about its space's sockets
        else if (url.pathname === SOCKET_PATH || url.pathname.startsWith(`${SOCKET_PATH}/`)) {
            return await this.#sockets(request);
        }
        // take telemetry: an instance's by its secret, a page's or a machine's by its verified caller
        else if (url.pathname.startsWith(`${OTLP_ORIGIN_PATH}/`)) {
            const verified = authentication ?? (await this.#signed(request));

            return await this.#telemetry(request, verified?.caller.subject);
        }
        // send a person's own tool's call through one of their connections, its credential applied
        else if (url.pathname.startsWith(`${CONNECTION_PATH}/`)) {
            return await this.#connect(url, request, authentication);
        }
        // serve the buckets over S3 at the host their client signed, which a front door forwards
        else if (isS3) {
            return await this.#serving.s3.fetch(Forwarded.restore(request));
        }
        // serve a call or a webhook to an installation
        else if (installed !== undefined) {
            return await this.#installation(installed, authentication);
        }
        // answer nothing for any other request
        else {
            return undefined;
        }
    }

    /** Send a person's request through a connection of the space, below its resource, as them. */
    async #connect(
        url: URL,
        request: Request,
        authentication: Authentication | undefined,
    ): Promise<Response> {
        // require a person, and the connection the path names
        if (authentication === undefined) {
            throw MachineError.of(
                "CONNECTION_UNAUTHENTICATED",
                "a call through a connection needs its caller",
            );
        }
        const [named = "", ...rest] = url.pathname.slice(CONNECTION_PATH.length + 1).split("/");
        const id = schema.identifier("connection").parse(named);

        // borrow its credential and resource at its home as the person, applied below the resource
        const lease = await this.#space.borrow(
            { scope: this.#space.id, id },
            authentication.caller.subject,
        );
        const below = new URL(`/${rest.join("/")}${url.search}`, url);
        const lent = await this.#apply(
            lease,
            copyRequest(request, {}, Gateway.#target(lease.resource, below)),
        );

        return this.#space.machine.send(lent.request, lent);
    }

    /** Apply a borrowed connection's credential to a call, at the machine's time. */
    #apply(lease: ConnectionLease, request: Request): Promise<ConnectionRequest> {
        const { clock, environment } = this.#space.machine;

        return ConnectionRequest.of(
            lease.protocol,
            lease.credential,
            request,
            clock.now(),
            environment.crypto,
        );
    }

    /** Answer an instance's request about its space's live sockets. */
    async #sockets(request: Request): Promise<Response> {
        const { hibernation } = this.#space.machine;
        if (hibernation === undefined) {
            throw MachineError.of("SOCKETS_MISSING", "this machine has no sockets");
        }

        return hibernation.answer(this.#instance(request).instanceId, request);
    }

    /** Verify a request's signed token toward the space's stack, absent for an instance secret. */
    async #signed(request: Request): Promise<Authentication | undefined> {
        const token = Bearer.read(request.headers);

        return token === undefined || Bearer.issuer(token) === undefined
            ? undefined
            : this.#space.machine.authenticate(request, stackPackage.id, this.#space.id);
    }

    /** Deliver an adapter's request, a routed request or a call to an installation. */
    async #installation(
        installed: NonNullable<ReturnType<typeof InstallationMount.of>>,
        authentication: Authentication | undefined,
    ): Promise<Response> {
        // hand an installation the space's store serves to its package there
        const { database, traffic } = this.#space;
        const served = await Rollout.requireServed(database, installed);
        if (this.#packages().some((handle) => handle.id === served.installation.packageId)) {
            const answered = await this.#store(served, installed, authentication);

            return traffic.countResponse(served.scope, served.installationId, answered);
        }

        // hand a request at an adapter its package declares to the newest deployment, as it speaks no release
        const path = new URL(installed.request.url).pathname;
        const own = PackageId.parse(served.installation.packageId);
        if (this.#isAdapted(served.installationId, own, path)) {
            const called = await this.#authenticate(installed, served, own, authentication);
            const answered = await this.#adapted(
                served.installationId,
                path,
                installed.request,
                called.authentication,
            );

            return traffic.countResponse(served.scope, served.installationId, answered);
        }

        // verify the caller for the installation's package and route the call to the deployment serving its release
        const called = await this.#authenticate(installed, served, own, authentication);
        const answered = await this.ingress(
            called.rollout.installationId,
            path,
            installed.request,
            called.authentication,
        );

        return traffic.countResponse(called.rollout.scope, called.rollout.installationId, answered);
    }

    /** Hand a request below an installation's mount to its package in the store. */
    async #store(
        served: InstallationRollout,
        installed: NonNullable<ReturnType<typeof InstallationMount.of>>,
        authentication: Authentication | undefined,
    ): Promise<Response> {
        // hand a call below its package's mount or a protocol's the package implements over as its verified caller
        const packageId = PackageId.parse(served.installation.packageId);
        const path = new URL(installed.request.url).pathname;
        const mounted = PackageMount.of(installed.request);
        const handle = this.#packages().find((each) => each.id === packageId);
        const isServed =
            mounted !== undefined &&
            (mounted.packageId === packageId ||
                (handle !== undefined &&
                    this.#space.instance.implements(handle, mounted.packageId, mounted.request)));
        if (mounted === undefined || !isServed) {
            throw MachineError.of(
                "NOTHING_SERVED",
                `the installation's package has nothing at ${path}`,
            );
        }
        const audience = PackageId.parse(mounted.packageId);
        const called = await this.#authenticate(installed, served, audience, authentication);

        return this.#space.call(packageId, mounted.request, called.authentication);
    }

    /** Verify an installation's caller for a package it serves, taking one handed verified. */
    async #authenticate(
        installed: NonNullable<ReturnType<typeof InstallationMount.of>>,
        served: InstallationRollout,
        audience: PackageId,
        handed: Authentication | undefined,
    ) {
        // verify another space's installation by its space's token
        const { request } = installed;
        const peer = PeerCall.of(request);
        if (peer !== undefined) {
            return peer.receive(this.#space.host, installed);
        }

        // take a caller verified for the called package, else verify the request
        const authentication =
            handed?.credential.audience === audience
                ? handed
                : await this.#space.machine.authenticate(request, audience, this.#space.id);

        return { rollout: served, authentication };
    }

    /** Serve a call at its path below an installation, the mount of the package it calls first. */
    async ingress(
        installationId: Identifier<"installation">,
        path: string,
        request: Request,
        authentication: Authentication | undefined,
    ): Promise<Response> {
        // require the release the caller speaks
        const header = request.headers.get(VERSION_HEADER);
        const version = header === null ? undefined : Version.safeParse(header).data;
        if (version === undefined) {
            const message =
                header === null
                    ? `requires ${VERSION_HEADER}`
                    : `invalid ${VERSION_HEADER}: ${header}`;
            throw MachineError.of("VERSION_INVALID", message);
        }

        // pick the newest running deployment serving that release, a dev build serving its release's callers
        const spoken = Version.release(version);
        const serving = await this.#newest(
            installationId,
            (endpoint) =>
                Version.compare(spoken, Version.release(endpoint.release)) <= 0 &&
                (endpoint.since === undefined ||
                    Version.compare(Version.release(endpoint.since), spoken) <= 0),
            `runs release ${version}`,
        );

        // count the request and limit it by the installation's space
        this.#space.traffic.countRequest(serving.scope, installationId);
        await this.#limit(serving.scope);

        // forward the caller
        const runtime = this.#runtime(serving.runtime);
        const named = request.headers.get(PARTITION_HEADER) ?? undefined;

        // forward a large body or one of unknown length once, to the partition the call selects
        const resendable = await Gateway.#resendable(request);
        if (resendable === undefined) {
            const answered = await runtime.fetch(
                serving.instanceId,
                path,
                request,
                authentication,
                named,
            );

            return Gateway.#accepted(answered, serving.instanceId);
        }

        // resend a small body once to the partition the store points to
        const { body } = resendable;
        const answered = await CallError.follow((misdirected) => {
            const partition =
                misdirected !== undefined && "partition" in misdirected
                    ? misdirected.partition
                    : named;
            const call = copyRequest(request, { body });

            return runtime.fetch(serving.instanceId, path, call, authentication, partition);
        });

        return Gateway.#accepted(answered, serving.instanceId);
    }

    /** Add the instance that accepts a socket to its answer to an upgrade. */
    static #accepted(answer: Response, instanceId: string): Response {
        // pass on an answer accepting no socket
        if (!answer.headers.has(SOCKET_HEADER)) {
            return answer;
        }

        // add the instance beside the acceptance
        const headers = new Headers(answer.headers);
        headers.set(SOCKET_INSTANCE_HEADER, instanceId);

        return new Response(answer.body, { status: answer.status, headers });
    }

    /** Report whether a path below an installation is at an adapter its package declares. */
    #isAdapted(
        installationId: Identifier<"installation">,
        packageId: PackageId,
        path: string,
    ): boolean {
        const mounted = PackageMount.parse(path);

        return (
            mounted !== undefined &&
            mounted.packageId === packageId &&
            this.#space
                .adapters(installationId)
                .some(
                    (adapter) =>
                        mounted.path === adapter.path ||
                        mounted.path.startsWith(`${adapter.path}/`),
                )
        );
    }

    /** Forward a request at an adapter to the newest running deployment, as its caller. */
    async #adapted(
        installationId: Identifier<"installation">,
        path: string,
        request: Request,
        authentication: Authentication | undefined,
    ): Promise<Response> {
        const newest = await this.#newest(installationId, () => true, "serves its adapters");

        return this.#runtime(newest.runtime).fetch(
            newest.instanceId,
            path,
            request,
            authentication,
        );
    }

    /** Send an installation's sandbox a request to run a hook, at its newest hooks instance. */
    async hook(
        installationId: Identifier<"installation">,
        request: HandlerRequest,
    ): Promise<HandlerAnswer> {
        // pick the newest running instance of the hooks workload
        const newest = await this.#newest(
            installationId,
            (endpoint) => endpoint.workload === HOOK_WORKLOAD,
            "handles hooks",
        );

        // send the request and read its answer, throwing a refusal
        const sent = new Request(`http://localhost${HOOK_PATH}`, {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify(request),
        });
        const answered = await this.#runtime(newest.runtime).deliver(
            newest.instanceId,
            HOOK_PATH,
            sent,
        );
        if (!answered.ok) {
            throw Failure.error(await Failure.read(answered));
        }

        return HandlerAnswer.parse(await answered.json());
    }

    /** Find an installation's newest running deployment that serves a call, refusing none. */
    async #newest(
        installationId: Identifier<"installation">,
        serves: (endpoint: Endpoint) => boolean,
        serving: string,
    ): Promise<Endpoint> {
        // take the newest release among the running deployments that serve the call
        const endpoints = await this.#endpoints(installationId);
        const [newest] = endpoints
            .filter(serves)
            .toSorted((left, right) => Version.compare(right.release, left.release));
        if (newest !== undefined) {
            return newest;
        }

        // ask the caller to wait while the installation still deploys, else refuse
        const installed = await Snapshot.live(this.#space.database)
            .read(installation.table)
            .findFirst({ where: { id: installationId } });
        if (installed !== undefined && Observation.progress(installed) === "waiting") {
            throw MachineError.of(
                "DEPLOYMENT_STARTING",
                `the deployment of this installation that ${serving} is starting`,
                { data: { retryAfter: DEPLOYMENT_RETRY_MILLISECONDS } },
            );
        } else {
            throw MachineError.of(
                "RUNNING_DEPLOYMENT_MISSING",
                `no running deployment of this installation ${serving}`,
            );
        }
    }

    /** List the running instances serving an installation. */
    #endpoints(installationId: Identifier<"installation">): Promise<readonly Endpoint[]> {
        return Instance.endpoints(this.#space.database, installationId);
    }

    /** Limit a call into a space, refusing one past its throttling entitlement. */
    async #limit(scope: Identifier<"space">): Promise<void> {
        // read whether the space's throttling entitlement is exceeded, passing a space within it
        const throttled = accountObject.Space.throttled(
            await accountObject.Space.read(Snapshot.live(this.#space.database), scope),
        );
        if (throttled === undefined) {
            return;
        }

        // refuse a call past the space's tokens, saying how long until the next one
        const taken = await Bucket.take(this.#space.database, {
            name: EXCEEDED_LIMIT,
            key: scope,
            rate: EXCEEDED_RATE,
            burst: EXCEEDED_BURST,
            now: this.#space.machine.clock.now(),
        });
        if (taken.kind === "limited") {
            throw MachineError.of(
                "RATE_LIMITED",
                `space ${scope} is rate-limited while its ${throttled} entitlement is exceeded`,
                { data: { retryAfter: taken.retryAfter } },
            );
        }
    }

    /** Serve a workload's call to an address below the machine's egress as its installation. */
    async egress(request: Request): Promise<Response> {
        return await this.send(EgressCaller.of(this.#instance(request)), request);
    }

    /** Send an installation's call to an address below the egress, from an instance or a store. */
    async send(caller: EgressCaller, request: Request): Promise<Response> {
        // require an address
        const routed = Egress.of(request);
        if (routed === undefined) {
            throw MachineError.of("EGRESS_ADDRESS_MISSING", "the request has no egress address");
        }

        // drop the caller's secret and identity claims, and the route only a machine marks
        const headers = new Headers(routed.request.headers);
        headers.delete("authorization");
        headers.delete(CALLER_HEADER);
        headers.delete("cookie");
        headers.delete(ROUTE_HEADER);

        // stream a large body or one of unknown length once
        const resendable = await Gateway.#resendable(routed.request);
        if (resendable === undefined) {
            return this.#send(caller, routed.address, copyRequest(routed.request, { headers }));
        }

        // buffer a small body for a second attempt
        const call = () => copyRequest(routed.request, { headers, body: resendable.body });

        // resolve the address again once the machine called answers misdirected
        return CallError.follow(() => this.#send(caller, routed.address, call()));
    }

    /** Read a request's body to send it again. */
    static async #resendable(
        request: Request,
    ): Promise<{ readonly body: ArrayBuffer | null } | undefined> {
        const length = request.headers.get("content-length");
        // resend an empty body
        if (request.body === null) {
            return { body: null };
        }
        // resend no unknown or large body
        else if (length === null || Number(length) > MAX_RESENT_BODY_BYTES) {
            return undefined;
        }

        return { body: await request.arrayBuffer() };
    }

    /** Address a request's path and query below a destination's URL. */
    static #target(base: string, url: URL): string {
        return `${base.replace(/\/$/u, "")}${url.pathname}${url.search}`;
    }

    /** Resolve an address an installation calls, and send the call as it or its subject. */
    async #send(caller: EgressCaller, address: string, sent: Request): Promise<Response> {
        // take off the exchange the call asks for
        const asked = Exchange.asking(sent);
        const unasked = new Headers(sent.headers);
        unasked.delete(Exchange.header);
        const call = copyRequest(sent, { headers: unasked });

        // resolve the address for the path called below it
        const url = new URL(call.url);
        const destination = await this.resolve(caller, address, url.pathname);

        // call an external API with the connection's credential applied
        // TODO #Incomplete: lend connections to TCP protocols, such as SQL and SMTP with XOAUTH2, after launch
        if (destination.kind === "external") {
            const target = Gateway.#target(destination.url, url);
            const headers = new Headers(call.headers);
            const lent = await destination.lend(copyRequest(call, { headers }, target));

            return this.#space.machine.send(lent.request, lent);
        }
        const acting = principal.installation.reference(caller.scope, caller.installationId);
        const target = {
            audience: destination.audience,
            ...Gateway.#scope(caller, destination),
        };
        const own = () =>
            Gateway.#authentication(caller, destination, this.#space.machine.clock.now());

        // serve an installation or package on this machine, as the caller or its asked subject
        if (destination.kind === "installation" || destination.kind === "package") {
            const authentication =
                asked === undefined ? own() : await this.act(acting, asked, target);

            return destination.kind === "installation"
                ? this.ingress(destination.installationId, destination.path, call, authentication)
                : destination.fetch(call, authentication);
        }
        // call out with the installation's token, or its subject's exchanged one
        else {
            const token =
                asked === undefined
                    ? await this.#sign(own(), caller.scope)
                    : await this.exchange(acting, asked, target);
            const headers = new Headers(call.headers);
            headers.set("authorization", `Bearer ${token}`);
            const remote = Gateway.#target(destination.url, url);

            return this.#space.machine.send(copyRequest(call, { headers }, remote));
        }
    }

    /** Resolve an address an installation calls, from an instance or its store. */
    async resolve(caller: EgressCaller, address: string, path: string): Promise<Destination> {
        // take a package the space serves for every workload as a caller verified here
        const served = this.#packages().find((handle) => handle.name === address);
        if (served !== undefined) {
            return {
                kind: "package",
                audience: served.id,
                fetch: (request, authentication) =>
                    this.#space.call(served.id, request, authentication),
            };
        }

        // take a space's own packages at the machine serving the space
        const spaceAddress = schema.identifier("space").safeParse(address);
        const mount = PackageMount.parse(path);
        if (spaceAddress.success && mount !== undefined) {
            const located = await this.#space.host.machine.directory.locate(
                spaceAddress.data,
                this.#space.machine.clock.now(),
            );
            const url = SpaceMount.url(located.url, spaceAddress.data);
            const audience = PackageId.parse(mount.packageId);

            return { kind: "remote", scope: spaceAddress.data, url, audience };
        }

        // take an installation at its address: its object APIs from anyone, its procedures once bound
        const peer = Egress.installation(address);
        if (peer !== undefined) {
            // refuse a path below no package mount
            if (mount === undefined) {
                throw MachineError.of(
                    "PACKAGE_MOUNT_MISSING",
                    `${path} is not a package mount of installation ${peer.name}`,
                );
            }
            // refuse an installation the caller does not bind
            else if (!EgressCaller.binds(caller, peer, mount.path)) {
                throw MachineError.of(
                    "INSTALLATION_UNBOUND",
                    `this installation does not bind installation ${peer.name}`,
                );
            }

            return this.#resolvePeer(peer.name, peer.spaceId, path);
        }

        // apply a bound connection's credential to calls below its resource, bound at its identifier
        const connected = caller.resources.find(
            (resource) => resource.kind === ConnectionKind.name && resource.reference === address,
        );
        if (connected !== undefined) {
            return this.#resolveConnection(caller, connected);
        }

        // serve a bucket the instance binds over S3 as its space
        const bucketId = schema.identifier("bucket").safeParse(address);
        if (bucketId.success) {
            return this.#resolveBucket(caller, bucketId.data);
        }

        // require an installation binding with the address and call the package it depends on
        const bound = caller.resources.find(
            (resource) => resource.kind === InstallationKind.name && resource.reference === address,
        );
        if (bound === undefined) {
            throw MachineError.of("ADDRESS_UNBOUND", `this installation does not bind ${address}`);
        }
        const audience = InstallationBindingSpec.parse(bound.spec).package;

        return this.#resolveBound(caller, address, audience, path);
    }

    /** Resolve a bucket the instance binds to the machine's S3 server. */
    #resolveBucket(caller: EgressCaller, bucketId: Identifier<"bucket">): Destination {
        // refuse a bucket the instance binds none of, and a machine serving none
        const isBound = caller.resources.some(
            (resource) => resource.kind === BucketKind.name && resource.id === bucketId,
        );
        if (!isBound) {
            throw MachineError.of(
                "BUCKET_UNBOUND",
                `this installation does not bind bucket ${bucketId}`,
            );
        }
        const { s3 } = this.#serving;

        return {
            kind: "package",
            audience: S3_PACKAGE.id,
            fetch: (request) => {
                // address the key path-style below the bucket, as the space's own access key
                const url = new URL(request.url);
                url.pathname = `/${bucketId}${url.pathname}`;

                return s3.serve(copyRequest(request, {}, url.href), caller.scope);
            },
        };
    }

    /** Resolve a bound connection to its API, applying its credential as the installation's. */
    #resolveConnection(caller: EgressCaller, resource: WorkloadResource): Destination {
        // read the connection's API, the installation consuming it and the connection
        const { resource: url } = ConnectionSpec.parse(resource.spec);
        const consumer = principal.installation.reference(caller.scope, caller.installationId);
        const id = schema.identifier("connection").parse(resource.id);

        return {
            kind: "external",
            url,
            // borrow the bound connection's credential as the installation, applied to the call
            lend: async (request) => {
                const lease = await this.#space.borrow({ scope: caller.scope, id }, consumer);

                return this.#apply(lease, request);
            },
        };
    }

    /** Resolve a bound installation's address for a path below its package's mount. */
    async #resolveBound(
        caller: EgressCaller,
        address: string,
        audience: PackageId,
        path: string,
    ): Promise<Destination> {
        // call a package of the universe at the universe's mount
        if (PackageName.safeParse(address).success) {
            const url = PackageMount.url(
                new URL(this.#space.host.machine.universe.accounts).origin,
                audience,
            );

            return { kind: "universe", url, audience };
        }

        // refuse an unknown name or another package and call one of the caller's space
        const name = DeclarationName.safeParse(address);
        const found = name.success
            ? await Rollout.find(this.#space.database, caller.scope, name.data)
            : undefined;
        if (found === undefined || found.installation.packageId !== audience) {
            throw MachineError.of(
                "INSTALLATION_NOT_FOUND",
                `installation ${address} of ${audience} not found in ${caller.scope}`,
            );
        }

        return {
            kind: "installation",
            scope: caller.scope,
            installationId: found.installationId,
            audience,
            path: `${PackageMount.path(audience)}${path}`,
        };
    }

    /** Resolve an installation in a space for a path below it. */
    async #resolvePeer(
        name: string,
        spaceId: Identifier<"space">,
        path: string,
    ): Promise<Destination> {
        // call an installation this machine serves in the space, by its name there
        const [named] = await Snapshot.live(this.#space.database)
            .read(installation.table)
            .findMany({
                where: {
                    scope: spaceId,
                    name,
                },
            });
        const served =
            named === undefined
                ? undefined
                : await Rollout.findServed(this.#space.database, named.id);
        if (served?.scope === spaceId) {
            const audience = PackageId.parse(served.installation.packageId);

            return {
                kind: "installation",
                scope: spaceId,
                installationId: served.installationId,
                audience,
                path,
            };
        }

        // call an installation the machine serving its space answers for
        const located = await this.#space.host.machine.directory.locate(
            spaceId,
            this.#space.machine.clock.now(),
        );

        return {
            kind: "remote",
            scope: spaceId,
            url: InstallationMount.url(located.url, spaceId, name),
            audience: stackPackage.id,
        };
    }

    /** Exchange an installation's token for a subject's under its grant to it, at the universe. */
    exchange(acting: Subject, asked: ExchangeSubject, target: ExchangeTarget): Promise<string> {
        return this.#tokens.exchange(acting, asked, target);
    }

    /** Act for a subject under its grant to an installation, as the universe verifies them. */
    async act(
        acting: Subject,
        asked: ExchangeSubject,
        target: ExchangeTarget,
    ): Promise<Authentication> {
        const token = await this.exchange(acting, asked, target);

        return this.#tokens.verify(token, target);
    }

    /** Sign an installation's token for a call leaving the machine, as the space. */
    async #sign(authentication: Authentication, spaceId: Identifier<"space">): Promise<string> {
        const { database, machine } = this.#space;
        const issued = await this.#space.host.keystore
            .issuer(database, spaceId)
            .issue(authentication, machine.clock.now(), machine.environment);

        return issued.accessToken;
    }

    /** Bind a call's authentication to the space it targets. */
    static #scope(
        caller: EgressCaller,
        destination: Exclude<Destination, { kind: "external" }>,
    ): { readonly scope?: Identifier<"space"> } {
        // bind the instance's own space for a mounted package
        if (destination.kind === "package") {
            return { scope: caller.scope };
        }
        // bind no space for the universe
        else if (destination.kind === "universe") {
            return {};
        }
        // bind the targeted space otherwise
        else {
            return { scope: destination.scope };
        }
    }

    /** Authenticate the installation calling a destination now, in its deployment if any. */
    static #authentication(
        caller: EgressCaller,
        destination: Exclude<Destination, { kind: "external" }>,
        now: number,
    ): Authentication {
        // act as the installation, by its instance in its deployment, or by itself in its store
        const subject = principal.installation.reference(caller.scope, caller.installationId);
        const { runs } = caller;
        const id = runs.kind === "instance" ? runs.instanceId : caller.installationId;

        return {
            caller: Caller.of(subject),
            credential: {
                kind: "installation",
                id,
                audience: destination.audience,
                ...Gateway.#scope(caller, destination),
                verifiedAt: now,
                expiresAt: now + TOKEN_LIFETIME_MILLISECONDS,
                ...(runs.kind === "instance"
                    ? { deployments: [{ subject, id: runs.deploymentId }] }
                    : {}),
            },
        };
    }
    /** Take an instance's telemetry by its secret, a page's or a machine's by its caller. */
    async #telemetry(request: Request, subject: Subject | undefined): Promise<Response> {
        // read the export by its signal's path
        const signal = OtlpSignal.at(new URL(request.url).pathname);
        if (signal === undefined) {
            throw MachineError.of("SIGNAL_UNKNOWN", "no OTLP signal at this path");
        }

        // forward a machine's own export, its token bound to this space by the universe
        if (subject !== undefined && principal.machine.is(subject)) {
            const body = await RequestBody.json(request, `an OTLP/JSON ${signal} export`);
            await this.#space.observe(signal, body);
        }
        // relay an instance's export by its secret, counting the CPU time its metrics report
        else {
            const spec = this.#instance(request);
            const body = await RequestBody.json(request, `an OTLP/JSON ${signal} export`);
            await this.telemetry(spec, signal, body);
            if (signal === "metrics") {
                this.#space.traffic.countMetrics(spec.scope, spec.installationId, body);
            }
        }

        return Response.json({});
    }

    /** Append an instance's OTLP/JSON export to its space as the machine, stamped by emitter. */
    async telemetry(spec: InstanceSpec, signal: OtlpSignal, body: unknown): Promise<void> {
        const emitter = { installation: spec.installationId, instance: spec.instanceId };
        await this.record(
            spec.manifest === undefined ? emitter : { ...emitter, build: spec.manifest },
            signal,
            body,
        );
    }

    /** Record an installation's export: stamp its emitter, attribute its frames, forward it. */
    async record(
        emitter: {
            readonly installation: Identifier<"installation">;
            readonly instance?: Identifier<"instance">;
            readonly build?: Digest;
        },
        signal: OtlpSignal,
        body: unknown,
    ): Promise<void> {
        const stamped = new OtlpExport(signal, body).stamp(emitter);
        const attributed = await this.#attribute(stamped, emitter.installation, emitter.build);

        await this.#space.observe(signal, attributed.body);
    }

    /** Attribute an export's exception frames through its installation's build. */
    async #attribute(
        exported: OtlpExport,
        installationId: Identifier<"installation">,
        manifest: Digest | undefined,
    ): Promise<OtlpExport> {
        // leave an export of no build as it is
        if (manifest === undefined) {
            return exported;
        }

        return exported.attribute(async (frames) =>
            (await this.#attribution(installationId, manifest)).frames(frames),
        );
    }

    /** Read an installation's served build's attribution once, replacing an earlier one. */
    #attribution(
        installationId: Identifier<"installation">,
        manifest: Digest,
    ): Promise<Attribution> {
        // reuse the attribution of the build the installation still serves
        const known = this.#attributions.get(installationId);
        if (known?.manifest === manifest) {
            return known.attribution;
        }

        // read the build and the code index attached to its manifest, forgetting a failed read
        const packages = this.#space.builds;
        const attribution = Promise.all([
            packages.contents(manifest),
            CodeIndex.read(packages, manifest),
        ]).then(([reader, index]) => new Attribution(reader, index));
        const entry = { manifest, attribution };
        this.#attributions.set(installationId, entry);
        attribution.catch(() => {
            if (this.#attributions.get(installationId) === entry) {
                this.#attributions.delete(installationId);
            }
        });

        return attribution;
    }

    /** Identify the instance whose secret a request presents. */
    #instance(request: Request): InstanceSpec {
        // find the instance the secret proves in the runtime running it
        const secret = Bearer.token(request.headers.get("authorization"));
        const runtimes = [...this.#runtimes.values()];
        const spec =
            secret === undefined
                ? undefined
                : runtimes.map((runtime) => runtime.identify(secret)).find(Boolean);
        if (spec === undefined) {
            throw MachineError.of("INSTANCE_SECRET_INVALID", "invalid instance secret");
        }

        return spec;
    }

    /** Find the runtime of a server runtime, refusing one the machine does not run. */
    #runtime(name: DeploymentRuntime): Runtime {
        const runtime = this.#runtimes.get(name);
        if (runtime === undefined) {
            throw MachineError.of("RUNTIME_MISSING", `this machine has no ${name} runtime`);
        }

        return runtime;
    }
}
