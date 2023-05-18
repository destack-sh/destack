<script lang="ts" setup>
import { getInterface } from "@/components/cells/interfaces";
import type { SimpleType } from "@/components/statement";
import { useAppearance } from "@/state/appearance";
import { computed, ref } from "vue";

import CheckboxInterface from "@/components/cells/interfaces/CheckboxInterface.vue";
import ToggleInterface from "@/components/cells/interfaces/ToggleInterface.vue";
import { onClickOutside } from "@vueuse/core";

const INTERFACES: Record<string, any> = {
  "boolean.checkbox": CheckboxInterface,
  "boolean.toggle": ToggleInterface,
};

const props = defineProps<{
  modelValue: any;
  type: SimpleType;
  readonly: boolean;
  immediate: boolean;
  active: boolean;
  debounced?: boolean;
  supportsDrop?: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: any): void;
  (e: "dropFiles", p: "above" | "below", v: File[]): void;
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "navigateLeft"): void;
  (e: "navigateRight"): void;
  (e: "deleteSelf"): void;
  (e: "enter"): void;
  (e: "escape"): void;
  (e: "edit"): void;
  (e: "focus", event: FocusEvent): void;
}>();

function emitPrevent(event: any, e: string, ...args: any[]) {
  event.preventDefault();
  emit(e, ...args);
}

const editing = ref(false);
const previewButtonRef = ref<HTMLDivElement | null>(null);
const previewRef = ref<any | null>(null);
const editableRef = ref<any | null>(null);

const valueInterface = computed(() => getInterface(props.type));
const readValue = computed(() => {
  if (valueInterface.value?.read != null) {
    return valueInterface.value.read(props.modelValue);
  } else {
    return props.modelValue;
  }
});

onClickOutside(editableRef, () => {
  editing.value = false;
});

function writeValue(value: any) {
  if (valueInterface.value?.write != null) {
    value = valueInterface.value.write(value);
  }
  emit("update:modelValue", value);
}

function edit() {
  if (valueInterface.value == null) return;
  if (valueInterface.value.inline) {
    previewRef.value.click?.();
    return;
  }

  editing.value = true;
  emit("edit");
}

function focus() {
  previewButtonRef.value?.focus();
}

function blur() {
  editing.value = false;
  previewButtonRef.value?.blur();
  previewRef.value?.blur();
  editableRef.value?.blur();
}

const appearance = useAppearance();

defineExpose({
  editing,
  focus,
  blur,
});
</script>
<template>
  <!-- Value container -->
  <div
    class="relative"
    :class="{
      'font-mono': appearance.fontMono,
      'text-sm': appearance.textSmall,
      'text-md': !appearance.textSmall,
    }"
  >
    <!-- Preview -->
    <div
      ref="previewButtonRef"
      class="mousetrap-no-tab relative h-full w-full overflow-y-hidden text-left outline-none"
      :class="[readonly ? '' : 'cursor-pointer']"
      tabindex="-1"
      :disabled="readonly"
      @click="edit"
      @keydown.enter.exact="edit"
      @keydown.space.exact="edit"
      @keydown.left.exact="editing || emitPrevent($event, 'navigateLeft')"
      @keydown.right.exact="editing || emitPrevent($event, 'navigateRight')"
      @keydown.up.exact="editing || emitPrevent($event, 'navigateUp')"
      @keydown.down.exact="editing || emitPrevent($event, 'navigateDown')"
      @keydown.backspace.exact.prevent="editing || emit('deleteSelf')"
      @keydown.delete.exact.prevent="editing || emit('deleteSelf')"
    >
      <component
        v-if="valueInterface"
        class=""
        ref="previewRef"
        :is="INTERFACES[valueInterface.id]"
        :model-value="readValue"
        @update:model-value="writeValue($event)"
        :readonly="readonly"
        :active="active"
        preview
      />
      <div v-else class="text-red-600">!!!</div>
    </div>
    <!-- Editable popover -->
    <div
      v-if="editing && valueInterface"
      ref="editableRef"
      class="absolute -left-1 -top-1 min-w-full rounded-sm bg-white p-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
    >
      <component
        :is="INTERFACES[valueInterface.id]"
        :model-value="readValue"
        @update:model-value="writeValue($event)"
        :readonly="readonly"
        :preview="false"
        :active="active"
      />
    </div>
  </div>
</template>
