import { definePackage } from "./declare/package.ts";
import { grant, invitation, proof, relationship, role } from "./access/index.ts";
import { call } from "./audit/call.ts";
import { installation } from "./installation/installation.ts";
import { branch, branchChange, branchEntry } from "./branch/object.ts";
import { approval, changeRequest } from "./approval/index.ts";
import { ObjectError } from "./error/error.ts";
import { chunk, chunkRun } from "./text/table.ts";

/** The handle a space and a machine serve their call log and access objects through. */
export default definePackage({
    objects: {
        call,
        role,
        relationship,
        invitation,
        grant,
        proof,
        installation,
        branch,
        branchEntry,
        branchChange,
        approval,
        changeRequest,
    },
    // keep the text of every package's objects in each space, whose chunk tables object owns
    tables: [chunk, chunkRun],
    errors: ObjectError,
});
