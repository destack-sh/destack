import { AccessError } from "@destack/access";
import { DatabaseError } from "@destack/db";
import { expect, refusal, test } from "@destack/test";
import { OAuthError } from "../authentication/token.ts";
import { TokenEndpoint } from "./token.ts";

test("read a refused grant check as an invalid grant, and pass an OAuth error and an outage on as they are", async () => {
    // read a refusal, an OAuth error and an outage
    const outage = DatabaseError.of("CONNECTION_CLOSED", "the database closed");
    const pending = OAuthError.of("AUTHORIZATION_PENDING", "the person has yet to decide");
    const refused = TokenEndpoint.refusal(AccessError.of("ASSERTION_EXPIRED", "assertion expired"));

    expect([
        await refusal(Promise.reject(refused)),
        TokenEndpoint.refusal(pending),
        TokenEndpoint.refusal(outage),
    ]).toEqual([["BAD_REQUEST", "INVALID_GRANT", "assertion expired"], pending, outage]);
});
