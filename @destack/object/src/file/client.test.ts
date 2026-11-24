import { Digest, schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import { CallError } from "../error/index.ts";
import { ObjectFileClient, ObjectFileInput, PART_BYTES, type Upload } from "./index.ts";

/** A server that takes parts and makes files of them, unless told to answer otherwise. */
function serve(
    answer: (request: Request, count: number) => Response | Error | undefined = () => undefined,
) {
    const simulation = testSimulation();
    const environment = simulation.environment("client");
    const requests: string[] = [];
    const client = new ObjectFileClient({
        url: "https://space.test",
        environment,
        fetch: async (request) => {
            const { pathname } = new URL(request.url);
            requests.push(`${request.method} ${pathname.split("/").length}`);
            const told = answer(request, requests.length);
            if (told instanceof Error) {
                throw told;
            } else if (told !== undefined) {
                return told;
            }
            request.signal.throwIfAborted();
            const bytes = await request.bytes();
            if (request.method === "PUT") {
                const digest = await Digest.of(bytes, environment.crypto);

                return pathname.endsWith(digest)
                    ? Response.json({
                          digest,
                          size: bytes.byteLength,
                          expiresAt: 1,
                          signature: "AAAA",
                      })
                    : Response.json({ message: "other bytes" }, { status: 400 });
            }
            const input = ObjectFileInput.parse(JSON.parse(new TextDecoder().decode(bytes)));
            const { parts, ...file } = input;

            return Response.json({
                ...file,
                parts: parts.map(({ digest }) => digest),
                size: parts.reduce((total, { size }) => total + size, 0),
                picture: null,
                signature: "AAAA",
            });
        },
    });

    return { simulation, environment, requests, client };
}

/** Watch how far an upload is at each change. */
function progressOf(upload: Upload): string[] {
    const seen: string[] = [];
    upload.watch(() => void seen.push(`${upload.status} ${String(upload.loaded)}`));

    return seen;
}

/** What the uploads are called and are. */
const INPUT = { name: "a.bin", type: "application/octet-stream" };

test("send a file in parts in its order, telling how far it is", async () => {
    const { client, environment, requests } = serve();
    const body = new Blob([new Uint8Array(PART_BYTES).fill(1), "end"]);
    const upload = client.upload(body, INPUT);
    const seen = progressOf(upload);
    const file = await upload.done;
    const digests = [
        await Digest.of(new Uint8Array(PART_BYTES).fill(1), environment.crypto),
        await Digest.of(new TextEncoder().encode("end"), environment.crypto),
    ];

    expect({
        file,
        seen: seen.slice(1),
        requests: requests.toSorted(),
        size: upload.size,
    }).toEqual({
        file: { ...INPUT, parts: digests, size: PART_BYTES + 3, picture: null, signature: "AAAA" },
        seen: [`sending ${String(PART_BYTES + 3)}`, `done ${String(PART_BYTES + 3)}`],
        requests: ["POST 3", "PUT 4", "PUT 4"],
        size: PART_BYTES + 3,
    });
});

test("send a part again after a lost connection and a busy server, and fail on a refusal", async () => {
    const { client, requests, simulation } = serve((_, count) =>
        count === 1
            ? new TypeError("connection lost")
            : count === 2
              ? new Response(null, { status: 503 })
              : undefined,
    );
    const upload = client.upload(new Blob(["hello"]), INPUT);
    await simulation.sleep(5000);
    const file = await upload.done;
    const refusing = serve(() =>
        Response.json(CallError.http("BAD_REQUEST", { message: "part is invalid" }).toJSON(), {
            status: 400,
        }),
    );
    const refused = refusing.client.upload(new Blob(["hello"]), INPUT);
    const failure = await refused.done.catch((error: unknown) => String(error));

    const lost = serve(() => new TypeError("connection lost"));
    const dropped = lost.client.upload(new Blob(["hello"]), INPUT);
    await lost.simulation.sleep(10_000);
    const loss = await dropped.done.catch((error: unknown) => String(error));

    expect({
        size: file.size,
        requests,
        failure,
        status: refused.status,
        loss,
        reasons: [upload.reason, refused.reason, dropped.reason],
    }).toEqual({
        size: 5,
        requests: ["PUT 4", "PUT 4", "PUT 4", "POST 3"],
        failure: "Error: part is invalid",
        status: "failed",
        loss: "TypeError: connection lost",
        reasons: [undefined, "refusal", "connection"],
    });
});

test("wait as long as a server over its rate asks, however often", async () => {
    const { client, requests, simulation } = serve((_, count) =>
        count <= 6
            ? new Response(null, { status: 429, headers: { "retry-after": "60" } })
            : undefined,
    );
    const upload = client.upload(new Blob(["hello"]), INPUT);
    await simulation.sleep(6 * 60_000 - 1);
    const before = [upload.status, requests.length];
    await simulation.sleep(1);
    const file = await upload.done;

    expect({ before, size: file.size, requests: requests.length }).toEqual({
        before: ["sending", 6],
        size: 5,
        requests: 8,
    });
});

test("pause before the next part, go on, and cancel for good", async () => {
    const { client, requests } = serve();
    const paused = client.upload(new Blob(["hello"]), { ...INPUT, type: "" });
    paused.pause();
    const before = [paused.status, requests.length];
    paused.resume();
    await paused.done;
    const cancelled = client.upload(new Blob(["hello"]), INPUT);
    cancelled.cancel();
    const failure = await cancelled.done.catch((error: unknown) => String(error));

    expect({
        type: (await paused.done).type,
        before,
        after: paused.status,
        failure,
        status: cancelled.status,
    }).toEqual({
        type: "application/octet-stream",
        before: ["paused", 0],
        after: "done",
        failure: "AbortError: the upload was cancelled",
        status: "cancelled",
    });
});

test("write the address of an object's file", () => {
    const { client } = serve();
    const id = schema.identifier("photo").parse("photo-01996ab0-0000-7000-8000-0000000000c1");

    expect(
        client.address(
            {
                scope: "space-a",
                packageId: schema
                    .identifier("package")
                    .parse("package-01996ab0-0000-7000-8000-0000000000da"),
                type: "photo",
                id,
            },
            "image",
        ),
    ).toBe(`https://space.test/store/file/space-a/photo/${id}/image`);
});
