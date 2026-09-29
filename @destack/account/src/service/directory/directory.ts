import { Cell, Claim, ClaimOwner, ObjectClaims, Zone } from "@destack/directory";
import { identifier, schema } from "@destack/schema";
import { defineProcedure, eventIterator } from "@destack/service";
import { AccountHandle } from "../../object/handle.ts";

/** The request whose write reserved claims. */
const Request = schema.object({
    /** The request's identifier. */
    requestId: schema.string().min(1),
});

/** The universe's directory behind the account service. */
export const directory = {
    // zones and cells
    /** Place a zone in its cell at an epoch. */
    place: procedure("activity")
        .route({ method: "POST", path: "/directory/zones" })
        .input(schema.object({ zone: Zone }))
        .output(schema.object({})),
    /** Withdraw a zone its cell serves at an epoch. */
    withdraw: procedure("activity")
        .route({ method: "POST", path: "/directory/zones/withdraw" })
        .input(schema.object({ zone: Zone }))
        .output(schema.object({})),
    /** Find the zone of a scope's databases. */
    locate: procedure("access")
        .route({ method: "GET", path: "/directory/zones/{scope}" })
        .input(schema.object({ scope: schema.string().min(1) }))
        .output(Zone.nullable()),
    /** List the zones a scope contains, such as an account's spaces. */
    list: procedure("access")
        .route({ method: "GET", path: "/directory/scopes/{scope}/zones" })
        .input(schema.object({ scope: schema.string().min(1) }))
        .output(schema.array(Zone)),
    /** Mark a zone the calling host's cell serves as moving to a target cell. */
    move: procedure("activity")
        .route({ method: "POST", path: "/directory/zones/move" })
        .input(schema.object({ zone: Zone, target: schema.string().min(1) }))
        .output(schema.object({})),
    /** Follow the zones moving to a cell the calling host acts for. */
    incoming: procedure("access")
        .route({ method: "POST", path: "/directory/cells/{cell}/incoming" })
        .input(schema.object({ cell: schema.string().min(1) }))
        .output(eventIterator(schema.array(Zone))),
    /** Record the URL a cell the calling host acts for answers at. */
    publish: procedure("activity")
        .route({ method: "PUT", path: "/directory/cells/{cell}" })
        .input(
            schema.object({
                /** The region or host. */
                cell: schema.string().min(1),
                /** The scope it belongs to. */
                scope: schema.string().min(1),
                /** The URL it answers at. */
                endpoint: schema.url(),
            }),
        )
        .output(schema.object({})),
    /** Read a cell and the URL it answers at. */
    cell: procedure("access")
        .route({ method: "GET", path: "/directory/cells/{cell}" })
        .input(schema.object({ cell: schema.string().min(1) }))
        .output(Cell.nullable()),

    // claims
    /** Reserve a request's claims until its write commits. */
    claim: procedure("activity")
        .route({ method: "POST", path: "/directory/claims" })
        .input(Request.extend({ claims: schema.array(Claim).min(1) }))
        .output(schema.object({})),
    /** Confirm a request's reserved claims. */
    confirm: procedure("activity")
        .route({ method: "POST", path: "/directory/claims/confirm" })
        .input(Request.extend({ owned: schema.array(ObjectClaims) }))
        .output(schema.object({})),
    /** Release the reservations of a request with a failed write. */
    release: procedure("activity")
        .route({ method: "POST", path: "/directory/claims/release" })
        .input(Request)
        .output(schema.object({})),
    /** Replace an object's claims after a write that reserved none. */
    replace: procedure("activity")
        .route({ method: "PUT", path: "/directory/claims" })
        .input(Request.extend({ owned: ObjectClaims }))
        .output(schema.object({})),
    /** List the expired reservations of some indexes. */
    expired: procedure("access")
        .route({ method: "POST", path: "/directory/claims/expired" })
        .input(schema.object({ indexes: schema.array(schema.string().min(1)) }))
        .output(
            schema.object({
                /** The expired reservations in zones the caller's cells serve. */
                claims: schema.array(Claim),
                /** When the next reservation expires, in UTC epoch milliseconds. */
                next: schema.number().int().optional(),
            }),
        ),
    /** Find the object owning a confirmed name. */
    owner: procedure("access")
        .route({ method: "POST", path: "/directory/claims/owner" })
        .input(schema.object({ index: schema.string().min(1), key: schema.string() }))
        .output(ClaimOwner.nullable()),

    // accounts
    /** Find the account with a handle, absent once its deletion was requested. */
    account: procedure("access")
        .route({ method: "GET", path: "/directory/accounts/{handle}" })
        .input(schema.object({ handle: AccountHandle }))
        .output(identifier("account").nullable()),
};

/** Declare a directory procedure recording its writes, and its reads when reads are audited. */
function procedure(audit: "activity" | "access") {
    return defineProcedure({ authentication: "identity", permission: null, audit });
}
