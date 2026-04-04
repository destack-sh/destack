import './index.css'
import { renderProtocolStatus } from "./status.ts";

console.log("protocol-dependent-import", renderProtocolStatus("ready"));
