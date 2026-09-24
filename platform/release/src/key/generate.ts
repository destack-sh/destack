import { initialize } from "./initialize.ts";

/** New private directory selected for independent online keys. */
const [directory, ...extra] = process.argv.slice(2);
if (!directory || extra.length) {
    throw new Error("usage: key.ts <new-private-directory>");
}
await initialize(directory);
