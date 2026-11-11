import { Failure } from "@destack/error";
import { MACHINE_KEYS } from "./computer.ts";
import { expect, refusal, test } from "@destack/test";
import { SpaceMount } from "@destack/object";
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

    // receive the failure its start met for want of the machine's key, captured once by its close
    const { code, reason, message } = Failure.of(await opened.reported);
    const missing = "key of this machine not found";
    await opened.close();
    expect({
        reported: [code, reason, message],
        captured: world.telemetry.take().map((captured) => captured.message),
    }).toEqual({ reported: ["NOT_FOUND", "MACHINE_KEY_NOT_FOUND", missing], captured: [missing] });
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
            "SERVER_UNAVAILABLE",
            `server '${home.machine.id}' is unavailable: it is stopped: it stopped serving`,
        ],
    ]);
});

test("answer a served space's liveness and readiness below its mount, as a server answers its own", async () => {
    // probe the owner's served home
    const { home } = await TestComputer.home();
    const spaceId = home.world.owner.home;
    await home.ready(spaceId);
    const space = await home.machine.space(spaceId);
    const probe = async (path: string) => {
        const answered = await space.fetch(
            new Request(`${SpaceMount.url("http://127.0.0.1", spaceId)}${path}`),
        );

        return [answered.status, await answered.text()];
    };

    // answer both ok while every installation serves its revision
    expect([await probe("/livez"), await probe("/readyz")]).toEqual([
        [200, "ok\n"],
        [200, "ok\n"],
    ]);
});
