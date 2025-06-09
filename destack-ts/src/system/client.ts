import { NODE_TYPES } from "@/language/core/const";
import { NodeGraph, type ReadNodeGraph } from "@/language/core/graph";
import { ClientOriginData, ClientType, LocalStorage, NodeType, ObjectType, SpaceData } from "@/proto/wire";
import { describeNode, makeScope, nodeReference, toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { getBrowserName, getBrowserVersion, getDeviceType, getOperatingSystem } from "@/utils/browser";
import { IS_DEVELOPER_MODE as globalIsDeveloperMode } from "@/utils/globals";
import { log } from "@/utils/log";
import { computedValue, pickRef, pretendReadonly } from "@/utils/ref";
import { pseudoRandomNumber, xorString } from "@/utils/string";
import { syncRef, useLocalStorage } from "@vueuse/core";
import { v4 } from "uuid";
import { computed, readonly, shallowRef, type Ref } from "vue";

const DESTACK_LOCAL_STORAGE_PREFIX = "destack-";

export const LOCAL_DESTACK_ID = "00000000-0000-0000-0000-000000000000";
export const LOCAL_PACKAGE_ID = "00000000-0000-0000-0000-000000000001";
export const LOCAL_SPACE_ID = "00000000-0000-0000-0000-000000000002";

export const LOCAL_DESTACK_PTR = nodeReference(NodeType.SPACE, LOCAL_DESTACK_ID);
export const LOCAL_PACKAGE_PTR = nodeReference(NodeType.PACKAGE, LOCAL_PACKAGE_ID, { destackId: LOCAL_DESTACK_ID });
export const LOCAL_SPACE_PTR = nodeReference(NodeType.SPACE, LOCAL_SPACE_ID, { destackId: LOCAL_DESTACK_ID });

//
// NOTE: we use a simple semi-randomised XOR shift encoding for local storage.
//  This isn't meant to be secure, just to make it marginally more annoying to read & modify the data.
//
const ENCODE_LOCAL_STORAGE = true;

/**
 * Uses a value in our web-local storage.
 * Each top-level key is a separate local storage property (key = field number), the value is base64 of the message bytes.
 * All fields must be plain messages or repeated messages (for simpler encoding/decoding). :LocalStorageEncoding
 */
export function useLocal<T extends keyof LocalStorage>(key: T): Ref<LocalStorage[T] | null> {
  const field = LocalStorage.fields.find((f) => f.localName === key);
  if (field == null) throw new Error(`local field ${key} not found`);
  if (field.kind != "message") throw new Error(`local field ${key} is not a message`);
  const localStorageKey = DESTACK_LOCAL_STORAGE_PREFIX + field.no;
  const localStorageValue = useLocalStorage<string | null>(localStorageKey, null);
  const localEncodeKey = pseudoRandomNumber(field.no);

  const decodedValue = computed({
    get() {
      try {
        // decode into binary parts
        let encodedValue = localStorageValue.value;
        if (encodedValue == null) return null;
        if (ENCODE_LOCAL_STORAGE) encodedValue = xorString(encodedValue, localEncodeKey);
        // decode into single/multiple values
        const values: any[] = [];
        const encodedParts = encodedValue.split("-");
        for (const part of encodedParts) {
          const bytes = Uint8Array.from(atob(part), (c) => c.charCodeAt(0));
          values.push(field.T().fromBinary(bytes));
        }
        if (field.repeat) return values;
        else return values[0];
      } catch (e) {
        log.error("local.decodeFailed", { key, error: e });
        localStorageValue.value = null; // clear invalid value
        return null;
      }
    },
    set(value: LocalStorage[T] | null) {
      if (value == null) {
        localStorageValue.value = null;
      } else {
        // encode into binary parts
        const values = (Array.isArray(value) ? value : [value]) as any[];
        const encodedParts: string[] = [];
        for (const v of values) {
          const part = btoa(String.fromCharCode(...field.T().toBinary(v)));
          encodedParts.push(part);
        }
        // encode into local storage string
        let encodedValue = encodedParts.join("-");
        if (ENCODE_LOCAL_STORAGE) encodedValue = xorString(encodedValue, localEncodeKey);
        localStorageValue.value = encodedValue;
      }
    },
  });
  return decodedValue;
}

//
// NOTE: local storage is mostly just pointers to the currently active X.
// So 'destackPtr' is the current Destack, 'packagePtr', is the current main Package in that Destack, etc.
// Other stuff may be loaded as well, we just need one main Destack/Package/Space/....
// For consistency we type these refs as readonly and mutate them only in specific places.
//

//
// Auth
//

export const CLIENT_TYPE = ClientType.WEB; // NOTE: will need to detect/change this later :HeterogenousClients
export const nonce = v4(); // changes per page load
export const origin: Readonly<Ref<ClientOriginData>> = pretendReadonly(
  computed(() => ({
    metatype: ObjectType.CLIENT_ORIGIN,
    type: CLIENT_TYPE,
    id: _clientInfo.value?.id ?? nonce,
    nonce,
  })),
);
export function isSameOrigin(other: ClientOriginData): boolean {
  return origin.value.id == other.id && origin.value.nonce == other.nonce;
}

const _persistentInfo = useLocal("persistentInfo");
const _userInfo = useLocal("userInfo");
const _clientInfo = useLocal("clientInfo");
export const persistentInfo = pretendReadonly(_persistentInfo);
export const userInfo = pretendReadonly(_userInfo);
export const clientInfo = pretendReadonly(_clientInfo);
export const userPtr = computed(() =>
  _userInfo.value?.id != null ? nodeReference(NodeType.USER, _userInfo.value.id) : null,
);
export const userOrNullPtr = computed(() =>
  userPtr.value != null ? userPtr.value : nodeReference(NodeType.USER, "00000000-0000-0000-0000-000000000000"),
);

// ensure persistent info is set
if (_persistentInfo.value?.placeId == null) {
  _persistentInfo.value = { placeId: v4(), ..._persistentInfo.value };
}

const isOpera = !!(window as any).opera;
export const clientMeta = readonly(
  shallowRef({
    type: CLIENT_TYPE,
    deviceType: getDeviceType(window.navigator.userAgent),
    operatingSystem: getOperatingSystem(window),
    browserName: getBrowserName(window.navigator.userAgent, window.navigator.vendor, isOpera),
    browserVersion: getBrowserVersion(window.navigator.userAgent, window.navigator.vendor, isOpera)?.toString(),
    nonce: v4(),
  }),
);

export const IS_CHROMIUM = clientMeta.value.browserName == "Chrome";

function setUser(info: { user: Required<LocalStorage>["userInfo"]; client: Required<LocalStorage>["clientInfo"] }) {
  log.trace("local.setUser", { user: info.user });
  _userInfo.value = info.user;
  _clientInfo.value = info.client;
}

function clearUser() {
  log.trace("local.clearUser");
  _userInfo.value = null;
  _clientInfo.value = null;
}

//
// Space
//

// Current Space. May be local if not in current Destack.
const _spacePtr = useLocal("spacePtr") as Ref<TypedNodeReferenceData<NodeType.SPACE> | null>;
// Current Destack.
const _destackPtr = useLocal("destackPtr") as Ref<TypedNodeReferenceData<NodeType.SPACE> | null>;
// The Destack->Package mappings.
const _packagePtrs = useLocal("packagePtrs") as Ref<TypedNodeReferenceData<NodeType.PACKAGE>[]>;
// The Destack->Space mappings.
const _spacePtrs = useLocal("spacePtrs") as Ref<TypedNodeReferenceData<NodeType.SPACE>[]>;
// packagePtr is derived from destackPtr+packagePtrs
const packageIdByDestackId = computed(() => {
  const packageIdByDestackId: Record<string, string> = {};
  for (const pkg of _packagePtrs.value) {
    if (packageIdByDestackId[pkg.destackId!] != null) continue; // ignore duplicates
    packageIdByDestackId[pkg.destackId!] = pkg.id!;
  }
  return packageIdByDestackId;
});
export const spacePtr = pretendReadonly(computed(() => _spacePtr.value ?? LOCAL_SPACE_PTR));
export const destackPtr = pretendReadonly(_destackPtr);
export const packagePtr = computed(() => {
  if (_destackPtr.value == null) return null;
  else
    return nodeReference(NodeType.PACKAGE, packageIdByDestackId.value[_destackPtr.value.id!], {
      destackId: _destackPtr.value.id!,
    });
}) as Readonly<Ref<TypedNodeReferenceData<NodeType.PACKAGE> | null>>;
export const CURRENT_DESTACK_SCOPE = computedValue(() => makeScope({ destackId: _destackPtr.value?.id }));
export const CURRENT_PACKAGE_SCOPE = computedValue(() =>
  makeScope({ destackId: _destackPtr.value?.id, packageIds: packagePtr.value?.id != null ? [packagePtr.value.id] : [] }),
);

// current local Space graph (not yet persisted).
const _spaceLocal = {
  metatype: ObjectType.SPACE,
  name: "Local",
  id: LOCAL_SPACE_ID,
  packagePtr: LOCAL_PACKAGE_PTR,
  destackPtr: LOCAL_DESTACK_PTR,
} as SpaceData;
const _spaceGraphLocal = new NodeGraph({
  scope: makeScope({ destackId: LOCAL_DESTACK_ID, packageIds: [LOCAL_PACKAGE_ID] }),
  nodeTypes: new Set(NODE_TYPES),
});
_spaceGraphLocal.add(_spaceLocal);
export const spaceGraphLocal = _spaceGraphLocal as ReadNodeGraph;

/** Sets the active space. Must be local or from the current package. Also replaces main space for that destack. */
function setSpace(space: TypedNodeReferenceData<NodeType.SPACE>) {
  if (space.destackId != LOCAL_DESTACK_ID && packageIdByDestackId.value[space.destackId!] == null) {
    throw new Error(
      `${describeNode(space)} is not in the active Package ${packageIdByDestackId.value[space.destackId!] ?? "<unset>"}`,
    );
  }
  log.trace("local.setSpace", space);
  _spacePtr.value = space;
  _spacePtrs.value = (_spacePtrs.value?.filter((s) => s.destackId != space.destackId) ?? []).concat(space);
}

/** Resets the space to the local space. Does not affect the Destack. */
function setSpaceToLocal() {
  log.trace("local.setSpaceToLocal");
  _spacePtr.value = nodeReference(NodeType.SPACE, LOCAL_SPACE_ID, { destackId: LOCAL_DESTACK_ID });
}

function getSpacePtr(destackId: string): TypedNodeReferenceData<NodeType.SPACE> | null {
  return _spacePtrs.value?.find((s) => s.destackId == destackId) ?? null;
}

/**
 * Sets the current Destack/Package and Space.
 * If it doesn't match the current space, it tries to restore the last spacethe space is reset to local.
 **/
function setDestack(set: {
  pkg: TypedNodeReferenceData<NodeType.PACKAGE>;
  space?: TypedNodeReferenceData<NodeType.SPACE> | null;
}) {
  log.trace("local.setDestack", set);
  if (set.pkg.destackId == null || set.pkg.destackId == LOCAL_DESTACK_ID) {
    throw new Error(`package ${set.pkg?.id} is not in a real Destack?`);
  }

  // set destack/package
  const destack = nodeReference(NodeType.SPACE, set.pkg.destackId!);
  _destackPtr.value = destack;
  if (_packagePtrs.value == null) _packagePtrs.value = [];
  _packagePtrs.value = _packagePtrs.value.filter((p) => p.destackId != destack.id).concat(set.pkg);

  // set space
  if (set.space == null) set = { ...set, space: getSpacePtr(destack.id!) };
  if (set.space != null) {
    if (set.space.destackId != destack.id) throw new Error(`space ${set.space.id} is not in the active Destack ${destack.id}`);
    const adaptedSpace = toNodeRef(set.space);
    setSpace(adaptedSpace);
  } else {
    setSpaceToLocal();
  }
}

/** Resets current Destack/Package/Space. */
function clearDestack() {
  log.trace("local.clearDestack");
  if (_destackPtr.value != null) {
    _packagePtrs.value = (_packagePtrs.value ?? []).filter((p) => p.destackId != _destackPtr.value!.id);
    _destackPtr.value = null;
  }
  setSpaceToLocal();
}

function clearSpace() {
  log.trace("local.clearSpace");
  _spaceGraphLocal.clear();
  _spaceGraphLocal.add(_spaceLocal);
}
//
// Developer stuff
//

const developerSettings = useLocal("developerSettings");
export const isDeveloperMode = pickRef(developerSettings, "isDeveloperMode", false);
syncRef(isDeveloperMode, globalIsDeveloperMode);

// (re-)export some stuff in a wrapper for clarity

const local = {
  userInfo,
  clientInfo,
  clientMeta,
  setUser,
  clearUser,
  spacePtr,
  destackPtr,
  packagePtr,
  setSpace,
  setSpaceToLocal,
  getSpacePtr,
  setDestack,
  clearDestack,
  clearSpace,
  isDeveloperMode,
} as const;

export default local;
