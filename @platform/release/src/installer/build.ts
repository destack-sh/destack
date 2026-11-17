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

    // build it from the archives and print where it is
    print(await buildFormat(installer, environment), environment);
}

/** Build an installer with its format's builder, from signed applications or the archives. */
export async function buildFormat(
    installer: Installer,
    environment: Environment<HostCapability>,
    signed: Readonly<Record<string, string>> = {},
): Promise<string> {
    // build a disk image on macOS
    switch (installer.format) {
        case "dmg":
            return await buildMacInstaller(installer, environment, signed);
    }
}
