import { getLogger, getTracer } from "@destack/utils";
import { box, clear, color, print, warn } from "@destack-system/cli/lib";

const logger = getLogger("serve");
const tracer = getTracer("serve");

/** Serve the gRPC server */
export async function serve(options: { port: number; host: string }): Promise<void> {
  logger.info("server.starting");

  // clear the console and show status
  clear();

  const statusInfo = [
    color("destack-ts-system", "bold"),
    "",
    `Status: ${color("RUNNING", "green", "bold")}`,
    `Address: ${color(`${options.host}:${options.port}`, "cyan")}`,
    "",
    color("Press Ctrl+C to stop the server", "dim"),
  ].join("\n");

  print(box(statusInfo));

  // handle shutdown gracefully
  process.on("SIGINT", () => {
    warn("\nShutting down server...");
    process.exit(0);
  });
}
