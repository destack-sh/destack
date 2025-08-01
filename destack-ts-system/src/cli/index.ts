#!/usr/bin/env bun
import { serve } from "@destack-system/cli/serve";
import { parseArgs } from "destack";

const cli = parseArgs({
  description: "destack-ts-system CLI",
  usage: "$ destack-ts <command> [options]",
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
