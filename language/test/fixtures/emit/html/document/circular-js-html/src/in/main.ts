import page from "./page.html";
import { buildPageMessage } from "./shared/message.ts";
import { formatBootstrapMode } from "./shared/mode.ts";

const pageMessage = buildPageMessage("circular-import");

console.log("Main JS loaded page:", page, pageMessage, formatBootstrapMode("module"));
