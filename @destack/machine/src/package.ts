import { definePackage } from "@destack/object";
import { computer } from "./computer/device/object/index.ts";
import { checkout, preview } from "./computer/checkout/index.ts";
import { MachineError } from "./error/error.ts";

/** The handle of the machine on a computer: its record, its checkouts and their previews. */
export default definePackage({
    objects: [computer, checkout, preview],
    errors: MachineError,
});
