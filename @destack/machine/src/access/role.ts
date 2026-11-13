import { relationship, role } from "@destack/object";

/** Implement the roles a stack declares in its space. */
export function implementRole() {
    return declaredRole;
}

/** Implement the relationships a stack declares in its space. */
export function implementRelationship() {
    return relationship;
}

/** Roles a stack declares in its space, after every object their permissions may name. */
const declaredRole = role.implement({ declaration: { after: "every" } });
