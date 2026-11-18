import type { Clock, Environment } from "@destack/environment";
import { type Snapshot, TABLE } from "@destack/db";
import { Claim, ClaimLookup, claimTable, type ObjectType } from "@destack/object";
import { nth, present, schema, type Identifier } from "@destack/schema";
import {
    type Fetch,
    InstallationMount,
    type Package,
    PackageMount,
    SpaceMount,
} from "@destack/object";
import { type ClientOf, createClient, ObjectConnection } from "@destack/object/client";
import objectPackage from "@destack/object/package";
import type { ObjectReference, Publisher } from "@destack/sync";
import { account } from "../object/account.ts";
import { space } from "../object/space.ts";
import { Endpoint, endpoint } from "../object/endpoint.ts";
import { machine } from "../object/machine.ts";
import { placement, type PlacementEndpoint } from "../object/placement.ts";
import { SpaceAddress, type SpaceOrigin } from "./address.ts";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";

/** An object a name resolves to, and whether the name is its current or a former one. */
export interface DirectoryHolder<Identity extends string> {
    /** The object. */
    readonly id: Identifier<Identity>;
    /** Whether the object holds the name now, or held it before a rename. */
    readonly state: "active" | "former";
}

/** The directory: handles, space addresses `<name>--<handle>`, placements and machines. */
export class Directory<View extends Pick<Snapshot, "read"> = Snapshot> {
    /** The view of the replicated placements, names and endpoints. */
    readonly snapshot: View;

    /** Resolve over a view of the directory's rows, a snapshot or a call's reads. */
    constructor(snapshot: View) {
        this.snapshot = snapshot;
    }

    /** Find the account claiming a handle, none for a handle the platform organization reserves. */
    async account(handle: string): Promise<Identifier<"account"> | undefined> {
        return (await this.holder(handle))?.id;
    }

    /** Find the account claiming a handle now or before a rename, with which it is. */
    async holder(handle: string): Promise<DirectoryHolder<"account"> | undefined> {
        const owner = await Claim.owner(
            this.snapshot.read(claimTable),
            Claim.key(account, "handle", [handle]),
        );

        return owner !== undefined && account.identifies(owner.objectId)
            ? { id: account.identifier(owner.objectId), state: owner.state }
            : undefined;
    }

    /** Find the handle an account claims now, none for an account claiming none. */
    async handle(accountId: string): Promise<string | undefined> {
        const [claimed] = await this.snapshot.read(claimTable).findMany({
            where: {
                index: Claim.index(account, "handle"),
                objectId: accountId,
                state: { ne: "former" },
            },
        });

        return claimed === undefined
            ? undefined
            : schema.string().parse(nth(Claim.values(claimed), 0));
    }

    /** Find an account's object of a type at an address, and whether the address is former. */
    async find<Object extends ObjectType>(
        type: Object,
        address: string,
    ): Promise<
        | (ObjectReference & {
              readonly id: Identifier<Object["identity"]>;
              readonly state: "active" | "former";
          })
        | undefined
    > {
        // find the account with the handle, now or before a rename
        const parsed = SpaceAddress.safeParse(address);
        const origin = parsed.success ? SpaceAddress.origin(parsed.data) : undefined;
        const holder = origin === undefined ? undefined : await this.holder(origin.handle);
        if (origin === undefined || holder === undefined) {
            return undefined;
        }

        // find the object claiming the name within the account, former when either name is
        const claims = ClaimLookup.of(this.snapshot.read(claimTable));
        const found = await Claim.lookup(type, claims, "name", [origin.space], holder.id);
        const isFormer = holder.state === "former" || found?.state === "former";

        return found === undefined
            ? undefined
            : {
                  ...type.reference(found.scope, found.id),
                  id: schema.identifier<Object["identity"]>(type.identity).parse(found.id),
                  state: isFormer ? "former" : "active",
              };
    }

    /** Read the current address of a space: its name and its account's handle now. */
    async origin(spaceId: Identifier<"space">): Promise<SpaceOrigin | undefined> {
        // read the space's current name and the account it belongs to
        const [named] = await this.snapshot.read(claimTable).findMany({
            where: {
                index: Claim.index(space, "name"),
                objectId: spaceId,
                state: { ne: "former" },
            },
        });
        if (named === undefined) {
            return undefined;
        }

        // read the account's current handle
        const handle = await this.handle(named.owner);

        return handle === undefined
            ? undefined
            : { space: schema.string().parse(nth(Claim.values(named), 0)), handle };
    }

    /** Resolve an address of an object type to its placement and the URL its machine answers at. */
    async resolve<Object extends ObjectType>(
        type: Object,
        address: string,
        now: number,
    ): Promise<PlacementEndpoint & { readonly id: Identifier<Object["identity"]> }> {
        // find the object, then where its space answers
        const named = await this.find(type, address);
        if (named === undefined) {
            throw AccountError.of("ADDRESS_NOT_FOUND", `address '${address}' not found`);
        }
        const located = await this.locate(schema.identifier("space").parse(named.id), now);

        return { id: named.id, ...located };
    }

    /** Locate a space's placement and the URL its machine answers at now. */
    async locate(spaceId: Identifier<"space">, now: number): Promise<PlacementEndpoint> {
        // read the placement, then where its machine answers
        const placed = await this.placement(spaceId);
        if (placed === undefined) {
            throw AccountError.of("PLACEMENT_NOT_FOUND", "placement of this space not found");
        }
        const url = await this.answering(placed.placement.machine, now);

        return { ...placed, url };
    }

    /** Locate a package below an installation of a space, at the URL its machine answers at now. */
    async mount(
        spaceId: Identifier<"space">,
        installation: string,
        packageId: string,
        now: number,
    ): Promise<string> {
        const { url } = await this.locate(spaceId, now);

        return `${InstallationMount.url(url, spaceId, installation)}${PackageMount.path(packageId)}`;
    }

    /** Call a package below an installation of a space, locating its machine on each call. */
    client<Handle extends Package>(
        handle: Handle,
        spaceId: Identifier<"space">,
        installation: string,
        fetch: Fetch,
        environment: Environment,
    ): ClientOf<Handle> {
        const url = () => this.mount(spaceId, installation, handle.id, environment.clock.now());

        return createClient<Handle["router"]>(handle, { url, fetch }, environment);
    }

    /** Read a space's placement and its machine's URL now, absent until both are known. */
    async serving(
        spaceId: Identifier<"space">,
        now: number,
    ): Promise<PlacementEndpoint | undefined> {
        // read the placement, then the endpoint its machine publishes
        const placed = await this.placement(spaceId);
        const published =
            placed === undefined
                ? undefined
                : await this.snapshot
                      .read(endpoint.table)
                      .findFirst({ where: { parentId: placed.placement.machine } });

        return placed === undefined ||
            published === undefined ||
            !Endpoint.isAnswering(published, now)
            ? undefined
            : { ...placed, url: published.url };
    }

    /** Stream follows of a space's rows from its object package, where its machine answers. */
    publisher(
        this: Directory,
        scope: string,
        authorize: (request: Request) => Promise<Request>,
        environment: Environment,
    ): Publisher & Disposable {
        // connect to the space's object package where the directory places it, closing the connection to where it was
        const spaceId = schema.identifier("space").parse(scope);
        let current: ObjectConnection | undefined;
        const connect = async (located: PlacementEndpoint) => {
            // reuse the connection to where the space is, else open one
            const mounted = `${SpaceMount.url(located.url, spaceId)}${PackageMount.path(objectPackage.id)}`;
            const url = ObjectConnection.url(mounted);
            if (current?.url !== url) {
                current?.close();
                current = ObjectConnection.open({ scope, url, authorize, environment });
            }

            return current;
        };
        const whenPlaced = (signal: AbortSignal) =>
            this.waitPlaced(spaceId, environment.clock, signal);
        const located = () => this.locate(spaceId, environment.clock.now());

        return {
            stream: async function* (followed, signal) {
                // wait until the replicated directory places the space at a machine that answers
                const objects = await connect(await whenPlaced(signal));
                yield* objects.stream(followed, signal);
            },
            // request only after a stream, once the space is placed
            request: async (subscription) => {
                const objects = await connect(await located());
                await objects.request(subscription);
            },
            // close the connection to where the space is
            [Symbol.dispose]: () => current?.close(),
        };
    }

    /** Wait until the directory places a space at a machine that answers, and locate it. */
    async waitPlaced(
        this: Directory,
        spaceId: Identifier<"space">,
        clock: Pick<Clock, "now">,
        signal: AbortSignal,
    ): Promise<PlacementEndpoint> {
        // find the space at each commit of placements or endpoints until it is placed and its machine answers
        const watched = [placement.table[TABLE].sqlName, endpoint.table[TABLE].sqlName];
        let located: PlacementEndpoint | undefined;
        await this.snapshot.database.log.waitUntil(async (committed) => {
            if (watched.some((table) => committed.has(table))) {
                located = await this.serving(spaceId, clock.now());
            }

            return located !== undefined;
        }, signal);

        // refuse an aborted wait
        signal.throwIfAborted();

        return present(located, `the placement of ${spaceId}`);
    }

    /** Read the URL a machine answers at now. */
    async answering(machineId: Identifier<"machine">, now: number): Promise<string> {
        const published = await this.snapshot
            .read(endpoint.table)
            .findFirst({ where: { parentId: machineId } });
        if (published === undefined || !Endpoint.isAnswering(published, now)) {
            const named = await this.snapshot
                .read(machine.table)
                .findFirst({ where: { id: machineId } });
            throw AccountError.of(
                "ENDPOINT_NOT_FOUND",
                `published endpoint of '${present(named, `machine ${machineId}`).name}' not found`,
            );
        }

        return published.url;
    }

    /** Read a space's placement and whether its machine serves it yet, absent while unplaced. */
    async placement(
        spaceId: Identifier<"space">,
    ): Promise<Omit<PlacementEndpoint, "url"> | undefined> {
        const placed = await this.snapshot.read(placement.table).findFirst({ where: { spaceId } });

        return placed === undefined
            ? undefined
            : {
                  placement: {
                      space: placed.spaceId,
                      scope: placed.scope,
                      machine: placed.machineId,
                      epoch: placed.epoch,
                  },
                  status: placed.status,
              };
    }
}
