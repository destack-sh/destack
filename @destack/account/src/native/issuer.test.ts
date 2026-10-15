import type { Fetch } from "@destack/runtime";
import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { outcome } from "../test/outcome.ts";
import { Issuer } from "./issuer.ts";

/** The client polling for its device code. */
const CLIENT = schema.identifier("client").parse("client-01996ab0-0000-7000-8000-0000000000e1");

/** The person the issuer signs in. */
const USER = schema.identifier("user").parse("user-01996ab0-0000-7000-8000-0000000000e2");

/** An ID token of the person, its header and signature left out of what the client reads. */
const ID_TOKEN = `e30.${new TextEncoder().encode(JSON.stringify({ sub: USER })).toBase64({ alphabet: "base64url", omitPadding: true })}.signature`;

/** Answer a device token request as an issuer would, from an error code or granted tokens. */
function answer(error: string | undefined): Fetch {
    return async () =>
        error === undefined
            ? Response.json({
                  access_token: "access",
                  token_type: "Bearer",
                  expires_at: 1_767_225_660,
                  refresh_token: "refresh",
                  id_token: ID_TOKEN,
              })
            : Response.json({ error }, { status: 400 });
}

test("poll a device code into a grant, a wait, a slow down, a refusal, or a loud failure", async () => {
    // poll once per answer the device flow defines and once with an error it does not
    const poll = (error: string | undefined) =>
        new Issuer("https://accounts.destack.test/oauth", answer(error)).poll(
            CLIENT,
            "device-code",
        );
    const answers = [
        undefined,
        "authorization_pending",
        "slow_down",
        "access_denied",
        "expired_token",
    ];
    expect({
        polled: await Promise.all(answers.map(poll)),
        failed: await outcome(poll("invalid_client")),
    }).toEqual({
        polled: [
            {
                status: "granted",
                signedIn: {
                    tokens: {
                        accessToken: "access",
                        expiresAt: 1_767_225_660_000,
                        refreshToken: "refresh",
                    },
                    userId: USER,
                },
            },
            { status: "pending" },
            { status: "slow" },
            { status: "refused" },
            { status: "refused" },
        ],
        failed: "BAD_GATEWAY: the issuer refused the device code: invalid_client",
    });
});
