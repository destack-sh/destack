import { definePackage } from "@destack/object";
import { apply } from "./control/apply.ts";
import { ControlError } from "./error/error.ts";

/** The control package: the run applying the universe's declaration. */
export default definePackage({
    objects: apply.objects,
    errors: ControlError,
});
