import { Folder, Thread } from "@destack/language";
import { expect, test } from "vitest";

test("repr query", () => {
  // create query with sort and limit
  const query = Thread.search({
    sort: [Thread.property("createdAt").asc()],
    limit: 25,
  });
  const queryRepr = query.repr();
  expect(queryRepr).toBe(query.repr()); // cached (frozen Struct)
});

test("resolve property", () => {
  // test property name resolution for deleted_at
  expect(Folder.property("deleted_at").name).toBe("deleted_at");
  expect(Folder.property("deletedAt").name).toBe("deleted_at");
  expect(Folder.property("DeletedAt").name).toBe("deleted_at");

  // test property name resolution for parent
  expect(Folder.property("parent_ptr").name).toBe("parent");
  expect(Folder.property("parentPtr").name).toBe("parent");
  expect(Folder.property("parent").name).toBe("parent");
  expect(Folder.property("Parent").name).toBe("parent");
  expect(Folder.property("ParentPtr").name).toBe("parent");
});
