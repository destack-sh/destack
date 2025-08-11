import { afterEach, beforeEach, expect, test } from "bun:test";
import { MemoryGraph } from "@destack/graph/memory";
import { ACTIVE_SPACE, Folder, Region, Session, Space, Tag, Universe } from "@destack/language";
import { uuid4 } from "@destack/utils/uuid";

let session: Session;

beforeEach(async () => {
  session = new Session({
    epoch: 1,
    graph: new MemoryGraph(),
    actor: Universe.ACTOR,
    client: Universe.CLIENT,
    clientNonce: uuid4(),
  });
  await session.open();
});

afterEach(async () => {
  await session.close();
});

test("node space ptr", async () => {
  // add nodes that are spatial and check that they have the same space_ref
  const { space } = Space.createSpace({
    session,
    name: "MySpace",
    slug: "my-space",
    region: Region.ZURICH,
    ownedBy: session.actorRef,
  });
  ACTIVE_SPACE.set(space);

  const folder = new Folder({
    name: "MyFolder",
  });
  space.addChild(folder);
  expect(folder.parentRef).toBeTruthy();
  expect(folder.parentRef!.id).toBe(space.id);
  expect(folder.spaceRef).toBeTruthy();
  expect(folder.spaceRef!.id).toBe(space.id);

  const tags = [new Tag({ name: "A" }), new Tag({ name: "B" }), new Tag({ name: "C" })];
  folder.addChildren(tags);

  for (const tag of tags) {
    expect(tag.spaceRef).toBeTruthy();
    expect(tag.spaceRef!.id).toBe(space.id);
  }
});

test("node ordering", async () => {
  // add nodes that are IsOrdered and check that they are ordered
  const folder = new Folder({
    name: "MyFolder",
  });
  session.create(folder);

  const tags = [new Tag({ name: "A" }), new Tag({ name: "B" }), new Tag({ name: "C" })];
  folder.addChildren(tags);

  expect(folder.getChildren(Tag)).toEqual(tags);
  expect(tags.map((t) => t.orderKey)).toEqual(["a0", "a1", "a2"]);
});
