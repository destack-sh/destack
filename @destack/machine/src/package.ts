import { t } from "@destack/locale";
import { definePackage } from "@destack/object";
import { computer } from "./computer/device/object/index.ts";
import { checkout, preview } from "./computer/checkout/index.ts";
import { MachineError } from "./error/error.ts";

/** The handle of the machine on a computer: its record, its checkouts and their previews. */
export default definePackage({
    title: t`Machine`,
    description: t`Run a machine: serve the spaces placed on it with their modules (buckets, vaults, connections, messages, observability, audit), route their installations' calls and keep their builds.`,
    icon: "cpu",
    color: "bronze",
    objects: [computer, checkout, preview],
    errors: MachineError,
});
