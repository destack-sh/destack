import { join } from "node:path";
import type { Environment } from "@destack/environment";
import type { Platform } from "../distribution/platform.ts";
import { MacSigning } from "./apple.ts";

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

    /** Sign, verify and notarize a distribution's application. */
    async certify(directory: string, platform: Platform, signer: Signer): Promise<void> {
        // sign and verify the application and the commands inside it
        const application = join(directory, platform.bundle.application);
        await signer.sign(application);
        await signer.verify(application);

        // notarize the exact application the archive and installers ship
        await signer.notarize(application);
    },
};
