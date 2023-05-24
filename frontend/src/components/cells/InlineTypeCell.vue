<script lang="ts" setup>
import SelectTypeCell from "@/components/cells/SelectTypeCell.vue";
import SimpleTypePreview from "@/components/cells/SimpleTypePreview.vue";
import { ANY_TYPE_NODE, type SimpleType } from "@/components/statement";
import { pinAbsoluteElement } from "@/composables/useFixed";
import type { SimpleTypeNode } from "@/gql/graphql";
import { nextTick, ref, watch, type Ref } from "vue";

const props = defineProps<{
  modelValue?: SimpleType;
  readonly: boolean;
  inlined?: boolean;
  structrefOnly?: boolean;
  hideFlags?: boolean;
  hideIcon?: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: Pick<SimpleType, "name" | "tag" | "flags" | "reference">): void;
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "navigateLeft"): void;
  (e: "navigateRight"): void;
  (e: "deleteLeft"): void;
  (e: "deleteSelf"): void;
  (e: "enter"): void;
  (e: "escape"): void;
  (e: "focus", event: FocusEvent): void;
}>();

const value: Ref<SimpleType> = ref(props.modelValue ?? ANY_TYPE_NODE);
const editing = ref(false);

const buttonRef: Ref<HTMLButtonElement | null> = ref(null);
const valueRef: Ref<InstanceType<typeof SelectTypeCell> | null> = ref(null);
const editablePopoverRef: Ref<HTMLDivElement | null> = ref(null);
const popoverPin = pinAbsoluteElement(editablePopoverRef, { pos: true, keepInView: true });

function writeValue(type: SimpleTypeNode) {
  nextTick(() => buttonRef.value?.focus());
  // keep flags (they're configured in a separate interface)
  type = {
    ...type,
    flags: value.value.flags,
  };
  value.value = type;
  emit("update:modelValue", type);
  emit("escape");
}

// sync modelValue if changed externally
watch(
  () => [props.modelValue],
  () => {
    if (props.modelValue != value.value) {
      value.value = props.modelValue ?? ANY_TYPE_NODE;
    }
  }
);

function open() {
  if (!editing.value) {
    editing.value = true;
    nextTick(() => valueRef.value?.focus());
  }
}

function close() {
  if (editing.value) {
    editing.value = false;
    nextTick(() => buttonRef.value?.focus());
  }
}

function focus() {
  buttonRef.value?.focus();
}

function blur() {
  buttonRef.value?.blur();
}

defineExpose({
  editing,
  focus,
  blur,
});
</script>
<template>
  <div class="relative">
    <!-- Type preview -->
    <button
      ref="buttonRef"
      tabindex="-1"
      :disabled="readonly"
      @keydown.left.exact.prevent="emit('navigateLeft')"
      @keydown.right.exact.prevent="emit('navigateRight')"
      @keydown.up.exact.prevent="emit('navigateUp')"
      @keydown.down.exact.prevent="emit('navigateDown')"
      @keydown.delete.exact="editing || emit('deleteSelf')"
      class="h-full w-full text-left outline-none"
      @click="open"
      @keydown.enter.exact.prevent="open"
    >
      <SimpleTypePreview :type="value" :hide-icon="hideIcon || value.reference != null" hide-flags />
    </button>
    <!-- Prevent scroll and capture click outside -->
    <div v-if="editing" class="fixed left-0 top-0 z-40 h-full w-full overscroll-none" @click.stop="close" />
    <!-- Editable type :EditableCellStyle -->
    <div
      v-if="editing"
      ref="editablePopoverRef"
      class="absolute -left-2 z-50 flex flex-col gap-2 rounded-sm bg-white p-1 shadow-md ring-1 ring-orange-900 ring-opacity-40"
      :class="[hideFlags ? '-top-2' : '-top-10', popoverPin.pinned.value ? '' : 'absolute -left-2 -top-2']"
      @keydown.escape.exact.prevent.stop="close"
    >
      <span ref="popoverOpenRef" class="hidden" />
      <SelectTypeCell
        ref="valueRef"
        as="div"
        :model-value="value"
        @update:model-value="writeValue($event), close()"
        @escape="close"
        :inlined="inlined"
        :structref-only="structrefOnly"
        :hide-flags="hideFlags"
      />
    </div>
  </div>
</template>
