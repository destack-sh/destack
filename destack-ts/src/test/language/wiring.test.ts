import { MemoryGraph } from "@destack/graph/memory";
import {
  BuiltinObject,
  Encoding,
  EventCursor,
  Folder,
  Join,
  JoinType,
  NodeReference,
  NodeType,
  Session,
  User,
  UserStatus,
} from "@destack/language";
import { ENCODERS } from "@destack/language/core/builtin/const";
import { createAndActivateSpace } from "@destack/test/conftest";
import { uuid4 } from "@destack/utils";
import { afterEach, beforeEach, expect, test } from "bun:test";

let session: Session;

beforeEach(async () => {
  session = new Session({ epoch: 1, graph: new MemoryGraph() });
  await session.open();
  const { space } = createAndActivateSpace({ session });
});

afterEach(async () => {
  await session.close();
});

/** Test that a BuiltinObject can be packed and unpacked in a nice roundtrip. */
function _testRoundtripObject(obj: BuiltinObject, session: Session): void {
  for (const [_, encoder] of Object.entries(ENCODERS)) {
    // pack/unpack as object
    const packedObj = encoder.packObject({
      kind: (obj.constructor as typeof BuiltinObject).__kind__,
      metatype: obj.metatype,
      object: obj,
    });
    const packedObjBytes = encoder.packObjectBytes({
      kind: (obj.constructor as typeof BuiltinObject).__kind__,
      metatype: obj.metatype,
      object: obj,
    });
    const unpackedObj = encoder.unpackObject({
      kind: (obj.constructor as typeof BuiltinObject).__kind__,
      metatype: obj.metatype,
      value: packedObj,
      _session: session,
      _graph: new MemoryGraph(),
      _connection: null,
    });
    expect(unpackedObj.equals(obj)).toBe(true);
    expect(unpackedObj.hash()).toEqual(obj.hash());

    // pack/unpack as bytes
    const packedObjBytes2 = encoder.packObjectBytes({
      kind: (obj.constructor as typeof BuiltinObject).__kind__,
      metatype: obj.metatype,
      object: obj,
    });
    const unpackedObjBytes = encoder.unpackObjectBytes({
      kind: (obj.constructor as typeof BuiltinObject).__kind__,
      metatype: obj.metatype,
      value: packedObjBytes2,
      _session: session,
      _graph: new MemoryGraph(),
      _connection: null,
    });
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
    definitionId: uuid4(),
  });
  _testRoundtripObject(nodeRef, session);
});

test("roundtrip query", () => {
  // pack and unpack a Query
  const query = Folder.search({
    sort: [Folder.property("created_at").asc()],
    limit: 25,
    cursor: EventCursor.get({
      join: Join.of(JoinType.LEFT, { on: EventCursor.property("created_epoch").eq(5) }),
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
    space: new NodeReference({ id: uuid4(), type: NodeType.SPACE }),
  });
  _testRoundtripObject(user, session);
});
