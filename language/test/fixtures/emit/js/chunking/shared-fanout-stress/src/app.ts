import { getOverviewPage } from "./pages/overview.ts";
import { renderShell } from "./shared/render/shell.ts";

console.log("app", renderShell(getOverviewPage()));
