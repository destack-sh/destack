import { listInstallers } from "../distribution/index.ts";
import { buildMacInstaller } from "./macos.ts";
import { print } from "../output/index.ts";

/** The installers of the released platforms. */
const installers = listInstallers().map(({ installer }) => installer);
/** The installer selected by its download name. */
const installer = installers.find(({ name }) => name === process.argv[2]);

// refuse an unknown installer name
if (installer === undefined) {
    throw new Error(`select an installer: ${installers.map(({ name }) => name).join(", ")}`);
}

// build the installer with its format's builder
switch (installer.format) {
    case "dmg":
        print(await buildMacInstaller(installer));
        break;
}
