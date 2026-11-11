import type { Environment, HostCapability } from "@destack/environment";
import { Installer } from "../distribution/index.ts";
import { buildMacInstaller } from "./apple.ts";
import { print } from "../distribution/output.ts";

/** Build the installer the command line selects by its download name. */
export async function buildInstaller(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // select the installer of a released platform by its download name
    const installers = Installer.list().map(({ installer }) => installer);
    const installer = installers.find(({ name }) => name === arguments_[0]);
    if (installer === undefined) {
        throw new Error(`select an installer: '${installers.map(({ name }) => name).join(", ")}'`);
    }

    // build the installer with its format's builder
    switch (installer.format) {
        case "dmg":
            print(await buildMacInstaller(installer, environment), environment);
            break;
    }
}
