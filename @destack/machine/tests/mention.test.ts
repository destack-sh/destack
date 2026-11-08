import { and, eq, Snapshot } from "@destack/db";
import * as accountObject from "@destack/account/object";
import type { Login } from "@destack/account/native";
import type { TestPerson } from "@destack/account/test";
import accountPackage from "@destack/account/package";
import { accessRelationship, Caller, principal, Role } from "@destack/access";
import {
    OWNER_EMAIL,
    PUBLISHER,
    SCENARIO_MILLISECONDS,
    TestComputer,
    type TestComputerOptions,
    type TestWorld,
    WorldDriver,
} from "@destack/machine/test";
import { activity } from "@destack/notification/object";
import notificationPackage from "@destack/notification/package";
import messagePackage from "@destack/message/package";
import vaultPackage from "@destack/vault/package";
import { installation, PackageMount } from "@destack/object";
import * as stackObject from "@destack/stack/object";
import { createClient } from "@destack/object/client";
import { found, Identifier, present, schema } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { expect, test } from "@destack/test";
import boardPackage from "@destack/machine-mention-fixture/package";

/** The email of the person the owner shares the board's account with. */
const STRANGER_EMAIL = "stranger@example.test";

/** A computer of the owner's serving a space, with the owner signed in on its device. */
interface Served {
    /** The computer. */
    readonly computer: TestComputer;
    /** The owner's login on the computer's device. */
    readonly login: Login["row"];
    /** The space the computer serves. */
    readonly space: Identifier<"space">;
}

test(
    "project a mention on a card in a sandboxed space on one computer into the inbox of the person's home on another",
    { timeout: SCENARIO_MILLISECONDS },
    async () => {
        // serve the owner's home on the cloud computer, its inbox in process as computers run it
        const { home, organizations } = await TestComputer.home({ organizations: [PUBLISHER] });
        const { world } = home;

        // serve the board's space on another computer behind the relay, shared with another person
        const relay = await TestComputer.relay(world);
        const account = found(organizations, PUBLISHER.handle);
        const source = await openBoard(home, account, { relay: relay.url });
        const stranger = await grantStranger(world, source);

        // let that person mention the owner on a card the owner may read
        const recipient = principal.user.reference(Scope.universe.id, world.owner.id);
        const client = boardClient(source, stranger);
        const spaceId = source.space;
        const now = () => RequestId.create(world.universe.environment);
        const created = await client.card.create({ spaceId, requestId: now(), title: "Launch" });
        await client.card.grant({
            spaceId,
            requestId: now(),
            id: created.id,
            relation: "viewer",
            subject: recipient,
        });
        await client.card.remark({
            spaceId,
            requestId: now(),
            id: created.id,
            text: "a question for you",
            mentions: [recipient],
        });

        // read the owner's notifications from the board's space once one arrives in the home's inbox
        const notifications = (await home.machine.space(world.owner.home)).database;
        const read = async () =>
            (await notifications.select().from(activity.table))
                .filter((row) => row.recipientId === recipient.id && row.scope === spaceId)
                .map((row) => [row.content.body, row.content.title, row.summary]);
        await home.waitUntil(notifications, async () => (await read()).length === 1);
        expect(await read()).toEqual([["⁨a question for you⁩", "Mentioned", "1 mention"]]);
    },
);

/** Serve a board's space of a shared account on a computer, with its vault, messages and inbox. */
async function openBoard(
    home: TestComputer,
    account: Identifier<"account">,
    options: TestComputerOptions = {},
): Promise<Served> {
    // serve the board's space of the shared account on a computer enrolled in it
    const { world } = home;
    const computer = await TestComputer.open({ ...options, world });
    const login = await computer.signIn(OWNER_EMAIL);
    await computer.enroll(login, account);
    const space = await computer.space(login, account, "board");

    // install the board and the platform packages it needs that the space's stack lacks
    const database = (await computer.machine.space(space)).database;
    for (const [packageId, fixture, name] of [
        [vaultPackage.id, "vault", "vault"],
        [messagePackage.id, "message", "message"],
        [notificationPackage.id, "notification", "inbox"],
        [boardPackage.id, "mention", "board"],
    ] as const) {
        const installed = await Snapshot.live(database)
            .read(installation.table)
            .findFirst({ where: { scope: space, packageId } });
        // publish a missing one as the platform's operator, and install it
        if (installed === undefined) {
            const build = await WorldDriver.fixture(fixture);
            await world.distribution.publish(build, await world.universe.operator(), home.machine);
            await computer.install(login.id, space, build, name);
        }
    }
    const served = { computer, login, space };
    await computer.waitUntil(database, async () => (await storePackage(served)) !== undefined);

    // wait until the board's sandboxed instance serves
    const board = await Snapshot.live(database)
        .read(installation.table)
        .findFirst({ where: { scope: space, packageId: boardPackage.id } });
    const boardId = present(board, "the board's installation").id;
    await computer.waitUntil(
        database,
        async () => (await stackObject.Instance.endpoints(database, boardId)).length > 0,
    );

    return served;
}

/** Find the board package a computer's space store serves. */
async function storePackage(served: Served) {
    const { instance } = await served.computer.machine.space(served.space);

    return instance.packages.find((handle) => handle.id === boardPackage.id);
}

/** Let another person own a space's account with the owner, waiting for the computer's replica. */
async function grantStranger(world: TestWorld, served: Served): Promise<TestPerson> {
    // grant the stranger the predefined owner role on the account as its owner
    const stranger = await world.universe.signIn(STRANGER_EMAIL);
    const { computer, space } = served;
    const [placed] = await accountObject.Placement.find(computer.machine.directory.snapshot, [
        space,
    ]);
    const accountId = schema
        .identifier("account")
        .parse(present(placed, "the space's placement").scope);
    await world.owner.client(accountPackage).account.grant({
        scope: world.owner.id,
        id: accountId,
        requestId: RequestId.create(computer.environment),
        role: Role.OWNER,
        subject: principal.user.reference(Scope.universe.id, stranger.id),
    });

    // wait for the computer's replica of the grant
    const database = (await computer.machine.space(space)).database;
    await computer.waitUntil(
        database,
        async () =>
            (
                await database
                    .select({ id: accessRelationship.id })
                    .from(accessRelationship)
                    .where(
                        and(
                            eq(accessRelationship.objectId, accountId),
                            eq(accessRelationship.subjectId, stranger.id),
                        ),
                    )
            ).length === 1,
    );

    return stranger;
}

/** Call the board package in a computer's space store as a person. */
function boardClient(served: Served, person: TestPerson) {
    const { computer, space } = served;
    const subject = principal.user.reference(Scope.universe.id, person.id);

    return createClient(
        boardPackage,
        {
            url: `http://board.test${PackageMount.path(boardPackage.id)}`,
            fetch: async (request) => {
                // find the store's board package and hand it the call as the person
                const board = present(await storePackage(served), "the board");
                const now = computer.clock.now();
                const authentication = {
                    caller: Caller.of(subject),
                    credential: {
                        kind: "session",
                        id: Identifier.create("session", computer.environment),
                        audience: boardPackage.id,
                        scope: space,
                        verifiedAt: now,
                        expiresAt: now + 60_000,
                    },
                };
                const mounted = present(PackageMount.of(request), "the board's mount");
                const { instance } = await computer.machine.space(space);

                return instance.fetch(board, mounted.request, authentication);
            },
        },
        computer.device.environment,
    );
}
