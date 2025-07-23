import { afterEach, beforeEach, expect, test } from "bun:test";
import { MemoryGraph } from "@destack/graph/memory";
import {
  BuiltinObject,
  Folder,
  Join,
  JoinType,
  NodeReference,
  NodeType,
  Region,
  Session,
  Universe,
  User,
  UserStatus,
  WORLD_ORACLE,
} from "@destack/language";
import { ENCODERS } from "@destack/language/core/builtin/const";
import { createAndActivateSpace } from "@destack/test/conftest";
import { uuid4 } from "@destack/utils";

let session: Session;

beforeEach(async () => {
  const graph = new MemoryGraph();
  await graph.open();
  session = new Session({
    graph,
    epoch: 1,
    actor: Universe.ACTOR,
    client: Universe.CLIENT,
    clientNonce: uuid4(),
    oracle: WORLD_ORACLE,
  });
  await session.open();
  const { space } = createAndActivateSpace({
    session,
    region: Region.ZURICH,
    ownedBy: session.actorPtr,
    name: "My Space",
    slug: "my-space",
  });
});

afterEach(async () => {
  await session.close();
});

/** Test that a BuiltinObject can be packed and unpacked in a nice roundtrip. */
function _testRoundtripObject(obj: BuiltinObject, session: Session): void {
  for (const [_, encoder] of Object.entries(ENCODERS)) {
    // pack/unpack as object
    const packedObj = encoder.packObject(
      (obj.constructor as typeof BuiltinObject).__kind__,
      obj.metatype,
      obj,
    );
    const packedObjBytes = encoder.packObjectBinary(
      (obj.constructor as typeof BuiltinObject).__kind__,
      obj.metatype,
      obj,
    );
    const unpackedObj = encoder.unpackObject(
      (obj.constructor as typeof BuiltinObject).__kind__,
      obj.metatype,
      packedObj,
      session,
    );
    expect(unpackedObj.equals(obj)).toBe(true);
    expect(unpackedObj.hash()).toEqual(obj.hash());

    // pack/unpack as bytes
    const packedObjBytes2 = encoder.packObjectBinary(
      (obj.constructor as typeof BuiltinObject).__kind__,
      obj.metatype,
      obj,
    );
    const unpackedObjBytes = encoder.unpackObjectBinary(
      (obj.constructor as typeof BuiltinObject).__kind__,
      obj.metatype,
      packedObjBytes2,
      session,
    );
    expect(unpackedObjBytes.equals(obj)).toBe(true);
    expect(unpackedObjBytes.hash()).toEqual(obj.hash());
  }
}

test("roundtrip node reference", () => {
  // pack and unpack a NodeReference
  const nodeRef = new NodeReference({
    type: NodeType.FOLDER,
    id: uuid4(),
    spaceId: uuid4(),
    branchId: uuid4(),
    snapshotId: uuid4(),
    definitionId: uuid4(),
  });
  _testRoundtripObject(nodeRef, session);
});

test("roundtrip query", () => {
  // pack and unpack a Query
  const query = Folder.search({
    sort: [Folder.property("created_at").asc()],
    limit: 25,
    subfolders: Folder.get({
      join: Join.of(JoinType.LEFT, { on: Folder.property("created_epoch").eq(5) }),
    }),
  });
  _testRoundtripObject(query, session);
});

test("roundtrip user", () => {
  // pack and unpack a User
  const user = new User({
    status: UserStatus.ACTIVE,
    name: "Florian",
    slug: "florian",
    space: new NodeReference({
      id: uuid4(),
      type: NodeType.SPACE,
      spaceId: uuid4(),
      branchId: uuid4(),
      snapshotId: uuid4(),
    }),
  });
  _testRoundtripObject(user, session);
});
