import { testSimulation } from "@destack/object/test";
import { expect, test } from "@destack/test";
import { Proof } from "@destack/identity";
import { present } from "@destack/schema";
import { ClientRegistration } from "../object/client.ts";
import { KeyRegistration } from "../object/key.ts";
import { BrowserClient, type ClientRecord, type ClientStore } from "./browser-client.ts";

/** The origin of the host the client registers and binds at. */
const ORIGIN = "https://home--home--ada.destack.space";

/** The path below which the host registers and binds the client. */
const PATH = "/.destack/client";

/** Keep a browser's client in memory, as its IndexedDB keeps it across visits. */
function memoryStore(): ClientStore {
    let stored: ClientRecord | undefined;

    return {
        load: async () => stored,
        createIfAbsent: async (created) => (stored ??= created),
        save: async (record) => {
            stored = record;
        },
        clear: async () => {
            stored = undefined;
        },
    };
}

test("register the browser's client and key once, bind each visit, and register anew once the host answers the client gone", async () => {
    // answer as a host would: register, take the key, bind until the person removes the client
    const world = testSimulation().environment("test");
    const removed = new Set<string>();
    const calls: string[] = [];
    const host = async (request: Request): Promise<Response> => {
        // register the client its registration names
        const path = new URL(request.url).pathname.slice(PATH.length);
        if (path === "") {
            const { id } = ClientRegistration.parse(await request.json());
            calls.push(`register ${id}`);

            return Response.json({ client: id }, { status: 201 });
        }
        // take the key of the client its registration names
        else if (path === "/key") {
            const { client } = KeyRegistration.parse(await request.json());
            calls.push(`key ${client}`);

            return Response.json({});
        }
        // bind the client its proof names, answering a removed one as gone
        else {
            const client = Proof.read(present(request.headers.get("dpop"), "the proof")).subject;
            calls.push(`bind ${client}`);

            return removed.has(client)
                ? new Response(null, { status: 410 })
                : Response.json({ client });
        }
    };
    const browser = new BrowserClient({
        path: PATH,
        origin: ORIGIN,
        store: memoryStore(),
        environment: { ...world, network: { ...world.network, fetch: host } },
    });
    const registration = { name: "Safari on macOS", platform: "macOS" } as const;

    // bind twice, then once more after the person removed the client
    const first = await browser.bind(registration);
    const second = await browser.bind(registration);
    removed.add(first);
    const third = await browser.bind(registration);

    expect({ isSame: second === first, isNew: third !== first, calls }).toEqual({
        isSame: true,
        isNew: true,
        calls: [
            `register ${first}`,
            `key ${first}`,
            `bind ${first}`,
            `bind ${first}`,
            `bind ${first}`,
            `register ${third}`,
            `key ${third}`,
            `bind ${third}`,
        ],
    });
});
