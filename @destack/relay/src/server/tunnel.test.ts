import { principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { account, space } from "@destack/account/object";
import { DOMAINS } from "@destack/account/address";
import { TestDatabase } from "@destack/db/test";
import { Clock } from "@destack/environment";
import { implementPackage, ObjectStore } from "@destack/object/server";
import { testEvents } from "@destack/object/test";
import type { CallError } from "@destack/object";
import { testCallKey } from "@destack/object/test";
import type { Socket } from "@destack/object/workload";
import { schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { TestTelemetry } from "@destack/telemetry/test";
import { expect, onTestFinished, test } from "@destack/test";
import { connection, tunnel } from "../object/index.ts";
import relayPackage from "../package.ts";
import { MAX_STREAMS, Session } from "../session/index.ts";
import { homeDatabase } from "../test/database.ts";
import { machineTokens } from "../test/token.ts";
import { Tunnel } from "./tunnel.ts";

/** The machine whose tunnel the scenarios open. */
const machineId = schema
    .identifier("machine")
    .parse("machine-01996ab0-0000-7000-8000-0000000000d1");

/** The residency home the relay installation serves in. */
const homeId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000d2");

/** The account of the home. */
const accountId = schema
    .identifier("account")
    .parse("account-01996ab0-0000-7000-8000-0000000000d3");

/** The relay installation serving the tunnel. */
const installation = principal.installation.reference(
    homeId,
    schema.identifier("installation").parse("installation-01996ab0-0000-7000-8000-0000000000d4"),
);

/** The name the relay routes to the machine. */
const NAME = "laptop--acme.destack.computer";

/** The relay's tunnel URL the machine dials. */
const TUNNEL_URL = "https://relay.destack.space/tunnel";

/** A machine's tunnel in its partition, with the machine's ends of its connections in memory. */
async function openTunnel(answerTimeout?: number) {
    // store the tunnel in its partition, served by the relay installation
    const storage = await TestDatabase.create("sqlite", homeDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const partition = Tunnel.id(machineId);
    const access = new AccessFixture(storage.database);
    await access.replicateScope(account.reference(Scope.universe.id, accountId));
    await access.replicateScope(space.reference(accountId, homeId), "own");
    await access.replicateScope(tunnel.reference(homeId, partition), "own");
    await access.replicateRelationship(
        partition,
        tunnel.reference(homeId, partition),
        "server",
        installation,
    );
    const store = new ObjectStore({
        servers: [
            implementPackage(relayPackage, {
                objects: [tunnel, connection],
                adapters: { tunnels: async () => new Response(null, { status: 404 }) },
            }),
        ],
        chain: async () => ({ kind: "home", parent: homeId }),
        principal: installation,
        policies: [account],
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        clock: Clock.system,
        origin: { package: relayPackage },
        partition,
    });

    // address each accepted socket by its identifier, delivering the relay's sends to the machine's session
    const machines = new Map<string, Session>();
    const accepted: string[] = [];
    const socket = (id: string): Socket => ({
        id,
        attachment: { machineId },
        send: (message) => setImmediate(() => machines.get(id)?.receive(message)),
        close: () => machines.get(id)?.terminate(),
    });
    const running = new Tunnel(machineId, {
        store,
        hibernation: {
            list: async () => [],
            socket: ({ id }) => {
                accepted.push(id);

                return socket(id);
            },
            accept: () => new Response(null),
        },
        url: TUNNEL_URL,
        tokens: machineTokens(machineId),
        domains: DOMAINS,
        ...(answerTimeout === undefined ? {} : { answerTimeout }),
    });

    // connect the machine with a session answering each request with a label, or not at all
    const connect = async (label: string, lapsesAt: number, answers = true): Promise<Socket> => {
        await running.accept({ machineId, lapsesAt, name: NAME });
        const id = schema.string().parse(accepted.at(-1));
        machines.set(
            id,
            new Session(
                {
                    send: (message) =>
                        setImmediate(() => void running.message(socket(id), message)),
                    close: () => {},
                },
                "client",
                {
                    accept: (stream) =>
                        answers
                            ? void stream.respond(
                                  new Response(`${label} ${new URL(stream.head.url).pathname}`),
                              )
                            : undefined,
                },
            ),
        );

        return socket(id);
    };

    // read the lapses of the connection rows
    const lapses = async () =>
        (
            await storage.database
                .select({ lapsesAt: connection.table.lapsesAt })
                .from(connection.table)
        ).map((row) => row.lapsesAt);

    return { tunnel: running, connect, machines, lapses };
}

/** Forward a request for a path, answering its body or the refusal's code. */
function forward(running: Tunnel, path: string): Promise<string> {
    return running.fetch(new Request(`https://notes--personal--acme.destack.space${path}`)).then(
        (response) => response.text(),
        (error: CallError<string, unknown>) => error.code,
    );
}

test("forward through the newest connection, fall back to the older once the newest closes, and end the rest at once", async () => {
    const { tunnel: running, connect } = await openTunnel();
    const later = Date.now() + 60_000;

    // open an older and a newer connection
    await connect("older", later);
    const newer = await connect("newer", later + 1);
    const both = await forward(running, "/a");

    // close the newer's socket, then end the tunnel
    await running.close(newer);
    const older = await forward(running, "/b");
    const ended = await running.end();

    expect([both, older, ended, await forward(running, "/c"), running.isEmpty]).toEqual([
        "newer /a",
        "older /b",
        true,
        "SERVICE_UNAVAILABLE",
        true,
    ]);
});

test("renew a connection's token through a stream the machine opens, answering its name and lapse, and lapse the connection on a refused token", async () => {
    using telemetry = TestTelemetry.register();
    const { tunnel: running, connect, machines, lapses } = await openTunnel();
    const opened = await connect("host", Date.now() + 1000);
    const machine = machines.get(opened.id);

    // renew with a token lasting 5 s and call no other path before refusing an empty one
    const renew = (path: string, token: string) =>
        schema
            .instanceof(Session)
            .parse(machine)
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
    const [afterRenewal] = await lapses();
    const other = await renew("/other", "5000");
    const refused = await renew("/tunnel", "");
    const [afterRefusal] = await lapses();

    // record the renewed lapse 5 s out, then the refused one as lapsed already, capturing no failure
    expect([
        renewed,
        other,
        refused,
        (afterRenewal ?? 0) >= before + 5000,
        (afterRefusal ?? Infinity) <= Date.now(),
        telemetry.exceptions(),
        running.isEmpty,
    ]).toEqual([[200, NAME], [404, null], [401, null], true, true, [], false]);
});

test("refuse a machine name the machine no longer has, as after its rename", async () => {
    const { tunnel: running, connect } = await openTunnel();
    await connect("host", Date.now() + 60_000);

    // rename the machine and call it by its new name only
    await running.rename("renamed--acme.destack.computer");
    const answer = (name: string) =>
        running.fetch(new Request(`https://${name}/status`)).then(
            (response) => response.text(),
            (error: CallError<string, unknown>) => error.code,
        );
    expect([
        await answer(NAME),
        await answer("renamed--acme.destack.computer"),
        await answer("notes--personal--acme.destack.space"),
    ]).toEqual(["MISDIRECTED_REQUEST", "host /status", "host /status"]);
});

test("refuse a request beyond a machine's open requests, and time out the ones the machine answers no head for", async () => {
    // connect a machine that never answers, with a short answer timeout
    const { tunnel: running, connect } = await openTunnel(50);
    await connect("silent", Date.now() + 60_000, false);

    // fill the machine's open requests, refuse one more and time the open ones out
    const open = Array.from({ length: MAX_STREAMS }, () => forward(running, "/slow"));
    const refused = await forward(running, "/one-more");
    const timedOut = new Set(await Promise.all(open));
    expect([refused, [...timedOut]]).toEqual(["SERVICE_UNAVAILABLE", ["GATEWAY_TIMEOUT"]]);
});
