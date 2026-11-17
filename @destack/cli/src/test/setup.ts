import { setupRun } from "@destack/machine/computer/test/setup";
import { fileURLToPath } from "node:url";

/** Set the run up with the notebooks and the starter, which the scenarios read as published. */
export default setupRun({
    notebooks: fileURLToPath(new URL("../../tests/fixture/notebooks/", import.meta.url)),
    starter: fileURLToPath(new URL("../../tests/fixture/starter/", import.meta.url)),
});
