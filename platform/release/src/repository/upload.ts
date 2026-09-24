import { fileURLToPath } from "node:url";
import { resolve } from "node:path";
import { publish } from "./publish.ts";

await publish(
    resolve(
        fileURLToPath(new URL("../../../../", import.meta.url)),
        process.argv.slice(2)[0] ?? "dist/update",
    ),
);
