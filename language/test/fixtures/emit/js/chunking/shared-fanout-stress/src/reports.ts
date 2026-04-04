import { getReportsPage } from "./pages/reports.ts";
import { renderShell } from "./shared/render/shell.ts";

console.log("reports", renderShell(getReportsPage()));
