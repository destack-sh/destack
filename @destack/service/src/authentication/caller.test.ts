import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { createCaller } from "../server/tests/fixture.ts";
import { Caller, CALLER_HEADER } from "./caller.ts";

test("forward a caller to a runner and read it back, refusing a malformed one", () => {
    // forward alice in a request's headers
    const alice = createCaller("alice");
    const headers = new Headers();
    alice.forward(headers);

    // read her back, nothing from a request without her, and refuse a malformed header
    const read = Caller.forwarded(new Request("http://runner.test", { headers }));
    const absent = Caller.forwarded(new Request("http://runner.test"));
    const malformed = () =>
        Caller.forwarded(new Request("http://runner.test", { headers: { [CALLER_HEADER]: "{}" } }));

    expect([read?.authentication, absent]).toEqual([alice.authentication, null]);
    expect(malformed).toThrow(schema.Error);
});
