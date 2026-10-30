import { TEST_START, testSimulation } from "@destack/test/world";
import { Failure } from "@destack/error";
import type { Socket } from "@destack/object/workload";
import { schema } from "@destack/schema";
import { TestTelemetry } from "@destack/telemetry/test";
import { expect, refusal, test } from "@destack/test";
import { RelayError } from "../error/index.ts";
import { MAX_STREAMS, Session } from "../session/index.ts";
import type { TunnelToken } from "./token.ts";
import {
    ANSWER_TIMEOUT_MILLISECONDS,
    Tunnel,
    TunnelAttachment,
    type TunnelStanding,
} from "./tunnel.ts";

/** The machine whose tunnel the scenarios open. */
const machineId = schema
    .identifier("machine")
    .parse("machine-01996ab0-0000-7000-8000-0000000000d1");

/** Another machine, whose tokens the tunnel refuses. */
const otherId = schema.identifier("machine").parse("machine-01996ab0-0000-7000-8000-0000000000d2");

/** The name the relay routes to the machine. */
const NAME = "laptop--acme.destack.computer";

/** The account the machine is enrolled in. */
const ACCOUNT = schema.identifier("account").parse("account-01996ab0-0000-7000-8000-0000000000d3");

/** The relay's tunnel URL the machine dials. */
const TUNNEL_URL = "https://relay.destack.space/tunnel";

/** The universe key that signs the scenarios' tokens. */
const KID = "key-1";

/** How a scenario's tunnel answers: its machine's standing, and the keys the identities list. */
interface Standing {
    /** Read whether the machine stands. */
    standing: () => Promise<TunnelStanding>;
    /** The keys the universe's identities list. */
    keys: Set<string>;
}

/** Verify test tokens `<machine>.<lifetime>`, each the machine's for a lifetime in milliseconds. */
function testTokens(standing: Standing): Pick<TunnelToken, "verify" | "isStanding"> {
    return {
        verify: async (token, now) => {
            const [machine = "", lifetime = ""] = token.split(".");

            return {
                machineId: schema.identifier("machine").parse(machine),
                lapsesAt: now + Number(lifetime),
                kid: KID,
            };
        },
        isStanding: async (kid) => standing.keys.has(kid),
    };
}

/** A machine's tunnel over sockets hibernated in memory, with the machine's ends of them. */
function openTunnel() {
    // keep each accepted socket's stored value, delivering the relay's sends to the machine's session
    const simulation = testSimulation();
    const machines = new Map<string, Session>();
    const attachments = new Map<string, TunnelAttachment>();
    const socket = (id: string): Socket => ({
        id,
        attachment: attachments.get(id) ?? null,
        send: (message) =>
            void simulation.clock.track(
                new Promise<void>((resolve) => {
                    setImmediate(() => {
                        machines.get(id)?.receive(message);
                        resolve();
                    });
                }),
            ),
        close: () => machines.get(id)?.terminate(),
    });

    // stand the machine with its key, counting the lapses' requeues
    const standing: Standing = {
        standing: async () => ({ name: NAME, account: ACCOUNT }),
        keys: new Set([KID]),
    };
    let changes = 0;
    const published: [string, number][] = [];
    const options = {
        hibernation: {
            list: async () => [...attachments.keys()].map(socket),
            socket: (accepted: { readonly id: string; readonly attachment: unknown }) => {
                attachments.set(accepted.id, TunnelAttachment.parse(accepted.attachment));

                return socket(accepted.id);
            },
            accept: (accepted: { readonly id: string; readonly attachment: unknown }) => {
                attachments.set(accepted.id, TunnelAttachment.parse(accepted.attachment));

                return new Response(null);
            },
            attach: (id: string, attachment: unknown) =>
                attachments.set(id, TunnelAttachment.parse(attachment)),
        },
        url: TUNNEL_URL,
        tokens: testTokens(standing),
        identities: async () => {},
        standing: () => standing.standing(),
        changed: () => {
            changes += 1;
        },
        publish: async (account: string, lapsesAt: number) => {
            published.push([account, lapsesAt]);
        },
        environment: {
            clock: simulation.clock,
            random: simulation.random,
            telemetry: simulation.telemetry,
        },
    };
    const running = new Tunnel(machineId, options);

    // accept a machine's socket, without its host delivering any event yet
    const accept = (lapsesAt: number): string => {
        running.accept({ machineId, lapsesAt, kid: KID, name: NAME, account: ACCOUNT });

        return schema.string().parse([...attachments.keys()].at(-1));
    };

    // connect the machine with a session answering each request with a label, a status or nothing, once it pinged
    const connect = async (
        label: string,
        lapsesAt: number,
        answer: "label" | "silent" | number = "label",
    ): Promise<Socket> => {
        const id = accept(lapsesAt);
        const session = new Session(
            {
                send: (message) =>
                    void simulation.clock.track(
                        new Promise<void>((resolve) => {
                            setImmediate(() => {
                                void running.message(socket(id), message);
                                resolve();
                            });
                        }),
                    ),
                close: () => {},
            },
            "client",
            {
                accept: (stream) => {
                    // answer the label with the path, a bare status, or nothing
                    if (answer === "label") {
                        void stream.respond(
                            new Response(`${label} ${new URL(stream.head.url).pathname}`),
                        );
                    } else if (answer !== "silent") {
                        void stream.respond(new Response(null, { status: answer }));
                    }
                },
            },
        );
        machines.set(id, session);
        await session.ping();

        return socket(id);
    };

    // wake a tunnel of a new instance over the same hibernated sockets
    const wake = () => new Tunnel(machineId, options);

    return {
        tunnel: running,
        accept,
        connect,
        wake,
        machines,
        attachments,
        simulation,
        standing,
        changes: () => changes,
        published,
        telemetry: new TestTelemetry(simulation.telemetry),
    };
}

/** Forward a request for a path, answering its body or the refusal's code, reason and message. */
function forward(
    running: Tunnel,
    path: string,
): Promise<string | readonly [string, string, string]> {
    return running.fetch(new Request(`https://notes--personal--acme.destack.space${path}`)).then(
        (response) => response.text(),
        (error: unknown) => refusal(Promise.reject(error)),
    );
}

/** Ask the relay through a machine's session for a path with a token, answering status and body. */
async function ask(machine: Session, path: string, token: string) {
    const response = await machine.fetch(
        new Request(`https://relay.destack.space${path}`, {
            method: "PUT",
            headers: { authorization: `Bearer ${token}` },
        }),
    );
    if (response.ok) {
        const body: unknown = await response.json();

        return [response.status, body];
    }
    const { code, reason, message } = await Failure.read(response);

    return [response.status, [code, reason, message]];
}

/** Describe a refusal's code, reason and message. */
function refused(code: string, reason: string, message: string) {
    return [code, reason, message];
}

/** Describe a tunnel event of the scenarios' machine, with its reason for a close. */
function event(name: string, reason?: string) {
    return {
        scope: "@destack/relay",
        event: name,
        severity: "INFO",
        attributes: reason === undefined ? { machine: machineId } : { machine: machineId, reason },
    };
}

test("forward through the newest socket, fall back to the older once the newest closes, and close the rest at once", async () => {
    const { tunnel: running, connect, simulation, telemetry: recorded } = openTunnel();
    using telemetry = recorded;
    const later = simulation.clock.now() + 60_000;

    // open an older and a newer socket
    await connect("older", later);
    const newer = await connect("newer", later + 1);
    const both = await forward(running, "/a");

    // close the newer's socket, then close the tunnel
    await running.close(newer);
    const older = await forward(running, "/b");
    await running.closeAll("machine-revoked");

    expect([both, older, await forward(running, "/c"), telemetry.records()]).toEqual([
        "newer /a",
        "older /b",
        ["SERVICE_UNAVAILABLE", "MACHINE_DISCONNECTED", `machine ${machineId} is disconnected`],
        [
            event("tunnel.open"),
            event("tunnel.open"),
            event("tunnel.close", "closed"),
            event("tunnel.close", "machine-revoked"),
        ],
    ]);
});

test("run an accepted socket only once its host delivers its first event, and forget one that closes before", async () => {
    const {
        tunnel: running,
        accept,
        connect,
        attachments,
        simulation,
        changes,
        telemetry: recorded,
    } = openTunnel();
    using telemetry = recorded;

    // accept a socket that never connects, then connect one, then close the first at the host
    const later = simulation.clock.now() + 60_000;
    const stray = accept(later + 1);
    const before = await forward(running, "/a");
    await connect("host", later);
    const after = await forward(running, "/b");
    const strayed = {
        id: stray,
        attachment: attachments.get(stray) ?? null,
        send: () => {},
        close: () => {},
    };
    await running.close(strayed);

    // forward through the connected socket alone, opening and requeueing it once
    expect([before, after, changes(), telemetry.records()]).toEqual([
        ["SERVICE_UNAVAILABLE", "MACHINE_DISCONNECTED", `machine ${machineId} is disconnected`],
        "host /b",
        1,
        [event("tunnel.open")],
    ]);
});

test("renew a socket's token through a stream the machine opens, answering its name and lapse and publishing the tunnel, and refuse other paths, other machines and lapsed tokens", async () => {
    const {
        tunnel: running,
        connect,
        machines,
        attachments,
        simulation,
        published,
        telemetry: recorded,
    } = openTunnel();
    using telemetry = recorded;
    const opened = await connect("host", simulation.clock.now() + 1000);
    const machine = schema.instanceof(Session).parse(machines.get(opened.id));

    // renew with a token lasting 5 s, then ask another path, then renew with another machine's token
    const renewed = await ask(machine, "/tunnel", `${machineId}.5000`);
    const afterRenewal = attachments.get(opened.id);
    const other = await ask(machine, "/other", `${machineId}.5000`);
    const foreign = await ask(machine, "/tunnel", `${otherId}.5000`);
    const afterRefusal = attachments.get(opened.id);

    // keep the renewed lapse 5 s out, then the refused one as lapsed already, and close the socket
    const wait = await running.closeLapsed();
    expect([
        renewed,
        other,
        foreign,
        afterRenewal,
        afterRefusal,
        wait,
        telemetry.records(),
        published,
    ]).toEqual([
        [200, { name: NAME, lapsesAt: TEST_START + 5000 }],
        [404, refused("NOT_FOUND", "RENEWAL_ONLY", "machines can only renew their own tunnel")],
        [403, refused("FORBIDDEN", "TUNNEL_FOREIGN", "machines can only renew their own tunnels")],
        { machineId, lapsesAt: TEST_START + 5000, kid: KID },
        { machineId, lapsesAt: TEST_START, kid: KID },
        undefined,
        [event("tunnel.open"), event("tunnel.close", "lapsed")],
        // publish the tunnel once it opened and once it renewed, never at a refused renewal
        [
            [ACCOUNT, TEST_START + 1000],
            [ACCOUNT, TEST_START + 5000],
        ],
    ]);
});

test("refuse a renewal of a machine revoked, left keyless or missing from the directory meanwhile, lapsing its socket at once", async () => {
    const {
        tunnel: running,
        connect,
        machines,
        attachments,
        simulation,
        standing,
        telemetry: recorded,
    } = openTunnel();
    using telemetry = recorded;
    const opened = await connect("host", simulation.clock.now() + 60_000);
    const machine = schema.instanceof(Session).parse(machines.get(opened.id));

    // renew after the machine's revoke, then after its keys' revoke
    standing.standing = async () => ({ reason: "machine-revoked" });
    const revoked = await ask(machine, "/tunnel", `${machineId}.5000`);
    const afterRevoke = attachments.get(opened.id);
    standing.standing = async () => ({ reason: "key-revoked" });
    const keyless = await ask(machine, "/tunnel", `${machineId}.5000`);
    const afterKeys = attachments.get(opened.id);
    standing.standing = async () => ({ reason: "machine-unknown" });
    const unknown = await ask(machine, "/tunnel", `${machineId}.5000`);
    const afterUnknown = attachments.get(opened.id);

    // refuse each for its reason, the socket lapsed at once and closed
    expect([
        revoked,
        keyless,
        unknown,
        afterRevoke,
        afterKeys,
        afterUnknown,
        await running.closeLapsed(),
        telemetry.records(),
    ]).toEqual([
        [401, refused("UNAUTHORIZED", "MACHINE_REVOKED", `machine ${machineId} is revoked`)],
        [401, refused("UNAUTHORIZED", "MACHINE_UNKEYED", `machine ${machineId} has no key`)],
        [
            401,
            refused(
                "UNAUTHORIZED",
                "MACHINE_UNKNOWN",
                `machine ${machineId} is not in the directory`,
            ),
        ],
        { machineId, lapsesAt: TEST_START, kid: KID },
        { machineId, lapsesAt: TEST_START, kid: KID },
        { machineId, lapsesAt: TEST_START, kid: KID },
        undefined,
        [event("tunnel.open"), event("tunnel.close", "lapsed")],
    ]);
});

test("refuse a rename the machine refuses to renew for", async () => {
    const { tunnel: running, connect, telemetry: recorded } = openTunnel();
    using _telemetry = recorded;
    await connect("host", TEST_START + 60_000, 503);

    // rename the machine, whose session answers its renewal with a 503
    expect(await refusal(running.rename("renamed--acme.destack.computer"))).toEqual([
        "BAD_GATEWAY",
        "RENEWAL_REFUSED",
        `machine ${machineId} refused to renew: 503`,
    ]);
});

test("forward through the newest socket whose token has not lapsed, before the lapses run", async () => {
    const { tunnel: running, connect, simulation, telemetry: recorded } = openTunnel();
    using _telemetry = recorded;

    // open an older socket lasting a minute and a newer lasting a second, then pass the second
    await connect("older", TEST_START + 60_000);
    await connect("newer", TEST_START + 1000);
    await simulation.clock.sleep(TEST_START + 1000 - simulation.clock.now());
    await simulation.idle();

    expect(await forward(running, "/a")).toBe("older /a");
});

test("answer the wait until a socket's lapse, and close the socket once it lapsed", async () => {
    const { tunnel: running, connect, simulation, telemetry: recorded } = openTunnel();
    using telemetry = recorded;
    await connect("host", TEST_START + 1000);

    // read the next lapse, then close the socket past it
    const wait = await running.closeLapsed();
    await simulation.clock.sleep(TEST_START + 1000 - simulation.clock.now());
    await simulation.idle();
    const lapsed = await running.closeLapsed();
    expect([wait, lapsed, telemetry.records()]).toEqual([
        1000,
        undefined,
        [event("tunnel.open"), event("tunnel.close", "lapsed")],
    ]);
});

test("verify woken sockets once their standing reads, again after a failed read, closing those of a revoked key", async () => {
    const { connect, wake, standing, telemetry: recorded } = openTunnel();
    using telemetry = recorded;
    await connect("host", TEST_START + 60_000);

    // wake a new instance whose first standing read fails, then one after the key's revoke
    const woken = wake();
    standing.standing = async () => {
        throw new TypeError("the root is starting");
    };
    const failed = await woken.closeLapsed().catch((error: unknown) => error);
    standing.standing = async () => ({ name: NAME, account: ACCOUNT });
    standing.keys.clear();
    const verified = await woken.closeLapsed();

    // retry the read without poisoning the instance, then close the socket of the revoked key
    expect([failed, verified, await forward(woken, "/a"), telemetry.records()]).toEqual([
        new TypeError("the root is starting"),
        undefined,
        ["SERVICE_UNAVAILABLE", "MACHINE_DISCONNECTED", `machine ${machineId} is disconnected`],
        [event("tunnel.open"), event("tunnel.close", "key-revoked")],
    ]);
});

test("forward only through woken sockets whose key the universe still lists", async () => {
    // wake a new instance after the key's revoke, before any lapse check runs
    const { connect, wake, standing, telemetry: recorded } = openTunnel();
    using telemetry = recorded;
    await connect("host", TEST_START + 60_000);
    const woken = wake();
    standing.keys.clear();

    // refuse the forward, closing the socket of the revoked key first
    expect([await forward(woken, "/a"), telemetry.records()]).toEqual([
        ["SERVICE_UNAVAILABLE", "MACHINE_DISCONNECTED", `machine ${machineId} is disconnected`],
        [event("tunnel.open"), event("tunnel.close", "key-revoked")],
    ]);
});

test("answer a renewal the root fails to read without lapsing the socket", async () => {
    // renew while the root's standing read fails
    const { connect, machines, attachments, standing, telemetry: recorded } = openTunnel();
    using _telemetry = recorded;
    const opened = await connect("host", TEST_START + 60_000);
    const machine = schema.instanceof(Session).parse(machines.get(opened.id));
    standing.standing = async () => {
        throw RelayError.of("ENDPOINT_UNREACHABLE", "the root does not answer");
    };
    const failed = await ask(machine, "/tunnel", `${machineId}.5000`);

    // refuse it for the root's reason, the socket keeping its lapse
    expect([failed, attachments.get(opened.id)]).toEqual([
        [502, refused("BAD_GATEWAY", "ENDPOINT_UNREACHABLE", "the root does not answer")],
        { machineId, lapsesAt: TEST_START + 60_000, kid: KID },
    ]);
});

test("refuse a request beyond a machine's open requests, and time out the ones the machine answers no head for", async () => {
    // connect a machine that never answers
    const { tunnel: running, connect, simulation, telemetry: recorded } = openTunnel();
    using _telemetry = recorded;
    await connect("silent", TEST_START + 600_000, "silent");

    // fill the machine's open requests, refuse one more and time the open ones out
    const open = Array.from({ length: MAX_STREAMS }, () => forward(running, "/slow"));
    const refusedOne = await forward(running, "/one-more");
    await simulation.clock.sleep(TEST_START + ANSWER_TIMEOUT_MILLISECONDS - simulation.clock.now());
    await simulation.idle();
    const timedOut = await Promise.all(open);
    const timeout = [
        "GATEWAY_TIMEOUT",
        "HEAD_TIMEOUT",
        `machine ${machineId} sent no response head within ${ANSWER_TIMEOUT_MILLISECONDS} ms`,
    ];
    expect([refusedOne, timedOut]).toEqual([
        [
            "SERVICE_UNAVAILABLE",
            "STREAMS_EXHAUSTED",
            `machine ${machineId} already has ${MAX_STREAMS} open requests`,
        ],
        Array.from({ length: MAX_STREAMS }, () => timeout),
    ]);
});
