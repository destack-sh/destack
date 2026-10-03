import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { createAuthentication } from "../test/fixture.ts";
import { Authentication, AUTHENTICATION_HEADER } from "./authentication.ts";

/** Read a caller forwarded in a malformed header. */
function malformed(): Authentication | null {
    return Authentication.forwarded(
        new Request("http://runner.test", { headers: { [AUTHENTICATION_HEADER]: "{}" } }),
    );
}

test("forward a caller to a runner and read it back, refusing a malformed one", () => {
    // forward alice in a request's headers
    const alice = createAuthentication("alice");
    const headers = new Headers();
    alice.forward(headers);

    // read her back, nothing from a request without her, and refuse a malformed header
    const read = Authentication.forwarded(new Request("http://runner.test", { headers }));
    const absent = Authentication.forwarded(new Request("http://runner.test"));

    expect([read?.claims, absent]).toEqual([alice.claims, null]);
    expect(malformed).toThrow(schema.Error);
});
