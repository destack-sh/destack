import { Jwt } from "@destack/identity";
import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import accountPackage from "../package.ts";
import { TestAccountStore } from "../test/index.ts";
import { SECOND_MILLISECONDS } from "../authentication/token.ts";

/** How far the store's clock runs ahead of the wall clock in the test: one day. */
const AHEAD_MILLISECONDS = 86_400_000;

test("answer an exchanged token's expiry on the store's clock, as its signed claims carry it", async () => {
    // exchange a person's token for a package once the store's clock runs a day ahead of the wall clock
    await using accounts = await TestAccountStore.open();
    const person = await accounts.signIn("ada@example.test");
    await accounts.simulation.jump(accounts.clock.now() + AHEAD_MILLISECONDS);
    const exchanged = await person.exchange(accountPackage.id);

    // answer the signed token's expiry, in milliseconds
    const { exp } = Jwt.claims(exchanged.accessToken);
    expect(exchanged.expiresAt).toBe(present(exp, "the token's expiry") * SECOND_MILLISECONDS);
});
