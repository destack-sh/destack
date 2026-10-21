import { expect, test } from "@destack/test";
import { copyRequest } from "./request.ts";

test("copy a request with its headers replaced, empty ones included", () => {
    // strip every header of a request, and replace one of another
    const sent = new Request("https://egress.test/a", {
        headers: { authorization: "Bearer secret", cookie: "c" },
    });
    const stripped = copyRequest(sent, { headers: new Headers() }, "https://api.test/b");
    const replaced = copyRequest(stripped, { headers: { accept: "text/plain" } });

    expect({
        stripped: [stripped.url, [...stripped.headers]],
        replaced: [...replaced.headers],
    }).toEqual({
        stripped: ["https://api.test/b", []],
        replaced: [["accept", "text/plain"]],
    });
});
