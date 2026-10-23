import { definePackage } from "@destack/object";
import { apply } from "./universe/apply.ts";
import { UniverseError } from "./error/error.ts";

/** The universe package: the run applying the universe's declaration. */
export default definePackage({
    objects: { ...apply.objects },
    errors: UniverseError,
});
