import { expect, test } from "@destack/test";
import { Keychain, MemoryKeychain } from "./index.ts";

/** A keychain in memory recording whether each compare-and-set stored its secret. */
class RecordingKeychain extends MemoryKeychain {
    /** Whether each compare-and-set stored its secret, in order. */
    readonly outcomes: boolean[] = [];

    /** Store a secret while the stored one equals the expected one, recording the outcome. */
    override async saveIf(
        name: string,
        expected: string | undefined,
        value: string,
    ): Promise<boolean> {
        const isSaved = await super.saveIf(name, expected, value);
        this.outcomes.push(isSaved);

        return isSaved;
    }
}

test("apply both of two racing changes by retrying the loser over the winner's secret, and adopt the stored secret without writing", async () => {
    // append to one secret from two racing writers
    const keychain = new RecordingKeychain();
    const raced = await Promise.all([
        Keychain.update(keychain, "secret", (secret) => `${secret ?? ""}a`),
        Keychain.update(keychain, "secret", (secret) => `${secret ?? ""}b`),
    ]);
    const adopted = await Keychain.update(keychain, "secret", (secret) => secret ?? "c");

    // store both changes and write nothing for the adopted secret
    expect([raced, adopted, keychain.outcomes, await keychain.load("secret")]).toEqual([
        ["a", "ab"],
        "ab",
        [true, false, true],
        "ab",
    ]);
});

test("lock a name exclusively, letting the next lock in once the first disposes, and other names at once", async () => {
    // lock a name, then ask for it again and for another name
    const keychain = new MemoryKeychain();
    const order: string[] = [];
    const first = await keychain.lock("root");
    const second = keychain.lock("root").then((locked) => {
        order.push("second");

        return locked;
    });
    const other = await keychain.lock("other");
    order.push("other");

    // let the second lock in once the first disposes
    await Promise.resolve();
    order.push("first released");
    await first[Symbol.asyncDispose]();
    await (await second)[Symbol.asyncDispose]();
    await other[Symbol.asyncDispose]();

    expect(order).toEqual(["other", "first released", "second"]);
});
