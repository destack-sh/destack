import { setupRun } from "@destack/machine/computer/test/setup";
import { TestComputer } from "@destack/machine/computer/test";
import { TestRun } from "@destack/db/test/run";
import { fileURLToPath } from "node:url";

/** Set the run up with the notebooks and the starter, building the home every terminal forks. */
export default setupRun(
    {
        notebooks: fileURLToPath(new URL("../../tests/fixture/notebooks/", import.meta.url)),
        starter: fileURLToPath(new URL("../../tests/fixture/starter/", import.meta.url)),
    },
    [(run) => TestRun.image(TestComputer.homeTemplate([]), run)],
);
