import {
  Client,
  ClientType,
  Folder,
  FolderType,
  FrameView,
  Join,
  JoinType,
  LabelView,
  Message,
  NodeReference,
  NodeType,
  Reaction,
  Scene,
  Session,
  Star,
  StoreType,
  TextView,
  User,
  UserStatus,
  View,
} from "@destack/language";
import { MemoryEntityStore } from "@destack/store/memory";
import { v4 as uuid4 } from "uuid";
import { expect, test } from "vitest";

const sessionTest = test.extend<{ session: Session }>({
  session: async ({ task }, use) => {
    const store = new MemoryEntityStore({
      types: [StoreType.GLOBAL_ENTITY_PRIMARY, StoreType.SPATIAL_ENTITY_PRIMARY],
    });
    const session = new Session({ store });
    await session.open();
    await use(session);
    await session.close();
  },
});

sessionTest("create user with clients", async ({ session }) => {
  // create user
  const user = new User({
    status: UserStatus.ACTIVE,
    name: "Floof",
    slug: "floof",
    space: new NodeReference({ type: NodeType.SPACE, id: uuid4() }),
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
  expect(user.equals(userUnpacked));

  // query user by slug
  const userUnpackedBySlug = await User.search({
    where: User.property("slug").eq("flotothemoon"),
  }).executeOne();
  expect(userUnpackedBySlug.equals(user));
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
  expect(clients).toHaveLength(2);
  expect(clients[0].equals(clientB));
  expect(clients[1].equals(clientA));

  // query user with clients as children
  const connection = await User.get({
    where: User.property("id").eq(user.id),
    Clients: Client.search(),
  }).execute();
  const userUnpackedWithClients = connection.toOne();
  expect(userUnpackedWithClients.equals(user));
  const clientsUnpacked = userUnpackedWithClients.getChildren(Client);
  expect(clientsUnpacked).toHaveLength(2);
  expect(clientsUnpacked[0].equals(clientA));
  expect(clientsUnpacked[1].equals(clientB));

  // query clients with user as parent
  const clientConnection = await Client.search({
    where: Client.property("parent").eq(user),
    Parent: User.get({ join: Join.of(JoinType.PARENT) }),
  }).execute();
  const clientsUnpackedWithParent = clientConnection.toList();
  expect(clientsUnpackedWithParent).toHaveLength(2);
  expect(clientsUnpackedWithParent[0].equals(clientA));
  expect(clientsUnpackedWithParent[1].equals(clientB));
});

sessionTest("create star", async ({ session }) => {
  const users = Array.from(
    { length: 20 },
    (_, i) =>
      new User({
        name: `User${i}`,
        slug: `user${i}`,
        space: new NodeReference({ type: NodeType.SPACE, id: uuid4() }),
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

sessionTest("create folders recursive", async ({ session }) => {
  // create
  const rootFolder = new Folder({ name: "Folder", type: FolderType.HOME });
  session.create(rootFolder);
  const subtreeFolderCount = 4 * (1 + 4 * (1 + 4));
  for (const a of ["a", "b", "c", "d"]) {
    // create folder
    const folder = new Folder({ name: `Folder ${a}` });
    rootFolder.addChild(folder);
    // create folder tree
    for (let i = 0; i < 4; i++) {
      const subFolder = new Folder({ name: `Folder ${a}/${i}` });
      folder.addChild(subFolder);
      for (let j = 0; j < 4; j++) {
        const innerFolder = new Folder({ name: `Folder ${a}/${i}/${j}` });
        subFolder.addChild(innerFolder);
        for (let k = 0; k < 4; k++) {
          const innerInnerFolder = new Folder({ name: `Folder ${a}/${i}/${j}/${k}` });
          innerFolder.addChild(innerInnerFolder);
        }
      }
    }
    await session.commit();
  }
  expect(
    await Folder.count({ where: Folder.property("parent").eq(rootFolder) }).executeCount(),
  ).toBe(4);

  // query
  for (const folder of rootFolder.getChildren(Folder)) {
    // query folder root count
    const rootFolderCount = await Folder.count({
      where: Folder.property("parent").eq(folder),
    }).executeCount();
    expect(rootFolderCount).toBe(4);

    // query folder down (parent, recursive)
    const connection = await Folder.get({
      where: Folder.property("id").eq(folder.id),
      Folders: Folder.search({ join: Join.of(JoinType.CHILD, { recursive: true }) }),
    }).execute();
    const folderUnpacked = connection.toOne();
    const folderTreeUnpacked = folderUnpacked.getDescendants(Folder);
    expect(folderTreeUnpacked.length).toBe(subtreeFolderCount);

    // query folder up (parent, recursive)
    const folderLeaves = folder._graph.getLeaves(Folder, { node: folder });
    const connection2 = await Folder.get({
      where: Folder.property("id").eq(folderLeaves[0].id),
      Folders: Folder.search({
        join: Join.of(JoinType.PARENT, { recursive: true }),
      }),
    }).execute();
    const foldersUnpacked = connection2.graph.getRoots(Folder);
    expect(foldersUnpacked.length).toBe(1);
    expect(foldersUnpacked[0].equals(rootFolder));
  }

  // delete root folder (should cascade delete all folders)
  const numTotalFolders = await Folder.count({
    where: Folder.property("deleted_at").isNull(),
  }).executeCount();
  session.delete(rootFolder);
  await session.commit();
  expect(await Folder.count({ where: Folder.property("deleted_at").isNull() }).executeCount()).toBe(
    0,
  );
  // restore root folder (should restore all folders)
  session.restore(rootFolder);
  await session.commit();
  expect(await Folder.count({ where: Folder.property("deleted_at").isNull() }).executeCount()).toBe(
    numTotalFolders,
  );

  // delete and restore subfolders one at a time
  for (const [i, folder] of rootFolder.getChildren(Folder).entries()) {
    // delete just this subfolder (and its descendants)
    session.delete(folder);
    await session.commit();
    const connection = await Folder.get({
      where: Folder.property("id").eq(folder.id).and(Folder.property("deleted_at").isNull()),
      Folders: Folder.search({
        join: Join.of(JoinType.CHILD, { recursive: true }),
        where: Folder.property("deleted_at").isNull(),
      }),
    }).execute();
    expect(connection.toOneOrNone()).toBeNull();

    expect(
      await Folder.count({
        where: Folder.property("deleted_at").isNull(),
      }).executeCount(),
    ).toBe(numTotalFolders - (i + 1) * (subtreeFolderCount + 1));
  }
  // restore subfolders one at a time
  for (const [i, folder] of rootFolder.getChildren(Folder).entries()) {
    session.restore(folder);
    await session.commit();
    expect(
      await Folder.count({
        where: Folder.property("deleted_at").isNull(),
      }).executeCount(),
    ).toBe(1 + (i + 1) * (subtreeFolderCount + 1));
  }
});

sessionTest("create scene with heterogeneous views", async ({ session }) => {
  // create scene
  const scene = new Scene({ name: "Scene" });
  session.create(scene);
  await session.commit();

  const rootView = new FrameView({ name: "Container" });
  scene.addChild(rootView);

  // create views
  for (let i = 0; i < 4; i++) {
    const frameView = new FrameView({ name: `View ${i}` });
    rootView.addChild(frameView);

    for (let j = 0; j < 4; j++) {
      const labelView = new LabelView({ name: `Label ${i}/${j}` });
      frameView.addChild(labelView);

      for (let k = 0; k < 4; k++) {
        const textView = new TextView({ name: `Text ${i}/${j}/${k}` });
        labelView.addChild(textView);
      }
    }

    const labelView = new LabelView({ name: `Label ${i}` });
    rootView.addChild(labelView);
  }
  await session.commit();

  // query view (child, non-recursive)
  const sceneTree = await FrameView.get({
    where: FrameView.property("id").eq(rootView.id),
    Views: View.search({ join: Join.of(JoinType.CHILD) }),
  }).execute();
  const sceneUnpacked = sceneTree.toOne();
  const viewTreeUnpacked = sceneUnpacked.getDescendants(View);
  expect(viewTreeUnpacked.length).toBe(8);

  // query view (child, recursive)
  const sceneTree2 = await FrameView.get({
    where: FrameView.property("id").eq(rootView.id),
    Views: View.search({ join: Join.of(JoinType.CHILD, { recursive: true }) }),
  }).execute();
  const sceneUnpacked2 = sceneTree2.toOne();
  const viewTreeUnpacked2 = sceneUnpacked2.getDescendants(View);
  expect(viewTreeUnpacked2.length).toBe(4 + 4 * (1 + 4 * (1 + 4)));

  // query view (parent, recursive)
  const viewLeaves = scene._graph.getLeaves(TextView, { node: scene });
  const sceneTree3 = await TextView.get({
    where: TextView.property("id").eq(viewLeaves[0].id),
    Parents: View.search({ join: Join.of(JoinType.PARENT, { recursive: true }) }),
  }).execute();
  const sceneUnpacked3 = sceneTree3.graph.getRoots(View);
  expect(sceneUnpacked3.length).toBe(1);
  expect(sceneUnpacked3[0].equals(scene));
});

sessionTest("create reaction groups", async ({ session }) => {
  // create users
  const users = Array.from(
    { length: 10 },
    (_, i) =>
      new User({
        name: `User${i}`,
        slug: `user${i}`,
        space: new NodeReference({ type: NodeType.SPACE, id: uuid4() }),
      }),
  );
  for (const user of users) {
    session.create(user);
  }
  await session.commit();

  // create message
  const message = new Message({});
  session.create(message);
  await session.commit();

  // create reactions
  const reactionsContent = ["👍", "👎", "🤷", "🤔", "🤨"] as const;
  const reactions: Reaction[] = [];
  for (const user of users) {
    for (const reactionContent of reactionsContent) {
      const reaction = new Reaction({ parent: message, content: reactionContent, ownedBy: user });
      reactions.push(reaction);
      session.create(reaction);
    }
  }
  await session.commit();

  // scalar by group
  const messageTree = await Message.get({
    where: Message.property("id").eq(message.id),
    Reactions: Reaction.count({
      sort: [Reaction.property("createdAt").asc()],
      groupBy: [Reaction.property("content")],
    }),
  }).execute();
  const reactionsByGroup = messageTree.get("Reactions").toScalarByGroup();
  expect(reactionsByGroup).toEqual(
    Object.fromEntries(reactionsContent.map((content) => [content, 10])),
  );

  // node by group
  const messageTree2 = await Message.get({
    where: Message.property("id").eq(message.id),
    Reactions: Reaction.search({ groupBy: [Reaction.property("content")] }),
    ReactionsTotal: Reaction.count(),
  }).execute();
  const reactionsByContent = Object.fromEntries(
    reactionsContent.map((content) => [
      content,
      reactions.filter((reaction) => reaction.content === content),
    ]),
  );
  const reactionsByContentUnpacked = messageTree2.get("Reactions").toListByGroup();
  for (const reactionContent of reactionsContent) {
    const reactions = reactionsByContent[reactionContent];
    const reactionsUnpacked = reactionsByContentUnpacked[reactionContent];
    expect(new Set(reactions.map((r) => r.id))).toEqual(
      new Set(reactionsUnpacked.map((r) => r.id)),
    );
  }
});
