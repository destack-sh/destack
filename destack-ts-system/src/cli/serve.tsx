import { ServerStatus } from "@desys/cli/Status";
import * as grpc from "@grpc/grpc-js";
import { render } from "ink";

import { getLogger, getTracer } from "@destack/utils";

const logger = getLogger("serve");
const tracer = getTracer("serve");

/** Serve the gRPC server */
export async function serve(options: { port: number; host: string }): Promise<void> {
  logger.info("server.starting");

  const server = new grpc.Server();

  // add services
  server.addService(
    {
      check: {
        path: "/grpc.health.v1.Health/Check",
        requestStream: false,
        responseStream: false,
        requestSerialize: (value: any) => Buffer.from(JSON.stringify(value)),
        requestDeserialize: (value: Buffer) => JSON.parse(value.toString()),
        responseSerialize: (value: any) => Buffer.from(JSON.stringify(value)),
        responseDeserialize: (value: Buffer) => JSON.parse(value.toString()),
      },
    } as any,
    {
      check: (call: any, callback: any) => {
        callback(null, { status: "SERVING" });
      },
    },
  );

  // start server
  const address = `${options.host}:${options.port}`;
  server.bindAsync(address, grpc.ServerCredentials.createInsecure(), (error, port) => {
    if (error) {
      logger.error({ error }, "server.start.error");
      process.exit(1);
    }

    logger.info({ address: `${options.host}:${port}` }, "server.start");

    // render the Ink UI
    const app = render(<ServerStatus host={options.host} port={port} status="running" />);

    // handle graceful shutdown
    const stop = () => {
      logger.info("server.stopping");
      app.unmount();
      server.tryShutdown(() => {
        logger.info("server.stop");
        process.exit(0);
      });
    };

    process.on("SIGINT", stop);
    process.on("SIGTERM", stop);
  });
}
