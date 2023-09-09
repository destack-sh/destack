import { useDebounceFn } from "@vueuse/core";
import { ref, watchEffect, type Ref, onBeforeUnmount } from "vue";

const DEFAULT_DEBOUNCE_MS = 400;
const DEFAULT_DEBOUNCE_MAX_WAIT = 2000;

export function syncProperty(property: {
  read: () => void;
  write: () => void;
  debounceMs?: number;
  debounceMaxWait?: number;
  enabled?: Ref<boolean>;
}) {
  /* 
  Syncs a property (or set of properties) given a reactive read() function.
  Use onLocalWrite to trigger a write from this user session. 
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
  watchEffect(() => {
    if (!pendingSave.value) {
      property.read();
    }
  });

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
