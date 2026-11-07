import { Forge, Registry } from "@destack/registry/client";
import { type Authentication, principal } from "@destack/access";
import * as accountObject from "@destack/account/object";
import type { CatalogueBucketHost } from "@destack/bucket/provider";
import { S3Credentials, type S3Server } from "@destack/bucket/s3";
import connectionPackage, { CONNECTION_INSTALLATION } from "@destack/connection/client";
import type { ConnectionLease } from "@destack/connection/object";
import {
    BlobStore,
    type DatabaseConnection,
    type Controller,
    Snapshot,
    type Table,
} from "@destack/db";
import { BuildStore } from "@destack/build/store";
import * as stackObject from "@destack/stack/object";
import type { Volume } from "./storage.ts";
import {
    ClaimLookup,
    claimTable,
    HookSandbox,
    installation,
    PackageDescription,
    type CursorPage,
} from "@destack/object";
import {
    type ImplementationInput,
    type ObjectStore,
    type PackageImplementation,
} from "@destack/object/server";
import { PackageId } from "@destack/package";
import { type Lease, ResourceHandle } from "@destack/resource";
import { ResourceContext } from "@destack/resource/context";
import { Health, type PackageHandler } from "@destack/object/server";
import { defineWorkload, type WorkloadContext, WorkloadInstance } from "@destack/object/workload";
import { type Identifier, present } from "@destack/schema";
import { Database, Rollout, rollout, type InstallationRollout } from "@destack/stack/object";
import stackPackage, { STACK_INSTALLATION } from "@destack/stack/package";
import {
    SpaceBuildStore,
    type SpaceOptions,
    serveS3,
    SpaceStore,
    type SpaceStart,
    SpaceHost,
    type SpaceMachine,
    type SpaceRole,
} from "@destack/stack/server";
import { REQUEST_LIFETIME_MILLISECONDS, Scope, type Subject } from "@destack/sync";
import type { OtlpSignal } from "@destack/telemetry/otlp";
import type { InstanceSpec } from "@destack/stack/runtime";
import { Gateway } from "../gateway/index.ts";
import { Traffic } from "../gateway/traffic.ts";
import { DRAIN_TIMEOUT_MILLISECONDS, type Machine } from "../machine/index.ts";
import { SpaceMeter, Reading } from "../meter/index.ts";
import { MachineError } from "../error/index.ts";

/** The adapter of OTLP/HTTP, which the installations taking a space's telemetry serve. */
const OTLP_ADAPTER = "otlp";

/** What one start of a space's workload serves with: its keys, its store and its buckets. */
export interface SpaceRun {
    /** The installations taking the space's telemetry: their package and the adapter's path. */
    receivers: readonly { readonly packageId: PackageId; readonly path: string }[];
    /** The host of the space's buckets. */
    readonly buckets: CatalogueBucketHost;
    /** The S3 API over the space's buckets, and the host name it answers at. */
    readonly s3: { readonly server: Pick<S3Server, "fetch" | "serve">; readonly hostname: string };
    /** The space's store, once it serves. */
    readonly store: PromiseWithResolvers<ObjectStore>;
    /** The gateway routing the installations' calls, once the store serves. */
    readonly gateway: PromiseWithResolvers<Gateway>;
    /** The space's role on the machine this start serves. */
    readonly role: SpaceRole;
    /** The revision this start serves each installation at. */
    revisions: ReadonlyMap<Identifier<"installation">, Identifier<"installation-revision">>;
    /** Serve a changed installed set in this start, keeping the unchanged installations served. */
    readonly apply: (installed: Awaited<ReturnType<typeof Database.installed>>) => Promise<void>;
    /** Start the workload again, once the space's role here changed. */
    readonly restart: () => void;
}

/** A space a machine serves: its database, events, traffic, workload and outbound calls. */
export class Space {
    /** The installations, rollouts and main, which decide what a primary serves. */
    readonly watches: readonly Table[] = [
        installation.table,
        rollout.table,
        stackObject.database.table,
    ];
    /** The machine serving the space. */
    readonly machine: Machine;
    /** The space's volume on its machine: its database, events, blobs, buckets and sockets. */
    readonly volume: Volume;
    /** The space's identity, its link to the universe and its placement on the machine. */
    readonly host: SpaceHost;
    /** The space's blobs, stored while its rows reference them. */
    readonly blobs: BlobStore;
    /** The space's builds, read through to the machine's cache. */
    readonly builds: SpaceBuildStore;
    /** The space's traffic, which its gateway counts and its meters take. */
    readonly traffic = new Traffic();
    /** Whether the space serves every installation at its revision, and why not. */
    readonly health: Health;
    /** The space's running workload, once started. */
    #instance: WorkloadInstance | undefined;
    /** What the workload's current start serves with. */
    #serving: SpaceRun | undefined;
    /** The space's store of the current start, which applies its installed set's changes. */
    #store: SpaceStore | undefined;

    /** Take a space its machine opened. */
    private constructor(fields: Pick<Space, "machine" | "volume" | "host" | "blobs" | "builds">) {
        // set the space and what its machine opened for it
        this.machine = fields.machine;
        this.volume = fields.volume;
        this.host = fields.host;
        this.blobs = fields.blobs;
        this.builds = fields.builds;
        this.health = new Health(`space ${fields.host.id}`);
    }

    /** The space. */
    get id(): Identifier<"space"> {
        return this.host.id;
    }

    /** The space's database. */
    get database(): DatabaseConnection {
        return this.volume.database;
    }

    /** Open a space on its machine and start its workload. */
    static async start(machine: Machine, id: Identifier<"space">): Promise<Space> {
        // open the space's volume, its blobs and the builds in them, read through to the machine's cache
        const volume = await machine.spaces.open(id);
        try {
            const { environment } = machine;
            const { database } = volume;
            const blobs = new BlobStore({ storage: volume.blobs, database, environment });
            const builds = new SpaceBuildStore(
                new BuildStore({ blobs, environment }),
                machine.cache,
            );

            // lend the machine's directory, universe and the space's builds to its host
            const lent: SpaceMachine = {
                id: machine.id,
                environment: machine.environment,
                keyring: machine.keyring,
                get origin() {
                    return machine.origin;
                },
                directory: machine.directory,
                identities: await machine.identities(),
                universe: await machine.universe(),
                fetch: (audience) => machine.fetch(audience),
                send: (request) => machine.send(request),
                builds,
                linked: machine.linked,
            };
            const space = new Space({
                machine,
                volume,
                host: new SpaceHost(lent, id, database),
                blobs,
                builds,
            });

            // start the machine workload serving the space
            space.#instance = await WorkloadInstance.start(spaceWorkload, {
                environment: machine.environment,
                resources: new ResourceContext().bind(machineSpace, space),
                callKey: () => space.host.callKey(),
                ...(machine.alarm === undefined ? {} : { alarm: machine.alarm }),
                report: (error) => machine.report(error),
                serverOptions: (handle) => ({
                    environment: machine.environment,
                    authenticate: (request) => machine.authenticate(request, handle.id, id),
                    authorizeMachine: (call) => machine.authorizeSpace(call),
                    drainTimeout: DRAIN_TIMEOUT_MILLISECONDS,
                    instance: id,
                }),
            });

            return space;
        }
        // close the space's volume when its start fails, as a close does
        catch (error) {
            await volume.close();
            throw error;
        }
    }

    /** The space's running workload. */
    get instance(): WorkloadInstance {
        return present(this.#instance, "the space's workload");
    }

    /** The host of the space's buckets, which the workload's current start opened. */
    get buckets(): CatalogueBucketHost {
        return this.#current().buckets;
    }

    /** Hand a request to one of the space's packages as a verified caller. */
    async call(
        packageId: PackageId,
        request: Request,
        authentication?: Authentication,
    ): Promise<Response> {
        const served = this.instance.packages.find((handle) => handle.id === packageId);
        if (served === undefined) {
            throw MachineError.of("PACKAGE_NOT_SERVED", "this space serves no such package");
        }

        return this.instance.fetch(served, request, authentication);
    }

    /** Serve a request to the space through its gateway. */
    async fetch(request: Request, authentication?: Authentication): Promise<Response> {
        const gateway = await this.#current().gateway.promise;

        return gateway.serve(request, authentication);
    }

    /** Stop the space's workload, then release what the machine opened for it. */
    async close(): Promise<void> {
        try {
            await this.#instance?.close();
        } finally {
            await this.volume.close();
            this.health.set("stopped", "the space closed");
        }
    }

    /** Serve one start of the space's workload: its store, installations, gateway and meters. */
    async serve(
        context: WorkloadContext,
    ): Promise<{ readonly packages: readonly PackageHandler[] }> {
        // open the space's resources and store its keys under the machine's keyring
        const { machine, database, volume } = this;
        const { sockets, providers } = volume;
        const { keystore } = this.host;
        await this.host.open();

        // serve the space's buckets over S3, presigned with credentials its root secret derives
        const buckets = volume.buckets(async (scope) => {
            const root = scope === this.id ? await keystore.deriver(database, scope) : undefined;

            return root === undefined ? undefined : S3Credentials.derive(root, scope);
        });

        // serve a primary's applied installations, applying their changes in this start
        const hosted = await this.host.options();
        const { role } = hosted;
        const installed =
            role === "primary"
                ? await Database.installed(
                      Snapshot.live(database),
                      this.id,
                      (packageId, manifest) =>
                          this.builds.pull(machine.releases, packageId, manifest),
                  )
                : [];
        this.#serving = {
            receivers: Space.#receivers(installed),
            buckets,
            s3: {
                server: serveS3(buckets, database, this.machine.environment),
                hostname: buckets.endpoint.hostname,
            },
            store: Promise.withResolvers(),
            gateway: Promise.withResolvers(),
            role,
            revisions: Space.#revisions(installed),
            apply: (changed) => this.#apply(changed, start, context),
            restart: () => {
                context.reload().catch((error: unknown) => context.report(error));
            },
        };

        // load the installations' servers through the space's store and gateway, once they serve
        const start: SpaceStart = {
            call: (packageId, request, authentication) =>
                this.call(packageId, request, authentication),
            store: this.#serving.store.promise,
            providers,
            connectors: volume.connectors,
            buckets,
            egress: async (caller, request) =>
                (await this.#current().gateway.promise).send(caller, request),
            server: (manifest) => machine.server(manifest),
            report: (error) => context.report(error),
            sandbox: (row, server, declarations) =>
                new HookSandbox({
                    server,
                    declarations,
                    send: async (request) =>
                        (await this.#current().gateway.promise).hook(row.installationId, request),
                }),
        };

        // fail the store and gateway waits of a start that fails, which its calls await
        const serving = this.#serving;
        try {
            const { servers, sandboxes } = await this.host.servers(
                installed,
                start,
                context.signal,
            );

            // serve the space with the host's own jobs: what it serves, and a rated primary's metering
            const { skus } = machine;
            const isMetered = skus !== undefined && role !== "standby";
            const meters: NonNullable<ImplementationInput["controllers"]> = isMetered
                ? [
                      (store) =>
                          new SpaceMeter(store, {
                              space: this.id,
                              measure: (from, to) =>
                                  volume.measure(buckets, from, to, this.traffic),
                              skus,
                              payer: async () =>
                                  accountObject.Account.payer(
                                      this.machine.directory.snapshot,
                                      await this.host.account(),
                                  ),
                              traffic: (signal) => this.traffic.next(signal),
                          }).controller(),
                  ]
                : [];
            const controllers = [() => this.controller(), ...meters];
            const cursors: NonNullable<ImplementationInput["cursors"]> = isMetered
                ? [
                      // append each payer's uses of a page an hour at a time, kept while a billing call may retry
                      stackObject.reading.cursor("metering", (page) => this.#append(page), {
                          hold: { milliseconds: REQUEST_LIFETIME_MILLISECONDS },
                          batch: { hours: 1 },
                      }),
                  ]
                : [];
            const served = this.#serveSpace(
                hosted,
                { controllers, cursors },
                { providers, sockets },
                servers,
                sandboxes,
            );
            this.#store = served;
            this.#serving.store.resolve(served.store);

            // route the installations' calls through the space's gateway
            this.#serving.gateway.resolve(new Gateway(this, this.#serving, served.store));

            return { packages: served.packages };
        }
        // reject the waits, each handled here as well as by its callers
        catch (error) {
            serving.store.reject(error);
            serving.gateway.reject(error);
            serving.store.promise.catch(() => undefined);
            serving.gateway.promise.catch(() => undefined);
            throw error;
        }
    }

    /** Build the controller serving the space's installed set at each start and change of it. */
    controller(): Controller<typeof Scope.table> {
        return {
            name: "space",
            table: Scope.table,
            pending: { scope: this.id },
            // serve again as the installed set or its installations' and resources' status moves
            watches: this.watches.map((table) => ({
                table,
                isObserving: true,
                keys: () => [{ scope: this.id }],
            })),
            reconcile: () => this.serveInstalled(),
        };
    }

    /** Serve a primary's changed installed set in place, then record the revisions it serves. */
    async serveInstalled(): Promise<void> {
        // apply a primary's changed installed set, then record its served revisions
        const serving = present(this.#serving, `the serving of ${this.id}`);
        if (serving.role === "primary") {
            const installed = await Database.installed(
                Snapshot.live(this.database),
                this.id,
                (packageId, manifest) =>
                    this.builds.pull(this.machine.releases, packageId, manifest),
            );
            if (Space.#key(Space.#revisions(installed)) !== Space.#key(serving.revisions)) {
                await serving.apply(installed);
            }
            await Rollout.serve(await serving.store.promise, this.id, serving.revisions);
        }

        // report whether every installation serves its revision
        const [status, reason] = await this.#readiness(serving.role);
        this.health.set(status, reason);
    }

    /** Read whether the space serves each installation at its revision, else the lagging one. */
    async #readiness(role: SpaceRole): Promise<["starting" | "up", string]> {
        // serve a follower as its primary does
        if (role !== "primary") {
            return ["up", `it follows its primary as ${role}`];
        }

        // read the rollouts with their installations' conditions
        const snapshot = Snapshot.live(this.database);
        const rows = await Rollout.read(
            snapshot,
            await snapshot.read(rollout.table).findMany({ where: { scope: this.id } }),
        );
        const lagging = rows.find(
            (row) =>
                row.revisionId === null ||
                row.appliedRevisionId !== row.revisionId ||
                row.servedRevisionId !== row.revisionId,
        );

        // wait for a first installation, then for each to serve its revision, naming its open condition
        if (rows.length === 0) {
            return ["starting", "no installation is applied yet"];
        } else if (lagging !== undefined) {
            const installed = lagging.installation.name;
            const open = Object.entries(lagging.installation.conditions).find(
                ([, condition]) => condition.status !== "true",
            );
            const condition =
                open === undefined ? "" : `: ${open[0]} ${open[1].reason}, ${open[1].message}`;

            return [
                "starting",
                `installation ${installed} does not serve its revision yet${condition}`,
            ];
        } else {
            return ["up", "every installation serves its revision"];
        }
    }

    /** Wait until an installation of the space serves its revision, refusing once a signal ends. */
    async waitServed(name: string, signal: AbortSignal): Promise<void> {
        const isServed = await this.database.log.waitUntil(async () => {
            // read the named installation's rollout with its revisions
            const snapshot = Snapshot.live(this.database);
            const rows = await Rollout.read(
                snapshot,
                await snapshot.read(rollout.table).findMany({ where: { scope: this.id } }),
            );
            const named = rows.find((row) => row.installation.name === name);

            return named !== undefined && Rollout.progress(named) === "served";
        }, signal);
        if (!isServed) {
            const { status, reason } = this.health;
            throw MachineError.of(
                "INSTALLATION_NOT_SERVED",
                `installation ${name} of space ${this.id} serves nothing yet, the space is ${status}: ${reason}`,
            );
        }
    }

    /** Restart the workload once the directory places the space here in another role. */
    async restartIfRoleChanged(): Promise<void> {
        const serving = this.#serving;
        if (serving !== undefined && (await this.host.role()) !== serving.role) {
            serving.restart();
        }
    }

    /** Serve a changed installed set in this start: load the changed servers, then apply them. */
    async #apply(
        installed: Awaited<ReturnType<typeof Database.installed>>,
        start: SpaceStart,
        context: WorkloadContext,
    ): Promise<void> {
        // load the changed installations' servers, apply them to the store and serve their packages
        const serving = present(this.#serving, `the serving of ${this.id}`);
        const { servers, sandboxes } = await this.host.servers(installed, start, context.signal);
        const packages = present(this.#store, `the store of ${this.id}`).apply(servers, sandboxes);
        context.serve(packages);

        // record what the start serves now
        serving.revisions = Space.#revisions(installed);
        serving.receivers = Space.#receivers(installed);
    }

    /** Read the installations taking the space's telemetry: their package and adapter's path. */
    static #receivers(
        installed: Awaited<ReturnType<typeof Database.installed>>,
    ): SpaceRun["receivers"] {
        return installed.flatMap((entry) => {
            const adapter = PackageDescription.find(entry.declarations)?.adapters[OTLP_ADAPTER];
            const packageId = PackageId.parse(entry.rollout.installation.packageId);

            return adapter === undefined ? [] : [{ packageId, path: adapter.path }];
        });
    }

    /** Read the revision each installed entry is served at. */
    static #revisions(
        installed: readonly {
            readonly rollout: InstallationRollout;
            readonly revisionId: Identifier<"installation-revision">;
        }[],
    ): ReadonlyMap<Identifier<"installation">, Identifier<"installation-revision">> {
        return new Map(installed.map((entry) => [entry.rollout.installationId, entry.revisionId]));
    }

    /** Key served revisions as one sorted string. */
    static #key(
        revisions: ReadonlyMap<Identifier<"installation">, Identifier<"installation-revision">>,
    ): string {
        return [...revisions]
            .map(([installationId, revisionId]) => `${installationId}@${revisionId}`)
            .toSorted()
            .join(",");
    }

    /** Lease a repository of the space's account to read, at the forge in its residency's home. */
    async lease(repositoryId: Identifier<"repository">): Promise<Lease> {
        // call the forge of the space's account as the space, a peer call to its home's stack
        const self = principal.space.reference(Scope.universe.id, this.id);
        const forges = Registry.of(
            this.machine.directory,
            (account) => this.#residencyHome(account),
            () => this.host.source(self).fetchSigned(stackPackage.id),
            this.machine.environment,
            this.machine.registry,
        );

        return forges.lease(await this.host.account(), repositoryId);
    }

    /** Forward an instance's telemetry export, stamped with its installation and instance. */
    async telemetry(spec: InstanceSpec, signal: OtlpSignal, body: unknown): Promise<void> {
        const gateway = await this.#current().gateway.promise;
        await gateway.telemetry(spec, signal, body);
    }

    /** Forward a telemetry export to each installation taking it, as the space itself. */
    async observe(signal: OtlpSignal, body: unknown): Promise<void> {
        const self = principal.space.reference(Scope.universe.id, this.id);
        for (const receiver of this.#current().receivers) {
            // post the export below the receiver's adapter, refusing a failed answer
            const url = new URL(`${receiver.path}/${signal}`, this.machine.origin);
            const request = new Request(url.href, {
                method: "POST",
                headers: { "content-type": "application/json" },
                body: JSON.stringify(body),
            });
            const authentication = this.host
                .source(self)
                .authentication(receiver.packageId, this.id);
            const response = await this.call(receiver.packageId, request, authentication);
            const answer = await response.text();
            if (!response.ok) {
                throw MachineError.of(
                    "TELEMETRY_REFUSED",
                    `the telemetry receiver refused a ${signal} export with ${response.status}: ${answer}`,
                );
            }
        }
    }

    /** Borrow a connection's credential and its resource at its account's home, as the consumer. */
    async borrow(
        lent: { readonly scope: string; readonly id: Identifier<"connection"> },
        consumer: Subject,
    ): Promise<ConnectionLease> {
        // call the connections' primary in the home of the scope's account as the consumer
        const { environment } = this.machine;
        const home = await accountObject.Account.homeOf(Snapshot.live(this.database), lent.scope);
        const source = this.host.source(consumer);
        const primary = this.machine.directory.client(
            connectionPackage,
            home,
            CONNECTION_INSTALLATION,
            source.fetchSigned(connectionPackage.id),
            environment,
        );

        return primary.connection.lease({ scope: lent.scope, id: lent.id });
    }

    /** Append each payer's uses of a page of readings in its residency's home, as the machine. */
    async #append(page: CursorPage): Promise<void> {
        const { directory, environment } = this.machine;
        for (const use of await Reading.uses(page, environment.crypto)) {
            // call the stack in the home of the payer's residency
            const home = await this.#residencyHome(use.accountId);
            const fetch = this.machine.fetch(stackPackage.id);
            const stack = directory.client(
                stackPackage,
                home,
                STACK_INSTALLATION,
                fetch,
                environment,
            );
            await stack.usage.append(use);
        }
    }

    /** Read what the current start of the workload serves with, refusing before it serves. */
    #current(): SpaceRun {
        return present(this.#serving, `the workload of ${this.id}`);
    }

    /** Read the home of an account's residency through the universe, as the machine. */
    async #residencyHome(accountId: Identifier<"account">): Promise<Identifier<"space">> {
        return this.host.accounts().residency.locate({ account: accountId });
    }

    /** Serve the space's store with its servers and sandboxes, replicating account rows. */
    #serveSpace(
        hosted: Awaited<ReturnType<SpaceHost["options"]>>,
        jobs: Pick<SpaceOptions, "controllers" | "cursors">,
        opened: Pick<SpaceOptions, "providers" | "sockets">,
        servers: readonly PackageImplementation[],
        sandboxes: readonly HookSandbox[],
    ): SpaceStore {
        // read external object types from builds, following the forge's references and tags when building
        const { machine, volume } = this;
        const { buckets } = this.#current();
        const external = volume.graphs(this.builds);
        const claims = ClaimLookup.of(machine.directory.snapshot.read(claimTable));
        const forge = machine.isBuilding
            ? new Forge(claims, (account) => this.#residencyHome(account), machine.id)
            : null;

        return new SpaceStore({
            ...hosted,
            report: (error) => machine.report(error),
            buildRunner: machine.isBuilding ? machine.id : null,
            handoff: (source, spaceId) => machine.handoff(source, spaceId),
            isEnforcingEntitlements: machine.isEnforcingEntitlements,
            destack: machine.destack,
            isServingHostnames: machine.isServingHostnames,
            runtimes: machine.runtimes,
            registry: machine.releases,
            buckets,
            builds: this.builds,
            blobs: this.blobs,
            events: this.volume.events,
            forge,
            follows: forge === null ? [] : forge.follows(),
            ...jobs,
            external,
            ...opened,
            main: machine.spaces.main,
            servers,
            sandboxes,
        });
    }
}

/** A machine's space, which the process running the machine binds for its workload. */
export const machineSpace = new ResourceHandle<Space>(import.meta.destack.package, "space");

/** The workload of one space a machine serves. */
export const spaceWorkload = defineWorkload({
    name: "machine",
    start: (context) => machineSpace.get(context.resources).serve(context),
});
