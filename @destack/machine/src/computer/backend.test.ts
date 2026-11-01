import { Failure } from "@destack/error";
import { MACHINE_KEYS } from "./computer.ts";
import { expect, refusal, test } from "@destack/test";
import { OWNER_EMAIL, TestComputer } from "./test/index.ts";

test("report a restarted machine's missing key as the failure of its start", async () => {
    // enroll a machine, then lose its key from the system's keychain and start it again
    const first = await TestComputer.open();
    const { world, directory } = first;
    await first.enroll(await first.signIn(OWNER_EMAIL), world.owner.account);
    await first.close();
    const keychains = Object.fromEntries(
        Object.entries(first.secrets()).filter(([service]) => service !== MACHINE_KEYS),
    );
    const opened = await TestComputer.open({ world, directory, keychains });

    // receive the failure its start met for want of the machine's key
    const { code, reason, message } = Failure.of(await opened.reported);
    expect([code, reason, message]).toEqual([
        "NOT_FOUND",
        "MACHINE_KEY_MISSING",
        `machine ${opened.machine.id} has no key`,
    ]);
});

test("start a space while the machine serves, and refuse to start it once the machine closed", async () => {
    // serve the owner's home, then close the machine
    const { home } = await TestComputer.home();
    const spaceId = home.world.owner.home;
    await home.ready(spaceId);
    const served = (await home.machine.space(spaceId)).id;
    await home.close();

    // refuse to start the space again, as no stop would close it
    expect([served, await refusal(home.machine.space(spaceId))]).toEqual([
        spaceId,
        [
            "SERVICE_UNAVAILABLE",
            "MACHINE_NOT_SERVING",
            `machine ${home.machine.id} cannot serve spaces while idle`,
        ],
    ]);
});
