import { Failure } from "@destack/error";
import { expect, refusal, test } from "@destack/test";
import { TokenResponse } from "./token.ts";

/** Answer a token endpoint's refusal as a status and a JSON or text body. */
function answered(status: number, body: unknown): Response {
    return typeof body === "string"
        ? new Response(body, { status })
        : Response.json(body, { status });
}

test("refuse a token request with its OAuth error, else as unavailable or as a bad gateway", async () => {
    // read refusals with an OAuth error, with and without a description, and answers without one
    const grant = { error: "invalid_grant", error_description: "assertion has no standing key" };
    const refusals = await Promise.all(
        [
            answered(400, grant),
            answered(401, { error: "invalid_client" }),
            answered(400, { error: "access_denied", error_description: "machine is disabled" }),
            answered(400, { error: "server_busy" }),
            answered(404, "not found"),
            answered(500, "upstream failed"),
            answered(503, "starting"),
            answered(429, "slow down"),
        ].map(async (response) =>
            refusal(Promise.reject(await TokenResponse.refusal(response, 0))),
        ),
    );

    expect(refusals).toEqual([
        [
            "BAD_REQUEST",
            "GRANT_INVALID",
            "the token endpoint refused the request: invalid_grant: assertion has no standing key",
        ],
        [
            "UNAUTHORIZED",
            "CLIENT_UNAUTHENTICATED",
            "the token endpoint refused the request: invalid_client",
        ],
        [
            "BAD_REQUEST",
            "CONSENT_INVALID",
            "the token endpoint refused the request: access_denied: machine is disabled",
        ],
        [
            "BAD_GATEWAY",
            "TOKEN_ENDPOINT_UNREACHABLE",
            `could not call the token endpoint: 400 '{"error":"server_busy"}'`,
        ],
        [
            "BAD_GATEWAY",
            "TOKEN_ENDPOINT_UNREACHABLE",
            "could not call the token endpoint: 404 'not found'",
        ],
        [
            "SERVICE_UNAVAILABLE",
            "TOKEN_ENDPOINT_UNAVAILABLE",
            "token endpoint is unavailable: it answered '500'",
        ],
        [
            "SERVICE_UNAVAILABLE",
            "TOKEN_ENDPOINT_UNAVAILABLE",
            "token endpoint is unavailable: it answered '503'",
        ],
        [
            "SERVICE_UNAVAILABLE",
            "TOKEN_ENDPOINT_UNAVAILABLE",
            "token endpoint is unavailable: it answered '429'",
        ],
    ]);
});

test("read the wait a failing token endpoint states, in seconds or as a date, and none without one", async () => {
    // answer as limited with a wait of two seconds, as failing until a date, and as failing alone
    const now = Date.parse("2026-11-21T10:00:00Z");
    const waits = await Promise.all(
        [
            new Response("slow down", { status: 429, headers: { "retry-after": "2" } }),
            new Response("starting", {
                status: 503,
                headers: { "retry-after": "Sat, 21 Nov 2026 10:00:05 GMT" },
            }),
            new Response("upstream failed", { status: 500 }),
        ].map(async (response) => {
            const failure = Failure.of(await TokenResponse.refusal(response, now));

            return [failure.reason, Failure.retryAfter(failure), Failure.isTransient(failure)];
        }),
    );

    expect(waits).toEqual([
        ["TOKEN_ENDPOINT_UNAVAILABLE", 2000, true],
        ["TOKEN_ENDPOINT_UNAVAILABLE", 5000, true],
        ["TOKEN_ENDPOINT_UNAVAILABLE", undefined, true],
    ]);
});
