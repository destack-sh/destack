import { definePackage } from "@destack/runtime";
import { approval, change, changeRequest } from "./approval/index.ts";

/** The workflow package: approvals, change requests and the runs applying them. */
export default definePackage({ objects: { approval, changeRequest, ...change.objects } });
