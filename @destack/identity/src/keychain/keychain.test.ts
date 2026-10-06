import { expect, test } from "@destack/test";
import { Keychain, MemoryKeychain } from "./index.ts";

/** A keychain in memory recording whether each compare-and-set kept its secret. */
class RecordingKeychain extends MemoryKeychain {
    /** Whether each compare-and-set kept its secret, in order. */
    readonly outcomes: boolean[] = [];

    /** Keep a secret while the kept one equals the expected one, recording the outcome. */
    override async saveIf(
        name: string,
        expected: string | undefined,
        value: string,
    ): Promise<boolean> {
        const isKept = await super.saveIf(name, expected, value);
        this.outcomes.push(isKept);

        return isKept;
    }
}

test("apply both of two racing changes by retrying the loser over the winner's secret, and adopt the kept secret without writing", async () => {
    // append to one secret from two racing writers
    const keychain = new RecordingKeychain();
    const raced = await Promise.all([
        Keychain.update(keychain, "secret", (kept) => `${kept ?? ""}a`),
        Keychain.update(keychain, "secret", (kept) => `${kept ?? ""}b`),
    ]);
    const adopted = await Keychain.update(keychain, "secret", (kept) => kept ?? "c");

    // keep both changes and write nothing for the adopted secret
    expect([raced, adopted, keychain.outcomes, await keychain.load("secret")]).toEqual([
        ["a", "ab"],
        "ab",
        [true, false, true],
        "ab",
    ]);
});
