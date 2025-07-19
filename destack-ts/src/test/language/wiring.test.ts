import {
  EventCursor,
  Folder,
  Join,
  JoinType,
  NodeReference,
  NodeType,
  Query,
  Session,
  Space,
  User,
  UserStatus,
} from "@destack/language";
import { NodeReferenceProto, QueryProto, UserProto } from "@destack/proto";
import { createAndActivateSpace } from "@destack/test/conftest";
import { uuid4 } from "@destack/utils";
import { afterEach, beforeEach, expect, test } from "bun:test";

let session: Session;

beforeEach(async () => {
  session = new Session({ epoch: 1 });
  await session.open();
  const { space } = createAndActivateSpace({ session });
});

afterEach(async () => {
  await session.close();
});

test("roundtrip node reference", () => {
  // pack and unpack a NodeReference as value
  const nodeRef = new NodeReference({
    type: NodeType.FOLDER,
    id: uuid4(),
    spaceId: uuid4(),
    definitionId: uuid4(),
  });

  // value
  const nodeRefValue = nodeRef.toValue();
  const nodeRefValueStr = JSON.stringify(nodeRefValue, null, 2);
  const unpackedNodeRefValue = JSON.parse(nodeRefValueStr);
  const unpackedNodeRef = NodeReference.fromValue(unpackedNodeRefValue);
  expect(unpackedNodeRef.equals(nodeRef)).toBe(true);
  expect(unpackedNodeRef.toValue()).toEqual(unpackedNodeRefValue);
  expect(unpackedNodeRef.hash()).toEqual(nodeRef.hash());

  // proto
  const nodeRefProto = nodeRef.toProto();
  const nodeRefProtoBytes = NodeReferenceProto.toBinary(nodeRefProto);
  const unpackedNodeRefProto = NodeReferenceProto.fromBinary(nodeRefProtoBytes);
  const unpackedNodeRef2 = NodeReference.fromProto(unpackedNodeRefProto);
  expect(unpackedNodeRef2.equals(nodeRef)).toBe(true);
  expect(unpackedNodeRef2.hash()).toEqual(nodeRef.hash());
});

test("roundtrip query", () => {
  // pack and unpack a Query as value
  const query = Folder.search({
    sort: [Folder.property("created_at").asc()],
    limit: 25,
    cursor: EventCursor.get({
      join: Join.of(JoinType.LEFT, { on: EventCursor.property("created_epoch").eq(5) }),
    }),
  });

  // value
  const queryValue = query.toValue();
  const queryValueStr = JSON.stringify(queryValue, null, 2);
  const unpackedQueryValue = JSON.parse(queryValueStr);
  const unpackedQuery = Query.fromValue(unpackedQueryValue);
  expect(unpackedQuery.equals(query)).toBe(true);
  expect(unpackedQuery.toValue()).toEqual(unpackedQueryValue);
  expect(unpackedQuery.hash()).toEqual(query.hash());

  // proto
  const queryProto = query.toProto();
  const queryProtoBytes = QueryProto.toBinary(queryProto);
  const unpackedQueryProto = QueryProto.fromBinary(queryProtoBytes);
  const unpackedQuery2 = Query.fromProto(unpackedQueryProto);
  expect(unpackedQuery2.equals(query)).toBe(true);
  expect(unpackedQuery2.hash()).toEqual(query.hash());
});

test("roundtrip user", () => {
  // pack and unpack a User as value
  const user = new User({
    status: UserStatus.ACTIVE,
    name: "Florian",
    slug: "florian",
    space: new NodeReference({ id: uuid4(), type: NodeType.SPACE }),
  });

  // value
  const userValue = user.toValue();
  const userValueStr = JSON.stringify(userValue, null, 2);
  const unpackedUserValue = JSON.parse(userValueStr);
  const unpackedUser = User.fromValue(unpackedUserValue);
  expect(unpackedUser.equals(user)).toBe(true);
  expect(unpackedUser.hash()).toEqual(user.hash());

  // proto
  const userProto = user.toProto();
  const userProtoBytes = UserProto.toBinary(userProto);
  const unpackedUserProto = UserProto.fromBinary(userProtoBytes);
  const unpackedUser2 = User.fromProto(unpackedUserProto);
  expect(unpackedUser2.equals(user)).toBe(true);
  expect(unpackedUser2.hash()).toEqual(user.hash());
});
