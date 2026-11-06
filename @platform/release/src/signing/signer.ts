import { join } from "node:path";
import type { Environment } from "@destack/environment";
import { COMMANDS } from "../distribution/distribution.ts";
import type { Platform } from "../distribution/platform.ts";
import { MacSigning } from "./apple.ts";
import { Toolchain } from "@destack/check/toolchain";

/** The code signer of one platform's applications and executables. */
export interface Signer {
    /** Sign every executable of an application before the application itself. */
    readonly sign: (application: string) => Promise<void>;
    /** Notarize or timestamp a signed application. */
    readonly notarize: (application: string) => Promise<void>;
    /** Verify the signatures of an application. */
    readonly verify: (application: string) => Promise<void>;
}

/** The code signer of one platform's applications and executables. */
export const Signer = {
    /** Create a platform's signer, or none for a platform without one. */
    of(platform: Platform, environment: Environment<"files" | "process">): Signer | undefined {
        // select the signer from the platform row
        switch (platform.signer) {
            case "apple":
                return new MacSigning(environment);
            case undefined:
                return undefined;
        }
    },

    /** Sign a distribution's application, put the signed commands in `bin` and notarize it. */
    async certify(
        directory: string,
        platform: Platform,
        signer: Signer,
        environment: Environment<"files">,
    ): Promise<void> {
        // sign and verify the application and the commands inside it
        const application = join(directory, platform.bundle.application);
        const helpers = join(application, platform.bundle.helpers);
        await signer.sign(application);
        await signer.verify(application);

        // replace the standalone commands and toolchain with their signed copies
        const { files } = environment;
        const bin = join(directory, "bin");
        for (const name of COMMANDS) {
            await files.copyFile(join(helpers, name), join(bin, name));
        }
        await files.remove(Toolchain.of(join(bin, "destack")), { recursive: true });
        await files.copy(
            Toolchain.of(join(helpers, "destack")),
            Toolchain.of(join(bin, "destack")),
        );

        // notarize the exact application the archive and installers ship
        await signer.notarize(application);
    },
};
