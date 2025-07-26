import { getLogger, getTracer } from "@destack/utils";
import { ServerStatus } from "@destack-system/cli/Status";
import { render } from "ink";

const logger = getLogger("serve");
const tracer = getTracer("serve");

/** Serve the gRPC server */
export async function serve(options: { port: number; host: string }): Promise<void> {
  logger.info("server.starting");

  // render the Ink UI
  const app = render(<ServerStatus host={options.host} port={options.port} status="running" />);
}
