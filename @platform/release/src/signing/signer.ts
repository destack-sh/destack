import { cp, rm } from "node:fs/promises";
import { join } from "node:path";
import { COMMANDS } from "../distribution/distribution.ts";
import type { Platform } from "../distribution/platform.ts";
import { MacSigning } from "../apple/signing.ts";

/** The code signer of one platform's applications and executables. */
export interface Signer {
    /** Sign every executable of an application before the application itself. */
    sign(application: string): Promise<void>;
    /** Notarize or timestamp a signed application. */
    notarize(application: string): Promise<void>;
    /** Verify the signatures of an application. */
    verify(application: string): Promise<void>;
}

/** Create the signer a platform names, or none for a platform without one. */
export function createSigner(platform: Platform): Signer | undefined {
    // select the signer from the platform row
    switch (platform.signer) {
        case "apple":
            return new MacSigning();
        case undefined:
            return undefined;
    }
}

/** Sign a distribution's application, replace its `bin` with the signed commands and notarize it. */
export async function signDistribution(
    directory: string,
    platform: Platform,
    signer: Signer,
): Promise<void> {
    // sign and verify the application and the commands inside it
    const application = join(directory, platform.bundle.application);
    const helpers = join(application, platform.bundle.helpers);
    await signer.sign(application);
    await signer.verify(application);

    // replace the standalone commands and toolchain with their signed copies
    const bin = join(directory, "bin");
    for (const name of COMMANDS) {
        await cp(join(helpers, name), join(bin, name));
    }
    await rm(join(bin, "toolchain"), { recursive: true });
    await cp(join(helpers, "toolchain"), join(bin, "toolchain"), { recursive: true });

    // notarize the exact application the archive and installers ship
    await signer.notarize(application);
}
