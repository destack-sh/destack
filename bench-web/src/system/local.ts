import { LocalStorage, NodeType } from "@/proto/wire";
import { nodeReference, type TypedNodeReferenceData } from "@/proto/wiring";
import { getBrowserName, getBrowserVersion, getDeviceType, getOperatingSystem } from "@/utils/client";
import { useLocalStorage } from "@vueuse/core";
import { v4 } from "uuid";
import { computed, shallowRef, type Ref } from "vue";

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

// user
export const userInfo = useLocal("userInfo");
export const clientInfo = useLocal("clientInfo");
const isOpera = !!(window as any).opera;
export const clientMeta = shallowRef({
  deviceType: getDeviceType(window.navigator.userAgent),
  operatingSystem: getOperatingSystem(window),
  browserName: getBrowserName(window.navigator.userAgent, window.navigator.vendor, isOpera),
  browserVersion: getBrowserVersion(window.navigator.userAgent, window.navigator.vendor, isOpera)?.toString(),
  nonce: v4(),
});

// space
export const spacePtr = useLocal("spacePtr") as Ref<TypedNodeReferenceData<NodeType.SPACE> | null>;
export const packagePtrs = useLocal("packagePtrs") as Ref<TypedNodeReferenceData<NodeType.PACKAGE>[]>;
export const packageIdByBenchId = computed(() => {
  const packageIdByBenchId: Record<string, string> = {};
  for (const pkg of packagePtrs.value) {
    packageIdByBenchId[pkg.benchId!] = pkg.id!;
  }
  return packageIdByBenchId;
});
export const benchPtr: Ref<TypedNodeReferenceData<NodeType.BENCH> | null> = computed(() => {
  if (spacePtr.value == null || spacePtr.value.benchId == LOCAL_BENCH_ID) return null;
  else return nodeReference(NodeType.BENCH, spacePtr.value.benchId!);
});
export const packagePtr: Ref<TypedNodeReferenceData<NodeType.PACKAGE> | null> = computed(() => {
  if (spacePtr.value?.benchId == null || spacePtr.value.benchId == LOCAL_BENCH_ID) return null;
  else return nodeReference(NodeType.PACKAGE, packageIdByBenchId.value[spacePtr.value.benchId], spacePtr.value.benchId);
});
