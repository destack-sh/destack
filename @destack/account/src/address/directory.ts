import type { Clock, Environment } from "@destack/environment";
import { type Snapshot, TABLE } from "@destack/db";
import { Claim, claimTable, type ObjectType } from "@destack/object";
import { nth, present, schema, type Identifier } from "@destack/schema";
import { type Fetch, PackageMount, SpaceMount } from "@destack/object";
import { createClient } from "@destack/object/client";
import objectPackage from "@destack/object/package";
import type { ObjectReference, Publisher } from "@destack/sync";
import { account } from "../object/account.ts";
import { Endpoint, endpoint } from "../object/endpoint.ts";
import { placement, type PlacementEndpoint } from "../object/placement.ts";
import { SpaceAddress } from "./address.ts";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";

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
        const owner = await Claim.owner(
            this.snapshot.read(claimTable),
            Claim.key(account, "handle", [handle]),
        );

        return owner !== undefined && account.identifies(owner.objectId)
            ? account.identifier(owner.objectId)
            : undefined;
    }

    /** Find the handle an account claims, none for an account claiming none. */
    async handle(accountId: string): Promise<string | undefined> {
        const [claimed] = await this.snapshot.read(claimTable).findMany({
            where: { index: Claim.index(account, "handle"), objectId: accountId },
        });

        return claimed === undefined
            ? undefined
            : schema.string().parse(nth(Claim.values(claimed), 0));
    }

    /** Find the object at an address among an account's objects of a type, absent for none. */
    async find<Object extends ObjectType>(
        type: Object,
        address: string,
    ): Promise<(ObjectReference & { readonly id: Identifier<Object["identity"]> }) | undefined> {
        // find the account with the handle
        const parsed = SpaceAddress.safeParse(address);
        const origin = parsed.success ? SpaceAddress.origin(parsed.data) : undefined;
        const accountId = origin === undefined ? undefined : await this.account(origin.handle);
        if (origin === undefined || accountId === undefined) {
            return undefined;
        }

        // find the object claiming the name within the account
        const claims = { owner: (index: string, key: string) => this.#owner(index, key) };
        const found = await Claim.lookup(type, claims, "name", [origin.space], accountId);

        return found === undefined
            ? undefined
            : {
                  ...type.reference(found.scope, found.id),
                  id: schema.identifier<Object["identity"]>(type.identity).parse(found.id),
              };
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
            throw AccountError.of("ADDRESS_NOT_FOUND", `nothing responds at ${address}`);
        }
        const located = await this.locate(schema.identifier("space").parse(named.id), now);

        return { id: named.id, ...located };
    }

    /** Locate a space's placement and the URL its machine answers at now. */
    async locate(spaceId: Identifier<"space">, now: number): Promise<PlacementEndpoint> {
        // read the placement, then where its machine answers
        const placed = await this.#placement(spaceId);
        if (placed === undefined) {
            throw AccountError.of("SPACE_NOT_PLACED", `nothing responds for ${spaceId}`);
        }
        const url = await this.answering(placed.machine, now);

        return { placement: placed, url };
    }

    /** Find a space's placement and its machine's URL now, absent until both are known. */
    async #find(spaceId: Identifier<"space">, now: number): Promise<PlacementEndpoint | undefined> {
        // read the placement, then the endpoint its machine publishes
        const placed = await this.#placement(spaceId);
        const published =
            placed === undefined
                ? undefined
                : await this.snapshot
                      .read(endpoint.table)
                      .findFirst({ where: { id: placed.machine } });

        return placed === undefined ||
            published === undefined ||
            !Endpoint.isAnswering(published, now)
            ? undefined
            : { placement: placed, url: published.url };
    }

    /** Stream follows of a space's rows from its object package, where its machine answers. */
    publisher(this: Directory, scope: string, fetch: Fetch, environment: Environment): Publisher {
        // connect to the space's object package where the directory places it
        const space = schema.identifier("space").parse(scope);
        const connect = async (located: PlacementEndpoint) => {
            const url = `${SpaceMount.url(located.url, space)}${PackageMount.path(objectPackage.id)}`;

            return createClient(objectPackage, { url, fetch }, environment);
        };
        const whenPlaced = (signal: AbortSignal) =>
            this.waitForPlacement(space, environment.clock, signal);
        const located = () => this.locate(space, environment.clock.now());

        return {
            stream: async function* (followed, signal) {
                // wait until the replicated directory places the space at a machine that answers
                const objects = await connect(await whenPlaced(signal));
                yield* await objects.store.stream(followed, { signal });
            },
            // confirm only after a stream, once the space is placed
            confirm: async (subscription, watermark) => {
                const objects = await connect(await located());
                await objects.store.confirm({ subscription, watermark });
            },
        };
    }

    /** Wait until the directory places a space at a machine that answers, and locate it. */
    async waitForPlacement(
        this: Directory,
        spaceId: Identifier<"space">,
        clock: Pick<Clock, "now">,
        signal: AbortSignal,
    ): Promise<PlacementEndpoint> {
        // find the space at each commit of placements or endpoints until it is placed and its machine answers
        const watched = [placement.table[TABLE].sqlName, endpoint.table[TABLE].sqlName];
        let located: PlacementEndpoint | undefined;
        await this.snapshot.database.log.until(async (committed) => {
            if (watched.some((table) => committed.has(table))) {
                located = await this.#find(spaceId, clock.now());
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
            .findFirst({ where: { id: machineId } });
        if (published === undefined || !Endpoint.isAnswering(published, now)) {
            throw AccountError.of(
                "MACHINE_NOT_PUBLISHED",
                `machine ${machineId} has no published endpoint`,
            );
        }

        return published.url;
    }

    /** Read a space's placement, absent for a space placed nowhere. */
    async #placement(
        spaceId: Identifier<"space">,
    ): Promise<PlacementEndpoint["placement"] | undefined> {
        const placed = await this.snapshot
            .read(placement.table)
            .findFirst({ where: { id: spaceId } });

        return placed === undefined
            ? undefined
            : {
                  id: placed.id,
                  scope: placed.scope,
                  machine: placed.machineId,
                  epoch: placed.epoch,
              };
    }

    /** Find the object an active name resolves to. */
    #owner(index: string, key: string) {
        return Claim.owner(this.snapshot.read(claimTable), { index, key });
    }
}
