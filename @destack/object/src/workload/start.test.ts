import { expect, test } from "@destack/test";
import { CLIENT_HEAD_HEADER, ClientHead } from "./start.ts";

test("restore a client's request a front door handed on with exactly its head and body, dropping the headers the hand-off added", async () => {
    // hand on a client's request with its own credential, beside the host's transport headers
    const client = new Request("https://notes.acme.com/blog?page=2", {
        method: "POST",
        headers: { authorization: "Bearer client", "x-note": "1" },
        body: "pinned",
    });
    const handed = new Request("http://runner/names", {
        method: "POST",
        headers: {
            [CLIENT_HEAD_HEADER]: ClientHead.write(client),
            authorization: "Bearer host",
            "destack-partition": "tunnel-1",
        },
        body: "pinned",
    });

    // restore the client's request, and nothing from one without a head
    const restored = ClientHead.restore(handed);
    const unhanded = ClientHead.restore(new Request("http://runner/"));
    expect({
        method: restored?.method,
        url: restored?.url,
        headers: [...(restored?.headers ?? [])],
        body: await restored?.text(),
        unhanded,
    }).toEqual({
        method: "POST",
        url: "https://notes.acme.com/blog?page=2",
        headers: [
            ["authorization", "Bearer client"],
            ["x-note", "1"],
        ],
        body: "pinned",
        unhanded: undefined,
    });
});
