import {
  ACTIVE_BRANCH,
  ACTIVE_SNAPSHOT,
  ACTIVE_SPACE,
  Branch,
  BranchType,
  NodeReference,
  NodeType,
  Region,
  Session,
  Snapshot,
  SnapshotType,
  Space,
  SpaceStatus,
} from "@destack/language";
import { uuid4 } from "@destack/utils";
import { Temporal } from "temporal-polyfill";

export function createSpace(options: {
  session: Session;
  id?: string;
  name?: string;
  slug?: string;
}): {
  space: Space;
  branch: Branch;
  snapshot: Snapshot;
} {
  const { session, id, name, slug } = options;
  const epoch = session.epoch;
  const now = Temporal.Now.zonedDateTimeISO("UTC");

  const spaceId = id ?? uuid4();
  const spacePtr = new NodeReference({
    type: NodeType.SPACE,
    id: spaceId,
    spaceId,
  });

  const branchId = uuid4();
  const branchPtr = new NodeReference({
    type: NodeType.BRANCH,
    id: branchId,
    spaceId,
  });

  const snapshotId = uuid4();
  const snapshotPtr = new NodeReference({
    type: NodeType.SNAPSHOT,
    id: snapshotId,
    spaceId,
    branchId,
  });

  const branch = new Branch({
    id: branchId,
    name: "Main",
    space: spacePtr,
    createdEpoch: epoch,
    createdAt: now,
    updatedEpoch: epoch,
    updatedAt: now,
    type: BranchType.ROOT,
    branch: branchPtr,
    snapshot: snapshotPtr,
  });
  session.create(branch);
  const snapshot = new Snapshot({
    id: snapshotId,
    name: "Root",
    space: spacePtr,
    createdEpoch: epoch,
    createdAt: now,
    updatedEpoch: epoch,
    updatedAt: now,
    type: SnapshotType.FULL,
    branch: branchPtr,
  });
  session.create(snapshot);
  const space = new Space({
    id: spaceId,
    name: name ?? "Space",
    slug: slug ?? "space",
    status: SpaceStatus.ACTIVE,
    region: Region.ZURICH,
    branch: branchPtr,
    snapshot: snapshotPtr,
    createdEpoch: epoch,
    createdAt: now,
    updatedEpoch: epoch,
    updatedAt: now,
  });
  session.create(space);

  return { space, branch, snapshot };
}

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
