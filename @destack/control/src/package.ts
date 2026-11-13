import { t } from "@destack/locale";
import { definePackage } from "@destack/object";
import { apply } from "./control/apply.ts";
import { ControlError } from "./error/error.ts";

/** The control package: the run applying the universe's declaration. */
export default definePackage({
    title: t`Control`,
    description: t`Run a universe: the account, object, setting and control packages over one store at one origin.`,
    icon: "sliders-horizontal",
    color: "bronze",
    objects: apply.objects,
    errors: ControlError,
});
