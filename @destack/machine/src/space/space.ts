import { Forge, Registry } from "@destack/forge/client";
import { type Authentication, principal } from "@destack/access";
import * as accountObject from "@destack/account/object";
import type { CatalogueBucketHost } from "@destack/bucket/provider";
import { S3Credentials, type S3Server } from "@destack/bucket/s3";
import { connection, ConnectionRequest } from "@destack/connection";
import connectionPackage, { CONNECTION_INSTALLATION } from "@destack/connection/package";
import { BlobStore, Change, type DatabaseConnection, Snapshot, type Table } from "@destack/db";
import { BuildStore } from "@destack/build/store";
import * as stackObject from "@destack/stack/object";
import type { Volume } from "./storage.ts";
import {
    ClaimLookup,
    claimTable,
    HookSandbox,
    installation,
    PackageDescription,
    type ReaderPage,
} from "@destack/object";
import {
    type ImplementationInput,
    type ObjectStore,
    type PackageImplementation,
} from "@destack/object/server";
import { reading } from "@destack/observability";
import { PackageId } from "@destack/package";
import { type Lease, ResourceHandle } from "@destack/resource";
import { ResourceContext } from "@destack/resource/context";
import type { ReconcileController } from "@destack/db";
import type { PackageHandler } from "@destack/object/server";
import { defineWorkload, type WorkloadContext, WorkloadInstance } from "@destack/object/workload";
import { type Identifier, present } from "@destack/schema";
import {
    binding,
    Database,
    Rollout,
    rollout,
    type InstallationRollout,
} from "@destack/stack/object";
import stackPackage from "@destack/stack/package";
import {
    SpaceBuildStore,
    type SpaceOptions,
    serveS3,
    serveSpace,
    SpaceHost,
    type SpaceMachine,
    type SpaceRole,
} from "@destack/stack/server";
import { Scope, type Subject } from "@destack/sync";
import type { OtlpSignal } from "@destack/telemetry/otlp";
import type { InstanceSpec } from "@destack/stack/runtime";
import { Gateway } from "../gateway/index.ts";
import { Traffic } from "../gateway/traffic.ts";
import { DRAIN_TIMEOUT_MILLISECONDS, type Machine } from "../machine/index.ts";
import { MeterController } from "../meter/index.ts";
import { Usage } from "@destack/finance/object";
import financePackage, { FINANCE_INSTALLATION } from "@destack/finance/package";
import { MachineError } from "../error/index.ts";

/** The adapter of OTLP/HTTP, which the installations taking a space's telemetry serve. */
const OTLP_ADAPTER = "otlp";

/** What one start of a space's workload serves with: its keys, its store and its buckets. */
export interface SpaceRun {
    /** The installations taking the space's telemetry: their package and the adapter's path. */
    readonly receivers: readonly { readonly packageId: PackageId; readonly path: string }[];
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
    readonly revisions: ReadonlyMap<
        Identifier<"installation">,
        Identifier<"installation-revision">
    >;
    /** Start the workload again, once what the space serves changed. */
    readonly restart: () => void;
}

/** A space a machine serves: its database, events, traffic, workload and outbound calls. */
export class Space implements ReconcileController {
    /** The space's name in controller reports. */
    readonly name = "space";
    /** The installations, their rollouts, bindings and main, which decide what a primary serves. */
    readonly watches: readonly Table[] = [
        installation.table,
        rollout.table,
        binding.table,
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
    /** The space's running workload, once started. */
    #instance: WorkloadInstance | undefined;
    /** What the workload's current start serves with. */
    #serving: SpaceRun | undefined;

    /** Take a space its machine opened. */
    private constructor(fields: Pick<Space, "machine" | "volume" | "host" | "blobs" | "builds">) {
        // set the space and what its machine opened for it
        this.machine = fields.machine;
        this.volume = fields.volume;
        this.host = fields.host;
        this.blobs = fields.blobs;
        this.builds = fields.builds;
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
                servers: machine.servers,
                bundled: machine.bundled,
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
            throw MachineError.of(
                "PACKAGE_NOT_SERVED",
                `space ${this.id} has no package ${packageId}`,
            );
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

        // serve a primary's applied installations, restarting when they change
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
            receivers: installed.flatMap((entry) => {
                const adapter = PackageDescription.find(entry.declarations)?.adapters[OTLP_ADAPTER];
                const packageId = PackageId.parse(entry.rollout.installation.packageId);

                return adapter === undefined ? [] : [{ packageId, path: adapter.path }];
            }),
            buckets,
            s3: {
                server: serveS3(buckets, database, this.machine.environment),
                hostname: buckets.endpoint.hostname,
            },
            store: Promise.withResolvers(),
            gateway: Promise.withResolvers(),
            role,
            revisions: Space.#revisions(installed),
            restart: () => {
                context.reload().catch((error: unknown) => context.report(error));
            },
        };
        // fail the store and gateway waits of a start that fails, which its calls await
        const serving = this.#serving;
        try {
            const { servers, sandboxes } = await this.host.servers(
                installed,
                {
                    call: (packageId, request, authentication) =>
                        this.call(packageId, request, authentication),
                    store: this.#serving.store.promise,
                    providers,
                    connectors: volume.connectors,
                    buckets,
                    egress: async (caller, request) =>
                        (await this.#current().gateway.promise).send(caller, request),
                    sandbox: (row, server, declarations) =>
                        new HookSandbox({
                            server,
                            declarations,
                            send: async (request) =>
                                (await this.#current().gateway.promise).hook(row.id, request),
                        }),
                },
                context.signal,
            );
            // serve the space with the host's controllers: its lifecycle, and a rated primary's metering
            const { rates } = machine;
            const metering: NonNullable<ImplementationInput["controllers"]> =
                rates === undefined || role === "standby"
                    ? []
                    : [
                          (store) =>
                              new MeterController(store, {
                                  space: this.id,
                                  measure: (from, to) =>
                                      volume.measure(buckets, from, to, this.traffic),
                                  skus: rates.skus,
                                  payer: async () =>
                                      accountObject.Account.payer(
                                          this.machine.directory.snapshot,
                                          await this.host.account(),
                                      ),
                                  traffic: (signal) => this.traffic.next(signal),
                              }),
                          // append each payer's uses of a page at their finance primary
                          reading.consume("metering", (page) => this.#append(page)),
                      ];
            const served = this.#serveSpace(
                hosted,
                [() => this, ...metering],
                { providers, sockets },
                servers,
                sandboxes,
            );
            this.#serving.store.resolve(served.store);

            // route the installations' calls through the space's gateway
            const gateway = new Gateway(
                this,
                this.#serving,
                served.packages.map((each) => each.package),
            );
            this.#serving.gateway.resolve(gateway);

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

    /** Select the space at each change of what it may serve. */
    keys(change: Change): readonly string[] {
        return this.watches.some((table) => Change.of(change, table)) ? [this.id] : [];
    }

    /** List the space. */
    async list(): Promise<readonly string[]> {
        return [this.id];
    }

    /** Restart a primary once its installed set changed, else record what it serves. */
    async reconcile(): Promise<undefined> {
        // restart a primary whose installed set changed, else record its served revisions
        const serving = present(this.#serving, `the serving of ${this.id}`);
        if (serving.role === "primary") {
            const installed = await Database.installed(
                Snapshot.live(this.database),
                this.id,
                (packageId, manifest) =>
                    this.builds.pull(this.machine.releases, packageId, manifest),
            );
            if (Space.#key(Space.#revisions(installed)) !== Space.#key(serving.revisions)) {
                serving.restart();
            }
            // record the revisions it serves otherwise
            else {
                await Rollout.serve(await serving.store.promise, this.id, serving.revisions);
            }
        }

        return undefined;
    }

    /** Restart the workload once the directory places the space here in another role. */
    async restartIfRoleChanged(): Promise<void> {
        const serving = this.#serving;
        if (serving !== undefined && (await this.host.role()) !== serving.role) {
            serving.restart();
        }
    }

    /** Read the revision each installed entry is served at. */
    static #revisions(
        installed: readonly {
            readonly rollout: InstallationRollout;
            readonly revisionId: Identifier<"installation-revision">;
        }[],
    ): ReadonlyMap<Identifier<"installation">, Identifier<"installation-revision">> {
        return new Map(installed.map((entry) => [entry.rollout.id, entry.revisionId]));
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
                    `${receiver.packageId} refused a ${signal} export with ${response.status}: ${answer}`,
                );
            }
        }
    }

    /** Borrow a connection's credential at its account's home, signed as the consumer. */
    async borrow(
        lent: { readonly scope: string; readonly id: Identifier<"connection"> },
        consumer: Subject,
        request: Request,
        now: number,
    ): Promise<ConnectionRequest> {
        // call the connections' primary in the home of the scope's account as the consumer
        const reference = connection.reference(lent.scope, lent.id);
        const { environment } = this.machine;
        const home = await accountObject.Account.homeOf(
            Snapshot.live(this.database),
            reference.scope,
        );
        const source = this.host.source(consumer);
        const primary = this.machine.directory.client(
            connectionPackage,
            home,
            CONNECTION_INSTALLATION,
            source.fetchSigned(connectionPackage.id),
            environment,
        );

        return ConnectionRequest.lease(primary, reference, request, now, environment.crypto);
    }

    /** Append each payer's uses of a page of readings at their finance primary, as the machine. */
    async #append(page: ReaderPage): Promise<void> {
        const { directory, environment } = this.machine;
        for (const use of await Usage.of(page, environment.crypto)) {
            // call the finance primary in the home of the payer's residency
            const home = await this.#residencyHome(use.accountId);
            const fetch = this.machine.fetch(financePackage.id);
            const finance = directory.client(
                financePackage,
                home,
                FINANCE_INSTALLATION,
                fetch,
                environment,
            );
            await finance.usage.append(use);
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
        controllers: NonNullable<ImplementationInput["controllers"]>,
        opened: Pick<SpaceOptions, "providers" | "sockets">,
        servers: readonly PackageImplementation[],
        sandboxes: readonly HookSandbox[],
    ): ReturnType<typeof serveSpace> {
        // read external object types from builds, following the forge's references and tags when building
        const { machine, volume } = this;
        const { buckets } = this.#current();
        const external = volume.graphs(this.builds);
        const claims = ClaimLookup.of(machine.directory.snapshot.read(claimTable));
        const forge = machine.isBuilding
            ? new Forge(claims, (account) => this.#residencyHome(account))
            : null;

        return serveSpace({
            ...hosted,
            buildRunner: machine.isBuilding ? machine.id : null,
            handoff: (source, spaceId) => machine.handoff(source, spaceId),
            isEnforcingEntitlements: machine.isEnforcingEntitlements,
            isServingHostnames: machine.isServingHostnames,
            runtimes: machine.runtimes,
            registry: machine.releases,
            buckets,
            builds: this.builds,
            blobs: this.blobs,
            events: this.volume.events,
            forge,
            follows: forge === null ? [] : forge.follows(),
            controllers,
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
