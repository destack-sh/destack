import { useDebounceFn } from "@vueuse/core";
import { ref, watch, watchEffect, type Ref } from "vue";

const DEFAULT_DEBOUNCE_MS = 400;
const DEFAULT_DEBOUNCE_MAX_WAIT = 2000;

export function syncProperty<T>(property: {
  value: Ref<T>;
  editing: Ref<boolean | undefined>;
  read: () => void;
  write: () => void;
  debounceMs?: number;
  debounceMaxWait?: number;
}) {
  const pendingSave = ref(false);
  function _saveProperty() {
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
  // save property while editing (write to cache/server)
  watch(
    () => [property.value],
    () => !property.editing.value || saveProperty(),
    { deep: true }
  );
  // sync property when not editing (read from cache/server)
  watchEffect(() => {
    if (!property.editing.value && !pendingSave.value) {
      property.read();
    }
  });
}
