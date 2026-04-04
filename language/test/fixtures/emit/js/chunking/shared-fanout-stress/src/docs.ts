import { getDocsPage } from "./pages/docs.ts";
import { renderShell } from "./shared/render/shell.ts";

console.log("docs", renderShell(getDocsPage()));
