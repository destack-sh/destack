import { appendFile, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

/** Private fixture directory supplied by the subprocess integration test. */
const directory = process.env.DESTACK_VAULT_FIXTURE!;
/** CLI arguments recorded to verify that credentials never appear in them. */
const arguments_ = process.argv.slice(2);
await appendFile(join(directory, "commands"), JSON.stringify(arguments_) + "\n");

// emulate the CLI protocol using a disposable local record
if (arguments_[0] === "status") {
    console.log(JSON.stringify({ status: "unlocked" }));
} else if (arguments_[0] === "sync") {
    console.log("sync complete");
} else if (arguments_[0] === "create") {
    const encoded = await new Response(Bun.stdin.stream()).text();
    const item = { ...JSON.parse(Buffer.from(encoded, "base64").toString()), id: "fixture-item" };
    await writeFile(join(directory, "item"), JSON.stringify(item));
    console.log(JSON.stringify(item));
} else if (arguments_[0] === "get") {
    console.log(await readFile(join(directory, "item"), "utf8"));
} else if (arguments_[0] === "list") {
    const file = Bun.file(join(directory, "item"));
    console.log(JSON.stringify((await file.exists()) ? [await file.json()] : []));
} else {
    throw new Error("unexpected fixture operation");
}
