import {
  Client,
  ClientType,
  Folder,
  Join,
  JoinType,
  NodeReference,
  NodeType,
  Session,
  Star,
  User,
  UserStatus,
} from "@destack/language";
import { v4 as uuid4 } from "uuid";
import { expect, test } from "vitest";

const sessionTest = test.extend<{ session: Session }>({
  session: async ({ task }, use) => {
    const session = new Session();
    await session.open();
    await use(session);
    await session.close();
  },
});

sessionTest("entity crud operations", async ({ session }) => {
  // create user
  const user = new User({
    status: UserStatus.ACTIVE,
    name: "Floof",
    slug: "floof",
    space: new NodeReference({ nodeType: NodeType.SPACE, id: uuid4() }),
  });
  session.create(user);
  await session.commit();

  // update user
  user.name = "Fluff";
  user.slug = "flotothemoon";
  await session.commit();

  // query user by id
  const userUnpacked = await User.get({ where: User.property("id").eq(user.id) }).executeOne();
  expect(userUnpacked.createdAt).toEqual(user.createdAt);
  expect(user.equals(userUnpacked)).toBe(true);

  // query user by slug
  const userUnpackedBySlug = await User.search({
    where: User.property("slug").eq("flotothemoon"),
  }).executeOne();
  expect(userUnpackedBySlug.equals(user)).toBe(true);
  expect(userUnpackedBySlug.name).toBe("Fluff");
  expect(userUnpackedBySlug.slug).toBe("flotothemoon");
  expect(userUnpackedBySlug.status).toBe(UserStatus.ACTIVE);

  // create clients
  const clientA = new Client({ type: ClientType.WEB, name: "Client A" });
  const clientB = new Client({ type: ClientType.WEB, name: "Client B" });
  user.addChildren([clientA, clientB]);
  await session.commit();

  // query clients
  const clients = await Client.search({ sort: [Client.property("name").desc()] }).executeList();
  expect(clients).toEqual([clientB, clientA]);

  // query user with clients as children
  const connection = await User.get({
    where: User.property("id").eq(user.id),
    Clients: Client.search(),
  }).execute();
  const userUnpackedWithClients = connection.toOne();
  expect(userUnpackedWithClients.equals(user)).toBe(true);
  const clientsUnpacked = userUnpackedWithClients.getChildren(Client);
  expect(clientsUnpacked).toEqual([clientA, clientB]);

  // query clients with user as parent
  const clientConnection = await Client.search({
    where: Client.property("parent").eq(user),
    Parent: User.get({ join: Join.of(JoinType.PARENT) }),
  }).execute();
  const clientsUnpackedWithParent = clientConnection.toList();
  expect(clientsUnpackedWithParent).toEqual([clientA, clientB]);
});

sessionTest("create star", async ({ session }) => {
  const users = Array.from(
    { length: 20 },
    (_, i) =>
      new User({
        name: `User${i}`,
        slug: `user${i}`,
        space: new NodeReference({ nodeType: NodeType.SPACE, id: uuid4() }),
      }),
  );
  for (const user of users) {
    session.create(user);
  }
  await session.commit();

  const folder = new Folder({ name: "Folder" });
  session.create(folder);
  await session.commit();

  for (const user of users) {
    const star = new Star({ parent: folder, ownedBy: user });
    session.create(star);
  }
  await session.commit();

  expect(await Star.count({ where: Star.property("parent").eq(folder) }).executeCount()).toBe(20);
});
