import { join } from "node:path";
import { FileSystemError } from "@destack/environment";
import { BunEnvironment } from "@destack/environment/bun";
import { schema } from "@destack/schema";

/** The host the fixture CLI runs on. */
const { files, process } = BunEnvironment.system;

/** Private fixture directory supplied by the subprocess integration test. */
const directory = process.variables["DESTACK_VAULT_FIXTURE"];
if (directory === undefined || directory === "") {
    throw new Error("set DESTACK_VAULT_FIXTURE to the fixture directory");
}
/** CLI arguments recorded to verify that credentials never appear in them. */
const arguments_ = process.arguments;
await files.writeTextFile(join(directory, "commands"), JSON.stringify(arguments_) + "\n", {
    append: true,
});

// emulate the CLI protocol using a disposable local record
if (arguments_[0] === "status") {
    process.stdout.writeSync(JSON.stringify({ status: "unlocked" }) + "\n");
} else if (arguments_[0] === "sync") {
    process.stdout.writeSync("sync complete\n");
} else if (arguments_[0] === "create") {
    const encoded = await new Response(process.stdin.readable).text();
    const item = {
        ...schema
            .record(schema.string(), schema.unknown())
            .parse(JSON.parse(Buffer.from(encoded, "base64").toString())),
        id: "fixture-item",
    };
    await files.writeTextFile(join(directory, "item"), JSON.stringify(item));
    process.stdout.writeSync(JSON.stringify(item) + "\n");
} else if (arguments_[0] === "get") {
    process.stdout.writeSync((await files.readTextFile(join(directory, "item"))) + "\n");
} else if (arguments_[0] === "list") {
    const text = await files
        .readTextFile(join(directory, "item"))
        .catch((error: unknown) => FileSystemError.absent(error));
    const items: unknown[] = text === undefined ? [] : [JSON.parse(text)];
    process.stdout.writeSync(JSON.stringify(items) + "\n");
} else {
    throw new Error("unexpected fixture operation");
}
