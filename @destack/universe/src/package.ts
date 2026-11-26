import { t } from "@destack/locale";
import { definePackage } from "@destack/object";
import { apply } from "./universe/apply.ts";
import { UniverseError } from "./error/error.ts";

/** The universe package: the run applying the universe's declaration. */
export default definePackage({
    title: t.context("title")`Universe`,
    description: t`Run a universe: the account, object, setting and universe packages over one store at one origin.`,
    icon: "sliders-horizontal",
    color: "brown",
    objects: apply.objects,
    errors: UniverseError,
});
