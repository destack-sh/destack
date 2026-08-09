import type { Connection, Workspace } from "../../src/index.js";
import { Destack } from "../../src/index.js";

declare const connection: Connection;

const destack = new Destack(connection);
const remote: Promise<Destack> = Destack.connect("ws://localhost/rpc");
const workspace: Promise<Workspace> = destack.openWorkspace("/project");

destack.daemon.shutdown();
destack.close();

void remote;
void workspace;
