import { expect, refusal, test } from "@destack/test";
import { TokenResponse } from "./token.ts";

/** Answer a token endpoint's refusal as a status and a JSON or text body. */
function answered(status: number, body: unknown): Response {
    return typeof body === "string"
        ? new Response(body, { status })
        : Response.json(body, { status });
}

test("refuse a token request as its endpoint's status says, with its OAuth error, and a non-OAuth answer as a bad gateway", async () => {
    // read refusals with an OAuth error, with and without a description, and answers without one
    const grant = { error: "invalid_grant", error_description: "assertion has no standing key" };
    const refusals = await Promise.all(
        [
            answered(400, grant),
            answered(401, { error: "invalid_client" }),
            answered(403, { error: "access_denied", error_description: "machine is disabled" }),
            answered(404, "not found"),
            answered(500, "upstream failed"),
        ].map(async (response) => refusal(Promise.reject(await TokenResponse.refusal(response)))),
    );

    expect(refusals).toEqual([
        ["BAD_REQUEST", "invalid_grant: assertion has no standing key"],
        ["UNAUTHORIZED", "invalid_client"],
        ["FORBIDDEN", "access_denied: machine is disabled"],
        ["BAD_GATEWAY", "the token endpoint answered 404: not found"],
        ["BAD_GATEWAY", "the token endpoint answered 500: upstream failed"],
    ]);
});
