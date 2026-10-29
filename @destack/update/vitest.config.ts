import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";
import { fileURLToPath } from "node:url";

/** The tests storing distributions, the only ones reading a database. */
const STORED = ["src/server/distribution.test.ts"];

/** The tests replacing real signed application bundles on a Mac. */
const INTEGRATION = ["src/apple/install.test.ts"];

export default defineConfiguration(
    {
        root: fileURLToPath(new URL(".", import.meta.url)),
        test: {
            testTimeout: 5000,
            hookTimeout: 5000,
            projects: [
                // run the tests without a database once
                {
                    extends: true,
                    test: {
                        name: "unit",
                        include: ["src/**/*.test.ts"],
                        exclude: [...STORED, ...INTEGRATION],
                    },
                },
                // run the stored tests against each database the package stores in
                {
                    extends: true,
                    targets: { database: ["sqlite"] },
                    test: { include: STORED },
                },
                // replace real application bundles once on a Mac, whose ditto, codesign and plutil they need
                ...(BunEnvironment.system.host.os === "darwin"
                    ? [{ extends: true, test: { name: "integration", include: INTEGRATION } }]
                    : []),
            ],
        },
    },
    BunEnvironment.system,
);
