#!/usr/bin/env bun
import { parseArgs } from "@destack/cli/lib";
import { serve } from "@destack-test/cli/serve";
import { setupLogging } from "@destack-test/utils/log";
import { setupTelemetry } from "@destack-test/utils/telemetry";

const cli = parseArgs({
  description: "destack-ts-test CLI",
  usage: "$ destack-ts-test <command> [options]",
  flags: {
    port: {
      type: "number",
      shortFlag: "p",
      default: 50051,
      description: "Port to run the server on",
    },
    host: {
      type: "string",
      shortFlag: "h",
      default: "0.0.0.0",
      description: "Host to bind the server to",
    },
  },
});

async function main() {
  const command = cli.input[0];

  setupTelemetry();
  setupLogging();

  switch (command) {
    case "serve":
      await serve({ port: cli.flags.port, host: cli.flags.host });
      break;
    default:
      cli.showHelp();
  }
}

main().catch((error) => {
  console.error("Fatal error:", error);
  process.exit(1);
});
