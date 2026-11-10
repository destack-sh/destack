import { Forge, Registry } from "@destack/registry/client";
import { type Authentication, type Caller, principal } from "@destack/access";
import { Exchange } from "@destack/access/token";
import * as accountObject from "@destack/account/object";
import type { CatalogueBucketHost } from "@destack/bucket/provider";
import { S3Credentials, type S3Server } from "@destack/bucket/s3";
import connectionPackage, { CONNECTION_INSTALLATION } from "@destack/connection/client";
import type { ConnectionLease } from "@destack/connection/object";
import { BlobStore, type DatabaseConnection, type Controller, Snapshot, TABLE } from "@destack/db";
import { BuildStore } from "@destack/build/store";
import * as stackObject from "@destack/stack/object";
import type { Volume } from "./storage.ts";
import {
    ClaimLookup,
    claimTable,
    HookSandbox,
    installation,
    Observation,
    PackageDescription,
    SpaceMount,
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
import { type Digest, type Identifier, present } from "@destack/schema";
import { Database, Rollout, rollout } from "@destack/stack/object";
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
import type { GrantRequest } from "@destack/account/authentication";
import type { PackageRelease } from "@destack/package";

/** The adapter of OTLP/HTTP, which the installations taking a space's telemetry serve. */
const OTLP_ADAPTER = "otlp";

/** An adapter an installation of the space declares, at its path below its package's mount. */
export interface SpaceAdapter {
    /** The installation answering it. */
    readonly installationId: Identifier<"installation">;
    /** The installation's package. */
    readonly packageId: PackageId;
    /** The adapter's name in its package. */
    readonly name: string;
    /** The adapter's path below the package's mount. */
    readonly path: string;
}

/** What one start of a space's workload serves with: its keys, its store and its buckets. */
export interface SpaceRun {
    /** The adapters the served installations declare. */
    adapters: readonly SpaceAdapter[];
    /** The host of the space's buckets. */
    readonly buckets: CatalogueBucketHost;
    /** The S3 API over the space's buckets. */
    readonly s3: Pick<S3Server, "fetch" | "serve">;
    /** The space's store, once it serves. */
    readonly store: PromiseWithResolvers<ObjectStore>;
    /** The gateway routing the installations' calls, once the store serves. */
    readonly gateway: PromiseWithResolvers<Gateway>;
    /** The space's role on the machine this start serves. */
    readonly role: SpaceRole;
    /** Apply a changed installed set in this start, the unchanged installations served on. */
    readonly apply: (installed: Awaited<ReturnType<typeof Database.installed>>) => Promise<void>;
    /** Start the workload again, once the space's role here changed. */
    readonly restart: () => void;
}

/** A space a machine serves: its database, events, traffic, workload and outbound calls. */
export class Space {
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
                shipped: machine.shipped,
                trusted: machine.trusted,
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

    /** Serve a request to the space: its health's probes, else through its gateway. */
    async fetch(request: Request, authentication?: Authentication): Promise<Response> {
        // answer the space's liveness and readiness, as a server answers its own, while it starts too
        const mounted = SpaceMount.parse(new URL(request.url).pathname);
        const probed =
            mounted === undefined
                ? undefined
                : this.health.probe(
                      new Request(new URL(mounted.path, request.url).href, {
                          method: request.method,
                      }),
                  );
        if (probed !== undefined) {
            return probed;
        }

        // serve anything else through the gateway once it serves
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
        await this.#root(installed);
        this.#serving = {
            adapters: Space.#adapters(installed),
            buckets,
            s3: serveS3(buckets, database, this.machine.environment),
            store: Promise.withResolvers(),
            gateway: Promise.withResolvers(),
            role,
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
            const { servers, sandboxes, revisions } = await this.host.servers(
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
                              account: () => this.host.account(),
                              traffic: (signal) => this.traffic.next(signal),
                          }).controller(),
                  ]
                : [];
            const controllers = [() => this.controller(), ...meters];
            const cursors: NonNullable<ImplementationInput["cursors"]> = isMetered
                ? [
                      // append each account's uses of a page an hour at a time, kept while a billing call may retry
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

            // record what this start serves before handing out its store, replacing an earlier start's claims
            if (role === "primary") {
                await Rollout.serve(served.store, this.id, revisions);
            }
            this.#serving.store.resolve(served.store);

            // route the installations' calls through the space's gateway
            this.#serving.gateway.resolve(new Gateway(this, this.#serving, served.store));

            return { packages: served.store.packages };
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

    /** Build the controller applying the space's installed set at each start and change of it. */
    controller(): Controller<typeof Scope.table> {
        return this.host.controller(() => this.#reconcile());
    }

    /** Retain the builds the space runs in the machine's cache while the machine stores it. */
    async #root(installed: Awaited<ReturnType<typeof Database.installed>>): Promise<void> {
        const { cache } = this.machine;
        const manifests = installed.map((each) =>
            cache.resolve(each.rollout.installation.packageId, each.manifest),
        );
        await cache.root(this.id, manifests);
    }

    /** Apply a primary's installed set in place, then report whether it serves each revision. */
    async #reconcile(): Promise<void> {
        // apply a primary's installed set in this start, recording its served revisions
        const serving = present(this.#serving, `the serving of ${this.id}`);
        if (serving.role === "primary") {
            const installed = await Database.installed(
                Snapshot.live(this.database),
                this.id,
                (packageId, manifest) =>
                    this.builds.pull(this.machine.releases, packageId, manifest),
            );
            await this.#root(installed);
            await serving.apply(installed);
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

    /** Wait until an installation of the space is current and served, refusing once aborted. */
    async waitServed(name: string, signal: AbortSignal): Promise<void> {
        // wake on changes of installations and their rollouts
        const installations = installation.table[TABLE].sqlName;
        const rollouts = rollout.table[TABLE].sqlName;
        const isServed = await this.database.log.waitUntil(async (tables) => {
            // skip commits that change no installation and no rollout
            if (!tables.has(installations) && !tables.has(rollouts)) {
                return false;
            }

            // read the installation by its name, current once this start serves its latest revision
            const snapshot = Snapshot.live(this.database);
            const installed = await snapshot
                .read(installation.table)
                .findFirst({ where: { scope: this.id, name } });
            const served =
                installed === undefined
                    ? undefined
                    : await snapshot
                          .read(rollout.table)
                          .findFirst({ where: { installationId: installed.id } });

            return (
                installed !== undefined &&
                Observation.progress(installed) === "current" &&
                served?.servedRevisionId === installed.revisionId
            );
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

    /** Serve an installed set in this start: load, apply and record its changed servers. */
    async #apply(
        installed: Awaited<ReturnType<typeof Database.installed>>,
        start: SpaceStart,
        context: WorkloadContext,
    ): Promise<void> {
        // apply the changed installations' servers to the store and serve their packages
        const store = present(this.#store, `the store of ${this.id}`);
        await this.host.apply(installed, start, context.signal, (servers, sandboxes) =>
            context.serve(store.apply(servers, sandboxes)),
        );

        // read the adapters the set declares
        present(this.#serving, `the serving of ${this.id}`).adapters = Space.#adapters(installed);
    }

    /** Read the adapters the installed set's packages declare. */
    static #adapters(installed: Awaited<ReturnType<typeof Database.installed>>): SpaceAdapter[] {
        return installed.flatMap((entry) => {
            // read the adapters the package declares, none for a library
            const { installationId } = entry.rollout;
            const packageId = PackageId.parse(entry.rollout.installation.packageId);
            const adapters = PackageDescription.find(entry.declarations)?.adapters ?? {};

            return Object.entries(adapters).map(([name, adapter]) => ({
                installationId,
                packageId,
                name,
                path: adapter.path,
            }));
        });
    }

    /** List the adapters an installation the space serves declares. */
    adapters(installationId: Identifier<"installation">): readonly SpaceAdapter[] {
        return this.#current().adapters.filter((each) => each.installationId === installationId);
    }

    /** Publish a build and its account's unpublished dependencies for a caller. */
    async #publish(
        store: Pick<BuildStore, "contents" | "units">,
        manifest: Digest,
        account: Identifier<"account">,
        tag: string,
        caller: Caller,
        requested: GrantRequest,
    ): Promise<readonly PackageRelease[]> {
        // act as the space for the caller, refusing a caller it cannot act for
        const space = principal.space.reference(Scope.universe.id, this.id);
        const { environment } = this.machine;
        const acting = Exchange.of(caller);
        if (acting === undefined) {
            throw MachineError.of("PUBLISHER_UNKNOWN", "a publish acts for a person or a delegate");
        }

        // publish through the forges in the accounts' residency homes, calling each for the caller under the grant the space asks
        const forges = Registry.of(
            this.machine.directory,
            (owner) => this.#residencyHome(owner),
            (audience) =>
                this.host
                    .session(this.host.identityOf(space).acting(acting, requested))
                    .fetch(audience),
            environment,
            this.machine.registry,
            [...this.machine.shipped.values()],
        );

        return await forges.publish(store, manifest, account, tag);
    }

    /** Lease a repository of the space's account to read, at the forge in its residency's home. */
    async lease(repositoryId: Identifier<"repository">): Promise<Lease> {
        // call the forge of the space's account as the space, a peer call to its home's stack
        const self = principal.space.reference(Scope.universe.id, this.id);
        const forges = Registry.of(
            this.machine.directory,
            (account) => this.#residencyHome(account),
            () => this.host.session(this.host.identityOf(self)).fetch(stackPackage.id),
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
        const receivers = this.#current().adapters.filter((each) => each.name === OTLP_ADAPTER);
        for (const receiver of receivers) {
            // post the export below the receiver's adapter, refusing a failed answer
            const url = new URL(`${receiver.path}/${signal}`, this.machine.origin);
            const request = new Request(url.href, {
                method: "POST",
                headers: { "content-type": "application/json" },
                body: JSON.stringify(body),
            });
            const authentication = this.host
                .identityOf(self)
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
        const identity = this.host.identityOf(consumer);
        const primary = this.machine.directory.client(
            connectionPackage,
            home,
            CONNECTION_INSTALLATION,
            this.host.session(identity).fetch(connectionPackage.id),
            environment,
        );

        return primary.connection.lease({ scope: lent.scope, id: lent.id });
    }

    /** Append an account's uses of a page of readings at its payer's home, as the machine. */
    async #append(page: CursorPage): Promise<void> {
        const { directory, environment } = this.machine;
        for (const use of await Reading.uses(page, environment.crypto)) {
            // bill the payer of the space this machine serves, at the stack in the home of the payer's residency
            const payer = await this.host
                .accounts()
                .placement.payer({ accountId: use.accountId, id: this.id });
            const home = await this.#residencyHome(payer);
            const fetch = this.machine.fetch(stackPackage.id);
            const stack = directory.client(
                stackPackage,
                home,
                STACK_INSTALLATION,
                fetch,
                environment,
            );
            await stack.usage.append({ ...use, accountId: payer });
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
            publish: (store, manifest, account, tag, caller, requested) =>
                this.#publish(store, manifest, account, tag, caller, requested),
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
