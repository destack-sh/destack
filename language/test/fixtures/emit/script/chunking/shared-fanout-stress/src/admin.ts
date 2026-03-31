import { getAdminPage } from "./pages/admin.ts";
import { renderShell } from "./shared/render/shell.ts";

console.log("admin", renderShell(getAdminPage()));
