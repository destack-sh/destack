import { currentLogin, currentSpace } from "@destack/account/native";
import { PackageMount } from "@destack/object";
import { DynamicObjectClient } from "@destack/object/client";
import { rollout } from "@destack/stack/object";
import stackPackage from "@destack/stack/package";
import { setting } from "@destack/setting/object";
import { Identifier, present } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { focus } from "@destack/notification/setting";
import { OWNER_EMAIL, TestComputer } from "../test/index.ts";
import machinePackage from "../../package.ts";
import { signIn } from "./object/index.ts";
import { PEOPLE_PATH } from "./credential.ts";

/** How long a sign-in may take on the simulated clock: a minute. */
const SIGN_IN_MILLISECONDS = 60_000;

/** The email of a second person signed in on the computer. */
const OTHER_EMAIL = "grace@example.test";

test("store this computer's space and login settings as the native device's local values", async () => {
    // sign the owner in, then read the defaults and set the space twice and the login once
    const computer = await TestComputer.open();
    const { device } = computer;
    const signedIn = await computer.signIn(OWNER_EMAIL);
    const defaults = [await device.setting(currentSpace), await device.setting(currentLogin)];
    const work = Identifier.create("space", device.environment);
    const home = Identifier.create("space", device.environment);
    await device.set(currentSpace, work);
    await device.set(currentSpace, home);
    await device.set(currentLogin, signedIn.id);
    const rows = await device.settings.query.deviceSetting.findMany();

    // expect one local value per setting
    expect({
        defaults,
        resolved: [await device.setting(currentSpace), await device.setting(currentLogin)],
        rows: rows.map((row) => [row.name, row.value, row.revision]),
    }).toEqual({
        defaults: [null, null],
        resolved: [home, signedIn.id],
        rows: [
            ["space", home, 2],
            ["login", signedIn.id, 1],
        ],
    });
});

test("act as the person of the login setting while several are signed in", async () => {
    // sign two people in on this computer
    const computer = await TestComputer.open();
    const { device } = computer;
    const owner = await computer.signIn(OWNER_EMAIL);
    const other = await computer.signIn(OTHER_EMAIL);

    // refuse to choose between them and act as the one the setting names
    const unchosen = await refusal(device.login());
    await device.set(currentLogin, other.id);
    expect({
        unchosen,
        chosen: (await device.login())?.id,
        signedIn: [owner.name, other.name],
    }).toEqual({
        unchosen: [
            "CONFLICT",
            "PERSON_AMBIGUOUS",
            "several people are signed in: choose one with the login setting",
        ],
        chosen: other.id,
        signedIn: [OWNER_EMAIL, OTHER_EMAIL],
    });
});

test("refuse to read the account of a machine nobody enrolled", async () => {
    const { device } = await TestComputer.open();

    // read the account before anyone enrolled the machine
    expect(await refusal(device.account())).toEqual([
        "PRECONDITION_FAILED",
        "ENROLLMENT_REQUIRED",
        "this machine is not enrolled in a personal account: sign in to enroll it",
    ]);
});

test("route a describe call without a scope in its body to the stack package of the space its path names", async () => {
    // call the stack package of the owner's home at its machine, whose describe body has no scope
    const { home, login } = await TestComputer.home();
    const { device, world } = home;
    const stack = new DynamicObjectClient({
        objects: [rollout],
        scope: world.owner.home,
        endpoint: (await device.launcher(login)).endpoint(
            stackPackage.id,
            await device.locate(world.owner.home),
        ),
        environment: home.device.environment,
    });

    // describe the rollouts
    const described = await stack.describe([rollout.typeReference]);

    // expect the rollout type first, then the types it refers to
    expect(described.map((type) => type.name)).toEqual([
        "rollout",
        "installation-revision",
        "build",
        "machine",
        "residency",
        "key",
        "account",
        "organization",
        "user",
        "space",
    ]);
});

test("push a person's mutation through a scope the session opens, confirmed once the universe applies it", async () => {
    // sign the owner in and follow their settings
    const computer = await TestComputer.open();
    const { device } = computer;
    const signedIn = await computer.signIn(OWNER_EMAIL);
    const session = await device.launcher(signedIn.id);
    await using stack = new AsyncDisposableStack();
    const settings = await device.open(session, { scope: signedIn.subjectId }, { setting }, stack);

    // set the focus through that client
    const value = { schedules: [], allowed: [], isTimeSensitiveAllowed: false };
    await settings.mutate(setting).create({
        ...focus.reference,
        value,
        release: focus.package.version,
        mode: "set",
    }).confirmed;

    // expect the universe to resolve it
    expect(await device.setting(focus)).toEqual(value);
});

test("forward the owner's calls as a signed-in person to a space's package, refusing an unknown target", async () => {
    // reach the home's stack package through the machine as the owner's person
    const { home, login } = await TestComputer.home();
    const { device, world } = home;
    const machine = device.loopback.options(machinePackage.id);
    const people = `http://127.0.0.1${PackageMount.path(machinePackage.id)}${PEOPLE_PATH}/${login}`;
    const stack = new DynamicObjectClient({
        objects: [rollout],
        scope: world.owner.home,
        endpoint: { ...machine, url: `${people}/space/${world.owner.home}/${stackPackage.id}` },
        environment: device.environment,
    });

    // describe the rollouts there, and through a target of no kind
    const described = await stack.describe([rollout.typeReference]);
    const nowhere = new DynamicObjectClient({
        objects: [rollout],
        scope: world.owner.home,
        endpoint: { ...machine, url: `${people}/elsewhere/${stackPackage.id}` },
        environment: device.environment,
    });

    // expect the space's answer, as the session gives it directly, and the target refused
    expect({
        described: described.map((type) => type.name).slice(0, 2),
        nowhere: await refusal(nowhere.describe([rollout.typeReference])),
    }).toEqual({
        described: ["rollout", "installation-revision"],
        nowhere: [
            "BAD_REQUEST",
            "TARGET_INVALID",
            "elsewhere is no target: use universe/<package>, space/<space>/<package> or installation/<space>/<installation>/<package>",
        ],
    });
});

test("resolve the owner's spaces by address, identifier, home and residency, refusing an unknown name", async () => {
    // resolve the owner's home three ways, their residency's space, and a space they have none of
    const { home, login } = await TestComputer.home();
    const { device, world } = home;
    const session = await device.launcher(login);
    const address = await session.address(world.owner.home);
    const residency = await session.residency(await session.account());

    // expect each name of the home to resolve to its space, and the other refused
    expect({
        resolved: [
            await session.resolve(address, {}),
            await session.resolve(world.owner.home, {}),
            await session.resolve("home", {}),
            await session.resolve("residency", {}),
        ],
        nowhere: await refusal(session.resolve("nowhere", {})),
    }).toEqual({
        resolved: [world.owner.home, world.owner.home, world.owner.home, residency],
        nowhere: ["NOT_FOUND", "SPACE_NOT_FOUND", "no space of yours is called nowhere"],
    });
});

test("sign a person up through a sign-in the machine carries out, answering the code mailed to them", async () => {
    // start signing up by mail through the machine, as a program of this computer
    const computer = await TestComputer.open();
    const { device, world } = computer;
    await using stack = new AsyncDisposableStack();
    const local = await device.objects({ signIn });
    stack.defer(() => local.close());
    const email = "lin@example.test";
    const signUp = { name: "Lin", handle: "lin", residencyId: "eu", timeZone: "UTC", locale: "en" };
    const { id } = await local.mutate(signIn).start({ method: { kind: "signUp", email, signUp } })
        .predicted;
    const read = async () =>
        present(await local.query.signIn.findFirst({ where: { id } }), "the sign-in").state;

    // answer the mailed code once the sign-in waits for it, then wait for the login
    const deadline = () => world.simulation.clock.now() + SIGN_IN_MILLISECONDS;
    await world.simulation.waitUntil(async () => (await read()).kind !== "starting", deadline());
    const waiting = await read();
    const codes = await world.universe.mail.codes();
    const code = codes.findLast((mailed) => mailed.email === email);
    await local.mutate(signIn).answer({ id, code: present(code, "the mailed code").otp }).predicted;
    await world.simulation.waitUntil(async () => (await read()).kind !== "answered", deadline());
    const signedIn = await read();

    // expect the wait for the mailed code, then the login the machine signed in
    const logins = await device.logins.query.login.findMany();
    expect({ waiting, signedIn, logins: logins.map((login) => login.id) }).toEqual({
        waiting: { kind: "waiting", email },
        signedIn: { kind: "signedIn", login: logins[0]?.id },
        logins: [expect.any(String)],
    });
});
