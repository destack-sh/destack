import { initialize } from "./initialize.ts";

/** New private directory selected for independent online keys. */
const [directory, ...extra] = process.argv.slice(2);
if (directory === undefined || extra.length > 0) {
    throw new Error("usage: generate.ts <new-private-directory>");
}
await initialize(directory);
