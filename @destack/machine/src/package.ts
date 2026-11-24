import { t } from "@destack/locale";
import { definePackage } from "@destack/object";
import { computer } from "./computer/device/object/index.ts";
import { checkout } from "./computer/checkout/index.ts";
import { MachineError } from "./error/error.ts";

/** The handle of the machine on a computer: its record and its checkouts. */
export default definePackage({
    title: t`Machine`,
    description: t`Run a machine: serve the spaces placed on it with their modules (buckets, vaults, connections, messages, observability, audit), route their installations' calls and keep their builds.`,
    icon: "cpu",
    color: "brown",
    objects: [computer, checkout],
    errors: MachineError,
});
