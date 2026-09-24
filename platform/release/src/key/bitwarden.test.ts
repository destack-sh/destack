import { expect, test } from "@destack/test";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { Bitwarden } from "./bitwarden.ts";

test("persist and verify hardware credentials through private CLI input", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-bitwarden-test-"));
    const previous = {
        PATH: process.env.PATH,
        BW_SESSION: process.env.BW_SESSION,
        DESTACK_VAULT_FIXTURE: process.env.DESTACK_VAULT_FIXTURE,
        DESTACK_BITWARDEN_RECORDS: process.env.DESTACK_BITWARDEN_RECORDS,
    };
    try {
        // replace only this process's CLI with a fixture that records its public arguments
        const source = fileURLToPath(new URL("./tests/bitwarden.ts", import.meta.url));
        await writeFile(
            join(directory, "bw"),
            `#!${process.execPath}\nimport ${JSON.stringify(source)};\n`,
            { mode: 0o700 },
        );
        process.env.PATH = directory + ":" + previous.PATH;
        process.env.BW_SESSION = "fixture-session";
        process.env.DESTACK_VAULT_FIXTURE = directory;
        process.env.DESTACK_BITWARDEN_RECORDS = join(directory, "records.json");
        Bitwarden.sync();
        expect(Bitwarden.get("23503038")).toBeUndefined();

        // verify generated credentials survive creation, synchronization and retrieval
        const credential = Bitwarden.create("23503038");
        expect(Bitwarden.get("23503038")).toEqual(credential);
        expect(Number(credential.pin).toString().padStart(8, "0")).toBe(credential.pin);
        expect(credential.puk).toHaveLength(8);
        expect(Buffer.from(credential.management, "hex").toString("hex")).toBe(
            credential.management,
        );
        expect(Buffer.from(credential.management, "hex")).toHaveLength(32);
        const commands = (await readFile(join(directory, "commands"), "utf8"))
            .trim()
            .split("\n")
            .map((line) => JSON.parse(line));
        expect(commands).toEqual([
            ["status"],
            ["sync"],
            ["list", "items", "--search", "Destack release root / YubiKey 23503038"],
            ["create", "item"],
            ["sync"],
            ["get", "item", "fixture-item"],
            ["list", "items", "--search", "Destack release root / YubiKey 23503038"],
        ]);
        const stored = JSON.parse(await readFile(join(directory, "item"), "utf8"));
        expect(stored.fields).toEqual([
            { name: "pin", value: credential.pin, type: 1 },
            { name: "puk", value: credential.puk, type: 1 },
            { name: "management", value: credential.management, type: 1 },
        ]);

        // persist a separate archive identity without including it in process arguments
        const recoveryName = "Destack release backup / fixture.age";
        const identity = "disposable recovery identity";
        Bitwarden.saveRecovery(recoveryName, identity);
        const recovery = JSON.parse(await readFile(join(directory, "item"), "utf8"));
        expect(recovery.name).toBe(recoveryName);
        expect(recovery.fields).toEqual([{ name: "identity", value: identity, type: 1 }]);
        expect(Bitwarden.readRecovery(recoveryName)).toBe(identity);
        expect(Bitwarden.readRecovery("absent backup")).toBeUndefined();
        expect(() => Bitwarden.saveRecovery(recoveryName, identity)).toThrow(
            "a Bitwarden recovery record with this name already exists",
        );

        // retain credential access after arbitrary display-name changes
        await writeFile(
            join(directory, "records.json"),
            JSON.stringify({
                [recoveryName]: "fixture-item",
                "Destack release root / YubiKey 23503038": "fixture-item",
            }),
        );
        await writeFile(
            join(directory, "item"),
            JSON.stringify({ ...recovery, name: "renamed recovery" }),
        );
        expect(Bitwarden.readRecovery(recoveryName)).toBe(identity);
        await writeFile(
            join(directory, "item"),
            JSON.stringify({ ...stored, name: "renamed hardware" }),
        );
        expect(Bitwarden.get("23503038")).toEqual(credential);
    } finally {
        for (const [name, value] of Object.entries(previous)) {
            if (value === undefined) {
                delete process.env[name];
            } else {
                process.env[name] = value;
            }
        }
        await rm(directory, { recursive: true, force: true });
    }
}, 5000);
