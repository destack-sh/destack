import { LocalStorage, NodeType } from "@/proto/wire";
import { nodeReference, type TypedNodeReferenceData } from "@/proto/wiring";
import { getBrowserName, getBrowserVersion, getDeviceType, getOperatingSystem } from "@/utils/browser";
import { pretendReadonly, pickRef } from "@/utils/ref";
import { useLocalStorage } from "@vueuse/core";
import { v4 } from "uuid";
import { computed, shallowRef, type Ref, readonly, toRef } from "vue";

const BENCH_LOCAL_STORAGE_PREFIX = "bench-";

export const LOCAL_BENCH_ID = "00000000-0000-0000-0000-000000000000";
export const LOCAL_PACKAGE_ID = "00000000-0000-0000-0000-000000000001";
export const LOCAL_SPACE_ID = "00000000-0000-0000-0000-000000000002";

export const LOCAL_BENCH_PTR = nodeReference(NodeType.BENCH, LOCAL_BENCH_ID);
export const LOCAL_PACKAGE_PTR = nodeReference(NodeType.PACKAGE, LOCAL_PACKAGE_ID, LOCAL_BENCH_ID);
export const LOCAL_SPACE_PTR = nodeReference(NodeType.SPACE, LOCAL_SPACE_ID, LOCAL_BENCH_ID);

/**
 * Uses a value in our web-local storage.
 * Each top-level key is a separate local storage property (key = field number), the value is base64 of the message bytes.
 * All fields must be plain messages or repeated messages (for simpler encoding/decoding). :LocalStorageEncoding
 */
export function useLocal<T extends keyof LocalStorage>(key: T): Ref<LocalStorage[T] | null> {
  const field = LocalStorage.fields.find((f) => f.localName === key);
  if (field == null) throw new Error(`local field ${key} not found`);
  if (field.kind != "message") throw new Error(`local field ${key} is not a message`);
  const localKey = BENCH_LOCAL_STORAGE_PREFIX + field.no;
  const localValue = useLocalStorage<string | null>(localKey, null);

  return computed({
    get() {
      const encodedValue = localValue.value;
      if (encodedValue == null) return null;
      const bytes = Uint8Array.from(atob(encodedValue), (c) => c.charCodeAt(0));
      return field.T().fromBinary(bytes);
    },
    set(value: LocalStorage[T] | null) {
      if (value == null) {
        localValue.value = null;
      } else {
        const bytes = field.T().toBinary(value);
        localValue.value = btoa(String.fromCharCode(...bytes));
      }
    },
  });
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

const _userInfo = useLocal("userInfo");
const _clientInfo = useLocal("clientInfo");
export const userInfo = pretendReadonly(_userInfo);
export const clientInfo = pretendReadonly(_clientInfo);
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

export function setUser(info: {
  user: Required<LocalStorage>["userInfo"];
  client: Required<LocalStorage>["clientInfo"];
}) {
  _userInfo.value = info.user;
  _clientInfo.value = info.client;
}

export function clearUser() {
  _userInfo.value = null;
  _clientInfo.value = null;
}

//
// Space
//

// space/package/bench are derived from spacePtr and packagePtrs (which )
const _spacePtr = useLocal("spacePtr") as Ref<TypedNodeReferenceData<NodeType.SPACE> | null>;
const _packagePtrs = useLocal("packagePtrs") as Ref<TypedNodeReferenceData<NodeType.PACKAGE>[]>;
export const spacePtr = pretendReadonly(_spacePtr);
export const packageIdByBenchId = computed(() => {
  const packageIdByBenchId: Record<string, string> = {};
  for (const pkg of _packagePtrs.value) {
    if (packageIdByBenchId[pkg.benchId!] != null) continue; // ignore duplicates
    packageIdByBenchId[pkg.benchId!] = pkg.id!;
  }
  return packageIdByBenchId;
});
export const benchPtr = computed(() => {
  if (_spacePtr.value == null || _spacePtr.value.benchId == LOCAL_BENCH_ID) return null;
  else return nodeReference(NodeType.BENCH, _spacePtr.value.benchId!);
}) as Readonly<Ref<TypedNodeReferenceData<NodeType.BENCH> | null>>;
export const packagePtr = computed(() => {
  if (_spacePtr.value?.benchId == null || _spacePtr.value.benchId == LOCAL_BENCH_ID) return null;
  else
    return nodeReference(NodeType.PACKAGE, packageIdByBenchId.value[_spacePtr.value.benchId], _spacePtr.value.benchId);
}) as Readonly<Ref<TypedNodeReferenceData<NodeType.PACKAGE> | null>>;

/** Sets the active space. Must be local or from the current package. */
export function setSpace(space: TypedNodeReferenceData<NodeType.SPACE>) {
  if (space.benchId != LOCAL_BENCH_ID && packageIdByBenchId.value[space.benchId!] == null) {
    throw new Error(`space ${space.id} is not in the active Package ${packageIdByBenchId.value[space.benchId!]}`);
  }
  _spacePtr.value = space;
}

/** Resets the space to the local space. */
export function setSpaceToLocal() {
  _spacePtr.value = nodeReference(NodeType.SPACE, LOCAL_SPACE_ID, LOCAL_BENCH_ID);
}

/** Sets the current Bench & Package. If it doesn't match the current space, the space is reset to local. */
export function setBench(
  bench: TypedNodeReferenceData<NodeType.BENCH>,
  pkg: TypedNodeReferenceData<NodeType.PACKAGE>,
) {
  throw new Error("not implemented");
}

//
// Developer stuff
//

const developerSettings = useLocal("developerSettings");
export const isDeveloperMode = pickRef(developerSettings, "isDeveloperMode", false);
