import type { Snapshot } from "@destack/db";
import { Claim, claimTable, type ObjectType } from "@destack/object";
import { schema, type Identifier } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import type { ObjectReference } from "@destack/sync";
import { account } from "../object/account.ts";
import { Endpoint, endpoint } from "../object/endpoint.ts";
import { type Placement, placement } from "../object/placement.ts";
import { SpaceAddress } from "./address.ts";

/** The resolver of handles and addresses `<name>--<handle>` over the directory's replica. */
export class AddressResolver<View extends Pick<Snapshot, "read"> = Snapshot> {
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
    ): Promise<
        Omit<Placement, "id"> & {
            readonly url: string;
            readonly id: Identifier<Object["identity"]>;
        }
    > {
        // find the object, then where its space answers
        const named = await this.find(type, address);
        if (named === undefined) {
            throw new CallError("NOT_FOUND", { message: `nothing answers at ${address}` });
        }
        const { scope, machine, epoch, url } = await this.locate(
            schema.identifier("space").parse(named.id),
            now,
        );

        return { scope, machine, epoch, id: named.id, url };
    }

    /** Locate a space's placement and the URL its machine answers at now. */
    async locate(
        spaceId: Identifier<"space">,
        now: number,
    ): Promise<Placement & { readonly url: string }> {
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
            id: placed.id,
            scope: placed.scope,
            machine: machineId,
            epoch: placed.epoch,
            url,
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
