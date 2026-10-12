import { expect, test } from "@destack/test";
import { DOMAINS } from "@destack/account/address";
import { Clock, schema } from "@destack/schema";
import type { ServiceError } from "@destack/service/error";
import { MAX_STREAMS, Session } from "../session/index.ts";
import { machineTokens } from "../test/token.ts";
import { Tunnel, type TunnelOptions } from "./tunnel.ts";

/** The machine whose tunnel the scenarios keep. */
const machineId = schema
    .identifier("machine")
    .parse("machine-01996ab0-0000-7000-8000-0000000000d1");

/** The name the relay routes to the machine. */
const NAME = "laptop--acme.destack.computer";

/** The relay's tunnel URL the machine dials. */
const TUNNEL_URL = "https://relay.destack.space/tunnel";

/** Keep the test machine's tunnel, recording its renewals and failing on any report. */
function options(renewals: [string, number][] = [], answerTimeout?: number): TunnelOptions {
    return {
        url: TUNNEL_URL,
        tokens: machineTokens(machineId),
        report: fail,
        domains: DOMAINS,
        renewed: async (id, lapsesAt) => void renewals.push([id, lapsesAt]),
        clock: Clock.system,
        ...(answerTimeout === undefined ? {} : { answerTimeout }),
    };
}

/** A machine's end of a connection joined to the relay in memory, answering with its label. */
function connect(tunnel: Tunnel, label: string): Session {
    // join the machine's session to the relay's
    let relay: Session | undefined;
    const machine: Session = new Session(
        { send: (message) => setImmediate(() => relay?.receive(message)), close: () => {} },
        "client",
        {
            accept: (stream) =>
                void stream.respond(new Response(`${label} ${new URL(stream.head.url).pathname}`)),
        },
    );
    relay = tunnel.attach(label, {
        send: (message) => setImmediate(() => machine.receive(message)),
        close: () => machine.terminate(),
    });

    return machine;
}

/** Fail on a reported failure, which no scenario expects. */
function fail(error: unknown): never {
    throw error;
}

/** Forward a request for a path, answering its body or the refusal's code. */
function forward(tunnel: Tunnel, path: string): Promise<string> {
    return tunnel.fetch(new Request(`https://notes--personal--acme.destack.space${path}`)).then(
        (response) => response.text(),
        (error: ServiceError<string, unknown>) => error.code,
    );
}

test("forward through the newest connection, fall back to the older once the newest ends, and close the rest at once", async () => {
    const tunnel = new Tunnel(machineId, NAME, options());

    // open an older and a newer connection
    connect(tunnel, "older");
    connect(tunnel, "newer");
    const both = await forward(tunnel, "/a");

    // end the newer, then close the tunnel
    tunnel.end("newer");
    const older = await forward(tunnel, "/b");
    const closed = tunnel.close();

    expect([both, older, closed, await forward(tunnel, "/c"), tunnel.isEmpty]).toEqual([
        "newer /a",
        "older /b",
        ["older"],
        "SERVICE_UNAVAILABLE",
        true,
    ]);
});

test("renew a connection's token through a stream the machine opens, answering its name and lifetime, and lapse the connection on a refused token", async () => {
    const renewals: [string, number][] = [];
    const tunnel = new Tunnel(machineId, NAME, options(renewals));
    const machine = connect(tunnel, "host");

    // renew with a token lasting 5 s and call no other path before refusing an empty one
    const renew = (path: string, token: string) =>
        machine
            .fetch(
                new Request(`https://relay.destack.space${path}`, {
                    method: "PUT",
                    headers: { authorization: `Bearer ${token}` },
                }),
            )
            .then(async (response) => [
                response.status,
                response.ok ? await response.json() : null,
            ]);
    const before = Date.now();
    const [status, renewal] = await renew("/tunnel", "5000");
    const renewed = [status, schema.looseObject({ name: schema.string() }).parse(renewal).name];
    const other = await renew("/other", "5000");
    const refused = await renew("/tunnel", "");

    // record the renewed lapse 5 s out, then the refused one as lapsed already
    const [first, second] = renewals;
    expect([
        renewed,
        other,
        refused,
        renewals.map(([id]) => id),
        (first?.[1] ?? 0) >= before + 5000,
        (second?.[1] ?? Infinity) <= Date.now(),
    ]).toEqual([[200, NAME], [404, null], [401, null], ["host", "host"], true, true]);
});

test("refuse a machine name the machine no longer has, as after its rename", async () => {
    const tunnel = new Tunnel(machineId, NAME, options());
    connect(tunnel, "host");

    // rename the machine and call it by its new name only
    await tunnel.rename("renamed--acme.destack.computer");
    const answer = (name: string) =>
        tunnel.fetch(new Request(`https://${name}/status`)).then(
            (response) => response.text(),
            (error: ServiceError<string, unknown>) => error.code,
        );
    expect([
        await answer(NAME),
        await answer("renamed--acme.destack.computer"),
        await answer("notes--personal--acme.destack.space"),
    ]).toEqual(["MISDIRECTED_REQUEST", "host /status", "host /status"]);
});

test("refuse a request beyond a machine's open requests, and time out the ones the machine answers no head for", async () => {
    // join a machine that never answers, with a short answer timeout
    const tunnel = new Tunnel(machineId, NAME, options([], 50));
    let relay: Session | undefined;
    const machine: Session = new Session(
        { send: (message) => setImmediate(() => relay?.receive(message)), close: () => {} },
        "client",
        { accept: () => {} },
    );
    relay = tunnel.attach("silent", {
        send: (message) => setImmediate(() => machine.receive(message)),
        close: () => machine.terminate(),
    });

    // fill the machine's open requests, refuse one more and time the open ones out
    const open = Array.from({ length: MAX_STREAMS }, () => forward(tunnel, "/slow"));
    const refused = await forward(tunnel, "/one-more");
    const timedOut = new Set(await Promise.all(open));
    expect([refused, [...timedOut], relay.size]).toEqual([
        "SERVICE_UNAVAILABLE",
        ["GATEWAY_TIMEOUT"],
        0,
    ]);
});
