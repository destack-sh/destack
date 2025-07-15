#!/usr/bin/env bun
import meow from 'meow';
import { serve } from './serve.js';

const cli = meow(
  `
  Usage
    $ destack-ts-system <command> [options]

  Commands
    serve    Start the gRPC server

  Options
    --port, -p    Port to run the server on (default: 50051)
    --host, -h    Host to bind the server to (default: 0.0.0.0)
    --verbose     Enable verbose logging

  Examples
    $ destack-ts-system serve
    $ destack-ts-system serve --port 3000
`,
  {
    importMeta: import.meta,
    flags: {
      port: {
        type: 'number',
        shortFlag: 'p',
        default: 50051,
      },
      host: {
        type: 'string',
        shortFlag: 'h',
        default: '0.0.0.0',
      },
      verbose: {
        type: 'boolean',
        default: false,
      },
    },
  }
);

async function main() {
  const command = cli.input[0];

  switch (command) {
    case 'serve':
      await serve({
        port: cli.flags.port,
        host: cli.flags.host,
        verbose: cli.flags.verbose,
      });
      break;
    default:
      cli.showHelp();
  }
}

main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
}); 