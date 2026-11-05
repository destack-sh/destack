import { currentLogin, currentSpace } from "@destack/account/native";
import { DynamicObjectClient } from "@destack/object/client";
import { rollout } from "@destack/stack/object";
import stackPackage from "@destack/stack/package";
import { setting } from "@destack/setting/object";
import { expect, refusal, test } from "@destack/test";
import { focus } from "@destack/notification/setting";
import { OWNER_EMAIL, TestComputer } from "../test/index.ts";

/** The email of a second person signed in on the computer. */
const OTHER_EMAIL = "grace@example.test";

test("store this computer's space and login settings as the native device's local values", async () => {
    // sign the owner in, then read the defaults and set the space twice and the login once
    const computer = await TestComputer.open();
    const { device } = computer;
    const signedIn = await computer.signIn(OWNER_EMAIL);
    const defaults = [await device.setting(currentSpace), await device.setting(currentLogin)];
    await device.set(currentSpace, "work--acme");
    await device.set(currentSpace, "home--acme");
    await device.set(currentLogin, signedIn.id);
    const rows = await device.settings.query.deviceSetting.findMany();

    // expect one local value per setting
    expect({
        defaults,
        resolved: [await device.setting(currentSpace), await device.setting(currentLogin)],
        rows: rows.map((row) => [row.name, row.value, row.revision]),
    }).toEqual({
        defaults: [null, null],
        resolved: ["home--acme", signedIn.id],
        rows: [
            ["space", "home--acme", 2],
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
        endpoint: (await device.session(login)).endpoint(
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
    const session = await device.session(signedIn.id);
    await using stack = new AsyncDisposableStack();
    const settings = await session.open({ scope: signedIn.subjectId }, { setting }, stack);

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
