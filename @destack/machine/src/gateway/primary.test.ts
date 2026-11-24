import { PART_BYTES, PackageMount, SpaceMount } from "@destack/object";
import { PackageVersion } from "@destack/package";
import { schema } from "@destack/schema";
import stackPackage from "@destack/stack/package";
import { expect, refusal, test } from "@destack/test";
import { ScopePath } from "@destack/view/declare";
import { ScopePrimary } from "./primary.ts";

/** A space another machine serves. */
const SPACE = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000c1");

/** Where the page's calls come from. */
const ORIGIN = "https://tasks.test";

/** The machine serving the space. */
const MACHINE = "https://machine.test";

/** Forward requests to a primary, which answers where they went and how long they were. */
function open(home: ScopePrimary["home"]) {
    const stack = PackageVersion.of(stackPackage);
    const mount = `${SpaceMount.path(SPACE)}${PackageMount.path(stack.id)}`;
    const primary = new ScopePrimary(
        { scope: SPACE, package: stack, url: `${MACHINE}${mount}`, home },
        async (sent) =>
            Response.json({
                url: sent.url,
                length: (await sent.arrayBuffer()).byteLength,
                cookie: sent.headers.get("cookie"),
            }),
    );
    const send = async (path: string, method: string, length: number) => {
        const url = new URL(`${ORIGIN}${ScopePath.of(SPACE)}${path}`);
        const headers = { cookie: "session" };
        const request =
            method === "GET"
                ? new Request(url.href, { headers })
                : new Request(url.href, { method, body: new Uint8Array(length), headers });
        const answer = await primary.forward(request, url);

        return schema.json().parse(await answer.json());
    };

    return { send, mount };
}

test("forward a part of a file whole, and refuse a longer one and a call as long", async () => {
    const { send, mount } = open({ kind: "space", origin: MACHINE });
    const below = PackageMount.path(stackPackage.id);

    expect({
        part: await send(`${below}/store/file/digest`, "PUT", PART_BYTES),
        read: await send(`${below}/store/file/a/b/c/d/0`, "GET", 0),
        longer: await refusal(send(`${below}/store/file/digest`, "PUT", PART_BYTES + 1)),
        call: await refusal(send(`${below}/installations`, "POST", PART_BYTES)),
    }).toEqual({
        part: { url: `${MACHINE}${mount}/store/file/digest`, length: PART_BYTES, cookie: null },
        read: { url: `${MACHINE}${mount}/store/file/a/b/c/d/0`, length: 0, cookie: null },
        longer: [
            "BAD_REQUEST",
            "BODY_INVALID",
            `'a page's call' is invalid: it exceeds ${PART_BYTES} bytes`,
        ],
        call: [
            "BAD_REQUEST",
            "BODY_INVALID",
            "'a page's call' is invalid: it exceeds 1048576 bytes",
        ],
    });
});

test("forward files of the universe's rows, and refuse anything else of it", async () => {
    const { send, mount } = open({ kind: "universe" });
    const below = PackageMount.path(stackPackage.id);
    const denied = [
        "NOT_FOUND",
        "SCOPE_PATH_NOT_FOUND",
        "scope path not found: a page must connect to the universe's primary at its store",
    ];

    expect({
        part: await send(`${below}/store/file/digest`, "PUT", PART_BYTES),
        beside: await refusal(send(`${below}/store/files`, "GET", 0)),
        call: await refusal(send(`${below}/installations`, "GET", 0)),
    }).toEqual({
        part: { url: `${MACHINE}${mount}/store/file/digest`, length: PART_BYTES, cookie: null },
        beside: denied,
        call: denied,
    });
});
