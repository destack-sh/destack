import { expect, onTestFinished, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { TEST_START, testSimulation } from "@destack/test/world";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { schema } from "@destack/schema";
import { Bitwarden } from "./bitwarden.ts";

/** The name and hidden fields of the vault item the fixture stores. */
const StoredItem = schema
    .object({
        name: schema.string(),
        fields: schema.array(
            schema.object({ name: schema.string(), value: schema.string(), type: schema.number() }),
        ),
    })
    .strip();

test("persist and verify hardware credentials through private CLI input", async () => {
    // keep the fixture's records in a directory of the test's own
    const { files, process } = testHost();
    const directory = await files.makeTemporaryDirectory("destack-bitwarden-test-");
    onTestFinished(() => files.remove(directory, { recursive: true, force: true }));

    // replace only the vault process's CLI with a fixture that records its public arguments, outside the workspace preload
    const source = fileURLToPath(new URL("./tests/bitwarden.ts", import.meta.url));
    await files.writeTextFile(
        join(directory, "bw"),
        `#!${process.executable} --config=/dev/null\nimport ${JSON.stringify(source)};\n`,
        { mode: 0o700 },
    );
    const simulation = testSimulation(TEST_START, {
        process: {
            arguments: [],
            variables: {
                PATH: `${directory}:${process.variables["PATH"] ?? ""}`,
                HOME: directory,
                BW_SESSION: "fixture-session",
                DESTACK_VAULT_FIXTURE: directory,
                DESTACK_BITWARDEN_RECORDS: join(directory, "records.json"),
            },
            workingDirectory: directory,
            id: 1,
            userId: process.userId,
            executable: process.executable,
            script: process.script,
            runtime: process.runtime,
        },
    });
    const bitwarden = new Bitwarden(simulation.environment("vault"));
    await bitwarden.sync();
    expect(await bitwarden.get("23503038")).toBeUndefined();

    // verify generated credentials survive creation, synchronization and retrieval
    const credential = await bitwarden.create("23503038");
    expect(await bitwarden.get("23503038")).toEqual(credential);
    expect(Number(credential.pin).toString().padStart(8, "0")).toBe(credential.pin);
    expect(credential.puk).toHaveLength(8);
    expect(Buffer.from(credential.management, "hex").toString("hex")).toBe(credential.management);
    expect(Buffer.from(credential.management, "hex")).toHaveLength(32);
    const commands = (await files.readTextFile(join(directory, "commands")))
        .trim()
        .split("\n")
        .map((line): unknown => JSON.parse(line));
    expect(commands).toEqual([
        ["status"],
        ["sync"],
        ["list", "items", "--search", "Destack release root / YubiKey 23503038"],
        ["create", "item"],
        ["sync"],
        ["get", "item", "fixture-item"],
        ["list", "items", "--search", "Destack release root / YubiKey 23503038"],
    ]);
    const stored = StoredItem.parse(JSON.parse(await files.readTextFile(join(directory, "item"))));
    expect(stored.fields).toEqual([
        { name: "pin", value: credential.pin, type: 1 },
        { name: "puk", value: credential.puk, type: 1 },
        { name: "management", value: credential.management, type: 1 },
    ]);

    // persist a separate archive identity without including it in process arguments
    const recoveryName = "Destack release backup / fixture.age";
    const identity = "disposable recovery identity";
    await bitwarden.saveRecovery(recoveryName, identity);
    const recovery = StoredItem.parse(
        JSON.parse(await files.readTextFile(join(directory, "item"))),
    );
    expect(recovery.name).toBe(recoveryName);
    expect(recovery.fields).toEqual([{ name: "identity", value: identity, type: 1 }]);
    expect(await bitwarden.readRecovery(recoveryName)).toBe(identity);
    expect(await bitwarden.readRecovery("absent backup")).toBeUndefined();
    await expect(bitwarden.saveRecovery(recoveryName, identity)).rejects.toThrow(
        "a Bitwarden recovery record with this name already exists",
    );

    // retain credential access after arbitrary display-name changes
    await files.writeTextFile(
        join(directory, "records.json"),
        JSON.stringify({
            [recoveryName]: "fixture-item",
            "Destack release root / YubiKey 23503038": "fixture-item",
        }),
    );
    await files.writeTextFile(
        join(directory, "item"),
        JSON.stringify({
            id: "fixture-item",
            name: "renamed recovery",
            fields: recovery.fields,
        }),
    );
    expect(await bitwarden.readRecovery(recoveryName)).toBe(identity);
    await files.writeTextFile(
        join(directory, "item"),
        JSON.stringify({ id: "fixture-item", name: "renamed hardware", fields: stored.fields }),
    );
    expect(await bitwarden.get("23503038")).toEqual(credential);
}, 5000);
