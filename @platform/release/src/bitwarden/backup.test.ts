import { expect, test } from "@destack/test";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { BackupKey, readBackup } from "./backup.ts";

test("encrypt and restore a signing archive without replacing an existing backup", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-backup-test-"));
    try {
        // exercise real tar and age with disposable signing material
        await writeFile(join(directory, "key.pem"), "disposable fixture key\n");
        const archive = readBackup(directory);
        const key = new BackupKey();
        const output = join(directory, "backup.age");
        await key.write(archive, output);
        const encrypted = await readFile(output);

        // reuse the persisted identity after an interrupted destination write
        const resumed = new BackupKey(key.identity);
        expect(resumed.recipient).toBe(key.recipient);
        await resumed.write(archive, join(directory, "resumed.age"));

        // preserve the first verified archive when a caller repeats the operation
        await expect(key.write(archive, output)).rejects.toMatchObject({ code: "EEXIST" });
        expect(await readFile(output)).toEqual(encrypted);
    } finally {
        await rm(directory, { recursive: true, force: true });
    }
}, 5000);
