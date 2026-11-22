import type { DatabaseConnection } from "@destack/db/database";
import type { Environment } from "@destack/environment";
import { universe } from "@destack/access";
import { type Claimant, ClaimRefusal, defineObject, field } from "@destack/object";
import { RemoteStore, Reservation } from "@destack/object/server";
import { RequestId, type Page, Scope } from "@destack/sync";
import { schema } from "@destack/schema";
import { expect } from "@destack/test";
import { account } from "../src/object/account.ts";
import { claim } from "../src/object/claim.ts";
import type { TestCaller } from "@destack/object/test";
import accountPackage from "../src/package.ts";
import { argument, plural, t } from "@destack/locale";

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

/** A page an endpoint sends the browser on to. */
export const Redirect = schema.looseObject({
    /** The page's address. */
    url: schema.string(),
});

/** A scope with a separate database, a name unique in its account and a global identifier. */
export const place = defineObject({
    name: "place",
    identity: "space",
    plural: "places",
    title: t`${plural(argument("count"), { one: "Place", other: "Places" })}`,
    scope: account,
    fields: { name: field.string() },
    indexes: {
        name: { on: ["name"], across: () => account },
        id: { on: ["id"], across: () => universe },
    },
    container: "scope",
    access: {
        permissions: ["read"],
    },
});

/** Read the first page a followed replica streams, refusing a stream that ends before it. */
export async function firstPage(pages: AsyncIterable<Page>): Promise<Page> {
    const next = await pages[Symbol.asyncIterator]().next();
    if (next.done === true) {
        throw new RangeError("the replica ended before its first page");
    }

    return next.value;
}

/** Reserve and confirm a place's names at the universe as a caller, in an environment. */
export async function claimNames(
    caller: Pick<TestCaller, "connect">,
    database: DatabaseConnection,
    claimant: Claimant,
    environment: Environment,
): Promise<void> {
    // reserve the names at the universe, refusing a taken one, then confirm them
    const home = RemoteStore.of(caller.connect(accountPackage), Scope.universe.id);
    const request = RequestId.create(environment);
    const [reserved] = await home.mutate({
        id: RequestId.create(environment),
        calls: [claim.call("reserve", { ...claimant, request })],
    });
    Reservation.refuse([place], ClaimRefusal.parse(reserved).refused);
    await home.mutate({
        id: RequestId.create(environment),
        calls: [claim.call("confirm", { ...claimant, request })],
    });
}
