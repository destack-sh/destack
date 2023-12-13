<script lang="ts" setup>
import { getInputInterface } from "@/components/inputs";
import CheckboxInput from "@/components/inputs/CheckboxInput.vue";
import EnumInput from "@/components/inputs/EnumInput.vue";
import BlobInput from "@/components/inputs/BlobInput.vue";
import NumberInput from "@/components/inputs/NumberInput.vue";
import RatingInput from "@/components/inputs/RatingInput.vue";
import SecretInput from "@/components/inputs/SecretInput.vue";
import ShortStringInput from "@/components/inputs/ShortStringInput.vue";
import StringInput from "@/components/inputs/StringInput.vue";
import StructInput from "@/components/inputs/StructInput.vue";
import ThumbsInput from "@/components/inputs/ThumbsInput.vue";
import ToggleInput from "@/components/inputs/ToggleInput.vue";
import type { Field } from "@/state/module";
import { pinAbsoluteElement } from "@/composables/useFixed";
import { useElementSize } from "@/composables/useSize";
import { useAppearance } from "@/state/appearance";
import { TypeFlag } from "@/state/module";
import { IS_DEBUG } from "@/utils/globals";
import { syncProperty } from "@/utils/sync";
import { computed, nextTick, ref, watch, type Ref } from "vue";
import VectorInput from "@/components/inputs/VectorInput.vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import CodeInput from "@/components/inputs/CodeInput.vue";
import RichTextInput from "@/components/inputs/RichTextInput.vue";
import DatetimeInput from "@/components/inputs/DatetimeInput.vue";
import NodeInput from "@/components/inputs/NodeInput.vue";

const INTERFACES: Record<string, any> = {
  "boolean.checkbox": CheckboxInput,
  "boolean.toggle": ToggleInput,
  "boolean.thumbs": ThumbsInput,
  string: StringInput,
  "string.code": CodeInput,
  "string.short": ShortStringInput,
  "string.datetime": DatetimeInput,
  "string.rich": RichTextInput,
  number: NumberInput,
  "number.rating": RatingInput,
  enum: EnumInput,
  struct: StructInput,
  blob: BlobInput,
  secret: SecretInput,
  vector: VectorInput,
  node: NodeInput,
};

const props = defineProps<{
  modelValue: unknown;
  type: Field;
  readonly?: boolean;
  active?: boolean;
  debounced?: boolean;
  supportsDrop?: boolean;
  wrap?: boolean;
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
  (e: "focus"): void;
}>();

function emitPrevent(event: any, e: string, ...args: any[]) {
  event.preventDefault();
  emit(e as any, ...(args as []));
}

const editing = ref(false);
const previewButtonRef = ref<HTMLDivElement | null>(null);
const previewRef = ref<any | null>(null);
const editableRef = ref<any | null>(null);
const editablePopoverRef: Ref<HTMLDivElement | null> = ref(null);
const editablePin = pinAbsoluteElement(editablePopoverRef, { pos: true, width: true, keepInView: true });
const previewSize = useElementSize(previewButtonRef);

const valueInterface = computed(() => {
  const iface = getInputInterface(props.type);
  if (INTERFACES[iface?.id as string] != null) {
    return iface;
  } else if (iface != null) {
    console.warn("no interface for", iface.id);
  }
  return null;
});

// debounce writes for selected interfaces (then flush on close/enter)
const debounce = computed(() => !props.readonly && props.debounced && valueInterface.value?.debounceMs != null);
const value: Ref<unknown> = ref<unknown>(props.modelValue);
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
  sync.onLocalWrite();
  if (!debounce.value) {
    emit("update:modelValue", newValue);
  }
}
// debounced sync
const sync = syncProperty({
  readDeps: () => [props.modelValue],
  read: () => {
    value.value = props.modelValue;
  },
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
  editableRef.value?.flush?.();
  sync?.flushNow();
  nextTick(() => {
    editing.value = false;
    previewButtonRef.value?.focus();
  }); // refocus preview
}

function enter() {
  editableRef.value?.flush?.();
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
  <!-- eslint-disable vue/use-v-on-exact -->
  <div
    class="group/iface relative"
    @click.stop.prevent="editing || (previewButtonRef?.parentNode?.contains($event.target as Node) && edit())"
    :class="[editing || readonly ? '' : 'cursor-pointer']"
  >
    <!-- Preview -->
    <div
      ref="previewButtonRef"
      class="mousetrap-no-tab scroll-hidden relative inline-block w-full text-left text-gray-900 outline-none"
      :class="[readonly ? '' : 'cursor-pointer']"
      tabindex="-1"
      :disabled="readonly"
      @click.stop="edit(), emit('focus')"
      @keydown="editing || previewRef?.onKeydown?.($event)"
      @keydown.enter.exact.stop.prevent="edit"
      @keydown.space.exact="edit"
      @keydown.left.exact="editing || emitPrevent($event, 'navigateLeft')"
      @keydown.right.exact="editing || emitPrevent($event, 'navigateRight')"
      @keydown.tab.exact="editing || emitPrevent($event, 'navigateRight')"
      @keydown.shift.tab.exact="editing || emitPrevent($event, 'navigateLeft')"
      @keydown.up.exact="editing || emitPrevent($event, 'navigateUp')"
      @keydown.down.exact="editing || emitPrevent($event, 'navigateDown')"
      @keydown.delete.exact.prevent.stop="editing || emit('deleteSelf')"
      v-bind="appearanceAttrs"
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
        :wrap="props.wrap"
        preview
      />
      <!-- Not found -->
      <div v-else-if="IS_DEBUG" class="h-full w-full bg-red-100 text-center font-mono text-xs text-red-600">
        {{ type.tag }}
        <template v-if="type.hint">({{ type.hint }})</template>
        <template v-if="type.referenceCk">({{ type.referenceCk }})</template>
        <template v-if="type.flags & TypeFlag.IS_SECRET">(secret)</template>
        <template v-if="type.flags & TypeFlag.IS_ARRAY">(array)</template>
      </div>
      <div v-else class="h-full w-full text-center text-red-600">??? &nbsp;</div>
    </div>
    <!-- Editable popover -->
    <!-- Popover position is pinned with fixed, see above -->
    <div
      v-if="(editing || editableRef?.pending) && valueInterface"
      ref="editablePopoverRef"
      class="z-50 rounded-sm bg-white p-2 text-gray-900 shadow-md ring-1 ring-orange-900 ring-opacity-40"
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
        @keydown.escape.exact.prevent.stop="close"
        @keydown.enter.exact.prevent.stop="enter"
        @keydown.tab.exact.prevent.stop="close(), $nextTick(() => emit('navigateRight'))"
        @keydown.shift.tab.exact.prevent.stop="close(), $nextTick(() => emit('navigateLeft'))"
        @close="close"
        @enter="enter"
        v-bind="appearanceAttrs"
        :style="{ maxHeight: '600px' } /* unfortunately hard-coded for now.. should be panel height? */"
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
