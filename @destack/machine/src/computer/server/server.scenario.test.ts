import { TEST_UNIVERSE } from "@destack/universe/test";
import { log, LogData } from "@destack/observability/object";
import { ObjectReference, Scope, RequestId } from "@destack/sync";
import { fileURLToPath } from "node:url";
import {
    AccessContext,
    Authentication,
    Caller,
    principal,
    accessRelationship,
    accessRole,
    Manager,
} from "@destack/access";
import * as accountObject from "@destack/account/object";
import { eq, or } from "@destack/db";
import type { DatabaseConnection } from "@destack/db/database";
import { Identifier, present, schema } from "@destack/schema";
import { CallError } from "@destack/object";
import observabilityPackage from "@destack/observability/package";
import { createClient } from "@destack/object/client";
import { expect, refusal, test } from "@destack/test";
import { checkout } from "../checkout/index.ts";
import { Authenticator } from "../device/authentication/index.ts";
import { SESSIONS } from "@destack/account/native";
import stackPackage from "@destack/stack/package";
import machinePackage from "../../package.ts";
import { ComputerServer } from "./computer.ts";
import { TOKEN_LIFETIME_MILLISECONDS } from "@destack/access/token";
import { OWNER_EMAIL, TestComputer } from "../test/index.ts";

/** The fixture stack package, used as a registered checkout. */
const stackFixture = fileURLToPath(new URL("../../../tests/fixture/stack/", import.meta.url));

/** Poll a machine following its status every few milliseconds, for as long as a replica takes. */
const PAUSE_POLL = { interval: 20 };

/** Connect to the stack package of a space at its machine as a person signed in on a computer. */
async function stackOf(
    computer: TestComputer,
    login: Identifier<"login">,
    spaceId: Identifier<"space">,
) {
    const { device } = computer;

    return createClient(
        stackPackage,
        (await device.launcher(login)).endpoint(stackPackage.id, await device.locate(spaceId)),
        device.environment,
    );
}

/** Call a procedure as an installation the gateway forwards. */
function installationCall(
    authentication: Parameters<Authenticator["authorizeMachine"]>[0]["access"]["authentication"],
    permission: "rows" | "own" | "read",
): Parameters<Authenticator["authorizeMachine"]>[0] {
    return {
        request: {
            credential: () => ({
                kind: "installation",
                id: "instance-1",
                audience: machinePackage.id,
                verifiedAt: 0,
                expiresAt: TOKEN_LIFETIME_MILLISECONDS,
            }),
        },
        access: {
            authentication,
            permission:
                permission === "rows"
                    ? "rows"
                    : permission === "own"
                      ? {
                            principals: [
                                {
                                    packageId: principal.machine.definition.packageId,
                                    type: principal.machine.definition.name,
                                },
                            ],
                        }
                      : {
                            packageId: machinePackage.id,
                            type: "computer",
                            name: permission,
                        },
        },
    };
}

/** Describe an authorization by its outcome: allowed, or the refusal's message. */
async function outcome(authorizing: Promise<void>): Promise<string> {
    const refused = await refusal(authorizing);

    return refused === "done" ? "allowed" : refused[2];
}

test("restrict machine procedures to the owner", async () => {
    const { authenticator } = (await TestComputer.open()).computer;

    // allow a served service's permissionless and public procedures
    const anonymous = {
        ...installationCall("public", "rows"),
        context: {
            credential: () => {
                throw CallError.http("UNAUTHORIZED", { message: "missing bearer credential" });
            },
        },
    };
    expect([
        await outcome(authenticator.authorizeSpace(anonymous)),
        await outcome(authenticator.authorizeSpace(installationCall("identity", "own"))),
        await outcome(authenticator.authorizeSpace(installationCall("machine", "own"))),
        await outcome(authenticator.authorizeMachine(installationCall("identity", "own"))),
        await outcome(authenticator.authorizeMachine(installationCall("identity", "read"))),
    ]).toEqual([
        "allowed",
        "allowed",
        "machine procedure is forbidden: only the owner can call machine procedures",
        "machine procedure is forbidden: only the owner can call the machine's own procedures",
        "allowed",
    ]);
});

test("authenticate the owner's direct calls and a client's key proof", async () => {
    // ask as the owner directly, from another computer through a proxy naming the loopback host, and from a browser
    const computer = await TestComputer.open();
    const { device: native, authenticator } = computer.computer;
    const asked = (headers: Record<string, string>) =>
        refusal(
            authenticator.authenticate(
                new Request("http://127.0.0.1/rpc", {
                    headers: { authorization: `Bearer ${authenticator.token}`, ...headers },
                }),
                machinePackage.id,
            ),
        );
    const [local, remote, browser] = [
        await asked({}),
        await asked({ forwarded: "for=203.0.113.9" }),
        await asked({ origin: "http://127.0.0.1:3000" }),
    ];

    // sign the owner in, then forget every stored session
    const signedIn = await computer.signIn(OWNER_EMAIL);
    computer.keychain(SESSIONS).secrets.clear();
    const registration = schema.identifier("client").parse(signedIn.clientId);
    const proof = await native.credential.prove(await native.signedIn(signedIn.id));

    // take the client's proof from this machine as the person
    const authentication = await authenticator.authenticate(
        new Request("http://127.0.0.1/status", { headers: { authorization: `Bearer ${proof}` } }),
        machinePackage.id,
    );
    const { verifiedAt } = present(authentication, "the person's authentication").credential;
    const owner = principal.user.reference(Scope.universe.id, computer.world.owner.id);

    // accept the owner's direct call alone, and the person's client by its proof, reading no session
    const refused = [
        "UNAUTHORIZED",
        "ORIGIN_UNAUTHENTICATED",
        "authentication required: the credential is not accepted from this origin",
    ];
    expect({ owner: [local, remote, browser], client: authentication }).toEqual({
        owner: ["done", refused, refused],
        // the person, contained in this computer's device
        client: {
            caller: { ...Caller.of(owner), groups: [{ ...native.subject, relation: "contained" }] },
            credential: {
                kind: "session",
                id: registration,
                client: registration,
                audience: machinePackage.id,
                verifiedAt,
                expiresAt: verifiedAt + TOKEN_LIFETIME_MILLISECONDS,
            },
        },
    });
});

test("record each start of the computer under one identifier", async () => {
    const opened = await TestComputer.open();
    const client = opened.connect(machinePackage);
    const machineId = opened.machine.id;
    const deviceId = opened.device.id;
    const device = principal.device.reference(Scope.universe.id, deviceId);
    const read = async () => (await client.computer.get({ deviceId, id: machineId })).tunnel;
    const record = async (startedAt: number) =>
        await ComputerServer.record(
            opened.computer.objects,
            { device: deviceId, machine: machineId },
            { version: machinePackage.version, pid: 4343, startedAt },
            undefined,
        );

    // record the next start's process on the same record, as the device storing it
    const { id: first, tunnel: unrelayed } = await client.computer.get({ deviceId, id: machineId });
    const startedAt = opened.clock.now();
    await record(startedAt);

    // store one machine record in the device, written by the device's own packages in order
    const writer = device;
    const { createdAt, updatedAt, ...status } = await client.computer.get({
        deviceId,
        id: machineId,
    });

    // open the tunnel, lose it, then start the next process
    const tunnels = [unrelayed];
    await ComputerServer.link(opened.computer.objects, machineId, "open");
    tunnels.push(await read());
    await ComputerServer.link(opened.computer.objects, machineId, "dialing");
    tunnels.push(await read());
    await record(opened.clock.now());
    tunnels.push(await read());

    // expect one record, no tunnel without a relay, then each status, then none for the new process
    expect([status, createdAt <= updatedAt, tunnels]).toEqual([
        {
            id: first,
            machineId,
            scope: deviceId,
            version: machinePackage.version,
            pid: 4343,
            startedAt,
            tunnel: "none",
            enrollment: null,
            tags: {},
            ...ObjectReference.columns("createdBy", writer),
            ...ObjectReference.columns("updatedBy", writer),
            ...Manager.values(null),
            revision: 2,
        },
        true,
        ["none", "open", "dialing", "none"],
    ]);
});

test("stop the machine only as its own device", async () => {
    const computer = await TestComputer.open();
    const opened = computer.computer;
    const machineId = computer.machine.id;
    const deviceId = computer.device.id;
    const other = Identifier.create("device", computer.environment);

    // call stop as a device on this computer's owner credential
    const stop = (id: Identifier<"device">) => {
        const subject = principal.device.reference(Scope.universe.id, id);
        const now = computer.clock.now();
        const authentication: Authentication = {
            caller: Caller.of(subject),
            credential: {
                kind: "local",
                id,
                audience: machinePackage.id,
                verifiedAt: now,
                expiresAt: now + TOKEN_LIFETIME_MILLISECONDS,
            },
        };
        const client = createClient(
            machinePackage,
            {
                url: "http://127.0.0.1/",
                fetch: (request: Request) =>
                    opened.instance.fetch(machinePackage, request, async () => authentication),
            },
            computer.device.environment,
        );

        return refusal(
            client.computer.stop({
                deviceId,
                id: machineId,
                requestId: RequestId.create(computer.device.environment),
            }),
        );
    };

    // refuse another device, which sees nothing of this device's scope
    expect([await stop(deviceId), await stop(other)]).toEqual([
        "done",
        ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope '${deviceId}' not found`],
    ]);
});

test("send a desktop's telemetry to its person's home", async () => {
    const { home: computer, login } = await TestComputer.home();
    const { device: native, machine } = computer;
    const home = accountObject.Account.home(await (await native.launcher(login)).account());

    // export a desktop's failure log to the home's observability as its person
    const now = computer.clock.now();
    const record = {
        timeUnixNano: String(now * 1_000_000),
        eventName: "inbox.failed",
        body: { stringValue: "the inbox stopped" },
        attributes: [],
    };
    const scope = { name: "@destack/desktop", version: "2026.9.2" };
    const body = {
        resourceLogs: [
            { resource: { attributes: [] }, scopeLogs: [{ scope, logRecords: [record] }] },
        ],
    };
    const exported = await (
        await native.launcher(login)
    ).send(
        new Request("http://127.0.0.1/v1/logs", {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify(body),
        }),
        observabilityPackage.id,
        await native.locate(home),
    );
    expect([exported.status, await exported.text()]).toEqual([200, "{}"]);

    // read it back from the home's events
    const { events } = (await machine.space(home)).volume;
    const logged = await events.query(log.eventKind(), { scopes: [home] }, { limit: 10 });
    expect(
        logged.events.map((found) => [found.keys["name"], LogData.parse(found.data).body]),
    ).toEqual([["inbox.failed", "the inbox stopped"]]);
});

test("serve spaces only while the machine is enrolled", async () => {
    const opened = await TestComputer.open();
    const { machine } = opened;
    const { owner, universe } = opened.world;
    const client = opened.connect(machinePackage);
    const machineId = machine.id;
    const deviceId = opened.device.id;

    // sign the owner in with the code their issuer mails
    const created = await opened.signIn(OWNER_EMAIL);

    // enroll as the owner and revoke
    const idle = await machine.isEnrolled();
    const enrolled = await client.computer.enroll({
        deviceId,
        requestId: RequestId.create(opened.environment),
        id: machineId,
        login: created.id,
        accountId: owner.account,
        name: "workstation",
    });
    const serving = await machine.isEnrolled();
    await client.computer.revoke({
        deviceId,
        requestId: RequestId.create(opened.environment),
        id: machineId,
    });
    const revoked = await client.computer.get({ deviceId, id: machineId });
    const again = await refusal(
        client.computer.enroll({
            deviceId,
            requestId: RequestId.create(opened.environment),
            id: machineId,
            login: created.id,
            accountId: owner.account,
        }),
    );
    const registered = await universe.storage.database
        .select({
            name: accountObject.machine.table.name,
            deletedAt: accountObject.machine.table.deletedAt,
        })
        .from(accountObject.machine.table)
        .where(eq(accountObject.machine.table.id, machineId));

    // serve spaces only while enrolled and refuse enrolling a revoked machine again
    expect({
        isEnrolled: [idle, serving, await machine.isEnrolled()],
        enrolled: enrolled.enrollment,
        revoked: revoked.enrollment,
        again,
        registered: registered.map((row) => ({
            name: row.name,
            isDeleted: row.deletedAt !== null,
        })),
    }).toEqual({
        isEnrolled: [false, true, false],
        enrolled: {
            kind: "personal",
            universe: TEST_UNIVERSE.toJSON(),
            accountId: owner.account,
            status: "enrolled",
        },
        revoked: { ...enrolled.enrollment, status: "revoked" },
        again: [
            "PRECONDITION_FAILED",
            "MACHINE_BLOCKED",
            "cannot enroll this machine: it was revoked, and a revoked machine can only enroll as a new machine",
        ],
        registered: [{ name: "workstation", isDeleted: true }],
    });
});

test("pause serving while the account drains the machine", async () => {
    const opened = await TestComputer.open();
    const { machine, device } = opened;
    const { owner } = opened.world;
    const signedIn = await opened.signIn(OWNER_EMAIL);
    await opened.enroll(signedIn, owner.account);
    const created = await opened.space(signedIn.id, owner.account, "work");
    const service = await stackOf(opened, signedIn.id, created);

    // drain or enable the machine as the owner, and list the space's sockets
    const accounts = (await device.launcher(signedIn.id)).session.accounts();
    const change = (name: "drain" | "enable") =>
        accounts.machine[name]({
            scope: owner.account,
            id: machine.id,
            requestId: RequestId.create(opened.environment),
        });
    const list = () => refusal(service.socket.list({ spaceId: created }));
    const paused = [
        "SERVICE_UNAVAILABLE",
        "SERVER_UNAVAILABLE",
        `server '${machine.id}' is unavailable: it is down: its account set its status to draining`,
    ];

    // drain the machine until it refuses the space's calls, and its starts once the space closed
    await change("drain");
    await expect.poll(list, PAUSE_POLL).toEqual(paused);
    await expect.poll(() => refusal(machine.space(created)), PAUSE_POLL).toEqual(paused);

    // enable it until it serves the space again
    await change("enable");
    await expect.poll(list, PAUSE_POLL).toBe("done");
});

test("decide checkout permissions for the owner, the machine and a stranger", async () => {
    // enroll a computer, and register a checkout as its local client
    const opened = await TestComputer.open();
    const { machine, world } = opened;
    await opened.enroll(await opened.signIn(OWNER_EMAIL), world.owner.account);
    const client = opened.connect(machinePackage);
    const registered = await client.checkout.create({
        machineId: machine.id,
        requestId: RequestId.create(opened.environment),
        directory: stackFixture,
        source: { kind: "existing" },
    });
    const read = await client.checkout.get({ machineId: machine.id, id: registered.id });

    // read the roles and bindings in the machine's partition and at the universe
    const bound = async (database: DatabaseConnection) => ({
        roles: (
            await database.select().from(accessRole).where(eq(accessRole.scope, machine.id))
        ).map((row) => row.name),
        bindings: (
            await database
                .select()
                .from(accessRelationship)
                .where(
                    or(
                        eq(accessRelationship.scope, machine.id),
                        eq(accessRelationship.objectId, machine.id),
                    ),
                )
        )
            .filter((row) => row.roleId !== null)
            .map((row) => [row.scope, row.subjectType, row.subjectId]),
    });
    const stored = {
        partition: await bound(opened.computer.machineObjects().database),
        universe: await bound(world.universe.storage.database),
        read: read.id,
    };

    // decide each caller's permissions on the checkout in the partition
    const partition = opened.computer.machineObjects();
    const owner = principal.user.reference(Scope.universe.id, world.owner.id);
    const local = principal.machine.reference(Scope.universe.id, machine.id);
    const stranger = principal.user.reference(
        Scope.universe.id,
        Identifier.create("user", opened.environment),
    );
    const target = checkout.reference(machine.id, registered.id);
    const decide = async (subject: typeof owner, name: "read" | "relocate" | "push") => {
        const snapshot = partition.database.snapshot();
        const access = await partition.authorizer.resolve(
            snapshot,
            machine.id,
            AccessContext.of(Caller.of(subject), opened.clock.now()),
        );
        const decided = await partition.authorizer.check(
            snapshot,
            checkout.permission(name),
            target,
            access,
        );

        return decided.isAllowed;
    };

    // wait until the chain above the machine synced, letting the owner operate the machine
    await opened.waitUntil(partition.database, async () =>
        decide(owner, "read").catch(() => false),
    );
    const permissions = async (subject: typeof owner) => [
        await decide(subject, "read"),
        await decide(subject, "relocate"),
        await decide(subject, "push"),
    ];
    const decided = {
        owner: await permissions(owner),
        local: await permissions(local),
        stranger: await permissions(stranger),
    };
    expect([stored, decided, opened.captured()]).toEqual([
        {
            // store no role, as the base packages request nothing there and act on their own types by their policies
            partition: { roles: [], bindings: [] },
            universe: { roles: [], bindings: [] },
            read: registered.id,
        },
        { owner: [true, true, true], local: [true, true, true], stranger: [false, false, false] },
        [],
    ]);
});
