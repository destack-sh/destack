import { ACTIVE_SPACE, Folder, Region, Session, Space, SpaceStatus, Tag } from "@destack/language";
import { expect, test } from "vitest";

const sessionTest = test.extend<{ session: Session }>({
  session: async ({ task }, use) => {
    const session = new Session({});
    await session.open();
    await use(session);
    await session.close();
  },
});

sessionTest("node space ptr", async ({ session }) => {
  // add nodes that are spatial and check that they have the same space_ptr
  const space = new Space({
    name: "MySpace",
    slug: "my-space",
    status: SpaceStatus.ACTIVE,
    region: Region.ZURICH,
  });
  ACTIVE_SPACE.set(space);
  session.create(space);

  const folder = new Folder({
    name: "MyFolder",
  });
  space.addChild(folder);
  expect(folder.spacePtr).toBeTruthy();
  expect(folder.spacePtr!.id).toBe(space.id);

  const tags = [new Tag({ name: "A" }), new Tag({ name: "B" }), new Tag({ name: "C" })];
  folder.addChildren(tags);

  for (const tag of tags) {
    expect(tag.spacePtr).toBeTruthy();
    expect(tag.spacePtr!.id).toBe(space.id);
  }
});

sessionTest("node ordering", async ({ session }) => {
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
