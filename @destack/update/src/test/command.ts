import { BunEnvironment } from "@destack/environment/bun";

// report the fixture's version on the host's standard output, as the compiled command runs
await BunEnvironment.system.process.stdout.write(`${JSON.stringify({ version: "2026.9.1" })}\n`);
