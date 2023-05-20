<script lang="ts" setup>
import { getInterface } from "@/components/cells/interfaces";
import CheckboxInterface from "@/components/cells/interfaces/CheckboxInterface.vue";
import EnumInterface from "@/components/cells/interfaces/EnumInterface.vue";
import FileInterface from "@/components/cells/interfaces/FileInterface.vue";
import NumberInterface from "@/components/cells/interfaces/NumberInterface.vue";
import RatingInterface from "@/components/cells/interfaces/RatingInterface.vue";
import SecretInterface from "@/components/cells/interfaces/SecretInterface.vue";
import ShortStringInterface from "@/components/cells/interfaces/ShortStringInterface.vue";
import StringInterface from "@/components/cells/interfaces/StringInterface.vue";
import ThumbsInterface from "@/components/cells/interfaces/ThumbsInterface.vue";
import ToggleInterface from "@/components/cells/interfaces/ToggleInterface.vue";
import type { SimpleType } from "@/components/statement";
import { pinAbsoluteElement } from "@/composables/useFixed";
import { useAppearance } from "@/state/appearance";
import { TypeFlag } from "@/state/runtime";
import { toValueRef } from "@/utils/functools";
import { syncProperty } from "@/utils/sync";
import { useElementSize } from "@vueuse/core";
import { computed, nextTick, ref, watch, type Ref } from "vue";

const INTERFACES: Record<string, any> = {
  "boolean.checkbox": CheckboxInterface,
  "boolean.toggle": ToggleInterface,
  "boolean.thumbs": ThumbsInterface,
  string: StringInterface,
  "string.short": ShortStringInterface,
  number: NumberInterface,
  "number.rating": RatingInterface,
  enum: EnumInterface,
  file: FileInterface,
  secret: SecretInterface,
};

const props = defineProps<{
  modelValue: any;
  type: SimpleType;
  readonly: boolean;
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
const editablePopoverRef: Ref<HTMLDivElement | null> = ref(null);
const editablePin = pinAbsoluteElement(editablePopoverRef, { pos: true, width: true });
const previewSize = useElementSize(previewButtonRef);
const previewSizeValue = {
  // :ReactiveGridFuckery
  width: toValueRef(previewSize.width),
  height: toValueRef(previewSize.height),
};

const valueInterface = computed(() => {
  const iface = getInterface(props.type);
  if (INTERFACES[iface?.id as string] != null) {
    return iface;
  } else if (iface != null) {
    console.log("no interface for", iface.id);
  }
  return null;
});

// debounce writes for selected interfaces (then flush on close/enter)
const debounce = props.debounced && valueInterface.value?.debounceMs != null;
const value: Ref<any> = ref<any>(props.modelValue);
const readValue = computed(() => {
  if (valueInterface.value?.read != null) {
    return valueInterface.value.read(props.type, value.value);
  } else {
    return value.value;
  }
});
function writeValue(newValue: any) {
  if (valueInterface.value?.write != null) {
    newValue = valueInterface.value.write(props.type, newValue);
  }
  value.value = newValue;
  if (!debounce) {
    emit("update:modelValue", newValue);
  }
}
let sync: any = null;
if (debounce) {
  sync = syncProperty({
    value,
    editing,
    read: () => (value.value = props.modelValue),
    write: () => emit("update:modelValue", value.value),
    debounceMs: props.debounced ? valueInterface.value?.debounceMs : 100,
  });
} else {
  watch(
    () => props.modelValue,
    (newValue) => (value.value = newValue)
  );
}

function edit() {
  if (editing.value) return;
  if (valueInterface.value == null) return;
  if (valueInterface.value.inline) {
    previewRef.value.click?.();
    previewButtonRef.value?.focus();
    return;
  }

  editing.value = true;
  emit("edit");
  nextTick(() => editableRef.value?.focus());
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

function close() {
  editing.value = false;
  sync?.flushNow();
  nextTick(() => previewButtonRef.value?.focus()); // refocus preview
}

function enter() {
  editing.value = false;
  sync?.flushNow();
  emit("navigateDown");
}

const appearance = useAppearance();
const appearanceAttrs = computed(() => {
  const classes = {
    "font-mono": appearance.fontMono,
    "text-sm": appearance.textSmall,
    "text-md": !appearance.textSmall,
  };
  return {
    class: classes,
    ...classes,
  };
});

defineExpose({
  editing,
  focus,
  blur,
  edit,
  close,
  previewSize: previewSizeValue,
});
</script>
<template>
  <!-- Value container -->
  <div class="relative" @click.stop.prevent="editing || edit()" :class="[readonly || editing ? '' : 'cursor-pointer']">
    <!-- Preview -->
    <div
      ref="previewButtonRef"
      class="mousetrap-no-tab relative inline-block w-full overflow-y-hidden text-left outline-none"
      :class="[readonly ? '' : 'cursor-pointer']"
      tabindex="-1"
      :disabled="readonly"
      @click.stop="edit"
      @keydown.enter.exact.stop.prevent="edit"
      @keydown.space.exact="edit"
      @keydown.left.exact="editing || emitPrevent($event, 'navigateLeft')"
      @keydown.right.exact="editing || emitPrevent($event, 'navigateRight')"
      @keydown.up.exact="editing || emitPrevent($event, 'navigateUp')"
      @keydown.down.exact="editing || emitPrevent($event, 'navigateDown')"
      @keydown.delete.exact.prevent.stop="editing || emit('deleteSelf')"
      @keydown="editing || previewRef?.onKeydown?.($event)"
    >
      <component
        v-if="valueInterface"
        ref="previewRef"
        :is="INTERFACES[valueInterface.id]"
        :type="type"
        :model-value="readValue"
        @update:model-value="writeValue($event)"
        :readonly="readonly"
        :active="active"
        preview
        v-bind="appearanceAttrs"
      />
      <!-- Not found (mainly for dev mode (hopefully)) -->
      <div v-else class="h-full w-full bg-red-100 text-center font-mono text-xs text-red-600">
        {{ props.type.tag }} ({{ props.type.hint }})
        <template v-if="props.type.flags & TypeFlag.IsSecret">(secret)</template>
        <template v-if="props.type.flags & TypeFlag.IsArray">(array)</template>
      </div>
    </div>
    <!-- Editable popover -->
    <!-- Popover position is pinned with fixed, see above -->
    <div
      v-if="editing && valueInterface"
      ref="editablePopoverRef"
      class="z-50 rounded-sm bg-white p-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
      :class="editablePin.pinned.value ? '' : 'absolute -left-1 -top-1 min-h-full min-w-full'"
    >
      <component
        :is="INTERFACES[valueInterface.id]"
        ref="editableRef"
        :type="type"
        :model-value="readValue"
        @update:model-value="writeValue($event)"
        :readonly="readonly"
        :preview="false"
        :active="active"
        :preview-width="previewSize.width.value"
        :preview-height="previewSize.height.value"
        v-bind="appearanceAttrs"
        @keydown.escape.exact.prevent.stop="close"
        @keydown.enter.exact.prevent.stop="enter"
        @close="close"
        @enter="enter"
      />
    </div>
    <!-- Invisible fixed overlay to prevent scrolling and capture clicks -->
    <div
      v-if="editing && valueInterface"
      class="fixed left-0 top-0 z-40 h-full w-full overscroll-none"
      @click.stop="close"
    />
  </div>
</template>
