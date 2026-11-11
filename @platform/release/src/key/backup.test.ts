import { expect, onTestFinished, refusal, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { join } from "node:path";
import { BackupKey, readBackup } from "./backup.ts";

test("encrypt and restore a signing archive without replacing an existing backup", async () => {
    // keep the disposable signing material in a directory of the test's own
    const environment = testHost();
    const { files } = environment;
    const directory = await files.makeTemporaryDirectory("destack-backup-test-");
    onTestFinished(() => files.remove(directory, { recursive: true, force: true }));

    // exercise real tar and age with disposable signing material
    await files.writeTextFile(join(directory, "key.pem"), "disposable fixture key\n");
    const archive = await readBackup(directory, environment);
    const key = await BackupKey.generate(environment);
    const output = join(directory, "backup.age");
    await key.write(archive, output);
    const encrypted = await files.readFile(output);

    // reuse the persisted identity after an interrupted destination write
    const resumed = await BackupKey.of(key.identity, environment);
    expect(resumed.recipient).toBe(key.recipient);
    await resumed.write(archive, join(directory, "resumed.age"));

    // preserve the first verified archive when a caller repeats the operation
    expect(await refusal(key.write(archive, output))).toEqual([
        "CONFLICT",
        "FILE_EXISTS",
        `cannot open ${output}: EEXIST`,
    ]);
    expect(await files.readFile(output)).toEqual(encrypted);
}, 5000);
