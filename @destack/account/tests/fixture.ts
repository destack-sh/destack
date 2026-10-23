import type { DatabaseConnection } from "@destack/db";
import type { Clock } from "@destack/environment";
import { type Claimant, defineObject, field } from "@destack/object";
import { RemoteStore, Reservation } from "@destack/object/server";
import { RequestId, type Page, Scope } from "@destack/sync";
import { schema } from "@destack/schema";
import { expect } from "@destack/test";
import { account } from "../src/object/account.ts";
import { claim } from "../src/object/claim.ts";
import type { TestCaller } from "@destack/object/test";
import accountPackage from "../src/package.ts";

/** Any string, for a value the issuer draws at random, such as a token's identifier. */
export const ANY_STRING: unknown = expect.any(String);

/** Any number, for a time Better Auth reads from the wall clock. */
export const ANY_NUMBER: unknown = expect.any(Number);

/** A device authorization the provider grants (RFC 8628). */
export const DeviceGrant = schema.looseObject({
    /** The code the device polls the token endpoint with. */
    device_code: schema.string(),
    /** The code the person confirms on the verification page. */
    user_code: schema.string(),
    /** The verification page with the user code. */
    verification_uri_complete: schema.string(),
});

/** The tokens a token endpoint issues (RFC 6749). */
export const Tokens = schema.looseObject({
    /** The kind of the access token. */
    token_type: schema.string(),
    /** The access token. */
    access_token: schema.string(),
    /** The refresh token, issued for offline access. */
    refresh_token: schema.string().exactOptional(),
    /** The ID token, issued for the openid scope. */
    id_token: schema.string().exactOptional(),
    /** The granted scopes. */
    scope: schema.string().exactOptional(),
});

/** A page an endpoint sends the browser on to. */
export const Redirect = schema.looseObject({
    /** The page's address. */
    url: schema.string(),
});

/** A scope with a separate database and a name unique in the account, like a space. */
export const place = defineObject({
    name: "place",
    identity: "space",
    plural: "places",
    scope: account,
    isScope: true,
    fields: { name: field.string() },
    indexes: { name: { on: ["name"], across: () => account } },
    permissions: ["read"],
});

/** Read the first page a followed replica streams, refusing a stream that ends before it. */
export async function firstPage(pages: AsyncIterable<Page>): Promise<Page> {
    const next = await pages[Symbol.asyncIterator]().next();
    if (next.done === true) {
        throw new RangeError("the replica ended before its first page");
    }

    return next.value;
}

/** Reserve and confirm a place's names at the universe as a caller, at a clock's time. */
export async function claimNames(
    caller: Pick<TestCaller, "client">,
    database: DatabaseConnection,
    claimant: Claimant,
    clock: Clock,
): Promise<void> {
    const service = {
        object: claim,
        home: RemoteStore.of(caller.client(accountPackage).store, Scope.universe.id),
    };
    const now = clock.now();
    const request = { id: RequestId.create(now), now };
    const reservation = await Reservation.reserve(service, [claimant], [place], request);
    await reservation.commit(database);
    await reservation.confirm();
}
