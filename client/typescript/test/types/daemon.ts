import type { Connection } from "../../src/index.js";
import { Daemon, DaemonClient, daemonService } from "../../src/index.js";

declare const connection: Connection;

const daemon = new Daemon(connection);
const client = new DaemonClient(connection);

daemon.shutdown();
client.shutdown(null);

const service: bigint = daemonService;
void service;
