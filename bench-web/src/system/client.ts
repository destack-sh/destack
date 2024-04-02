import { BenchType, LocalNodeGraph, LocalStorage, NodeType, SpaceData } from "@/proto/wire";
import { describeNode, nodeReference, toNodeReferenceInPackage, type TypedNodeReferenceData } from "@/proto/wiring";
import { getBrowserName, getBrowserVersion, getDeviceType, getOperatingSystem } from "@/utils/browser";
import { log } from "@/utils/log";
import { pickRef, pretendReadonly } from "@/utils/ref";
import { pseudoRandomNumber, xorString } from "@/utils/string";
import { syncRef, useLocalStorage } from "@vueuse/core";
import { v4 } from "uuid";
import { computed, readonly, shallowRef, type Ref } from "vue";
import { isDeveloperMode as globalIsDeveloperMode } from "@/utils/globals";
import { NodeGraph, type ReadNodeGraph } from "@/system/graph";

const BENCH_LOCAL_STORAGE_PREFIX = "bench-";

export const LOCAL_BENCH_ID = "00000000-0000-0000-0000-000000000000";
export const LOCAL_PACKAGE_ID = "00000000-0000-0000-0000-000000000001";
export const LOCAL_SPACE_ID = "00000000-0000-0000-0000-000000000002";

export const LOCAL_BENCH_PTR = nodeReference(NodeType.BENCH, LOCAL_BENCH_ID);
export const LOCAL_PACKAGE_PTR = nodeReference(NodeType.PACKAGE, LOCAL_PACKAGE_ID, { benchId: LOCAL_BENCH_ID });
export const LOCAL_SPACE_PTR = nodeReference(NodeType.SPACE, LOCAL_SPACE_ID, { benchId: LOCAL_BENCH_ID });

//
// NOTE: we use a simple semi-randomised XOR shift encoding for local storage.
//  This isn't meant to be secure, just to make it annoying to read out & modify the data.
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
  const localStorageKey = BENCH_LOCAL_STORAGE_PREFIX + field.no;
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
// So 'benchPtr' is the current Bench, 'packagePtr', is the current main Package in that Bench, etc.
// Other stuff may be loaded as well, we just need one main Bench/Package/Space/....
// For consistency we type these refs as readonly and mutate them only in specific places.
//

//
// Auth
//

const _persistentInfo = useLocal("persistentInfo");
const _userInfo = useLocal("userInfo");
const _clientInfo = useLocal("clientInfo");
export const persistentInfo = pretendReadonly(_persistentInfo);
export const userInfo = pretendReadonly(_userInfo);
export const clientInfo = pretendReadonly(_clientInfo);
export const userPtr = computed(() =>
  _userInfo.value?.id != null ? nodeReference(NodeType.USER, _userInfo.value.id) : null,
);

// ensure persistent info is set
if (_persistentInfo.value?.placeId == null) {
  _persistentInfo.value = { placeId: v4(), ..._persistentInfo.value };
}

const isOpera = !!(window as any).opera;
export const clientMeta = readonly(
  shallowRef({
    deviceType: getDeviceType(window.navigator.userAgent),
    operatingSystem: getOperatingSystem(window),
    browserName: getBrowserName(window.navigator.userAgent, window.navigator.vendor, isOpera),
    browserVersion: getBrowserVersion(window.navigator.userAgent, window.navigator.vendor, isOpera)?.toString(),
    nonce: v4(),
  }),
);

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

// Current Space. May be local if not in current Bench.
const _spacePtr = useLocal("spacePtr") as Ref<TypedNodeReferenceData<NodeType.SPACE> | null>;
export const spacePtr = pretendReadonly(computed(() => _spacePtr.value ?? LOCAL_SPACE_PTR));
// Current Bench.
const _benchPtr = useLocal("benchPtr") as Ref<TypedNodeReferenceData<NodeType.BENCH> | null>;
export const benchPtr = pretendReadonly(_benchPtr);
// The Bench->Package mappings.
const _packagePtrs = useLocal("packagePtrs") as Ref<TypedNodeReferenceData<NodeType.PACKAGE>[]>;
// The Bench->Space mappings.
const _spacePtrs = useLocal("spacePtrs") as Ref<TypedNodeReferenceData<NodeType.SPACE>[]>;
// packagePtr is derived from benchPtr+packagePtrs
const packageIdByBenchId = computed(() => {
  const packageIdByBenchId: Record<string, string> = {};
  for (const pkg of _packagePtrs.value) {
    if (packageIdByBenchId[pkg.benchId!] != null) continue; // ignore duplicates
    packageIdByBenchId[pkg.benchId!] = pkg.id!;
  }
  return packageIdByBenchId;
});
export const packagePtr = computed(() => {
  if (_benchPtr.value == null) return null;
  else
    return nodeReference(NodeType.PACKAGE, packageIdByBenchId.value[_benchPtr.value.id!], {
      benchId: _benchPtr.value.id!,
    });
}) as Readonly<Ref<TypedNodeReferenceData<NodeType.PACKAGE> | null>>;
// Local persisted graphs.
const _localGraphs = useLocal("localGraphs") as Ref<LocalNodeGraph[] | null>;
// Current local Space graph (not yet persisted).
const _spaceGraphLocal = new NodeGraph({ scope: { benchId: LOCAL_BENCH_ID, packageId: LOCAL_PACKAGE_ID } });
_spaceGraphLocal.add({
  metatype: BenchType.SPACE,
  name: "Local",
  id: LOCAL_SPACE_ID,
  packagePtr: LOCAL_PACKAGE_PTR,
  benchPtr: LOCAL_BENCH_PTR,
} as SpaceData);
export const spaceGraphLocal = _spaceGraphLocal as ReadNodeGraph;

/** Sets the active space. Must be local or from the current package. Also replaces main space for that bench. */
function setSpace(space: TypedNodeReferenceData<NodeType.SPACE>) {
  if (space.benchId != LOCAL_BENCH_ID && packageIdByBenchId.value[space.benchId!] == null) {
    throw new Error(
      `${describeNode(space)} is not in the active Package ${packageIdByBenchId.value[space.benchId!] ?? "<unset>"}`,
    );
  }
  log.trace("local.setSpace", space);
  _spacePtr.value = space;
  _spacePtrs.value = (_spacePtrs.value?.filter((s) => s.benchId != space.benchId) ?? []).concat(space);
}

/** Resets the space to the local space. Does not affect the Bench. */
function setSpaceToLocal() {
  log.trace("local.setSpaceToLocal");
  _spacePtr.value = nodeReference(NodeType.SPACE, LOCAL_SPACE_ID, { benchId: LOCAL_BENCH_ID });
}

function getSpacePtr(benchId: string): TypedNodeReferenceData<NodeType.SPACE> | null {
  return _spacePtrs.value?.find((s) => s.benchId == benchId) ?? null;
}

/**
 * Sets the current Bench/Package and Space.
 * If it doesn't match the current space, it tries to restore the last spacethe space is reset to local.
 **/
function setBench(set: {
  pkg: TypedNodeReferenceData<NodeType.PACKAGE>;
  space?: TypedNodeReferenceData<NodeType.SPACE> | null;
}) {
  log.trace("local.setBench", set);
  if (set.pkg.benchId == null || set.pkg.benchId == LOCAL_BENCH_ID)
    throw new Error(`package ${set.pkg?.id} is not in a real Bench?`);

  // set bench/package
  const bench = nodeReference(NodeType.BENCH, set.pkg.benchId!);
  _benchPtr.value = bench;
  if (_packagePtrs.value == null) _packagePtrs.value = [];
  _packagePtrs.value = _packagePtrs.value.filter((p) => p.benchId != bench.id).concat(set.pkg);

  // set space
  if (set.space == null) set = { ...set, space: getSpacePtr(bench.id!) };
  if (set.space != null) {
    if (set.space.benchId != bench.id) throw new Error(`space ${set.space.id} is not in the active Bench ${bench.id}`);
    const adaptedSpace = toNodeReferenceInPackage(set.space, set.pkg);
    setSpace(adaptedSpace);
  } else {
    setSpaceToLocal();
  }
}

/** Resets current Bench/Package/Space. */
function clearBench() {
  log.trace("local.clearBench");
  if (_benchPtr.value != null) {
    _packagePtrs.value = (_packagePtrs.value ?? []).filter((p) => p.benchId != _benchPtr.value!.id);
    _benchPtr.value = null;
    setSpaceToLocal();
  }
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
  benchPtr,
  packagePtr,
  setSpace,
  setSpaceToLocal,
  getSpacePtr,
  setBench,
  clearBench,
  isDeveloperMode,
} as const;

export default local;
