import * as accountObject from "@destack/account/object";
import { PROCESS_RUNTIME } from "@destack/stack/object";
import { KNOWN_TYPES } from "@destack/stack/stack";
import { and, eq, isNull, Snapshot } from "@destack/db";
import { principal } from "@destack/access";
import { hostname, machine } from "@destack/account/object";
import { type Identifier, schema } from "@destack/schema";
import { type Publisher, RequestId, type Subject, Scope } from "@destack/sync";
import {
    type Follow,
    grantShape,
    type ObjectType,
    type HookSandbox,
    relationship,
} from "@destack/object";
import {
    Account,
    placement,
    profileShape,
    residency,
    residentsShape,
    residencyShape,
    spaceShape,
    user,
    Entitlement,
} from "@destack/account/object";
import { implementPackage, ObjectStore, type PackageImplementation } from "@destack/object/server";
import { bucketProvider } from "@destack/bucket/provider";
import { implementBucket } from "../resource/bucket.ts";
import { settingServer } from "@destack/setting/server";
import type { BootstrapPackageName } from "@destack/stack/declare";
import { rollout } from "@destack/stack/object";
import objectPackage from "@destack/object/package";
import stackPackage from "@destack/stack/package";
import { implementRestore, implementSnapshot } from "../resource/snapshot.ts";
import { Binder, ResourceBindable } from "../resource/binding.ts";
import { ObjectBindable } from "@destack/stack/object";
import { implementDeployment, implementInstance } from "../deployment/deployment.ts";
import {
    type InstallationOptions,
    implementInstallations,
    implementRollouts,
} from "../installation/installation.ts";
import { implementInstallationRevision } from "../installation/revision.ts";
import { installationProvider, ProviderIndex } from "../resource/resource.ts";
import { type SpaceProvider } from "@destack/stack/object";
import { implementSpace } from "./placement.ts";
import { implementTransfer, type TransferOptions } from "../transfer/transfer.ts";
import { handoffMount } from "../transfer/handoff.ts";
import { implementBuild, pushMount, retainBuilds } from "../installation/build.ts";
import { SpaceTransfer } from "../transfer/transfer.ts";
import type { Space, SpaceStart } from "./space.ts";
import { hookMount } from "../installation/hook.ts";
import type { SocketRegistry } from "./socket.ts";
import { implementRoute } from "../access/route.ts";
import { implementRelationship, implementRole } from "../access/role.ts";

/** Serve the stack package over the spaces' objects, next to the other packages' resources. */
function stackServer(
    space: Space,
    start: SpaceStart,
    providers: ProviderIndex,
    binder: Binder,
): PackageImplementation {
    // configure the moves
    const listed = providers.own;
    const transfers = configureTransfers(space, start, providers);

    // serve the space's objects with the external types, a standby only those its moves write
    const isPrimary = start.role === "primary";
    const own = isPrimary
        ? ownObjects(space, start, providers, binder, transfers)
        : stackObjects([implementRestore(transfers), implementTransfer(transfers)]);

    return implementPackage(stackPackage, {
        objects: [...own, ...externalObjects(start, own)],
        external: start.external,
        providers: listed,
        follows: [
            ...spaceFollows(space, start),
            // follow the account's entitlements, which enforcement reads
            ...(isPrimary ? [Entitlement.follow, ...start.follows] : []),
        ],
        // answer moving spaces' targets and installations' hook sessions
        mounts: {
            handoff: handoffMount({
                database: space.database,
                publisher: start.uplink,
                providers,
                keystore: space.keystore,
                runtimes: space.machine.runtimes,
                blobs: space.blobs,
                builds: space.builds,
            }),
            push: pushMount(space.builds),
            hook: hookMount,
        },
        // receive moving spaces on a standby, run a primary's providers, and the host's own
        controllers: [
            ...(isPrimary
                ? space.volume.providers
                      .filter((each): each is SpaceProvider => !("connect" in each))
                      .flatMap((provider) =>
                          (provider.controllers ?? []).map((controller) => () => controller),
                      )
                : [(store: ObjectStore) => SpaceTransfer.controller(transfers, store)]),
            ...start.controllers,
        ],
        // finish moves in either role, and project a primary's birth, assignments, resources and retention
        cursors: [
            (store) => SpaceTransfer.projection(transfers, store),
            ...(isPrimary
                ? [
                      (store: ObjectStore) => space.placement(store, space.machine.volumes.main),
                      () => space.assignment(start.uplink),
                      (store: ObjectStore) =>
                          binder.specification(
                              store,
                              space.machine.volumes.main,
                              space.machine.linked,
                          ),
                      retainBuilds,
                  ]
                : []),
            ...start.cursors,
        ],
        responseHeaders: { "Cache-Control": "no-store" },
    });
}

/** Open a start's store: the stack's, access's and settings' servers, then the installed ones. */
export function openStore(
    space: Space,
    start: SpaceStart,
    servers: readonly PackageImplementation[],
    sandboxes: readonly HookSandbox[],
): ObjectStore {
    const store: ObjectStore = new ObjectStore({
        report: space.machine.report,
        servers,
        principal: space.principal,
        partitions: start.partitions,
        database: space.database,
        blobs: space.blobs.storage,
        // know the account's hostnames, which its spaces' routes are declared on, as replicated rows
        policies: [hostname],
        claims: start.claims,
        callKey: () => space.callKey(),
        environment: space.machine.environment,
        events: space.volume.events,
        ephemeral: space.volume.sockets.storage,
        // keep a socket of each principal's stream, recording a machine leaving with its last one
        connect: (subject) =>
            linkSocket(
                space.volume.sockets,
                schema.identifier("space").parse(space.principal.id),
                subject,
                () => store,
            ),
        origin: { package: stackPackage, machineId: space.machine.id },
        uplink: start.uplink,
        // open the release the space runs, or the version a value names
        release: (view, scope, packageId, version) =>
            space.release(view, schema.identifier("space").parse(scope), packageId, version),
        // root the chain above the space by its placement, as primary or move target
        chain: async (view) => {
            // read where the space is placed, and whether it moves here
            const served = schema.identifier("space").parse(space.principal.id);
            const row = await view.read(placement.table).findFirst({ where: { spaceId: served } });
            const isFollowing = start.role === "primary" || row?.targetId === space.machine.id;

            // root the chain at the placement, wait for a primary's, and follow none elsewhere
            if (row !== undefined && isFollowing) {
                return { kind: "home", parent: row.scope };
            } else if (row === undefined && start.role === "primary") {
                return { kind: "pending" };
            }

            return undefined;
        },
        // send to the universe by the uplink, and to other scopes at their homes by the machine's route
        publisher: (scope) => homePublisher(space, start, scope),
        machine: space.machine.id,
        sandboxes,
    });

    return store;
}

/** Build the stack's, access's and settings' servers over the installed providers and kinds. */
export function ownServers(
    space: Space,
    start: SpaceStart,
    installed: readonly PackageImplementation[],
    served: ConstructorParameters<typeof ProviderIndex>[2],
): readonly PackageImplementation[] {
    // index the machine's providers and bindable kinds, which the stack and the installations bind with
    const bindables = ObjectBindable.of(installed);
    const providers = new ProviderIndex(space.machine.id, spaceProviders(space, start), served);
    const binder = new Binder(
        providers.objects().map((type) => ResourceBindable.of(providers.kind(type), type)),
        bindables,
    );

    // serve the space's own packages on a standby: the stack with its access objects
    const standby: {
        readonly [Name in Exclude<BootstrapPackageName, "setting">]: PackageImplementation;
    } = {
        stack: stackServer(space, start, providers, binder),
        object: implementPackage(objectPackage, {
            objects: [
                implementRole(),
                implementRelationship(),
                implementInstallations(installationOptions(space, start, providers, binder)),
            ],
        }),
    };

    // serve every own package on a primary, its settings after rollout
    const primary: { readonly [Name in BootstrapPackageName]: PackageImplementation } = {
        ...standby,
        setting: settingServer({ after: [rollout] }),
    };

    return Object.values(start.role === "primary" ? primary : standby);
}

/** Serve the space's own objects. */
function ownObjects(
    space: Space,
    start: SpaceStart,
    providers: ProviderIndex,
    binder: Binder,
    transfers: TransferOptions,
) {
    // serve the space's types with their behaviour
    const { builds } = space;
    const served = [
        implementSnapshot(transfers),
        implementRestore(transfers),
        implementSpace({
            keystore: space.keystore,
            builds,
            blobs: space.blobs,
            publisher: start.uplink,
            isEnforcingEntitlements: space.machine.isEnforcingEntitlements,
        }),
        ...providers.provided(),
        implementRollouts(installationOptions(space, start, providers, binder)),
        implementInstallationRevision(),
        implementDeployment({
            providers,
            bindables: binder.objects(),
            machine: space.machine.id,
            runtimes: new Map(space.machine.runtimes.map((runtime) => [runtime.name, runtime])),
            space,
            trusted: space.machine.trusted,
        }),
        implementInstance(),
        implementBuild({
            buildRunner: space.machine.isBuilding ? space.machine.id : null,
            space,
            registry: space.machine.releases,
            publish: space.publish.bind(space),
        }),
        implementTransfer(transfers),
        implementRoute(space.machine.isServingHostnames),
    ];

    return stackObjects(served);
}

/** Read what the space's installations run with on this machine. */
function installationOptions(
    space: Space,
    start: SpaceStart,
    providers: ProviderIndex,
    binder: Binder,
): InstallationOptions {
    return {
        resources: providers.objects(),
        space,
        blobs: space.blobs,
        registry: space.machine.releases,
        binder,
        runtimes: space.machine.runtimes.filter((runtime) => runtime.name !== PROCESS_RUNTIME),
        destack: space.machine.destack,
        accounts: space.accounts(),
        forge: start.forge,
        known: KNOWN_TYPES,
        issuer: space.keystore.issuer(space.database, space.principal.id),
        isEnforcingEntitlements: space.machine.isEnforcingEntitlements,
        database: space.database,
        linked: space.machine.linked,
        shipped: space.machine.shipped,
        trusted: space.machine.trusted,
    };
}

/** Serve the stack's stored types, each implemented one in place, then the other served types. */
function stackObjects(served: readonly ObjectType[]): ObjectType[] {
    const own = Object.values(stackPackage.objects)
        .filter((type) => type.storage !== "external")
        .map((type) => served.find((each) => each.same(type)) ?? type);

    return [...own, ...served.filter((type) => !own.includes(type))];
}

/** Collect the external types, refusing one under a name the space serves. */
function externalObjects(start: SpaceStart, own: readonly ObjectType[]): readonly ObjectType[] {
    // refuse an external type under a name the space serves
    const external = start.external.objects;
    const taken = external.find((type) => own.some((each) => each.key === type.key));
    if (taken !== undefined) {
        throw new TypeError(`object '${taken.name}' is served by the space`);
    }

    return external;
}

/** List the providers of a space's resources. */
function spaceProviders(space: Space, start: SpaceStart) {
    return [
        installationProvider,
        bucketProvider(start.buckets, implementBucket(start.buckets)),
        ...space.volume.providers.filter((each): each is SpaceProvider => !("connect" in each)),
    ];
}

/** Move spaces through the universe, the sources' handoffs, the providers and the spaces' keys. */
function configureTransfers(
    space: Space,
    start: SpaceStart,
    providers: ProviderIndex,
): TransferOptions {
    return {
        role: start.role,
        machine: space.machine.id,
        database: space.database,
        publisher: start.uplink,
        source: (source, spaceId) => space.machine.handoff.bind(space.machine)(source, spaceId),
        providers,
        blobs: space.blobs,
        builds: space.builds,
        keystore: space.keystore,
        runtimes: space.machine.runtimes,
    };
}

/** Follow the space's row and residents, and on its primary its people's profiles and grants. */
function spaceFollows(space: Space, start: SpaceStart): readonly Follow[] {
    // follow the space's row as the machine wherever it is served, which it needs before acting as the space
    const served = schema.identifier("space").parse(space.principal.id);
    const host = principal.machine.reference(Scope.universe.id, space.machine.id);
    const follows: Follow[] = [
        spaceShape.at({
            key: { scope: Scope.universe.id, space: served, machine: space.machine.id },
            subject: host,
        }),
    ];

    // follow nothing the space signs for on a standby, which has none of its keys
    if (start.role !== "primary") {
        return follows;
    }

    return [
        ...follows,
        // follow the residencies and its residency's residents as the space, a resident itself, so it decides on them
        residencyShape.at({ key: { scope: Scope.universe.id } }),
        residentsShape.per(residency, {
            key: async (row) =>
                row.space === served
                    ? { key: { scope: Scope.universe.id, residency: row.code } }
                    : undefined,
        }),
        // follow the profiles of the people related to the space once it is placed here
        profileShape.per(relationship, {
            where: { subjectPackageId: user.typeReference.packageId, subjectType: user.name },
            key: async (row, view) => {
                const placed = await view
                    .read(placement.table)
                    .findFirst({ where: { spaceId: served } });
                const isPerson = row.subjectRelation === null && user.identifies(row.subjectId);

                return isPerson && placed?.machineId === space.machine.id
                    ? { key: { scope: Scope.universe.id, user: user.identifier(row.subjectId) } }
                    : undefined;
            },
        }),
        // follow the grants people delegate to the space once it is placed here
        grantShape.per(placement, {
            key: async (row) =>
                row.spaceId === served && row.machineId === space.machine.id
                    ? {
                          key: {
                              scope: Scope.universe.id,
                              delegatePackageId: space.principal.packageId,
                              delegateType: space.principal.type,
                              delegateScope: space.principal.scope,
                              delegateId: space.principal.id,
                          },
                      }
                    : undefined,
        }),
    ];
}

/** Open a scope's publisher: the uplink for the universe, space and machine, else its home. */
function homePublisher(space: Space, start: SpaceStart, scope: string): Publisher {
    // follow the universe, and the chains of the served space and its machine, through the uplink
    if ([Scope.universe.id, space.principal.id, space.machine.id].includes(scope)) {
        return start.uplink;
    }

    // follow a space at itself, and another scope in its account's partition at the account's home
    const homed = async () => {
        if (accountObject.space.identifies(scope)) {
            return space.publisherAt(scope, scope);
        }
        const { id } = await Account.of(Snapshot.live(space.database), scope);

        return space.publisherAt(Account.home(id), id);
    };

    return {
        async *stream(subscription, signal) {
            yield* (await homed()).stream(subscription, signal);
        },
        request: async (subscription) => (await homed()).request(subscription),
    };
}

/** Open a principal's socket to a space, recording a machine's leave at its last close. */
async function linkSocket(
    sockets: SocketRegistry,
    spaceId: Identifier<"space">,
    subject: Subject,
    store: () => ObjectStore,
): Promise<() => Promise<void>> {
    const lease = await sockets.link(spaceId, subject);

    return async () => {
        await lease.close();
        if (principal.machine.is(subject) && !(await sockets.isConnected(subject))) {
            await leaveMachine(store(), subject);
        }
    };
}

/** Record a machine leaving its home, where this store serves the machine's row. */
async function leaveMachine(store: ObjectStore, subject: Subject): Promise<void> {
    // leave a machine whose row lives elsewhere, or replicated here
    if (store.findImplementation(machine) === undefined || store.replicates(machine)) {
        return;
    }
    const [row] = await store.database
        .select({ scope: machine.table.scope, id: machine.table.id })
        .from(machine.table)
        .where(
            and(
                eq(machine.table.id, schema.identifier("machine").parse(subject.id)),
                isNull(machine.table.deletedAt),
            ),
        );
    if (row === undefined) {
        return;
    }

    // record the leave as the store
    await store.mutate(
        store.as(machine),
        {
            id: RequestId.create(store.environment),
            calls: [machine.call("leave", machine.target(row))],
        },
        { component: "presence" },
    );
}
