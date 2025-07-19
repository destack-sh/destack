import {
  ACTIVE_BRANCH,
  ACTIVE_SNAPSHOT,
  ACTIVE_SPACE,
  Branch,
  createSpace,
  Session,
  Snapshot,
  Space,
} from "@destack/language";

export function createAndActivateSpace(options: {
  session: Session;
  id?: string;
  name?: string;
  slug?: string;
}): {
  space: Space;
  branch: Branch;
  snapshot: Snapshot;
} {
  const { space, branch, snapshot } = createSpace(options);
  ACTIVE_SPACE.set(space);
  ACTIVE_BRANCH.set(branch);
  ACTIVE_SNAPSHOT.set(snapshot);
  return { space, branch, snapshot };
}
