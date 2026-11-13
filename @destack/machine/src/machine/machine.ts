import { StackError } from "@destack/stack/error";
import type { PackageFile } from "@destack/package/file";
import { type Lock, type PackageManifest } from "@destack/package/manifest";
import { type BuildStore } from "@destack/build/store";
import type * as packageRuntime from "@destack/package/runtime";
import type * as installation from "../installation/index.ts";
import { Traffic } from "../gateway/traffic.ts";
import { Authentication, Caller, type ChainOptions, principal } from "@destack/access";
import { Directory, DOMAINS, UniverseOrigin } from "@destack/account/address";
import { AccountError } from "@destack/account/error";
import {
    type Enrollment,
    type MachineKey,
    Session,
    type TokenBinding,
} from "@destack/account/client";
import { Bookmark } from "@destack/object/client";
import { IdentityRegistry } from "@destack/account/identity";
import * as accountObject from "@destack/account/object";
import { Identity, Key, placement } from "@destack/account/object";
import accountPackage from "@destack/account/package";
import {
    eq,
    or,
    type DatabaseConnection,
    type Select,
    Snapshot,
    type Table,
    type Controller,
    type Following,
} from "@destack/db";
import type { Environment, FetchOptions } from "@destack/environment";
import { Registry } from "@destack/registry/client";
import type { Keyring } from "@destack/identity";
import {
    CallError,
    ClaimLookup,
    claimTable,
    type Fetch,
    type Follow,
    type MethodInput,
    type Package,
    PackageMount,
    refusal,
    SpaceMount,
} from "@destack/object";
import type {
    PackageImplementation,
    ProcedureContext,
    RequestContext,
    Server,
} from "@destack/object/server";
import { PackageId, type PackageRelease } from "@destack/package";
import { TUNNEL_PATH, TunnelClient, type TunnelStatus } from "@destack/tunnel";
import { Bearer, type PartitionId, SPACE_KEY } from "@destack/access/token";
import { createClient, type ClientOf } from "@destack/object/client";
import { Health } from "@destack/object/server";
import type { Alarm } from "@destack/db";
import { copyRequest, waitAborted } from "@destack/environment";
import { RequestId, type Publisher, Scope } from "@destack/sync";
import type { HibernationHost } from "@destack/object/workload";
import {
    canonicalize,
    type Digest,
    type Identifier,
    present,
    schema,
    type VersionRange,
} from "@destack/schema";
import * as stackObject from "@destack/stack/object";
import stackPackage from "@destack/stack/package";
import { PROCESS_RUNTIME } from "@destack/stack/object";
import type { Runtime } from "../runtime/runtime.ts";
import { Handoff } from "../transfer/handoff.ts";
import { computer, MachineEnrollment } from "../computer/device/object/index.ts";
import { Space, type SpaceStorage, SpaceRequest } from "../space/index.ts";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../error/index.ts";

/** How long the machine's and its spaces' services drain accepted requests at a stop. */
export const DRAIN_TIMEOUT_MILLISECONDS = 10000;

/** The origin this machine calls its own spaces at, through itself, not the network. */
const OWN_MACHINE = "http://machine.invalid";

/** How long this machine's own calls as itself to its spaces last: a minute, one call's latency. */
const MACHINE_CALL_MILLISECONDS = 60_000;

/** The prefix of the root retaining a space's builds in its machine's builds. */
const SPACE_ROOT_PREFIX = "space:";

/** The oldest Destack version whose releases a machine runs. */
export const OLDEST_DESTACK = "2026.11.0";

/** What a machine serves its spaces with: its public and its private fields. */
export type MachineOptions = Pick<
    Machine,
    | "id"
    | "environment"
    | "identity"
    | "keyring"
    | "database"
    | "volumes"
    | "builds"
    | "targets"
    | "isBuilding"
    | "isEnforcingEntitlements"
    | "isServingHostnames"
    | "domain"
    | "runtimes"
    | "hibernation"
    | "alarm"
    | "server"
    | "skus"
    | "endpoint"
    | "authenticate"
    | "authorizeSpace"
    | "report"
> & {
    /** The releases of the packages the machine links, by package. */
    readonly linked: ReadonlyMap<PackageId, PackageRelease>;
    /** Read the machine's enrollment, through a controller's view when given. */
    readonly enrollment: (view?: Pick<Snapshot, "read">) => Promise<MachineEnrollment | undefined>;
    /** Serve the machine's own partition while it serves, absent for a machine without one. */
    readonly partition?: () => Promise<AsyncDisposable>;
    /** Read what the machine reports beside its version and runtimes, at each report. */
    readonly describe: () => Promise<
        Omit<MethodInput<typeof accountObject.machine, "report">, "id" | "version" | "runtimes">
    >;
    /** The forges' registry, by default the directory's forges called as the machine. */
    readonly releases?: installation.Registry;
    /** Every release the machine ships, whose manifests its registry answers before any forge's. */
    readonly shipped: readonly PackageRelease[];
    /** The installation residency homes serve the registry protocol at, such as forge. */
    readonly registry: string;
    /** The Destack versions whose releases the machine runs: the oldest it supports and its own. */
    readonly destack: VersionRange;
    /** The relay a personal machine tunnels through while enrolled, absent for none. */
    readonly tunnel?: {
        /** The relay's tunnel URL, its account residency's relay's when absent. */
        readonly url?: string;
        /** Answer a request for one of the machine's names. */
        readonly fetch: (request: Request) => Promise<Response>;
        /** Report a failed renewal of the tunnel's name. */
        readonly report: (error: unknown) => void;
        /** Learn that the tunnel opened, or ended and dials again. */
        readonly changed: (status: TunnelStatus) => void;
    };
};

/** The enrollment of a person's machine, as the approving person submits it. */
type PersonalEnrollment = Extract<Enrollment, { readonly kind: "personal" }>;

/** A machine: its standing at the universe, the storage of its spaces, and the spaces it serves. */
export class Machine {
    /** The universe's tables a machine replicates: machines, keys, names and placements. */
    static readonly replicated: readonly Table[] = [
        ...[
            accountObject.machine,
            accountObject.key,
            accountObject.endpoint,
            accountObject.claim,
            accountObject.identity,
            accountObject.operation,
        ].map((type) => type.table),
        accountObject.placement.table,
        accountObject.assignment.table,
    ];

    /** The machine. */
    readonly id: Identifier<"machine">;
    /** The clock the machine runs at, and the network it sends through. */
    readonly environment: Environment;
    /** The identity the machine proves with the key the universe enrolled. */
    readonly identity: MachineKey;
    /** The machine's root keys, wrapping vault and space keys. */
    readonly keyring: Keyring;
    /** The machine's own database, with its replicas. */
    readonly database: DatabaseConnection;
    /** The machine's replica of the universe's directory. */
    readonly directory: Directory;
    /** The storage of the machine's spaces. */
    readonly volumes: SpaceStorage;
    /** The builds the machine runs: pulled releases and copies of its spaces' builds. */
    readonly builds: BuildStore;
    /** The runtimes the machine pulls builds' closures for. */
    readonly targets: readonly packageRuntime.Runtime[];
    /** The locks pulled from their forges, by digest, each once. */
    readonly #pulled = new Map<Digest, Promise<Lock>>();
    /** The forges' registry of published releases, called as the machine. */
    readonly releases: installation.Registry;
    /** The installation residency homes serve the registry protocol at. */
    readonly registry: string;
    /** Whether the machine builds the commits its spaces' installations select. */
    readonly isBuilding: boolean;
    /** Whether the machine checks its spaces' use against their accounts' entitlements. */
    readonly isEnforcingEntitlements: boolean;
    /** Whether the machine serves requests for its spaces' accounts' own hostnames. */
    readonly isServingHostnames: boolean;
    /** The runtimes running the spaces' instances, in preference order. */
    readonly runtimes: readonly Runtime[];
    /** The open WebSockets of the machine's instances, absent where the runtime serves them. */
    readonly hibernation: Pick<HibernationHost, "answer"> | undefined;
    /** Set a wake-up for a space's earliest due controller key, absent where timers wake them. */
    readonly alarm: Alarm | undefined;
    /** Load the server a lock's root exports into the machine's process, absent for none. */
    readonly server: (lock: Digest) => Promise<PackageImplementation | undefined>;
    /** The locks of the releases the machine runs in its process: those it ships or links. */
    readonly #trusted: Set<Digest>;
    /** The releases of the packages the machine links, by package. */
    readonly #linked: Map<PackageId, PackageRelease>;
    /** The releases the machine ships, by package. */
    readonly shipped: ReadonlyMap<PackageId, PackageRelease>;
    /** The SKU key rating each meter at its attribute values, by `Meter.key`, absent unbilled. */
    readonly skus: ReadonlyMap<string, string> | undefined;
    /** The URL the machine answers at: a cloud machine's declared origin, else its local one. */
    readonly endpoint: string;
    /** The origin the machine answers at: its endpoint, else its relayed name once renewed. */
    #origin: string;
    /** The domain of the installation origins the machine's own host names answer. */
    readonly domain: string;
    /** Verify a caller of a package the machine serves, none for a call without a credential. */
    readonly authenticate: (
        request: Request,
        audience: PackageId,
        partition?: PartitionId,
    ) => Promise<Authentication | undefined>;
    /** Enforce the host's requirements on a call to a space's packages. */
    readonly authorizeSpace: (call: ProcedureContext<RequestContext>) => Promise<void>;
    /** Report a failure no request answers. */
    readonly report: (error: unknown) => void;
    /** The Destack versions whose releases the machine runs: the oldest it supports and its own. */
    readonly destack: VersionRange;
    /** The relay a personal machine tunnels through, absent when peers call its origin. */
    readonly #tunnel: MachineOptions["tunnel"];
    /** Read this machine's enrollment as its record states it, valid or not. */
    readonly #readEnrollment: MachineOptions["enrollment"];
    /** Serve the machine's own partition while it serves. */
    readonly #partition: MachineOptions["partition"];
    /** Read what the machine reports beside its version and runtimes. */
    readonly #describe: MachineOptions["describe"];
    /** The spaces started while serving, by space. */
    readonly #started = new Map<Identifier<"space">, Promise<Space>>();
    /** Whether the machine serves its spaces, and why not. */
    readonly health: Health;
    /** The requests the machine's installations serve, the bytes they move and their CPU time. */
    readonly traffic = new Traffic();

    /** Serve spaces with a machine's identity, its storage and its runtimes. */
    constructor(options: MachineOptions) {
        // set the machine's identity and its storage
        this.id = options.id;
        this.health = new Health(options.id);
        this.health.set("starting", "it has not read its enrollment yet");
        this.environment = options.environment;
        this.identity = options.identity;
        this.keyring = options.keyring;
        this.database = options.database;
        this.directory = new Directory(Snapshot.live(options.database));

        // set the spaces' storage, the builds and what the machine builds and enforces
        this.volumes = options.volumes;
        this.builds = options.builds;
        this.targets = options.targets;
        this.isBuilding = options.isBuilding;
        this.isEnforcingEntitlements = options.isEnforcingEntitlements;
        this.isServingHostnames = options.isServingHostnames;

        // set the runtimes and what the machine serves with them
        this.runtimes = options.runtimes;
        this.hibernation = options.hibernation;
        this.alarm = options.alarm;
        this.server = options.server;
        this.#linked = new Map(options.linked);
        this.shipped = new Map(
            options.shipped.map((release) => [PackageId.parse(release.package.id), release]),
        );
        this.#trusted = new Set(
            [...options.shipped, ...options.linked.values()].map((release) => release.lock),
        );
        this.skus = options.skus;

        // set where peers call the machine and how it verifies and reports
        this.endpoint = options.endpoint;
        this.#origin = options.endpoint;
        this.domain = options.domain;
        this.authenticate = options.authenticate;
        this.authorizeSpace = options.authorizeSpace;
        this.report = options.report;
        this.destack = options.destack;
        this.#tunnel = options.tunnel;
        this.#readEnrollment = options.enrollment;
        this.#partition = options.partition;
        this.#describe = options.describe;

        // read releases the machine ships first, else at the forges in their accounts' residency homes, bound to each home
        const home = async (account: Identifier<"account">) =>
            (await this.accounts()).residency.locate({ account });
        this.registry = options.registry;
        this.releases =
            options.releases ??
            Registry.of(
                this.directory,
                home,
                (audience, partition) => this.fetch(audience, { partition }),
                this.environment,
                options.registry,
                options.shipped,
            );
    }

    /** Pull a published lock with its closures once, from its package's forge. */
    pull(packageId: PackageId, lock: Digest): Promise<Lock> {
        // pull a lock once, forgetting a failed pull
        const known = this.#pulled.get(lock);
        if (known !== undefined) {
            return known;
        }
        const pulling = this.releases.pull(this.builds, packageId, lock, this.targets);
        this.#pulled.set(lock, pulling);
        pulling.catch(() => this.#pulled.delete(lock));

        return pulling;
    }

    /** Pull a file of a published build into the machine's builds from its package's forge. */
    async pullFile(
        build: { readonly digest: Digest; readonly manifest: PackageManifest },
        file: PackageFile,
    ): Promise<void> {
        // refuse a build without a forge, such as an npm unit
        const owner = build.manifest.package;
        if (!("id" in owner)) {
            throw StackError.of("BLOB_NOT_FOUND", `package file '${file.path}' not found`);
        }

        // write the file from the build's forge
        const forge = await this.releases.store(owner.id, {
            kind: "manifest",
            digest: build.digest,
        });
        await this.builds.write(forge.read([{ kind: "file", digest: file.digest }]));
    }

    /** Retain the builds a space runs in the machine's builds. */
    async retain(space: Identifier<"space">, locks: Iterable<Digest>): Promise<void> {
        await this.builds.retain(`${SPACE_ROOT_PREFIX}${space}`, new Set(locks));
    }

    /** Forget the retained builds of each space the machine no longer stores. */
    async #prune(spaces: readonly Identifier<"space">[]): Promise<void> {
        const kept = new Set(spaces.map((space) => `${SPACE_ROOT_PREFIX}${space}`));
        for (const name of (await this.builds.roots()).keys()) {
            if (name.startsWith(SPACE_ROOT_PREFIX) && !kept.has(name)) {
                await this.builds.forget(name);
            }
        }
    }

    /** Pull the locks of the releases the machine links, by package name. */
    async linkedLocks(): Promise<Map<string, Lock>> {
        const pulled = [...this.#linked.values()].map(
            async (release) =>
                [release.package.name, await this.pull(release.package.id, release.lock)] as const,
        );

        return new Map(await Promise.all(pulled));
    }

    /** The releases of the packages the machine links, by package. */
    get linked(): ReadonlyMap<PackageId, PackageRelease> {
        return this.#linked;
    }

    /** The locks of the releases the machine runs in its process: those it ships or links. */
    get trusted(): ReadonlySet<Digest> {
        return this.#trusted;
    }

    /** Link a package's release in place of the earlier one, as a newer distribution does. */
    link(release: PackageRelease): void {
        // trust the release's lock in place of the replaced one
        const id = PackageId.parse(release.package.id);
        const replaced = this.#linked.get(id);
        if (replaced !== undefined) {
            this.#trusted.delete(replaced.lock);
        }
        this.#linked.set(id, release);
        this.#trusted.add(release.lock);
    }

    /** Follow the directory, the identities and a machine's own placements from the universe. */
    static follows(machineId: Identifier<"machine">): readonly Follow[] {
        return [
            accountObject.directoryShape.at({ key: { scope: Scope.universe.id } }),
            accountObject.identityShape.at({ key: { scope: Scope.universe.id } }),
            accountObject.machineShape.at({
                key: { scope: Scope.universe.id, machine: machineId },
            }),
        ];
    }

    /** The clock the machine runs at. */
    get clock(): Environment["clock"] {
        return this.environment.clock;
    }

    // standing at the universe

    /** Read this machine's enrollment for an action, refusing it unenrolled or revoked. */
    async enrolled(action: string): Promise<MachineEnrollment> {
        const enrollment = await this.#enrollment();
        if (enrollment === undefined) {
            throw MachineError.unenrolled(action);
        }

        return enrollment;
    }

    /** Report whether this machine is enrolled. */
    async isEnrolled(): Promise<boolean> {
        return (await this.#enrollment()) !== undefined;
    }

    /** Read the account package's URL and the issuer of the universe this machine enrolled at. */
    async universe(): Promise<{ readonly accounts: string; readonly issuer: string }> {
        const enrollment = await this.enrolled("serving spaces");
        const accounts = PackageMount.url(enrollment.universe, accountPackage.id);

        return { accounts, issuer: UniverseOrigin.issuer(enrollment.universe) };
    }

    /** Wait until the directory replica is a complete snapshot, as informers sync caches. */
    async synced(signal: AbortSignal): Promise<boolean> {
        const key = { scope: Scope.universe.id };

        return this.database.log.waitUntil(
            () => accountObject.directoryShape.isSynced(this.database, key),
            signal,
        );
    }

    /** Connect to the account package of this machine's enrollment as this machine. */
    async accounts(): Promise<ClientOf<typeof accountPackage>> {
        const enrollment = await this.enrolled("serving spaces");

        return createClient(
            accountPackage,
            {
                url: PackageMount.url(enrollment.universe, accountPackage.id),
                fetch: this.#fetchAs(enrollment, accountPackage.id),
            },
            this.environment,
        );
    }

    /** Open the registry of this machine's spaces' identities at the universe. */
    async identities(): Promise<IdentityRegistry> {
        return IdentityRegistry.client(await this.accounts(), this.environment);
    }

    /** Create a fetch calling a package as this machine, in a space when given. */
    fetch(audience: PackageId, binding: Pick<TokenBinding, "partition"> = {}): Fetch {
        return async (request) => {
            const enrollment = await this.enrolled("calling packages");

            return this.#fetchAs(enrollment, audience, binding)(request);
        };
    }

    /** Send a request to another origin as it is, with a lent call's client certificate. */
    send(request: Request, options?: FetchOptions): Promise<Response> {
        return this.environment.network.fetch(request, options);
    }

    /** Read the home of this machine's owner: its account's, or a cloud machine's residency's. */
    async home(): Promise<Identifier<"space">> {
        // take a personal machine's account home, else the residency's home
        const enrollment = await this.enrolled("reading its owner");
        if (enrollment.kind === "personal") {
            return accountObject.Account.home(enrollment.accountId);
        }
        const accounts = await this.accounts();
        const { space: home } = await accounts.residency.get({ id: enrollment.residencyId });

        return schema
            .identifier("space")
            .parse(present(home, `the home of ${enrollment.residencyId}`));
    }

    /** Announce what this machine runs to its account: its version, runtimes and host. */
    async announce(): Promise<void> {
        const enrollment = await this.enrolled("reporting telemetry");
        await (
            await this.accounts()
        ).machine.report({
            scope: MachineEnrollment.scope(enrollment),
            id: this.id,
            requestId: RequestId.create(this.environment),
            version: this.destack.newest,
            runtimes: this.runtimes.flatMap((runtime) =>
                runtime.name === PROCESS_RUNTIME ? [] : [runtime.name],
            ),
            ...(await this.#describe()),
        });
    }

    /** Enroll this machine under an account as a personal machine with a new key. */
    async enroll(
        accounts: ClientOf<typeof accountPackage>,
        universe: string,
        input: Omit<PersonalEnrollment, "scope" | "requestId" | "kind"> & {
            readonly accountId: Identifier<"account">;
        },
    ): Promise<MachineEnrollment> {
        // enroll through the approving person's client with a new key
        const { accountId, ...enrollment } = input;
        await this.identity.enroll(accounts, {
            ...enrollment,
            scope: accountId,
            requestId: RequestId.create(this.environment),
            kind: "personal",
        });

        return { kind: "personal", universe, accountId, status: "enrolled" };
    }

    /** Revoke an enrollment of this machine at its universe as itself and forget its key. */
    async revoke(enrollment: MachineEnrollment): Promise<void> {
        const accounts = createClient(
            accountPackage,
            {
                url: PackageMount.url(enrollment.universe, accountPackage.id),
                fetch: this.#fetchAs(enrollment, accountPackage.id),
            },
            this.environment,
        );
        await accounts.machine.delete({
            scope: MachineEnrollment.scope(enrollment),
            id: this.id,
            requestId: RequestId.create(this.environment),
        });
        await this.identity.forget();
    }

    /** Connect to the handoff of a machine that moves a space here, at its stack package. */
    async handoff(machine: Identifier<"machine">, spaceId: Identifier<"space">) {
        // call the source machine's stack package in the space, as this machine
        const fetch = this.fetch(stackPackage.id, { partition: spaceId });
        const answering = await this.directory.answering(machine, this.clock.now());
        const url = `${SpaceMount.url(answering, spaceId)}${PackageMount.path(stackPackage.id)}`;

        return Handoff.connect(url, fetch, this.environment);
    }

    /** Read how this machine's store replicates its chain, absent before it enrolls. */
    async chain(): Promise<ChainOptions | undefined> {
        // read the enrollment, absent before the machine enrolls
        const enrollment = await this.#enrollment();
        if (enrollment === undefined) {
            return undefined;
        }

        // require the key the chain's follow proves the machine with
        await this.identity.key();

        return { kind: "home", parent: MachineEnrollment.scope(enrollment) };
    }

    /** Stream a followed scope's replicas: the universe's, this machine's own, or a space's. */
    publisher(scope: string): Publisher {
        // follow the universe and this machine's own chain at the account package, once enrolled
        if (scope === Scope.universe.id || scope === this.id) {
            const accounts = () => this.accounts();
            const isEnrolled = () => this.isEnrolled();

            return {
                stream: async function* (subscription, signal) {
                    // follow nothing while unenrolled, as a revoked machine's store drops the follow
                    if (!(await isEnrolled())) {
                        await waitAborted(signal);

                        return;
                    }
                    yield* await (await accounts()).store.stream(subscription, { signal });
                },
                request: async (subscription) => {
                    await (await accounts()).store.request({ subscription });
                },
            };
        }

        // follow a space at the stack package of the machine serving it, once this machine serves
        const spaceId = schema.identifier("space").parse(scope);
        const stackOf = async (signal: AbortSignal) => {
            // wait until this machine serves, refusing while it pauses, then call the space as itself
            await this.health.waitUp(signal);

            return this.connect(stackPackage, spaceId);
        };
        const placed = () => this.#placed(spaceId);
        const moved = (from: string | undefined, signal: AbortSignal) =>
            this.database.log.waitUntil(async () => (await placed()) !== from, signal);
        const isServing = () => this.health.status === "up";

        return {
            stream: async function* (subscription, signal) {
                // follow the space where the directory places it
                let isStreamed = false;
                while (!signal.aborted) {
                    // wait until this machine serves, then read where the directory places the space
                    const stack = await stackOf(signal);
                    const from = await placed();

                    // stream its rows, waiting out a move or a stop of this machine's serving
                    try {
                        for await (const page of await stack.store.stream(subscription, {
                            signal,
                        })) {
                            isStreamed = true;
                            yield page;
                        }
                    } catch (error) {
                        // fail on anything but a move or a stop of this machine's serving
                        const isMoved = CallError.data(error, "MISDIRECTED_REQUEST") !== undefined;
                        if (!isMoved && isServing()) {
                            throw error;
                        }

                        // wait for the next placement after a move with no pages
                        if (isMoved && !isStreamed) {
                            await moved(from, signal);
                            continue;
                        }
                    }

                    // end once pages arrived or this machine still serves
                    if (isStreamed || isServing()) {
                        return;
                    }
                }
            },
            request: async (subscription) => {
                await (await this.connect(stackPackage, spaceId)).store.request({ subscription });
            },
        };
    }

    /** Read a space's placement in the directory as JSON, to see it change. */
    async #placed(spaceId: Identifier<"space">): Promise<string | undefined> {
        const row = await this.directory.snapshot
            .read(accountObject.placement.table)
            .findFirst({ where: { spaceId } });

        return row === undefined ? undefined : JSON.stringify(row);
    }

    /** Verify the universe's or a space's token for a package, its binding left to the caller. */
    async verify(
        request: Request,
        audience: PackageId,
        partition?: PartitionId,
    ): Promise<Authentication> {
        // verify the token toward the package
        const { issuer } = await this.universe();
        const verified = await Identity.verify(
            this.directory.snapshot,
            Bearer.require(request.headers),
            {
                audience,
                universe: issuer,
                now: this.clock.now(),
                environment: this.environment,
            },
        );

        // place another space's caller inside its account's chain, as the space records it
        const isSpaceSigned = verified.credential.kind === SPACE_KEY;
        const space = schema.identifier("space").safeParse(partition);

        return isSpaceSigned && space.success
            ? (await this.space(space.data)).contain(verified)
            : verified;
    }

    /** Refuse a peer machine's caller whose key the directory replica no longer accepts. */
    async requireStanding(authentication: Authentication): Promise<void> {
        // require a peer machine as the subject
        const { subject } = authentication.caller;
        if (!principal.machine.is(subject)) {
            throw MachineError.of(
                "TOKEN_WITHOUT_MACHINE_UNAUTHENTICATED",
                "authentication required: a machine key's token has no machine",
            );
        }

        // refuse a key the directory replica no longer lets authenticate: revoked, suspended or expired
        if ((await this.#enrollment()) === undefined) {
            throw MachineError.unenrolled("verifying peers");
        }
        await Key.requireStanding(
            this.directory.snapshot,
            authentication.credential,
            this.clock.now(),
        );
    }

    // spaces

    /** Start a space once while serving, trying again after a failed start. */
    space(space: Identifier<"space">): Promise<Space> {
        // refuse unless serving, as no stop would close the space
        const refused = this.health.refusal();
        if (refused !== undefined) {
            return Promise.reject(refused);
        }

        // start it once
        let started = this.#started.get(space);
        if (started === undefined) {
            started = Space.start(this, space);
            started.catch(() => this.#started.delete(space));
            this.#started.set(space, started);
        }

        return started;
    }

    /** Locate a space by its identifier or address where its machine answers. */
    async locate(space: Identifier<"space"> | string): Promise<accountObject.PlacementEndpoint> {
        // find the space by its identifier, else by its address in the directory replica
        const parsed = schema.identifier("space").safeParse(space);
        const found = parsed.success
            ? { id: parsed.data }
            : await this.directory.find(accountObject.space, space);
        if (found === undefined) {
            throw AccountError.of("ADDRESS_NOT_FOUND", `address '${space}' not found`);
        }
        const spaceId = schema.identifier("space").parse(found.id);

        // answer this machine's own space at its origin, named by a relay or not, another where it answers
        const placed = await this.directory.placement(spaceId);

        return placed?.placement.machine === this.id
            ? { ...placed, url: this.origin }
            : this.directory.locate(spaceId, this.clock.now());
    }

    /** Connect to a package of the machine serving a space, calling it as this machine. */
    async connect<Handle extends Package>(
        handle: Handle,
        spaceId: Identifier<"space">,
    ): Promise<ClientOf<Handle>> {
        return createClient<Handle["router"]>(
            handle,
            await this.options(handle, spaceId),
            this.environment,
        );
    }

    /** Read the client options of a package of the machine serving a space, as this machine. */
    async options(
        handle: Pick<Package, "id">,
        spaceId: Identifier<"space">,
    ): Promise<{ readonly url: string; readonly fetch: (request: Request) => Promise<Response> }> {
        // call the space at another machine's endpoint with this machine's token bound to the space
        const located = await this.locate(spaceId);
        if (located.placement.machine !== this.id) {
            return {
                url: `${SpaceMount.url(located.url, spaceId)}${PackageMount.path(handle.id)}`,
                fetch: this.fetch(handle.id, { partition: spaceId }),
            };
        }

        // call this machine's own space as itself, through itself
        const authentication = this.#self();

        return {
            url: `${SpaceMount.url(OWN_MACHINE, spaceId)}${PackageMount.path(handle.id)}`,
            fetch: (request) => this.serve(request, authentication),
        };
    }

    /** Serve one of the spaces' packages below its mount, in the space each request names. */
    package(handle: Package): Pick<Server, "fetch"> | undefined {
        const { status } = this.health;

        return status === "up" || status === "down"
            ? { fetch: (request) => this.serve(Machine.#mounted(handle, request)) }
            : undefined;
    }

    /** Serve a request at the machine's own hosts, in the space it names. */
    async serve(request: Request, authentication?: Authentication): Promise<Response> {
        return this.#refusal() ?? this.#serve(await this.#route(request), authentication);
    }

    /** Serve a request to a name the machine takes: an installation origin's or a route's. */
    async serveName(request: Request): Promise<Response> {
        // refuse before the machine enrolls and while it is paused
        const refused = this.#refusal();
        if (refused !== undefined) {
            return refused;
        }

        // find the space of the name's installation origin, S3 origin or route
        const claims = ClaimLookup.of(this.directory.snapshot.read(claimTable));
        const routed = await SpaceRequest.named(request, {
            domain: DOMAINS.space,
            ...SpaceRequest.served(this.directory, this.id),
            route: (host, path) => stackObject.Route.locate(claims, host, path),
        });

        return this.#serve(routed);
    }

    /** Report whether a request is for a space started here with the package below its mount. */
    async isSpaceRequest(request: Request): Promise<boolean> {
        // find a started space the request names, none before the machine serves
        const routed = this.health.status === "up" ? await this.#route(request) : undefined;
        const started = routed === undefined ? undefined : this.#started.get(routed.space);
        const mounted =
            routed === undefined
                ? undefined
                : PackageMount.parse(new URL(routed.request.url).pathname);
        if (started === undefined || mounted === undefined) {
            return false;
        }

        // require the space to serve the mounted package
        const { instance } = await started;

        return instance.packages.some((handle) => handle.id === mounted.packageId);
    }

    /** Build the controller serving the spaces while this machine's record states an enrollment. */
    controller(): Controller<typeof computer.table> {
        return {
            name: "machine",
            table: computer.table,
            pending: { machineId: this.id },
            follow: (_rows, following) => this.follow(following),
        };
    }

    /** Update the machine's health from its directory copy: up while enrolled and enabled. */
    async updateHealth(): Promise<void> {
        // read the enrollment and the status the universe declares
        const enrollment = await this.#enrollment();
        const { status } = await this.#replicated();

        // wait for both, serve while enabled, and refuse with the status otherwise
        if (enrollment === undefined) {
            this.health.set("starting", "it has not enrolled yet");
        } else if (status === undefined) {
            this.health.set("starting", "its status has not replicated from the universe yet");
        } else if (status === "enabled") {
            this.health.set("up", "its account enables it");
        } else {
            this.health.set("down", `its account set its status to ${status}`);
        }
    }

    /** Serve the spaces while the account enables the machine, until it is revoked or stops. */
    async follow(following: Following): Promise<void> {
        // wait for the machine's record to state an enrollment
        const { signal, synced } = following;
        this.health.set("starting", "it reads its enrollment");
        while ((await this.#enrollment()) === undefined) {
            this.health.set("starting", "it has not enrolled yet");
            synced();
            await Promise.race([following.changed(), waitAborted(signal)]);
            if (signal.aborted) {
                return;
            }
        }

        // serve the machine's own partition, which follows its replicas, while enrolled
        this.health.set("starting", "it opens its own partition");
        const partition = await this.#partition?.();

        // announce what this machine runs whether it serves or pauses
        let tunnel: AsyncDisposable | undefined;
        const announced = this.#announceServing();
        try {
            // follow the replicas of this machine's status and the placements here until stopped
            let replicated = "";
            const isChanged = async () => canonicalize(await this.#replicated()) !== replicated;
            while (await this.database.log.waitUntil(isChanged, signal)) {
                // read the replicas
                const current = await this.#replicated();
                replicated = canonicalize(current);

                // open the tunnel peers reach a personal machine through before its spaces start, while enrolled
                if (current.status !== undefined && (await this.isEnrolled())) {
                    tunnel ??= await this.#relay();
                }

                // serve while the account enables the machine
                if (current.status === "enabled") {
                    await this.#serveSpaces(current.placements);
                }
                // pause while it drains or disables the machine
                else if (current.status !== undefined) {
                    await this.#pause(current.status);
                }
                // wait for the status
                else {
                    this.health.set(
                        "starting",
                        "its status has not replicated from the universe yet",
                    );
                }

                // report caught up once this machine acted on its first replicas
                synced();
            }
        }
        // stop the spaces however serving ended, then the machine's partition
        finally {
            try {
                await this.#stopServing(tunnel, announced);
            } finally {
                await partition?.[Symbol.asyncDispose]();
            }
        }
    }

    /** Find the space a request names at the machine's own hosts. */
    async #route(request: Request): Promise<SpaceRequest | undefined> {
        return SpaceRequest.of(request, {
            domain: this.domain,
            find: async (address) => (await this.directory.find(accountObject.space, address))?.id,
        });
    }

    /** Refuse a request with the machine's health unless it serves. */
    #refusal(): Response | undefined {
        const refused = this.health.refusal();

        return refused === undefined ? undefined : refusal(refused, this.environment.telemetry);
    }

    /** Serve a request in the space the machine found for it. */
    async #serve(
        routed: SpaceRequest | undefined,
        authentication?: Authentication,
    ): Promise<Response> {
        // answer 404 to a request naming no space
        if (routed === undefined) {
            return new Response(null, { status: 404 });
        }

        // start the space and hand it the request, refusing a space that fails to start
        try {
            const space = await this.space(routed.space);

            return await space.fetch(routed.request, authentication);
        } catch (error) {
            return refusal(error, this.environment.telemetry);
        }
    }

    /** Address a call below a package's path, as it arrives. */
    static #mounted(handle: Package, request: Request): Request {
        const url = new URL(request.url);
        url.pathname = `${PackageMount.path(handle.id)}${url.pathname}`;

        return copyRequest(request, {}, url.href);
    }

    /** Authenticate this machine calling its own spaces, for one call's latency. */
    #self(): Authentication {
        const now = this.clock.now();

        return {
            caller: Caller.of(principal.machine.reference(Scope.universe.id, this.id)),
            credential: {
                kind: "machine",
                id: this.id,
                audience: stackPackage.id,
                verifiedAt: now,
                expiresAt: now + MACHINE_CALL_MILLISECONDS,
            },
        };
    }

    /** Serve the spaces stored and placed here. */
    async #serveSpaces(rows: readonly Select<typeof placement.table>[]): Promise<void> {
        // start serving the spaces this machine stores
        const stored: Identifier<"space">[] = [];
        if (this.health.status !== "up") {
            this.health.set("up", "its account enables it");
            stored.push(...(await this.volumes.list()));

            // let the cache collect the builds of spaces this machine no longer stores
            await this.#prune(stored);
        }

        // start each stored space and each one placed or moving here, at once, reporting each failed start
        const spaces = new Set([...stored, ...rows.map((row) => row.spaceId)]);
        await Promise.all(
            [...spaces].map((space) =>
                this.space(space).catch((error: unknown) => this.report(error)),
            ),
        );

        // restart each started space whose role here changed
        const started = [...this.#started.values()];
        await Promise.all(
            started.map((starting) =>
                starting
                    .then((space) => space.restartIfRoleChanged())
                    .catch((error: unknown) => this.report(error)),
            ),
        );
    }

    /** Pause serving: refuse requests and close every space. */
    async #pause(status: Select<typeof accountObject.machine.table>["status"]): Promise<void> {
        // refuse requests until the next serve
        this.health.set("down", `its account set its status to ${status}`);

        // close every space
        await this.#closeSpaces();
    }

    /** Stop serving: close every space, even when one fails, and close the tunnel. */
    async #stopServing(
        tunnel: AsyncDisposable | undefined,
        announced: Promise<void>,
    ): Promise<void> {
        // refuse requests until the next serve
        this.health.set("draining", "it stops serving");

        // close every space, then the tunnel, and settle the announcement, idle again
        try {
            await this.#closeSpaces();
        } finally {
            await tunnel?.[Symbol.asyncDispose]();
            await announced;
            this.health.set("stopped", "it stopped serving");
        }
    }

    /** Close every space at once, refusing with each that failed to close. */
    async #closeSpaces(): Promise<void> {
        // close every space, even when one fails
        const spaces = [...this.#started.values()];
        this.#started.clear();
        const started = await Promise.allSettled(spaces);

        // close each started space, a failed start having answered its caller and opened nothing
        const closed = await Promise.allSettled(
            started.flatMap((result) =>
                result.status === "fulfilled" ? [result.value.close()] : [],
            ),
        );

        // refuse with every space that failed to close
        const failures = closed.flatMap((result): unknown[] =>
            result.status === "rejected" ? [result.reason] : [],
        );
        if (failures.length > 0) {
            throw new AggregateError(failures, "some spaces did not close");
        }
    }

    /** Announce what this machine runs once it serves, reporting a failed announcement. */
    async #announceServing(): Promise<void> {
        await this.announce().catch((error: unknown) => this.report(error));
    }

    /** Read the replicas of this machine's status, absent before it arrives, and its placements. */
    async #replicated() {
        // read the status
        const [row] = await this.database
            .select({ status: accountObject.machine.table.status })
            .from(accountObject.machine.table)
            .where(eq(accountObject.machine.table.id, this.id));

        // read the placements of the spaces the machine serves or receives
        const placements = await this.database
            .select()
            .from(placement.table)
            .where(
                or(eq(placement.table.machineId, this.id), eq(placement.table.targetId, this.id)),
            )
            .orderBy(placement.table.spaceId);

        return { status: row?.status, placements };
    }

    /** The origin the machine answers at: its endpoint, else its name at the relay once renewed. */
    get origin(): string {
        return this.#origin;
    }

    /** Keep a personal machine's tunnel open at its relay, renewed under each name it gives. */
    async #relay(): Promise<AsyncDisposable> {
        // open none for a cloud machine, which answers at its declared origin, nor for a personal one without a relay
        const enrollment = await this.enrolled("opening its tunnel");
        const relayed = this.#tunnel;
        if (enrollment.kind === "cloud" || relayed === undefined) {
            return { [Symbol.asyncDispose]: async () => {} };
        }

        // renew the tunnel at the configured relay, else the account residency's, at each renewal's name
        const issuer = Machine.#issuer(enrollment);
        const url =
            relayed.url ?? `${await this.#residencyRelay(enrollment.accountId)}${TUNNEL_PATH}`;
        const tunnel = TunnelClient.open({
            url,
            machineId: this.id,
            fetch: relayed.fetch,
            environment: this.environment,
            token: (audience) =>
                this.identity.at(issuer, (request) => this.send(request)).token(audience, {}),
            renewed: ({ name }) => {
                this.#origin = `https://${name}`;
            },
            changed: relayed.changed,
        });

        return { [Symbol.asyncDispose]: () => tunnel.close() };
    }

    /** Read the relay origin of the residency an account stores its data in. */
    async #residencyRelay(account: Identifier<"account">): Promise<string> {
        return (await this.accounts()).residency.relay({ account });
    }

    /** Wrap the machine's network to call a package as the machine. */
    #fetchAs(
        enrollment: MachineEnrollment,
        audience: PackageId,
        binding: Pick<TokenBinding, "partition"> = {},
    ): Fetch {
        const send: Fetch = (request) => this.send(request);
        const session = new Session(this.identity.at(Machine.#issuer(enrollment), send), {
            universe: enrollment.universe,
            send,
            bookmark: new Bookmark(),
            environment: this.environment,
        });

        return session.fetch(audience, binding);
    }

    /** Read the issuer of this machine's tokens: its universe's. */
    static #issuer(enrollment: MachineEnrollment): string {
        return UniverseOrigin.issuer(enrollment.universe);
    }

    /** Read this machine's valid enrollment, absent before it enrolls and after a revoke. */
    async #enrollment(view?: Pick<Snapshot, "read">): Promise<MachineEnrollment | undefined> {
        const enrollment = await this.#readEnrollment(view);

        return enrollment?.status === "enrolled" ? enrollment : undefined;
    }
}
