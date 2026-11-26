import { BunEnvironment } from "@destack/environment/bun";

// exit at once, as a machine failing before it serves does
BunEnvironment.system.process.exitCode = 1;
