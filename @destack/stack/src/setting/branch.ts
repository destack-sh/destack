import { schema } from "@destack/schema";
import { defineSetting } from "@destack/setting/declare";

/** The most approvals a space may require of a branch: ten, as GitHub's required reviews allow. */
const MAX_REQUIRED_APPROVALS = 10;

/** How many approvals a branch's change request needs before the branch merges into the space. */
export const branchApprovals = defineSetting({
    name: "branch.approvals",
    title: "Branch approvals",
    description:
        "How many approvals a branch's change request needs before the branch merges into the space's main line, none merging branches directly.",
    schema: schema.number().int().min(0).max(MAX_REQUIRED_APPROVALS),
    default: 0,
    scope: "space",
    overrides: [],
    apply: "immediate",
});
