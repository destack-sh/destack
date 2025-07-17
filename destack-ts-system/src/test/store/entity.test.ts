import { IndexedDBStore } from "@destack-web/store/indexeddb";
import { PostgresEntityStore } from "@desys/store/postgres";
import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import {
  ACTIVE_SPACE,
  Client,
  ClientType,
  Folder,
  FolderType,
  FrameView,
  getFromEnv,
  Join,
  JoinType,
  LabelView,
  Layer,
  MemoryStore,
  NodeReference,
  NodeType,
  Reaction,
  Region,
  Session,
  Space,
  SpaceStatus,
  Star,
  StoreKey,
  TextView,
  User,
  UserStatus,
  uuid4,
  View,
} from "destack";
import { IDBFactory } from "fake-indexeddb";
import "fake-indexeddb/auto";

const storeImplementations = [
  {
    name: "MemoryStore",
    createStore: async () => {
      const store = new MemoryStore({
        keys: [StoreKey.ENTITY_PRIMARY],
      });
      return store;
    },
    tearDown: async (store: MemoryStore) => {},
  },
  {
    name: "IndexedDBStore",
    createStore: async () => {
      // reset fake indexeddb
      globalThis.indexedDB = new IDBFactory();

      const store = new IndexedDBStore({
        keys: [StoreKey.ENTITY_PRIMARY],
      });
      await store.open();
      return store;
    },
    tearDown: async (store: IndexedDBStore) => {
      await store.close();
    },
  },
  {
    name: "PostgresEntityStore",
    createStore: async () => {
      const postgresUrl = await getFromEnv("POSTGRES_URL", "string");
      const store = new PostgresEntityStore({
        database: { url: postgresUrl },
        keys: [StoreKey.ENTITY_PRIMARY],
      });
      await store.open();
      return store;
    },
    tearDown: async (store: PostgresEntityStore) => {
      await store.close();
    },
  },
];

describe.each(storeImplementations)("$name", ({ createStore, tearDown }) => {
  let session: Session;
  let store: MemoryStore | IndexedDBStore | PostgresEntityStore;

  beforeEach(async () => {
    store = await createStore();
    // session
    session = new Session({ store, epoch: 1 });
    await session.open();
    // space
    const space = new Space({
      name: "My Space",
      slug: "my-space",
      status: SpaceStatus.ACTIVE,
      region: Region.ZURICH,
    });
    ACTIVE_SPACE.set(space);
  });

  afterEach(async () => {
    // teardown
    await session.close();
    await tearDown(store as any);
  });

  test("create user with clients", async () => {
    // create user
    const user = new User({
      status: UserStatus.ACTIVE,
      name: "Floof",
      slug: "floof",
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

  test("create star", async () => {
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

  const NUM_FOLDERS_PER_LEVEL = 4;
  test("create folders recursive", async () => {
    // create
    const rootFolder = new Folder({ name: "Folder", type: FolderType.HOME });
    session.create(rootFolder);
    const subtreeFolderCount =
      NUM_FOLDERS_PER_LEVEL * (1 + NUM_FOLDERS_PER_LEVEL * (1 + NUM_FOLDERS_PER_LEVEL));
    for (const a of Array.from({ length: NUM_FOLDERS_PER_LEVEL }, (_, i) => `a${i}`)) {
      // create folder
      const folder = new Folder({ name: `Folder ${a}` });
      rootFolder.addChild(folder);
      // create folder tree
      for (let i = 0; i < NUM_FOLDERS_PER_LEVEL; i++) {
        const subFolder = new Folder({ name: `Folder ${a}/${i}` });
        folder.addChild(subFolder);
        for (let j = 0; j < NUM_FOLDERS_PER_LEVEL; j++) {
          const innerFolder = new Folder({ name: `Folder ${a}/${i}/${j}` });
          subFolder.addChild(innerFolder);
          for (let k = 0; k < NUM_FOLDERS_PER_LEVEL; k++) {
            const innerInnerFolder = new Folder({ name: `Folder ${a}/${i}/${j}/${k}` });
            innerFolder.addChild(innerInnerFolder);
          }
        }
      }
      await session.commit();
    }

    const rootFolderChildCount = await Folder.count({
      where: Folder.property("parent").eq(rootFolder),
    }).executeCount();
    expect(rootFolderChildCount).toBe(NUM_FOLDERS_PER_LEVEL);

    // query
    for (const folder of rootFolder.getChildren(Folder)) {
      // query folder root count
      const rootFolderCount = await Folder.count({
        where: Folder.property("parent").eq(folder),
      }).executeCount();
      expect(rootFolderCount).toBe(NUM_FOLDERS_PER_LEVEL);

      // query folder down (parent, recursive)
      const connection = await Folder.get({
        where: Folder.property("id").eq(folder.id),
        Folders: Folder.search({ join: Join.of(JoinType.CHILD, { recursive: true }) }),
      }).execute();
      const folderUnpacked = connection.toOne();
      const folderTreeUnpacked = folderUnpacked.getDescendants(Folder);
      expect(folderTreeUnpacked.length).toBe(subtreeFolderCount);

      // query folder up (parent, recursive)
      const folderLeaves = folder._graph.getLeaves({
        nodeType: Folder.metatype,
        node: folder,
      }) as Folder[];
      const connection2 = await Folder.get({
        where: Folder.property("id").eq(folderLeaves[0].id),
        Folders: Folder.search({
          join: Join.of(JoinType.PARENT, { recursive: true }),
        }),
      }).execute();
      const foldersUnpacked = connection2.graph.getRoots({ nodeType: Folder.metatype }) as Folder[];
      expect(foldersUnpacked.length).toBe(1);
      expect(foldersUnpacked[0].equals(rootFolder));
    }

    // delete root folder (should cascade delete all folders)
    const numTotalFolders = await Folder.count({
      where: Folder.property("deletedAt").isNull(),
    }).executeCount();
    session.delete(rootFolder);
    await session.commit();

    const deletedFoldersCount = await Folder.count({
      where: Folder.property("deletedAt").isNull(),
    }).executeCount();
    expect(deletedFoldersCount).toBe(0);

    // restore root folder (should restore all folders)
    session.restore(rootFolder);
    await session.commit();

    const restoredFoldersCount = await Folder.count({
      where: Folder.property("deletedAt").isNull(),
    }).executeCount();
    expect(restoredFoldersCount).toBe(numTotalFolders);

    // delete and restore subfolders one at a time
    for (const [i, folder] of rootFolder.getChildren(Folder).entries()) {
      // delete just this subfolder (and its descendants)
      session.delete(folder);
      await session.commit();

      const connection = await Folder.get({
        where: Folder.property("id").eq(folder.id).and(Folder.property("deletedAt").isNull()),
        Folders: Folder.search({
          join: Join.of(JoinType.CHILD, { recursive: true }),
          where: Folder.property("deletedAt").isNull(),
        }),
      }).execute();
      expect(connection.toOneOrNone()).toBeNull();

      const remainingFoldersCount = await Folder.count({
        where: Folder.property("deletedAt").isNull(),
      }).executeCount();
      expect(remainingFoldersCount).toBe(numTotalFolders - (i + 1) * (subtreeFolderCount + 1));
    }
    // restore subfolders one at a time
    for (const [i, folder] of rootFolder.getChildren(Folder).entries()) {
      session.restore(folder);
      await session.commit();

      const restoredSubfoldersCount = await Folder.count({
        where: Folder.property("deletedAt").isNull(),
      }).executeCount();
      expect(restoredSubfoldersCount).toBe(1 + (i + 1) * (subtreeFolderCount + 1));
    }
  });

  test("create layer with heterogeneous views", async () => {
    // create layer
    const layer = new Layer({ name: "Layer" });
    session.create(layer);
    await session.commit();

    const rootView = new FrameView({ name: "Container" });
    layer.addChild(rootView);

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
    const layerTree = await FrameView.get({
      where: FrameView.property("id").eq(rootView.id),
      Views: View.search({ join: Join.of(JoinType.CHILD) }),
    }).execute();
    const layerUnpacked = layerTree.toOne();
    const viewTreeUnpacked = layerUnpacked.getDescendants(View);
    expect(viewTreeUnpacked.length).toBe(8);

    // query view (child, recursive)
    const layerTree2 = await FrameView.get({
      where: FrameView.property("id").eq(rootView.id),
      Views: View.search({ join: Join.of(JoinType.CHILD, { recursive: true }) }),
    }).execute();
    const layerUnpacked2 = layerTree2.toOne();
    const viewTreeUnpacked2 = layerUnpacked2.getDescendants(View);
    expect(viewTreeUnpacked2.length).toBe(4 + 4 * (1 + 4 * (1 + 4)));

    // query view (parent, recursive)
    const viewLeaves = layer._graph.getLeaves({
      nodeType: TextView.metatype,
      node: layer,
    }) as TextView[];
    const layerTree3 = await TextView.get({
      where: TextView.property("id").eq(viewLeaves[0].id),
      Parents: View.search({
        join: Join.of(JoinType.PARENT, { recursive: true }),
        where: View.property("deletedAt").isNull(),
        Layers: Layer.search({
          join: Join.of(JoinType.PARENT, { recursive: true }),
        }),
      }),
    }).execute();
    const layerUnpacked3 = layerTree3.graph.getRoots({ nodeType: Layer.metatype }) as Layer[];
    expect(layerUnpacked3[0].equals(layer));
  });

  test("create reaction groups", async () => {
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

    // create folder
    const folder = new Folder({ name: "Folder" });
    session.create(folder);
    await session.commit();

    // create reactions
    const reactionsContent = ["👍", "👎", "🤷", "🤔", "🤨"] as const;
    const reactions: Reaction[] = [];
    for (const user of users) {
      for (const reactionContent of reactionsContent) {
        const reaction = new Reaction({ parent: folder, content: reactionContent, ownedBy: user });
        reactions.push(reaction);
        session.create(reaction);
      }
    }
    await session.commit();

    // scalar by group
    const folderTree = await Folder.get({
      where: Folder.property("id").eq(folder.id),
      Reactions: Reaction.count({
        sort: [Reaction.property("createdAt").asc()],
        groupBy: [Reaction.property("content")],
      }),
    }).execute();
    const reactionsByGroup = folderTree.get("Reactions").toScalarByGroup();
    expect(reactionsByGroup).toEqual(
      Object.fromEntries(reactionsContent.map((content) => [content, 10])),
    );

    // node by group
    const folderTree2 = await Folder.get({
      where: Folder.property("id").eq(folder.id),
      Reactions: Reaction.search({ groupBy: [Reaction.property("content")] }),
      ReactionsTotal: Reaction.count(),
    }).execute();
    const reactionsByContent = Object.fromEntries(
      reactionsContent.map((content) => [
        content,
        reactions.filter((reaction) => reaction.content === content),
      ]),
    );
    const reactionsByContentUnpacked = folderTree2.get("Reactions").toListByGroup();
    for (const reactionContent of reactionsContent) {
      const reactions = reactionsByContent[reactionContent];
      const reactionsUnpacked = reactionsByContentUnpacked[reactionContent];
      expect(new Set(reactions.map((r) => r.id))).toEqual(
        new Set(reactionsUnpacked.map((r) => r.id)),
      );
    }
  });

  test("move views", async () => {
    const layer = new Layer({ name: "Layer" });
    session.create(layer);
    await session.commit();

    // create views
    const rootView = new FrameView({ name: "Root" });
    const frameViews: FrameView[] = [];
    layer.addChild(rootView);
    for (let i = 0; i < 4; i++) {
      const frameView = new FrameView({ name: `View ${i}` });
      frameViews.push(frameView);
      rootView.addChild(frameView);
      for (let j = 0; j < 4; j++) {
        const labelView = new LabelView({ name: `Label ${i}/${j}` });
        frameView.addChild(labelView);
      }
    }
    await session.commit();

    // detach views
    for (const frameView of frameViews) {
      frameView.detach();
      expect(frameView.parentPtr).toBeNull();
    }
    await session.commit();

    // reattach views
    for (const frameView of frameViews) {
      layer.addChild(frameView);
      expect(frameView.parentPtr).toEqual(layer.toRef());
    }
    await session.commit();

    // detach all the leaf label views
    const labelViews: LabelView[] = [];
    for (const frameView of frameViews) {
      for (const labelView of frameView.getChildren(LabelView)) {
        labelView.detach();
        labelViews.push(labelView);
        expect(labelView.parentPtr).toBeNull();
      }
    }
    await session.commit();

    // move all views to be directly parented by layer
    for (const view of [...frameViews, ...labelViews]) {
      view.moveTo(layer);
      expect(view.parentPtr).toEqual(layer.toRef());
    }
    await session.commit();

    const layerChildren = layer.getChildren(View);
    expect(layerChildren).toHaveLength(1 + 4 * (4 + 1));
  });
});
