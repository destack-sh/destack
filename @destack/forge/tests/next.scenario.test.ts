import { expect, test } from "@destack/test";
import { TestForge } from "../src/test/index.ts";
import { fixtureBuild } from "./fixture/build.ts";

test("read the version a name's next release takes: the month's first, or the one after a release of the month", async () => {
    // publish a release of an earlier month
    const forge = await TestForge.open();
    const published = await forge.publish((await fixtureBuild("answer")).reader);
    const client = forge.client(forge.owner);
    const next = async (name = "@example/answer") =>
        (await client.package.next({ spaceId: forge.space, name })).version;
    const month = new Date(forge.environment.clock.now()).toISOString().slice(0, 7);

    // start the clock's month at its first release
    const first = await next();

    // follow a release of the clock's month
    await forge.publish((await fixtureBuild("answer", first)).reader);
    expect([
        month,
        published.release.version,
        first,
        await next(),
        await next("@example/unpublished"),
    ]).toEqual(["2026-10", "2026.9.0", "2026.10.0", "2026.10.1", "2026.10.0"]);
});
