import type { Snapshot } from "@destack/db";
import { Claim, claimTable, type ObjectType } from "@destack/object";
import { schema, type Identifier } from "@destack/schema";
import { type Fetch, PackageMount, SpaceMount } from "@destack/runtime";
import { createClient } from "@destack/runtime/client";
import { CallError } from "@destack/runtime/error";
import objectPackage from "@destack/object/package";
import type { ObjectReference, Publisher } from "@destack/sync";
import { account } from "../object/account.ts";
import { Endpoint, endpoint } from "../object/endpoint.ts";
import { placement, type PlacementEndpoint } from "../object/placement.ts";
import { SpaceAddress } from "./address.ts";

/** The directory: handles, space addresses `<name>--<handle>`, placements and machines. */
export class Directory<View extends Pick<Snapshot, "read"> = Snapshot> {
    /** The view of the replicated placements, names and endpoints. */
    readonly snapshot: View;

    /** Resolve over a view of the directory's rows, a snapshot or a call's reads. */
    constructor(snapshot: View) {
        this.snapshot = snapshot;
    }

    /** Find the account claiming a handle, none for a handle the platform organisation reserves. */
    async account(handle: string): Promise<Identifier<"account"> | undefined> {
        const owner = await Claim.owner(
            this.snapshot.read(claimTable),
            Claim.key(account, "handle", [handle]),
        );

        return owner !== undefined && account.identifies(owner.objectId)
            ? account.identifier(owner.objectId)
            : undefined;
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
            throw new CallError("NOT_FOUND", { message: `nothing answers at ${address}` });
        }
        const located = await this.locate(schema.identifier("space").parse(named.id), now);

        return { id: named.id, ...located };
    }

    /** Locate a space's placement and the URL its machine answers at now. */
    async locate(spaceId: Identifier<"space">, now: number): Promise<PlacementEndpoint> {
        // read the placement, then where its machine answers
        const placed = await this.snapshot
            .read(placement.table)
            .findFirst({ where: { id: spaceId } });
        if (placed === undefined) {
            throw new CallError("NOT_FOUND", { message: `nothing answers for ${spaceId}` });
        }
        const { machineId } = placed;
        const url = await this.answering(machineId, now);

        return {
            placement: {
                id: placed.id,
                scope: placed.scope,
                machine: machineId,
                epoch: placed.epoch,
            },
            url,
        };
    }

    /** Stream follows of a space's rows from its object package, where its machine answers. */
    publisher(scope: string, fetch: Fetch, now: () => number): Publisher {
        const space = schema.identifier("space").parse(scope);
        const connect = async () => {
            // connect to the space's object package where its machine answers now
            const located = await this.locate(space, now());
            const url = `${SpaceMount.url(located.url, space)}${PackageMount.path(objectPackage.id)}`;

            return createClient(objectPackage, { url, fetch });
        };

        return {
            stream: async function* (followed, signal) {
                const objects = await connect();
                yield* await objects.store.stream(followed, { signal });
            },
            confirm: async (subscription, watermark) => {
                const objects = await connect();
                await objects.store.confirm({ subscription, watermark });
            },
        };
    }

    /** Read the URL a machine answers at now. */
    async answering(machineId: Identifier<"machine">, now: number): Promise<string> {
        const published = await this.snapshot
            .read(endpoint.table)
            .findFirst({ where: { id: machineId } });
        if (published === undefined || !Endpoint.isAnswering(published, now)) {
            throw new CallError("NOT_FOUND", { message: `machine ${machineId} answers nowhere` });
        }

        return published.url;
    }

    /** Find the object an active name resolves to. */
    #owner(index: string, key: string) {
        return Claim.owner(this.snapshot.read(claimTable), { index, key });
    }
}
