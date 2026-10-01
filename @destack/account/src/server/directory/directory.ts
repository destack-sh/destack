import { Authorization, type Authorizer, principal, Caller } from "@destack/access";
import { eq, inArray, type DatabaseConnection } from "@destack/db";
import { Snapshot } from "@destack/db/log";
import {
    type Claim,
    claimTable,
    DirectoryStore,
    type ObjectClaims,
    type Zone,
    zoneTable,
} from "@destack/directory";
import { conceal, ServiceError } from "@destack/service/error";
import { implement, type ServiceContext } from "@destack/service/server";
import { Scope, SyncError, type ObjectReference, type Subject } from "@destack/sync";
import { Resolver } from "../../directory/index.ts";
import { account } from "../../object/account.ts";
import { directory } from "../../service/directory/index.ts";

/** Typed implementations of the directory procedures. */
const implementation = implement(directory).$context<ServiceContext>();

/** Serve the universe's directory to hosts, and an account's zones to its members. */
export function directoryRouter(database: DatabaseConnection, authorizer: Authorizer) {
    const cells = new DirectoryStore(database);

    return implementation.router({
        // zones and cells
        place: implementation.place.handler(async ({ input, context }) => {
            // place a zone the caller's cell serves, or create one it may create
            const sender = await Sender.of(context, database);
            await sender.requirePlacement(input.zone);
            await cells.place(input.zone);

            return {};
        }),
        withdraw: implementation.withdraw.handler(async ({ input, context }) => {
            // withdraw a zone placed in a cell the caller acts for
            const sender = await Sender.of(context, database);
            sender.requireCell(input.zone.cell, input.zone.id);
            await cells.withdraw(input.zone);

            return {};
        }),
        locate: implementation.locate.handler(async ({ input, context }) => {
            // locate any zone for a host, else a zone of an account the caller reads, hiding the rest as unplaced
            const sending = Caller.sender(context.requireAuthentication().claims);
            const zone = await cells.locate(input.scope);
            const isVisible =
                zone !== undefined &&
                ((sending !== undefined && principal.host.is(sending)) ||
                    (await readsAccount(database, authorizer, context, zone.scope)));

            return isVisible ? zone : null;
        }),
        list: implementation.list.handler(async ({ input, context }) => {
            // answer an account the caller cannot read as an unknown one
            context.requireAuthentication();
            if (!(await readsAccount(database, authorizer, context, input.scope))) {
                const denial = new ServiceError("FORBIDDEN", {
                    message: `permission denied: read account ${input.scope}`,
                });
                throw conceal(denial, `no account ${input.scope}`);
            }

            return [...(await cells.list(input.scope))];
        }),
        move: implementation.move.handler(async ({ input, context }) => {
            // move a zone placed in a cell the caller acts for
            const sender = await Sender.of(context, database);
            sender.requireCell(input.zone.cell, input.zone.id);
            await cells.move(input.zone, input.target);

            return {};
        }),
        publish: implementation.publish.handler(async ({ input, context }) => {
            // publish the endpoint of a cell the calling host acts for, in the cell's own scope
            const sender = await Sender.of(context, database);
            sender.requireCell(input.cell);
            if (input.scope !== sender.scopeOf(input.cell)) {
                throw new ServiceError("FORBIDDEN", {
                    message: `${input.cell} belongs to another scope than ${input.scope}`,
                });
            }
            await cells.publish(input.cell, input.scope, input.endpoint);

            return {};
        }),
        cell: implementation.cell.handler(async ({ input, context }) => {
            // read a cell for any verified caller
            context.requireAuthentication();

            return (await cells.cell(input.cell)) ?? null;
        }),

        // claims
        claim: implementation.claim.handler(async ({ input, context }) => {
            // reserve claims of objects in zones the caller's cells serve
            const sender = await Sender.of(context, database);
            await sender.requireClaims(input.claims);
            const taken = await cells.claim(input.claims, input.requestId);

            return { taken: [...taken] };
        }),
        confirm: implementation.confirm.handler(async ({ input, context }) => {
            // confirm the caller's own reservations and the names its objects claim
            const sender = await Sender.of(context, database);
            await sender.requireRequest(input.requestId);
            await sender.requireOwned(input.owned);
            await cells.confirm(input.requestId, input.owned);

            return {};
        }),
        release: implementation.release.handler(async ({ input, context }) => {
            // release the caller's own reservations
            const sender = await Sender.of(context, database);
            await sender.requireRequest(input.requestId);
            await cells.release(input.requestId);

            return {};
        }),
        replace: implementation.replace.handler(async ({ input, context }) => {
            // replace the claims of an object in a zone the caller's cells serve
            const sender = await Sender.of(context, database);
            await sender.requireOwned([input.owned]);
            const taken = await cells.replace(input.owned, input.requestId);

            return { taken: [...taken] };
        }),
        expired: implementation.expired.handler(async ({ input, context }) => {
            // list the expired reservations in zones the caller's cells serve
            const sender = await Sender.of(context, database);
            const expiry = await cells.expired(input.indexes);
            const claims = await sender.served(expiry.claims);

            return expiry.next === undefined ? { claims } : { claims, next: expiry.next };
        }),
        owner: implementation.owner.handler(async ({ input, context }) => {
            // find a name's owner for any verified caller
            context.requireAuthentication();

            return (await cells.owner(input.index, input.key)) ?? null;
        }),

        // accounts
        account: implementation.account.handler(async ({ input, context }) => {
            // find a handle's account for any verified caller
            context.requireAuthentication();

            return (await Resolver.account(database, input.handle)) ?? null;
        }),
    });
}

/** Report whether the caller reads an account, false for an unknown one. */
async function readsAccount(
    database: DatabaseConnection,
    authorizer: Authorizer,
    context: ServiceContext,
    scope: string,
): Promise<boolean> {
    // find the account and refuse an unknown scope
    let target: ObjectReference;
    try {
        target = await Scope.object(Snapshot.live(database), scope);
    } catch (error) {
        if (error instanceof SyncError && error.code === "NOT_FOUND") {
            return false;
        }
        throw error;
    }

    // decide reading it
    const authorization = new Authorization(authorizer, database, (within) =>
        context.access(within),
    );

    return (await authorization.check(account.permission("read"), target)).isAllowed;
}

/** The host sending a directory request, and the cells it acts for. */
export class Sender {
    /** The global database holding the zones and claims. */
    readonly database: DatabaseConnection;
    /** The sending host. */
    readonly host: Subject;
    /** The cells the host acts for: itself, and the regions it serves. */
    readonly cells: ReadonlySet<string>;
    /** Whether the host serves a region with hosts that create zones in every account. */
    readonly isRegional: boolean;

    /** Hold a verified sending host and the cells it acts for. */
    private constructor(database: DatabaseConnection, host: Subject, regions: readonly Subject[]) {
        // hold the host and every cell it acts for
        this.database = database;
        this.host = host;
        this.cells = new Set([host.id, ...regions.map((region) => region.id)]);
        this.isRegional = regions.length > 0;
    }

    /** Read the host that sends a request and refuse other callers. */
    static async of(context: ServiceContext, database: DatabaseConnection): Promise<Sender> {
        // require a host sending the request
        const authentication = context.requireAuthentication().claims;
        const sending = Caller.sender(authentication);
        if (sending === undefined || !principal.host.is(sending)) {
            throw new ServiceError("FORBIDDEN", { message: "only hosts reach the directory" });
        }

        return new Sender(
            database,
            sending,
            authentication.subjects.filter((entry) => principal.region.is(entry)),
        );
    }

    /** Read the scope a cell the host acts for belongs to. */
    scopeOf(cell: string): string {
        return cell === this.host.id ? this.host.scope : Scope.universe.id;
    }

    /** Require the host to act for a cell, the one serving a zone when given. */
    requireCell(cell: string, zoneId?: string): void {
        // refuse a cell of another host or region, reading the zone it serves when given
        if (!this.cells.has(cell)) {
            throw new ServiceError("FORBIDDEN", {
                message:
                    zoneId === undefined
                        ? `this host acts for no cell ${cell}`
                        : `${zoneId} is served by another host or region`,
            });
        }
    }

    /** Require every claim to belong to an object in, or being, a zone the host's cells serve. */
    async requireClaims(claims: readonly Claim[]): Promise<void> {
        if ((await this.served(claims)).length !== claims.length) {
            throw new ServiceError("FORBIDDEN", {
                message: "claims lie outside the zones this host serves",
            });
        }
    }

    /** Keep the claims of objects in, or being, zones the host's cells serve. */
    async served(claims: readonly Claim[]): Promise<Claim[]> {
        const served = await this.#served(claims.flatMap((entry) => [entry.scope, entry.objectId]));

        return claims.filter((entry) => served.has(entry.scope) || served.has(entry.objectId));
    }

    /** Require the objects to lie in zones the host's cells serve, with every name they claim now. */
    async requireOwned(owned: readonly ObjectClaims[]): Promise<void> {
        // read the names the directory records for the objects
        const recorded =
            owned.length === 0
                ? []
                : await this.database
                      .select()
                      .from(claimTable)
                      .where(
                          inArray(
                              claimTable.objectId,
                              owned.map((entry) => entry.objectId),
                          ),
                      );

        // require the host to reach both the written and the recorded names
        await this.requireClaims([...owned.flatMap((entry) => entry.claims), ...recorded]);
    }

    /** Require the reservations of a request to lie in zones the host's cells serve. */
    async requireRequest(requestId: string): Promise<void> {
        const reserved = await this.database
            .select()
            .from(claimTable)
            .where(eq(claimTable.requestId, requestId));
        await this.requireClaims(reserved);
    }

    /** Require the host's cells to serve a zone it places, or to create it. */
    async requirePlacement(zone: Zone): Promise<void> {
        // read the zone's current placement
        const [current] = await this.database
            .select()
            .from(zoneTable)
            .where(eq(zoneTable.id, zone.id));

        // move a served zone, or take an arriving one at the next epoch
        if (current !== undefined) {
            const isArriving =
                current.target !== null &&
                this.cells.has(current.target) &&
                zone.cell === current.target &&
                zone.epoch === current.epoch + 1;
            if (!this.cells.has(current.cell) && !isArriving) {
                throw new ServiceError("FORBIDDEN", {
                    message: `${zone.id} is served by another host or region`,
                });
            }
        }
        // create a zone in the host's account, or anywhere for a region
        else if (
            zone.epoch !== 1 ||
            !this.cells.has(zone.cell) ||
            (!this.isRegional && zone.scope !== this.host.scope)
        ) {
            throw new ServiceError("FORBIDDEN", {
                message: `this host creates no zone ${zone.id} in ${zone.scope} for ${zone.cell}`,
            });
        }
    }

    /** Find which of some scopes are zones the host's cells serve. */
    async #served(scopes: readonly string[]): Promise<Set<string>> {
        const zones =
            scopes.length === 0
                ? []
                : await this.database
                      .select({ id: zoneTable.id, cell: zoneTable.cell })
                      .from(zoneTable)
                      .where(inArray(zoneTable.id, [...new Set(scopes)]));

        return new Set(zones.filter((zone) => this.cells.has(zone.cell)).map((zone) => zone.id));
    }
}
