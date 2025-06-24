import {
  Join,
  JoinType,
  NodeReference,
  NodeType,
  Query,
  Session,
  Thread,
  ThreadCursor,
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

sessionTest("roundtrip node reference", ({ session }) => {
  // pack and unpack a NodeReference as value
  const nodeRef = new NodeReference({
    nodeType: NodeType.FOLDER,
    id: uuid4(),
    spaceId: uuid4(),
    definitionId: uuid4(),
  });

  // value
  const nodeRefValue = nodeRef.toValue();
  const nodeRefValueStr = JSON.stringify(nodeRefValue, null, 2);
  console.log(nodeRefValueStr);
  const unpackedNodeRefValue = JSON.parse(nodeRefValueStr);
  const unpackedNodeRef = NodeReference.fromValue(unpackedNodeRefValue);
  expect(unpackedNodeRef.equals(nodeRef)).toBe(true);
  expect(unpackedNodeRef.toValue()).toEqual(unpackedNodeRefValue);

  // proto
  const nodeRefProto = nodeRef.toProto();
  const unpackedNodeRef2 = NodeReference.fromProto(nodeRefProto);
  expect(unpackedNodeRef2.equals(nodeRef)).toBe(true);
});

sessionTest("roundtrip query", ({ session }) => {
  // pack and unpack a Query as value
  const query = Thread.search({
    // sort: [Thread.property("createdAt").asc()],
    // limit: 25,
    // cursor: ThreadCursor.get({
    //   join: Join.of(JoinType.LEFT, { on: ThreadCursor.property("ownedBy").eq(5) }),
    // }),
  });

  // value
  const queryValue = query.toValue();
  const queryValueStr = JSON.stringify(queryValue, null, 2);
  console.log(queryValueStr);
  const unpackedQueryValue = JSON.parse(queryValueStr);
  const unpackedQuery = Query.fromValue(unpackedQueryValue);
  expect(unpackedQuery.equals(query)).toBe(true);
  expect(unpackedQuery.toValue()).toEqual(unpackedQueryValue);

  // proto
  const queryProto = query.toProto();
  const unpackedQuery2 = Query.fromProto(queryProto);
  expect(unpackedQuery2.equals(query)).toBe(true);
});

sessionTest("roundtrip user", ({ session }) => {
  // pack and unpack a User as value
  const user = new User({
    status: UserStatus.ACTIVE,
    name: "Florian",
    slug: "florian",
    space: new NodeReference({ id: uuid4(), nodeType: NodeType.SPACE }),
  });

  // value
  const userValue = user.toValue();
  const userValueStr = JSON.stringify(userValue, null, 2);
  console.log(userValueStr);
  const unpackedUserValue = JSON.parse(userValueStr);
  const unpackedUser = User.fromValue(unpackedUserValue);
  expect(unpackedUser.equals(user)).toBe(true);

  // proto
  const userProto = user.toProto();
  const unpackedUser2 = User.fromProto(userProto);
  expect(unpackedUser2.equals(user)).toBe(true);
});
