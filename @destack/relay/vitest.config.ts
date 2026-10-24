import { BunEnvironment } from "@destack/environment/bun";
import { defineConfiguration } from "@destack/test/config";
import { fileURLToPath } from "node:url";

/** The tests playing a simulation, whose time and network use no database outside it. */
const SIMULATED = ["tests/relay.test.ts", "src/tunnel/client.test.ts"];

export default defineConfiguration(
    {
        root: fileURLToPath(new URL(".", import.meta.url)),
        test: {
            testTimeout: 5000,
            hookTimeout: 5000,
            projects: [
                // run the units, which use no database, once
                {
                    extends: true,
                    test: {
                        name: "unit",
                        include: ["src/**/*.test.ts", "tests/*.test.ts"],
                        exclude: SIMULATED,
                    },
                },
                // play the simulations over a database inside the process
                {
                    extends: true,
                    targets: { database: ["sqlite"] },
                    test: { name: "simulation", include: SIMULATED },
                },
            ],
        },
    },
    BunEnvironment.system,
);
