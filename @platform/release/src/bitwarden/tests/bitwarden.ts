import { appendFile, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { schema } from "@destack/schema";

/** Private fixture directory supplied by the subprocess integration test. */
const directory = process.env["DESTACK_VAULT_FIXTURE"];
if (directory === undefined || directory === "") {
    throw new Error("set DESTACK_VAULT_FIXTURE to the fixture directory");
}
/** CLI arguments recorded to verify that credentials never appear in them. */
const arguments_ = process.argv.slice(2);
await appendFile(join(directory, "commands"), JSON.stringify(arguments_) + "\n");

// emulate the CLI protocol using a disposable local record
if (arguments_[0] === "status") {
    process.stdout.write(JSON.stringify({ status: "unlocked" }) + "\n");
} else if (arguments_[0] === "sync") {
    process.stdout.write("sync complete\n");
} else if (arguments_[0] === "create") {
    const encoded = await new Response(Bun.stdin.stream()).text();
    const item = {
        ...schema
            .record(schema.string(), schema.unknown())
            .parse(JSON.parse(Buffer.from(encoded, "base64").toString())),
        id: "fixture-item",
    };
    await writeFile(join(directory, "item"), JSON.stringify(item));
    process.stdout.write(JSON.stringify(item) + "\n");
} else if (arguments_[0] === "get") {
    process.stdout.write((await readFile(join(directory, "item"), "utf8")) + "\n");
} else if (arguments_[0] === "list") {
    const file = Bun.file(join(directory, "item"));
    process.stdout.write(JSON.stringify((await file.exists()) ? [await file.json()] : []) + "\n");
} else {
    throw new Error("unexpected fixture operation");
}
