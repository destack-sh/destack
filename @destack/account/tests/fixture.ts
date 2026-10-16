import { AccessToken } from "@destack/runtime/authentication";
import type { DatabaseConnection } from "@destack/db";
import { type Claimant, defineObject, field } from "@destack/object";
import { RemoteStore, Reservation } from "@destack/object/server";
import { RequestId } from "@destack/runtime/request";
import { type Page, Scope } from "@destack/sync";
import { present, schema } from "@destack/schema";
import { account } from "../src/object/account.ts";
import { claim } from "../src/object/claim.ts";
import type { Actor } from "../src/test/index.ts";

/** A device authorization the provider grants (RFC 8628). */
export const DeviceGrant = schema.looseObject({
    /** The code the device polls the token endpoint with. */
    device_code: schema.string(),
    /** The code the person confirms on the verification page. */
    user_code: schema.string(),
    /** The verification page carrying the user code. */
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

/** An error a token endpoint answers (RFC 6749). */
export const Refusal = schema.looseObject({
    /** The error code. */
    error: schema.string(),
});

/** An authenticator enrollment: its TOTP URI and backup codes. */
export const Enrollment = schema.looseObject({
    /** The URI an authenticator app enrolls from. */
    totpURI: schema.string(),
    /** The single-use recovery codes. */
    backupCodes: schema.array(schema.string()),
});

/** A page an endpoint sends the browser on to. */
export const Redirect = schema.looseObject({
    /** The page's address. */
    url: schema.string(),
});

/** The signing keys the provider publishes (RFC 7517). */
export const KeySet = schema.looseObject({
    /** The public keys. */
    keys: schema.array(
        schema.looseObject({
            /** The key type. */
            kty: schema.string(),
            /** The key identifier. */
            kid: schema.string(),
        }),
    ),
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

/** Read who a caller is from its authentication: its subject, credential and profile. */
export function identity(current: {
    readonly subject?: unknown;
    readonly credential?: { readonly kind: string; readonly id: string };
    readonly profile: unknown;
}) {
    const { kind, id } = present(current.credential, "the caller's credential");

    return { subject: current.subject, token: { kind, id }, profile: current.profile };
}

/** Read the claims of an exchanged access token. */
export function claims(token: string): AccessToken {
    const [, payload] = token.split(".");
    const decoded = new TextDecoder().decode(
        Uint8Array.fromBase64(present(payload, "the token's payload"), { alphabet: "base64url" }),
    );

    return AccessToken.parse(JSON.parse(decoded));
}

/** Read the first page a followed replica streams, refusing a stream that ends before it. */
export async function firstPage(pages: AsyncIterable<Page>): Promise<Page> {
    const next = await pages[Symbol.asyncIterator]().next();
    if (next.done === true) {
        throw new RangeError("the replica ended before its first page");
    }

    return next.value;
}

/** Read the page a redirect response sends the browser to. */
export function locationOf(response: Response): string {
    return present(response.headers.get("location"), "the redirect location");
}

/** Read the item at an index, counting back from the end when negative, refusing a missing one. */
export function at<Item>(items: readonly Item[], index: number): Item {
    const item = items.at(index);
    if (item === undefined) {
        throw new RangeError(`a list of ${items.length} items lacks item ${index}`);
    }

    return item;
}

/** Claim a place's names at the universe as a caller, reserving and confirming them. */
export async function claimNames(
    caller: Pick<Actor, "client">,
    database: DatabaseConnection,
    claimant: Claimant,
): Promise<void> {
    const service = {
        object: claim,
        home: RemoteStore.of(caller.client.store, Scope.universe.id),
    };
    const now = Date.now();
    const request = { id: RequestId.create(now), now };
    const reservation = await Reservation.reserve(service, [claimant], [place], request);
    await reservation.commit(database);
    await reservation.confirm();
}
