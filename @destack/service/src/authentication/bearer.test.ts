import { expect, test } from "@destack/test";
import { Bearer } from "./bearer.ts";

/** Read the bearer token of a request with the given headers. */
function read(headers: Record<string, string>): string | undefined {
    return Bearer.read(new Headers(headers));
}

test("read the bearer token of a request, and none from other or ambiguous credentials", () => {
    // read a token, and nothing from a missing, malformed, other or cookie credential
    expect([
        read({ authorization: "Bearer dst_pat_secret" }),
        read({}),
        read({ authorization: "Bearer " }),
        read({ authorization: "Bearer two tokens" }),
        read({ authorization: "Basic dXNlcjpwYXNz" }),
        read({ authorization: "Bearer dst_pat_secret", cookie: "session=1" }),
    ]).toEqual(["dst_pat_secret", undefined, undefined, undefined, undefined, undefined]);
});
