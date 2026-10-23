import { expect, refusal, test } from "@destack/test";
import { RequestBody } from "./body.ts";

/** Build a request with a body. */
function post(body: string, headers: Record<string, string> = {}): Request {
    return new Request("https://host/journal", { method: "POST", body, headers });
}

test("read a JSON body within its limit", async () => {
    expect(await RequestBody.json(post('{"calls":[]}'), "a journal batch", 64)).toEqual({
        calls: [],
    });
});

test("refuse a body whose declared or streamed size passes the limit", async () => {
    const body = "x".repeat(65);
    expect(await refusal(RequestBody.text(post(body), "a journal batch", 64))).toEqual([
        "PAYLOAD_TOO_LARGE",
        "BODY_TOO_LARGE",
        "a journal batch exceeds 64 bytes",
    ]);
    expect(
        await refusal(
            RequestBody.text(post("{}", { "content-length": "65" }), "a journal batch", 64),
        ),
    ).toEqual(["PAYLOAD_TOO_LARGE", "BODY_TOO_LARGE", "a journal batch exceeds 64 bytes"]);
});
