import { join } from "node:path";
import { expect, onTestFinished, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { BunKeychain } from "./keychain.ts";

test("store a service's secrets in a private file, compare before replacing or forgetting one, and read them back", async () => {
    // open a file keychain in a fresh directory
    const { files } = testHost();
    const directory = await files.makeTemporaryDirectory("destack-keychain-");
    onTestFinished(() => files.remove(directory, { recursive: true, force: true }));
    const store = { kind: "file", directory: join(directory, "keychain") } as const;
    const keychain = new BunKeychain("app.destack.test", files, store);

    // save two secrets, replace one only at its expected value, forget the other only at its expected value
    await keychain.save("root", "first");
    await keychain.save("machine", "key");
    const outcomes = [
        await keychain.saveIf("root", "other", "second"),
        await keychain.saveIf("root", "first", "second"),
        await keychain.removeIf("machine", "other"),
        await keychain.removeIf("machine", "key"),
    ];

    // read them back through a second keychain on the same files, the file and its directory private
    const reopened = new BunKeychain("app.destack.test", files, store);
    const file = join(store.directory, "app.destack.test.json");
    expect({
        outcomes,
        root: await reopened.load("root"),
        machine: await reopened.load("machine"),
        stored: await files.readTextFile(file),
        fileMode: (await files.stat(file)).mode & 0o777,
        directoryMode: (await files.stat(store.directory)).mode & 0o777,
    }).toEqual({
        outcomes: [false, true, false, true],
        root: "second",
        machine: undefined,
        stored: JSON.stringify({ root: "second" }),
        fileMode: 0o600,
        directoryMode: 0o700,
    });
});
