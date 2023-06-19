<script lang="ts" setup>
import { getInterface } from "@/components/inputs";
import CheckboxInput from "@/components/inputs/CheckboxInput.vue";
import EnumInput from "@/components/inputs/EnumInput.vue";
import FileInput from "@/components/inputs/FileInput.vue";
import NumberInput from "@/components/inputs/NumberInput.vue";
import RatingInput from "@/components/inputs/RatingInput.vue";
import SecretInput from "@/components/inputs/SecretInput.vue";
import ShortStringInput from "@/components/inputs/ShortStringInput.vue";
import StringInput from "@/components/inputs/StringInput.vue";
import StructInput from "@/components/inputs/StructInput.vue";
import ThumbsInput from "@/components/inputs/ThumbsInput.vue";
import ToggleInput from "@/components/inputs/ToggleInput.vue";
import type { Field } from "@/state/statement";
import { pinAbsoluteElement } from "@/composables/useFixed";
import { useElementSize } from "@/composables/useSize";
import { useAppearance } from "@/state/appearance";
import { TypeFlag } from "@/state/module";
import { IS_DEBUG } from "@/utils/globals";
import { syncProperty } from "@/utils/sync";
import { computed, nextTick, ref, watch, type Ref } from "vue";
import VectorInput from "@/components/inputs/VectorInput.vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";

const INTERFACES: Record<string, any> = {
  "boolean.checkbox": CheckboxInput,
  "boolean.toggle": ToggleInput,
  "boolean.thumbs": ThumbsInput,
  string: StringInput,
  "string.short": ShortStringInput,
  number: NumberInput,
  "number.rating": RatingInput,
  enum: EnumInput,
  struct: StructInput,
  file: FileInput,
  secret: SecretInput,
  vector: VectorInput,
};

const props = defineProps<{
  modelValue: any;
  type: Field;
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
const editablePin = pinAbsoluteElement(editablePopoverRef, { pos: true, width: true, keepInView: true });
const previewSize = useElementSize(previewButtonRef);

const valueInterface = computed(() => {
  const iface = getInterface(props.type);
  if (INTERFACES[iface?.id as string] != null) {
    return iface;
  } else if (iface != null) {
    console.warn("no interface for", iface.id);
  }
  return null;
});

// debounce writes for selected interfaces (then flush on close/enter)
const debounce = computed(() => !props.readonly && props.debounced && valueInterface.value?.debounceMs != null);
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
  if (!debounce.value) {
    emit("update:modelValue", newValue);
  }
}
// debounced sync
const sync = syncProperty({
  value,
  editing,
  read: () => (value.value = props.modelValue),
  write: () => emit("update:modelValue", value.value),
  debounceMs: props.debounced ? valueInterface.value?.debounceMs : 100,
  enabled: debounce,
});
// immediate sync
watch(
  () => props.modelValue,
  (newValue) => {
    if (!debounce.value) {
      value.value = newValue;
    }
  }
);

function edit() {
  if (props.readonly) {
    previewButtonRef.value?.focus();
    return;
  }
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
  return {
    class: appearance.baseClass,
  };
});

defineExpose({
  editing,
  focus,
  blur,
  edit,
  close,
  value: readValue,
  previewSize,
});
</script>
<template>
  <!-- Value container -->
  <div
    class="group/iface relative"
    @click.stop.prevent="editing || (previewButtonRef?.parentNode?.contains($event.target as Node) && edit())"
    :class="[editing || readonly ? '' : 'cursor-pointer']"
  >
    <!-- Preview -->
    <div
      ref="previewButtonRef"
      class="mousetrap-no-tab scroll-hidden relative inline-block w-full text-left outline-none"
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
      <div v-else-if="IS_DEBUG" class="h-full w-full bg-red-100 text-center font-mono text-xs text-red-600">
        {{ type.tag }} ({{ type.hint }})
        <template v-if="type.flags & TypeFlag.IsSecret">(secret)</template>
        <template v-if="type.flags & TypeFlag.IsArray">(array)</template>
      </div>
      <div v-else>
        <!-- damn it -->
        &nbsp;
      </div>
    </div>
    <!-- Editable popover -->
    <!-- Popover position is pinned with fixed, see above -->
    <div
      v-if="(editing || editableRef?.pending) && valueInterface"
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
      <!-- pending indicator -->
      <span v-if="editableRef?.pending" class="absolute -right-6 top-1.5 mr-1 mt-1">
        <BusySpinnerIcon class="h-4 w-4 animate-spin text-gray-400" />
      </span>
    </div>
    <!-- Invisible fixed overlay to prevent scrolling and capture clicks -->
    <div
      v-if="editing && valueInterface"
      class="fixed left-0 top-0 z-40 h-full w-full overscroll-none"
      @click.stop="close"
    />
  </div>
</template>
