import { BunEnvironment } from "@destack/environment/bun";
import { present } from "@destack/schema";
import { LocalBucket } from "../index.ts";

/** The process this program runs as, beside the parent's. */
const environment = BunEnvironment.system;

/** The bucket directory the parent passes as the first argument. */
const directory = present(environment.process.arguments[0], "a bucket directory argument");

/** The bucket, opened beside the parent's open. */
await using bucket = await LocalBucket.open(directory, undefined, environment);

// write a file, then read the parent's
await bucket.put("child", "from child");

/** The parent's file, as this process reads it. */
const parent = await present(await bucket.get("parent"), "the parent's file").text();
await environment.process.stdout.write(`${JSON.stringify({ parent })}\n`);

// close when the parent closes stdin
for await (const chunk of environment.process.stdin.readable) {
    if (chunk.length !== 0) {
        throw new Error("the bucket opener expects stdin closure");
    }
}
