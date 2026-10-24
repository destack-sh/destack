import { BunEnvironment } from "@destack/environment/bun";
import { runLauncher } from "./launcher/index.ts";

await runLauncher(BunEnvironment.system);
