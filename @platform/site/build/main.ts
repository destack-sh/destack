import { BunEnvironment } from "@destack/environment/bun";
import { run } from "./site.ts";

await run(BunEnvironment.system.process.arguments[0], BunEnvironment.system);
