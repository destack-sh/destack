import { expect, test } from "@destack/test";
import { verification } from "../src/authentication/index.ts";
import { TestAccountStore } from "../src/test/index.ts";

test("remove an authentication challenge once it lapses, keeping a live one until then", async () => {
    // store a lapsed challenge and one lapsing in a minute
    await using fixture = await TestAccountStore.open();
    const now = fixture.clock.now();
    const challenge = (id: string, expiresAt: number) => ({
        id,
        identifier: `sign-in-otp-${id}@example.com`,
        value: "code",
        expiresAt,
        createdAt: now,
        updatedAt: now,
    });
    await fixture.database.transaction(
        async (transaction) =>
            await transaction
                .insert(verification)
                .values([challenge("lapsed", now - 1), challenge("live", now + 60_000)]),
    );
    const left = async () =>
        (await fixture.database.select({ id: verification.id }).from(verification))
            .map((row) => row.id)
            .toSorted();

    // remove the lapsed one at once, and the live one once its minute passes
    await fixture.simulation.idle();
    const first = await left();
    await fixture.simulation.jump(now + 60_000);
    await fixture.simulation.idle();
    expect([first, await left()]).toEqual([["live"], []]);
});
