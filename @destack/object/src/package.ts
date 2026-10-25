import { definePackage } from "./declare/package.ts";
import { grant, invitation, proof, relationship, role } from "./access/index.ts";
import { call } from "./audit/call.ts";
import { installation } from "./installation/installation.ts";
import { ObjectError } from "./error/error.ts";

/** The handle a space and a machine serve their call log and access objects through. */
export default definePackage({
    objects: { call, role, relationship, invitation, grant, proof, installation },
    errors: ObjectError,
});
