import { getSharedSectionList } from "./sections.ts";

export function getDocsFeatureSet() {
    return [...getSharedSectionList(), "api-reference", "migration-guide"];
}
