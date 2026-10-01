import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { createAuthentication } from "../server/tests/fixture.ts";
import { Authentication, AUTHENTICATION_HEADER } from "./authentication.ts";

test("forward a caller to a runner and read it back, refusing a malformed one", () => {
    // forward alice in a request's headers
    const alice = createAuthentication("alice");
    const headers = new Headers();
    alice.forward(headers);

    // read her back, nothing from a request without her, and refuse a malformed header
    const read = Authentication.forwarded(new Request("http://runner.test", { headers }));
    const absent = Authentication.forwarded(new Request("http://runner.test"));
    const malformed = () =>
        Authentication.forwarded(
            new Request("http://runner.test", { headers: { [AUTHENTICATION_HEADER]: "{}" } }),
        );

    expect([read?.claims, absent]).toEqual([alice.claims, null]);
    expect(malformed).toThrow(schema.Error);
});
