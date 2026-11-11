import { testHost } from "@destack/test/host";
import { log, LogData } from "@destack/observability/object";
import { ObjectReference, Scope, RequestId } from "@destack/sync";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
    AccessContext,
    accessRelationship,
    accessRole,
    Authentication,
    Caller,
    Manager,
    principal,
} from "@destack/access";
import * as accountObject from "@destack/account/object";
import { type DatabaseConnection, eq, or, Snapshot } from "@destack/db";
import { PackageManifest } from "@destack/package/manifest";
import { nth, Identifier, present, schema } from "@destack/schema";
import { CallError } from "@destack/object";
import observabilityPackage from "@destack/observability/package";
import { createClient } from "@destack/object/client";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { checkout, preview } from "../checkout/index.ts";
import { Authenticator } from "../device/authentication/index.ts";
import { SESSIONS } from "@destack/account/native";
import stackPackage from "@destack/stack/package";
import machinePackage from "../../package.ts";
import { ComputerServer } from "./computer.ts";
import { TOKEN_LIFETIME_MILLISECONDS } from "@destack/access/token";
import {
    OWNER_EMAIL,
    TestComputer,
    SCENARIO_MILLISECONDS,
    TestDistribution,
} from "../test/index.ts";

/** The template stack package, used as a registered checkout. */
const templateStack = dirname(
    dirname(fileURLToPath(import.meta.resolve("@destack/template-stack"))),
);

/** The stack package of the registered checkout's working tree. */
const STACK_CHECKOUT_PACKAGE = "package-01996ab0-0000-7000-8000-0000000000d1";

/** The installed stack package a checkout's stack builds against as its release. */
const INSTALLED_STACK = dirname(dirname(fileURLToPath(import.meta.resolve("@destack/stack"))));

/** A working tree of one stack package, which builds against the stack package's release. */
const STACK_CHECKOUT: Readonly<Record<string, string>> = {
    "bun.lock": JSON.stringify({
        lockfileVersion: 2,
        workspaces: { "": { name: "@acme/workspace" }, stack: { name: "@acme/stack" } },
        packages: {},
    }),
    ".gitignore": "node_modules/\n",
    "stack/package.json": JSON.stringify({
        name: "@acme/stack",
        version: "2026.9.0",
        type: "module",
        exports: { ".": "./src/index.ts" },
        dependencies: { "@destack/stack": "2026.9.0" },
    }),
    "stack/destack.json": JSON.stringify({
        $schema: "https://destack.app/schemas/2026.9.0/destack.json",
        id: STACK_CHECKOUT_PACKAGE,
        role: "library",
        language: "typescript",
        runtimes: ["bun", "workerd"],
    }),
    "stack/src/index.ts":
        'import { defineStack } from "@destack/stack";\n\n/** A space\'s stack, installing nothing. */\nexport const personal = defineStack([], {});\n',
};

/** The space a watched preview names, which the test never builds in. */
const PREVIEWED_SPACE = schema
    .identifier("space")
    .parse("space-01996ab0-0000-7000-8000-0000000000f1");

/** How long a preview's watch takes to act on settled changes. */
const SETTLED_MILLISECONDS = 600;

/** Poll a watched preview once per settling period. */
const WATCH_POLL = { interval: SETTLED_MILLISECONDS, timeout: 10_000 };

/** Poll a machine following its status every few milliseconds, for as long as a replica takes. */
const PAUSE_POLL = { interval: 20, timeout: 10_000 };

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

test("let clients call served services' own procedures, restricting machine procedures and the machine's own to the owner", async () => {
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

test("take the owner's token only from this computer outside a browser, by the request's first hop, whatever host it names", async () => {
    // ask as the owner directly, from another computer through a proxy naming the loopback host, and from a browser
    const { authenticator } = (await TestComputer.open()).computer;
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

    // accept the direct call alone
    const refused = [
        "UNAUTHORIZED",
        "ORIGIN_UNAUTHENTICATED",
        "authentication required: the credential is not accepted from this origin",
    ];
    expect([local, remote, browser]).toEqual(["done", refused, refused]);
});

test("record the computer at each start under the same identifier", async () => {
    const opened = await TestComputer.open();
    const client = opened.connect(machinePackage);
    const machineId = opened.machine.id;
    const deviceId = opened.device.id;
    const device = principal.device.reference(Scope.universe.id, deviceId);

    // record the next start's process on the same record, as the device storing it
    const { id: first } = await client.computer.get({ deviceId, id: machineId });
    const startedAt = opened.clock.now();
    const next = { version: machinePackage.version, pid: 4343, startedAt };
    await ComputerServer.record(
        opened.computer.objects,
        { device: deviceId, machine: machineId },
        next,
        undefined,
    );

    // store one machine record in the device, written by the device's own packages in order
    const writer = device;
    const { createdAt, updatedAt, ...status } = await client.computer.get({
        deviceId,
        id: machineId,
    });
    expect([status, createdAt <= updatedAt]).toEqual([
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
    ]);
});

test("record the tunnel opening and dialing on the computer's record, and none again at the next start", async () => {
    const opened = await TestComputer.open();
    const client = opened.connect(machinePackage);
    const machineId = opened.machine.id;
    const deviceId = opened.device.id;
    const read = async () => (await client.computer.get({ deviceId, id: machineId })).tunnel;

    // open the tunnel, lose it, then start the next process
    const tunnels = [await read()];
    await ComputerServer.link(opened.computer.objects, machineId, "open");
    tunnels.push(await read());
    await ComputerServer.link(opened.computer.objects, machineId, "dialing");
    tunnels.push(await read());
    const next = { version: machinePackage.version, pid: 4343, startedAt: opened.clock.now() };
    await ComputerServer.record(
        opened.computer.objects,
        { device: deviceId, machine: machineId },
        next,
        undefined,
    );
    tunnels.push(await read());

    // expect no tunnel without a relay, then each status, then none for the new process
    expect(tunnels).toEqual(["none", "open", "dialing", "none"]);
});

test("stop the machine as its device's owner, and refuse another device stopping it", async () => {
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
                    opened.instance.fetch(machinePackage, request, authentication),
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

test(
    "send a desktop's telemetry export to its person's home, sent through the computer's machine",
    async () => {
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
    },
    SCENARIO_MILLISECONDS,
);

test("authenticate a signed-in person's own client by its key's proof, reading no session", async () => {
    // sign the owner in, then forget every stored session
    const computer = await TestComputer.open();
    const { device: native, authenticator } = computer.computer;
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
    expect(authentication).toEqual({
        // the person, contained in this computer's device
        caller: { ...Caller.of(owner), groups: [{ ...native.subject, relation: "contained" }] },
        credential: {
            kind: "session",
            id: registration,
            client: registration,
            audience: machinePackage.id,
            verifiedAt,
            expiresAt: verifiedAt + TOKEN_LIFETIME_MILLISECONDS,
        },
    });
});

test("enroll as the signed-in person, serve spaces only while enrolled, and revoke at the universe", async () => {
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
            revokedAt: accountObject.machine.table.revokedAt,
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
            isRevoked: row.revokedAt !== null,
        })),
    }).toEqual({
        isEnrolled: [false, true, false],
        enrolled: {
            kind: "personal",
            universe: universe.origin,
            accountId: owner.account,
            status: "enrolled",
        },
        revoked: { ...enrolled.enrollment, status: "revoked" },
        again: [
            "PRECONDITION_FAILED",
            "MACHINE_BLOCKED",
            "cannot enroll this machine: it was revoked, and a revoked machine can only enroll as a new machine",
        ],
        registered: [{ name: "workstation", isRevoked: true }],
    });
});

test(
    "pause serving while the account drains the machine, refusing its space's calls and starts, and serve again once enabled",
    { timeout: SCENARIO_MILLISECONDS },
    async () => {
        const opened = await TestComputer.open();
        const { machine, device } = opened;
        const { owner } = opened.world;
        const signedIn = await opened.signIn(OWNER_EMAIL);
        await opened.enroll(signedIn, owner.account);
        const created = await opened.space(signedIn, owner.account, "work");
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

        // drain the machine until it refuses the space's calls and starts
        await change("drain");
        await expect.poll(list, PAUSE_POLL).toEqual(paused);
        const started = await refusal(machine.space(created));

        // enable it until it serves the space again
        await change("enable");
        await expect.poll(list, PAUSE_POLL).toBe("done");
        expect(started).toEqual(paused);
    },
);

test("receive the machine scope in its partition with its own packages, letting the machine register checkouts and writing nothing at the universe", async () => {
    // enroll a computer, and register a checkout as its local owner
    const opened = await TestComputer.open();
    const { machine, world } = opened;
    await opened.enroll(await opened.signIn(OWNER_EMAIL), world.owner.account);
    const client = opened.connect(machinePackage);
    const registered = await client.checkout.create({
        machineId: machine.id,
        requestId: RequestId.create(opened.environment),
        directory: templateStack,
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
    expect({
        partition: await bound(opened.computer.machineObjects().database),
        universe: await bound(world.universe.storage.database),
        read: read.id,
    }).toEqual({
        // store no role, as the base packages request nothing there and act on their own types by their policies
        partition: { roles: [], bindings: [] },
        universe: { roles: [], bindings: [] },
        read: registered.id,
    });
});

test("let the machine's local client and its account's owner work its checkouts once the partition's chain synced, and refuse a stranger", async () => {
    // enroll a computer, and register a checkout as its local client
    const opened = await TestComputer.open();
    const { machine, world } = opened;
    await opened.enroll(await opened.signIn(OWNER_EMAIL), world.owner.account);
    const registered = await opened.connect(machinePackage).checkout.create({
        machineId: machine.id,
        requestId: RequestId.create(opened.environment),
        directory: templateStack,
        source: { kind: "existing" },
    });

    // decide each caller's permissions on the checkout in the partition
    const partition = opened.computer.machineObjects();
    const owner = principal.user.reference(Scope.universe.id, world.owner.id);
    const local = principal.machine.reference(Scope.universe.id, machine.id);
    const stranger = principal.user.reference(
        Scope.universe.id,
        Identifier.create("user", opened.environment),
    );
    const target = checkout.reference(machine.id, registered.id);
    const decide = async (subject: typeof owner, name: "read" | "relocate" | "submit") => {
        const snapshot = Snapshot.live(partition.database);
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
        await decide(subject, "submit"),
    ];
    const decided = {
        owner: await permissions(owner),
        local: await permissions(local),
        stranger: await permissions(stranger),
    };
    expect([decided, opened.reports]).toEqual([
        { owner: [true, true, true], local: [true, true, true], stranger: [false, false, false] },
        [],
    ]);
});

test("register a working tree's directory, build its stack in a space as the person, and refuse a directory outside it", async () => {
    // release the stack package with the board's closure, which the working tree installs
    const world = await TestComputer.world({ bundled: [await TestDistribution.fixture("board")] });
    const opened = await TestComputer.open({ world });
    const { machine } = opened;
    const { owner } = opened.world;
    const signedIn = await opened.signIn(OWNER_EMAIL);
    await opened.enroll(signedIn, owner.account);
    const space = await opened.space(signedIn, owner.account, "work");
    const service = await stackOf(opened, signedIn.id, space);
    const client = opened.connect(machinePackage);
    const machineId = machine.id;
    const self = principal.machine.reference(Scope.universe.id, machineId);

    // lay out a working tree of one stack package, its stack package installed as a release
    const { files } = opened.environment;
    const root = await files.realPath(await files.makeTemporaryDirectory("destack-checkout-"));
    onTestFinished(() => files.remove(root, { recursive: true }));
    for (const [path, text] of Object.entries(STACK_CHECKOUT)) {
        await files.mkdir(dirname(join(root, path)), { recursive: true });
        await files.writeTextFile(join(root, path), text);
    }
    await files.mkdir(join(root, "stack", "node_modules", "@destack"), { recursive: true });
    await files.symlink(INSTALLED_STACK, join(root, "stack", "node_modules", "@destack", "stack"));
    expect(await testHost().process.spawn(["git", "init", "--quiet", root]).exited).toBe(0);

    // register its stack package's directory
    const stackDirectory = join(root, "stack");
    const registered = await client.checkout.create({
        machineId,
        requestId: RequestId.create(opened.environment),
        directory: stackDirectory,
        source: { kind: "existing" },
    });

    // build and evaluate its personal export in the space
    const followed = {
        kind: "checkout",
        machine: machineId,
        checkout: registered.id,
        directory: "stack",
    };
    const submit = (directory: string) =>
        client.checkout.submit({
            machineId,
            id: registered.id,
            requestId: RequestId.create(opened.environment),
            space,
            login: signedIn.id,
            directory,
            export: "personal",
            parameters: {},
        });
    const { submission } = await submit("./stack/");
    const { build, ...evaluated } = submission;
    if (build.kind !== "build") {
        throw new TypeError("the submission names no build");
    }
    const built = await service.build.get({ spaceId: space, id: build.build });
    if (built.manifest === null) {
        throw new TypeError("the build has no manifest");
    }
    const reader = await (await machine.space(space)).builds.contents(built.manifest);
    expect({
        checkout: registered,
        submission: evaluated,
        build,
        built: [built.source, built.outputs, built.state, built.phase, built.failure],
        stored: [PackageManifest.package(reader.manifest).id, Object.keys(reader.manifest.outputs)],
    }).toEqual({
        checkout: {
            id: registered.id,
            scope: machineId,
            directory: stackDirectory,
            root,
            tags: {},
            createdAt: registered.createdAt,
            ...ObjectReference.columns("createdBy", self),
            updatedAt: registered.createdAt,
            ...ObjectReference.columns("updatedBy", self),
            revision: 1,
            managerInstallationId: null,
            managerPackageId: null,
            managerName: null,
            detachedAt: null,
        },
        submission: {
            selection: followed,
            stack: { export: "personal", parameters: {} },
        },
        build: { kind: "build", build: built.id },
        built: [followed, ["bun", "workerd"], "succeeded", null, null],
        stored: [STACK_CHECKOUT_PACKAGE, ["bun", "workerd"]],
    });

    // refuse a stack directory outside the working tree
    await expect(submit("..")).rejects.toEqual(
        new CallError("BAD_REQUEST", {
            reason: "OUTSIDE_CHECKOUT_INVALID",
            domain: "@destack/machine",
            message: "package directory is invalid: it is outside the checkout",
        }),
    );
}, 90_000);

test("derive a workspace's members, and start a preview's next generation once its own or a dependency's sources settle", async () => {
    // enroll the owner's computer, which stores checkouts and previews once enrolled
    const computer = await TestComputer.open();
    const { machine } = computer;
    const client = computer.connect(machinePackage);
    const machineId = machine.id;
    const settle = () => computer.world.simulation.sleep(SETTLED_MILLISECONDS);
    const previewer = await computer.signIn(OWNER_EMAIL);
    await computer.enroll(previewer, computer.world.owner.account);

    // lay out a working tree whose app depends on its library
    const { files } = computer.environment;
    const root = await files.realPath(await files.makeTemporaryDirectory("destack-workspace-"));
    onTestFinished(() => files.remove(root, { recursive: true }));
    expect(await testHost().process.spawn(["git", "init", "--quiet", root]).exited).toBe(0);
    await files.writeTextFile(
        join(root, "bun.lock"),
        JSON.stringify({
            lockfileVersion: 1,
            workspaces: {
                "": { name: "@acme/workspace" },
                "packages/library": { name: "@acme/library" },
                "packages/app": {
                    name: "@acme/app",
                    dependencies: { "@acme/library": "workspace:*" },
                },
                "tools/check": { name: "@acme/check" },
            },
            packages: {},
        }),
    );
    for (const member of ["packages/library", "packages/app", "tools/check"]) {
        await files.mkdir(join(root, member, "src"), { recursive: true });
        await files.mkdir(join(root, member, "node_modules"));
        await files.writeTextFile(join(root, member, "src/index.ts"), "export {};\n");
    }
    for (const member of ["packages/library", "packages/app"]) {
        await files.writeTextFile(join(root, member, "destack.json"), "{}\n");
    }

    // register the working tree and preview its app as a signed-in person
    const registered = await client.checkout.create({
        machineId,
        requestId: RequestId.create(computer.environment),
        directory: root,
        source: { kind: "existing" },
    });
    const previewed = await client.preview.create({
        machineId,
        requestId: RequestId.create(computer.environment),
        checkoutId: registered.id,
        package: "packages/app",
        space: PREVIEWED_SPACE,
        login: previewer.id,
    });
    const generation = async () =>
        nth(
            await machine.database
                .select({ generation: preview.table.generation })
                .from(preview.table)
                .where(eq(preview.table.id, previewed.id)),
            0,
        ).generation;

    // edit the library until the machine's source controller starts the preview's next generation
    const library = join(root, "packages/library/src/index.ts");
    await expect
        .poll(async () => {
            await files.writeTextFile(library, `export const edited = ${computer.clock.now()};\n`);
            await settle();

            return generation();
        }, WATCH_POLL)
        .toBeGreaterThan(1);

    // wait until the library's last edits settle into one generation, unchanged over a settle
    const unchanged = async () => {
        const before = await generation();
        await settle();

        return (await generation()) === before;
    };
    await expect.poll(unchanged, WATCH_POLL).toBe(true);
    const started = await generation();

    // edit the app once and change only an unrelated tool and installed dependencies after
    await files.writeTextFile(join(root, "packages/app/src/index.ts"), "export const app = 1;\n");
    const settled = async () => {
        await settle();

        return generation();
    };
    await expect.poll(settled, WATCH_POLL).toBe(started + 1);
    await files.writeTextFile(join(root, "packages/app/node_modules/marker"), "installed\n");
    await files.writeTextFile(join(root, "tools/check/src/index.ts"), "export const check = 1;\n");
    await settle();

    expect({
        packages: await client.checkout.members({ machineId, id: registered.id }),
        generation: await generation(),
    }).toEqual({
        packages: ["packages/app", "packages/library"],
        generation: started + 1,
    });
}, 30_000);
