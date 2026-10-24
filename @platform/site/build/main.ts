import { BunEnvironment } from "@destack/environment/bun";
import { run } from "./site.ts";
import process from "node:process";

await run(process.argv[2], BunEnvironment.system);
