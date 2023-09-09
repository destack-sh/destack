import { useDebounceFn } from "@vueuse/core";
import { ref, watchEffect, type Ref, onBeforeUnmount, watch } from "vue";

const DEFAULT_DEBOUNCE_MS = 400;
const DEFAULT_DEBOUNCE_MAX_WAIT = 2000;

export function syncProperty(property: {
  read: () => void;
  write: () => void;
  readDeps?: () => any[];
  debounceMs?: number;
  debounceMaxWait?: number;
  enabled?: Ref<boolean>;
}) {
  /* 
  Syncs a property (or set of properties) given a reactive read() function.
  Use onLocalWrite to trigger a write from this user session. 
  Use readDeps to track dependencies if read() includes dependencies that shouldn't trigger a read.
  */
  const pendingSave = ref(false);
  const destroyed = ref(false);

  onBeforeUnmount(() => {
    destroyed.value = true;
  });

  function _saveProperty() {
    if (destroyed.value || property.enabled?.value === false) return;
    property.write();
    pendingSave.value = false;
  }
  const _savePropertyDebounced = useDebounceFn(_saveProperty, property.debounceMs ?? DEFAULT_DEBOUNCE_MS, {
    maxWait: property.debounceMaxWait ?? DEFAULT_DEBOUNCE_MAX_WAIT,
  });
  function saveProperty() {
    pendingSave.value = true;
    _savePropertyDebounced();
  }

  // trigger writes manually
  function onLocalWrite() {
    if (destroyed.value || property.enabled?.value === false) return;
    saveProperty();
  }

  // sync property in whenever possible
  if (property.readDeps == null) {
    watchEffect(() => {
      if (!pendingSave.value && property.enabled?.value !== false) {
        property.read();
      }
    });
  } else {
    watch(property.readDeps, () => {
      if (!pendingSave.value && property.enabled?.value !== false) {
        property.read();
      }
    });
  }

  return {
    readNow: property.read,
    writeNow: _saveProperty,
    flushNow: () => {
      if (pendingSave.value) {
        _saveProperty();
      }
    },
    onLocalWrite,
  };
}
