import {
  ACTIVE_BRANCH,
  ACTIVE_SNAPSHOT,
  ACTIVE_SPACE,
  type Entity,
  type NodeReference,
  type Region,
  type Session,
  Space,
} from "@destack/language";

export function createAndActivateSpace(options: {
  session: Session;
  region: Region;
  ownedBy: Entity | NodeReference;
  name: string;
  slug: string;
  id?: string;
}) {
  const result = Space.createSpace(options);
  ACTIVE_SPACE.set(result.space);
  ACTIVE_BRANCH.set(result.rootBranch);
  ACTIVE_SNAPSHOT.set(result.headSnapshot);
  return result;
}
