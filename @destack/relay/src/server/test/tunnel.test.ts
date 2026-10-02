import { expect, test } from "@destack/test";
import { identifier } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import type { Alarm } from "@destack/service/control";
import { MAX_STREAMS, Session } from "../../session/index.ts";
import { Tunnel } from "../tunnel.ts";

/** The host whose tunnel the scenarios keep. */
const hostId = identifier("host").parse("host-01996ab0-0000-7000-8000-0000000000d1");

/** A host's end of a tunnel joined to the relay in memory, answering with its label. */
async function connect(tunnel: Tunnel, label: string, lapsesAt: number) {
    // join the host's session to the relay's
    let relay: Session | undefined;
    const host: Session = new Session(
        { send: (message) => setImmediate(() => relay?.receive(message)), close: () => {} },
        "client",
        {
            accept: (stream) =>
                void stream.respond(new Response(`${label} ${new URL(stream.head.url).pathname}`)),
        },
    );
    relay = tunnel.attach(
        {
            send: (message) => setImmediate(() => host.receive(message)),
            close: () => host.terminate(),
        },
        lapsesAt,
    );

    return host;
}

/** Fail on a reported failure, which no scenario expects. */
function fail(error: unknown): never {
    throw error;
}

/** An alarm recording the time it is set to. */
function recording(): Alarm & { at?: number } {
    const alarm: Alarm & { at?: number } = {
        setAlarm: async (at) => void (alarm.at = at),
        deleteAlarm: async () => void delete alarm.at,
    };

    return alarm;
}

/** Forward a request for a path, answering its body or the refusal's code. */
function forward(tunnel: Tunnel, path: string): Promise<string> {
    return tunnel.fetch(new Request(`https://notes.personal.acme.destack.space${path}`)).then(
        (response) => response.text(),
        (error: ServiceError<string, unknown>) => error.code,
    );
}

test("forward through the newest tunnel, close each tunnel once its token lapses, and set the alarm to the earliest lapse", async () => {
    const alarm = recording();
    const tunnel = new Tunnel(hostId, { verify: async () => 0, alarm, report: fail });

    // open an older and a newer tunnel
    await connect(tunnel, "older", 1000);
    await connect(tunnel, "newer", 2000);
    const both = [await forward(tunnel, "/a"), alarm.at];

    // close the older at its lapse, then the newer at its own
    await tunnel.lapse(1000);
    const newer = [await forward(tunnel, "/b"), alarm.at];
    await tunnel.lapse(2000);

    expect([both, newer, await forward(tunnel, "/c"), alarm.at, tunnel.isEmpty]).toEqual([
        ["newer /a", 1000],
        ["newer /b", 2000],
        "SERVICE_UNAVAILABLE",
        undefined,
        true,
    ]);
});

test("renew a tunnel's token through a stream the host opens, and close the tunnel on the verifier's refusal", async () => {
    const alarm = recording();
    const tunnel = new Tunnel(hostId, {
        verify: async (request) => {
            if (request.headers.get("authorization") !== "Bearer fresh") {
                throw new ServiceError("UNAUTHORIZED", { message: "invalid token" });
            }

            return 5000;
        },
        alarm,
        report: fail,
    });
    const host = await connect(tunnel, "host", 1000);

    // renew with a fresh token, reach no other path, then close the tunnel on a stale one
    const renew = (path: string, token: string) =>
        host
            .fetch(
                new Request(`https://relay.destack.space${path}`, {
                    method: "PUT",
                    headers: { authorization: `Bearer ${token}` },
                }),
            )
            .then((response) => response.status);
    const renewed = [await renew("/tunnel", "fresh"), alarm.at, await renew("/other", "fresh")];
    const refused = await renew("/tunnel", "stale");

    // lapse the refused tunnel at once, and close it as the alarm wakes
    if (alarm.at === undefined) {
        throw new TypeError("the refused tunnel set no alarm");
    }
    const isLapsed = alarm.at <= Date.now();
    await tunnel.lapse();
    expect([renewed, refused, isLapsed, tunnel.isEmpty, alarm.at]).toEqual([
        [204, 5000, 404],
        401,
        true,
        true,
        undefined,
    ]);
});

test("refuse a request beyond a host's open requests, and time out the ones the host answers no head for", async () => {
    // join a host that never answers, with a short answer timeout
    const tunnel = new Tunnel(hostId, {
        verify: async () => 0,
        alarm: recording(),
        report: fail,
        answerTimeout: 50,
    });
    let relay: Session | undefined;
    const host: Session = new Session(
        { send: (message) => setImmediate(() => relay?.receive(message)), close: () => {} },
        "client",
        { accept: () => {} },
    );
    relay = tunnel.attach(
        {
            send: (message) => setImmediate(() => host.receive(message)),
            close: () => host.terminate(),
        },
        Number.MAX_SAFE_INTEGER,
    );

    // fill the host's open requests, refuse one more, then time the open ones out
    const open = Array.from({ length: MAX_STREAMS }, () => forward(tunnel, "/slow"));
    const refused = await forward(tunnel, "/one-more");
    const timedOut = new Set(await Promise.all(open));
    expect([refused, [...timedOut], relay.size]).toEqual([
        "SERVICE_UNAVAILABLE",
        ["GATEWAY_TIMEOUT"],
        0,
    ]);
});
