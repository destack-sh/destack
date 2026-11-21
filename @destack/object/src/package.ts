import { t } from "@destack/locale";
import { definePackage } from "./declare/package.ts";
import { grant, invitation, proof, relationship, role } from "./access/index.ts";
import { call } from "./audit/call.ts";
import { branch, branchChange, branchEntry } from "./branch/object.ts";
import { approval, changeRequest } from "./approval/index.ts";
import { ObjectError } from "./error/error.ts";
import { chunk } from "./text/table.ts";
import { counter } from "./trait/numbered.ts";
import { formerKey } from "./object/former.ts";
import { activity } from "./history/activity.ts";

/** The handle a space and a machine serve their call log and access objects through. */
export default definePackage({
    title: t`Objects`,
    description: t`Declare object types with access, methods and sync, and call them from anywhere.`,
    icon: "cube",
    color: "blue",
    objects: [
        call,
        role,
        relationship,
        invitation,
        grant,
        proof,
        branch,
        branchEntry,
        branchChange,
        approval,
        changeRequest,
        activity,
    ],
    // keep the text, number counters and former keys of every package's objects in each space
    tables: [chunk, counter, formerKey],
    errors: ObjectError,
});
