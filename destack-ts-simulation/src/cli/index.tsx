#!/usr/bin/env bun
import { serve } from "@destack-system/cli/serve";
import { setupLogging } from "@destack-system/utils/log";
import { setupTelemetry } from "@destack-system/utils/telemetry";
import meow from "meow";

const cli = meow(
  `
  Usage
    $ destack-ts <command> [options]

  Commands
    serve    Start the gRPC server

  Options
    --port, -p    Port to run the server on (default: 50051)
    --host, -h    Host to bind the server to (default: 0.0.0.0)
`,
  {
    importMeta: import.meta,
    flags: {
      port: {
        type: "number",
        shortFlag: "p",
        default: 50051,
      },
      host: {
        type: "string",
        shortFlag: "h",
        default: "0.0.0.0",
      },
    },
  },
);

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
