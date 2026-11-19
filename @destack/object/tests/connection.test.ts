import { TOKEN_LIFETIME_MILLISECONDS } from "@destack/access/token";
import { type ClientSocket, WebSocketUpgrade } from "@destack/environment";
import { FOLLOWER_REVISION } from "@destack/sync/test";
import { type Page, RequestId, type Subscription } from "@destack/sync";
import { schema } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { CONNECTION_SCOPE_PARAMETER, ObjectConnection } from "../src/client/index.ts";
import { CallError } from "../src/error/call.ts";
import { HibernationHost } from "../src/workload/socket.ts";
import { TEST_CONNECTION_URL } from "../src/test/caller.ts";
import { notebook } from "../src/test/note.ts";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { tapSocket } from "./fixture/socket.ts";
import { users } from "./fixture/user.ts";

/** A space outside the notes' space, which a socket confined to the notes' space refuses. */
const OTHER_SPACE_ID = schema
    .identifier("space")
    .parse("space-01996ab0-0000-7000-8000-00000000b00f");

/** More follows than a browser opens HTTP/1.1 connections to one origin, which is six. */
const FOLLOWS = 8;

/** Connect alice to the served notes, recording each socket the connection opens. */
async function connect() {
    const served = await serveNotes();
    const opened: ClientSocket[] = [];
    const websocket = served.websocket(users.alice);
    const connection = ObjectConnection.open({
        scope: spaceId,
        url: TEST_CONNECTION_URL,
        authorize: served.authorize(users.alice),
        websocket: (url, protocols) => {
            const socket = tapSocket(websocket(url, protocols), (frame) => frame);
            opened.push(socket);

            return socket;
        },
        environment: served.environment,
    });

    return { served, connection, opened };
}

/** Follow alice's access rows of the space under a name, from a snapshot. */
function access(name: string): Subscription {
    return {
        name,
        publication: "access",
        scope: spaceId,
        target: spaceId,
        parameters: {},
        revision: FOLLOWER_REVISION,
    };
}

/** Read a follow's pages up to its first complete one. */
async function complete(pages: AsyncIterable<Page>): Promise<Page> {
    for await (const page of pages) {
        if (page.complete) {
            return page;
        }
    }
    throw new TypeError("the follow ended before a complete page");
}

test("follow more subscriptions than a browser's six connections and call procedures over one socket", async () => {
    const { connection, opened } = await connect();
    const stopping = new AbortController();

    // follow eight subscriptions at once while calling the store
    const follows = Array.from({ length: FOLLOWS }, (_, index) =>
        complete(connection.stream(access(`access-${index}`), stopping.signal)),
    );
    const watermark = await connection.store.watermark({ scope: spaceId, homes: [] });
    const pages = await Promise.all(follows);
    stopping.abort();

    // expect a snapshot for each follow and the space's watermark, all over the one socket
    expect({
        sockets: opened.length,
        snapshots: pages.map((page) => page.reset),
        log: watermark.log,
    }).toEqual({
        sockets: 1,
        snapshots: Array(FOLLOWS).fill(true),
        log: pages[0]?.position.log,
    });
});

test("stop one follow, leaving the others and the calls running on the socket", async () => {
    const { connection, opened } = await connect();
    const stopped = new AbortController();
    const running = new AbortController();

    // follow two subscriptions, then stop the first
    await complete(connection.stream(access("stopped"), stopped.signal));
    const kept = connection.stream(access("running"), running.signal);
    stopped.abort();

    // read the other one and call the store afterwards
    const page = await complete(kept);
    const watermark = await connection.store.watermark({ scope: spaceId, homes: [] });
    running.abort();

    expect({ sockets: opened.length, reset: page.reset, log: watermark.log }).toEqual({
        sockets: 1,
        reset: true,
        log: page.position.log,
    });
});

test("fail a dropped socket's follows as unavailable, and open a new socket at the next call", async () => {
    const { connection, opened } = await connect();
    const stopping = new AbortController();

    // follow, then drop the socket under the follow
    const pages = connection.stream(access("dropped"), stopping.signal)[Symbol.asyncIterator]();
    const first = await pages.next();
    opened[0]?.close(1001, "going away");
    const dropped = await refusal(complete({ [Symbol.asyncIterator]: () => pages }));

    // call again over a new socket
    const watermark = await connection.store.watermark({ scope: spaceId, homes: [] });
    stopping.abort();

    expect({ dropped, sockets: opened.length, log: watermark.log }).toEqual({
        dropped: [
            "SERVICE_UNAVAILABLE",
            "CONNECTION_UNAVAILABLE",
            `connection is unavailable: the socket to '${TEST_CONNECTION_URL}' closed with '1001': 'going away'`,
        ],
        sockets: 2,
        log: first.done === true ? undefined : first.value.position.log,
    });
});

test("open a socket anew at the next call after an open that failed before its socket", async () => {
    // connect alice through a credential exchange that loses its connection once, as a restarting issuer does
    const served = await serveNotes();
    const authorize = served.authorize(users.alice);
    const websocket = served.websocket(users.alice);
    const opened: ClientSocket[] = [];
    let isLost = true;
    const connection = ObjectConnection.open({
        scope: spaceId,
        url: TEST_CONNECTION_URL,
        authorize: async (request) => {
            if (isLost) {
                isLost = false;
                throw new TypeError("network connection lost");
            }

            return await authorize(request);
        },
        websocket: (url, protocols) => {
            const socket = websocket(url, protocols);
            opened.push(socket);

            return socket;
        },
        environment: served.environment,
    });

    // fail the first call as a lost connection before any socket, then answer the next over the one socket it opens
    const lost = await refusal(connection.store.watermark({ scope: spaceId, homes: [] }));
    await connection.store.watermark({ scope: spaceId, homes: [] });
    connection.close();

    expect({ lost, sockets: opened.length }).toEqual({
        lost: [
            "SERVICE_UNAVAILABLE",
            "CONNECTION_UNAVAILABLE",
            `connection is unavailable: the credential for '${TEST_CONNECTION_URL}' did not arrive: network connection lost`,
        ],
        sockets: 1,
    });
});

test("keep a socket open past its credential's lapse, its calls answering as before", async () => {
    const { served, connection, opened } = await connect();
    const stopping = new AbortController();

    // follow, then let the credential the socket opened with lapse
    const pages = connection.stream(access("lapsed"), stopping.signal)[Symbol.asyncIterator]();
    const first = await pages.next();
    await served.simulation.sleep(TOKEN_LIFETIME_MILLISECONDS + 1);

    // call the store afterwards on the same socket
    const watermark = await connection.store.watermark({ scope: spaceId, homes: [] });
    stopping.abort();

    expect({ sockets: opened.length, state: opened[0]?.readyState, log: watermark.log }).toEqual({
        sockets: 1,
        state: 1,
        log: first.done === true ? undefined : first.value.position.log,
    });
});

test("fail a refused socket's calls with the refusal its upgrade answered", async () => {
    const served = await serveNotes();
    const opened: ClientSocket[] = [];

    // refuse every upgrade with a declared failure, as a gateway answers
    const refused = new CallError("FORBIDDEN", {
        reason: "GRANT_INVALID",
        domain: "@destack/machine",
        message: "authentication required: grant 'grant-1' is revoked",
    });
    const host = new HibernationHost(
        async () => new Response(null, { status: 204 }),
        served.environment.telemetry,
    );
    const connection = ObjectConnection.open({
        scope: spaceId,
        url: TEST_CONNECTION_URL,
        websocket: (url, protocols) => {
            const socket = host.open(
                async () => Response.json(refused.toJSON(), { status: refused.status }),
                WebSocketUpgrade.to(url, protocols),
            );
            opened.push(socket);

            return socket;
        },
        environment: served.environment,
    });

    // call twice, each over a socket of its own
    const first = await refusal(connection.store.watermark({ scope: spaceId, homes: [] }));
    const second = await refusal(connection.store.watermark({ scope: spaceId, homes: [] }));

    // expect each call to fail with the refusal, not as an unavailable connection
    const expected = [
        "FORBIDDEN",
        "GRANT_INVALID",
        "authentication required: grant 'grant-1' is revoked",
    ];
    expect({ first, second, sockets: opened.length }).toEqual({
        first: expected,
        second: expected,
        sockets: 2,
    });
});

test("refuse a push outside the scope a socket's address confines it to", async () => {
    // connect alice through an address confined to the notes' space, as a page's primary connects
    const served = await serveNotes();
    const confined = new URL(TEST_CONNECTION_URL);
    confined.searchParams.set(CONNECTION_SCOPE_PARAMETER, spaceId);
    const connection = ObjectConnection.open({
        scope: spaceId,
        url: confined.href,
        authorize: served.authorize(users.alice),
        websocket: served.websocket(users.alice),
        environment: served.environment,
    });

    // expect a push to another space refused as outside the socket's scope
    const pushed = connection.store.push({
        scope: OTHER_SPACE_ID,
        mutations: [
            {
                id: RequestId.create(served.environment),
                calls: [
                    {
                        type: notebook.typeReference,
                        method: "create",
                        release: notebook.package.version,
                        edition: 1,
                        input: { name: "elsewhere" },
                    },
                ],
            },
        ],
    });
    expect(await refusal(pushed)).toEqual([
        "NOT_FOUND",
        "SCOPE_OUTSIDE_REQUEST_NOT_FOUND",
        `scope '${OTHER_SPACE_ID}' in the request's scope not found`,
    ]);
});
