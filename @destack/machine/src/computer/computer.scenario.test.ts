import { dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { CLIENT_KEYS, SESSIONS } from "@destack/account/native";
import { endpoint } from "@destack/account/object";
import { Snapshot } from "@destack/db";
import { RequestId } from "@destack/sync";
import { expect, test } from "@destack/test";
import machinePackage from "../package.ts";
import { Computer, MACHINE_KEYS, ROOT_KEYS } from "./computer.ts";
import { OWNER_EMAIL, TestComputer } from "./test/index.ts";

/** The template stack package, used as a registered checkout. */
const templateStack = dirname(
    dirname(fileURLToPath(import.meta.resolve("@destack/template-stack"))),
);

/** Another machine's entry in the system's keychains, which erasing leaves. */
const OTHER_MACHINE = "machine-01a10f00-0000-7000-8000-000000000003";

/** Another person's entry in the system's keychains, which erasing leaves. */
const OTHER_PERSON = "user-01a10f00-0000-7000-8000-000000000002@https://universe.test";

/** Another client's entry in the system's keychains, which erasing leaves. */
const OTHER_CLIENT = "client-01a10f00-0000-7000-8000-000000000005";

test("erase a stopped machine's keys, its logins' sessions and client keys and its directory, leaving other entries", async () => {
    // sign the owner in beside other machines', people's and clients' keychain entries
    const computer = await TestComputer.open();
    const { directory } = computer;
    await computer.signIn(OWNER_EMAIL);
    await computer.keychain(SESSIONS).save(OTHER_PERSON, "other");
    await computer.keychain(CLIENT_KEYS).save(OTHER_CLIENT, "other");
    for (const service of [MACHINE_KEYS, ROOT_KEYS]) {
        await computer.keychain(service).save(OTHER_MACHINE, "other");
    }

    // stop the computer and erase its state
    await computer.close();
    await Computer.erase(directory, (service) => computer.keychain(service), computer.environment);

    // leave only the other entries
    const remaining = [SESSIONS, CLIENT_KEYS, MACHINE_KEYS, ROOT_KEYS].map((service) => [
        service,
        [...computer.keychain(service).secrets.keys()],
    ]);
    expect({ remaining, isRetained: await computer.environment.files.exists(directory) }).toEqual({
        remaining: [
            [SESSIONS, [OTHER_PERSON]],
            [CLIENT_KEYS, [OTHER_CLIENT]],
            [MACHINE_KEYS, [OTHER_MACHINE]],
            [ROOT_KEYS, [OTHER_MACHINE]],
        ],
        isRetained: false,
    });
});

test("answer a relay-less computer's spaces at its loopback endpoint, and a relayed one's at the name its relay publishes", async () => {
    // serve a space on a computer without a relay, and on one through the residency's relay
    const world = await TestComputer.world();
    const relay = await TestComputer.relay(world);
    const [local, relayed] = [
        await TestComputer.open({ world }),
        await TestComputer.open({ world, relay: relay.url }),
    ];
    for (const computer of [local, relayed]) {
        const login = await computer.signIn(OWNER_EMAIL);
        await computer.enroll(login, world.owner.account);
        await computer.space(login, world.owner.account, `work-${computer.machine.id.slice(-6)}`);
    }

    // wait until the relay publishes the relayed computer's tunnel, at the name it told the computer
    const universe = world.universe.storage.database;
    const published = async () =>
        await Snapshot.live(universe)
            .read(endpoint.table)
            .findFirst({ where: { parentId: relayed.machine.id } });
    await relayed.waitUntil(universe, async () => (await published())?.relay?.url !== undefined);
    const tunnel = await published();
    expect([local.machine.origin, relayed.machine.origin, tunnel?.relay?.url]).toEqual([
        local.endpoint,
        tunnel?.url,
        new URL(relay.url).origin,
    ]);
});

test("list a relayed computer's checkouts as its account's owner on another computer, by a token bound to its partition", async () => {
    // enroll a computer tunnelled through the residency's relay, and register a checkout as its local owner
    const world = await TestComputer.world();
    const relay = await TestComputer.relay(world);
    const relayed = await TestComputer.open({ world, relay: relay.url });
    await relayed.enroll(await relayed.signIn(OWNER_EMAIL), world.owner.account);
    const registered = await relayed.connect(machinePackage).checkout.create({
        machineId: relayed.machine.id,
        requestId: RequestId.create(relayed.environment),
        directory: templateStack,
        source: { kind: "existing" },
    });

    // wait until the relay publishes the relayed computer's tunnel
    const universe = world.universe.storage.database;
    await relayed.waitUntil(universe, async () => {
        const published = await Snapshot.live(universe)
            .read(endpoint.table)
            .findFirst({ where: { parentId: relayed.machine.id } });

        return published?.relay?.url !== undefined;
    });

    // list the checkouts at the relayed name as the owner signed in on another computer
    const other = await TestComputer.open({ world });
    const { session } = await other.device.launcher((await other.signIn(OWNER_EMAIL)).id);
    const client = session.client(machinePackage, relayed.machine.origin, {
        partition: relayed.machine.id,
    });
    const { items } = await client.checkout.list({ machineId: relayed.machine.id });
    expect(items.map((item) => item.id)).toEqual([registered.id]);
});
