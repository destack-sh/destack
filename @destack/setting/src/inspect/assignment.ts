import { SettingAssignmentDefinition } from "../declare/assignment.ts";
import { SettingPolicyDefinition } from "../declare/policy.ts";

/** Describe a source-declared setting assignment for the package manifest. */
export function describeSettingAssignment(
    assignment: SettingAssignmentDefinition,
): SettingAssignmentDefinition {
    return SettingAssignmentDefinition.parse(assignment);
}

/** Describe a source-declared setting policy for the package manifest. */
export function describeSettingPolicy(policy: SettingPolicyDefinition): SettingPolicyDefinition {
    return SettingPolicyDefinition.parse(policy);
}
