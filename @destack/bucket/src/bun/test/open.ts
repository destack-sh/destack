import { Clock } from "@destack/environment";
import { present } from "@destack/schema";
import { LocalBucket } from "../index.ts";

/** The bucket directory the parent passes as the first argument. */
const directory = present(process.argv[2], "a bucket directory argument");

/** The bucket, opened beside the parent's open. */
await using bucket = await LocalBucket.open(directory, undefined, Clock.system);

// write a file, then read the parent's
await bucket.put("child", "from child");

/** The parent's file, as this process reads it. */
const parent = await present(await bucket.get("parent"), "the parent's file").text();
process.stdout.write(`${JSON.stringify({ parent })}\n`);

// close when the parent closes stdin
for await (const chunk of Bun.stdin.stream()) {
    if (chunk.length !== 0) {
        throw new Error("the bucket opener expects stdin closure");
    }
}
