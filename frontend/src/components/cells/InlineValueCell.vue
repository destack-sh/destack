<script lang="ts" setup>
import { TypeTag, type TypeNode } from "@/gql/graphql";
import { onClickOutside } from "@vueuse/core";
import { computed, nextTick, ref, watch, type Ref } from "vue";

const props = defineProps<{
  modelValue: any;
  placeholderValue?: any;
  type: TypeNode;
  readonly: boolean;
  editing: boolean;
  immediate: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: any): void;
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "navigateLeft"): void;
  (e: "navigateRight"): void;
  (e: "deleteLeft"): void;
  (e: "enter"): void;
  (e: "escape"): void;
  (e: "edit"): void;
}>();

// local copy of value
const value: Ref<any> = ref(props.modelValue);
const buttonRef: Ref<HTMLButtonElement | null> = ref(null);
const valueRef: Ref<HTMLInputElement | null> = ref(null);
const readValue = computed(() => {
  if (!props.editing && !props.modelValue && props.placeholderValue) {
    return props.placeholderValue;
  } else {
    return props.modelValue;
  }
});
function writeValue(val: any) {
  value.value = val;
  if (props.immediate) {
    emit("update:modelValue", val);
  }
}

function confirm() {
  if (!props.immediate) {
    emit("update:modelValue", value.value);
  }
  nextTick(() => emit("escape"));
}

function cancel() {
  emit("escape");
  value.value = props.modelValue;
}

onClickOutside(valueRef, () => {
  if (props.editing) {
    cancel();
  }
});

// re-focus when we start/stop editing
watch(
  () => props.editing,
  () => nextTick(focus)
);

function focus() {
  if (!props.editing) {
    buttonRef.value?.focus();
  } else {
    valueRef.value?.focus();
  }
}

defineExpose({
  focus,
  blur: () => {
    buttonRef.value?.blur();
    valueRef.value?.blur();
  },
});
</script>
<template>
  <!-- Wrapper for selectable value container -->
  <div class="relative">
    <button
      class="h-full w-full text-left outline-none outline-transparent ring-0"
      :class="readValue == placeholderValue ? 'text-gray-300' : ''"
      tabindex="-1"
      ref="buttonRef"
      @click="emit('edit')"
      @keydown.enter.exact="editing || emit('edit')"
      @keydown.left.exact="editing || emit('navigateLeft')"
      @keydown.right.exact="editing || emit('navigateRight')"
      @keydown.up.exact="editing || emit('navigateUp')"
      @keydown.down.exact="editing || emit('navigateDown')"
    >
      <!-- Content preview -->
      <!-- TODO @Incomplete: support other types -->
      <span ref="valueRef" v-if="type.tag == TypeTag.String">{{ readValue }}</span>
      <!-- Can't render this type! -->
      <span ref="valueRef" v-else class="text-red-500">{{ readValue }}</span>
    </button>
    <!-- Editable content (overlay) :EditableCellStyle -->
    <div
      class="absolute -left-0.5 -top-0.5 z-20 flex w-40 flex-row items-baseline rounded-sm border border-solid border-black bg-orange-50 p-1"
      v-if="editing"
      @click.prevent="emit('edit')"
    >
      <!-- Button to confirm if not immediate -->
      <button class="absolute bottom-1.5 right-1 text-xs text-gray-500" @click="confirm" v-if="!immediate">!</button>
      <!-- TODO @Incomplete: support other types & type constraints (e.g. length) -->
      <input
        :value="value"
        @input="(e) => writeValue(e.target?.value)"
        ref="valueRef"
        type="text"
        class="mousetrap w-full min-w-0 rounded-none border-none bg-transparent p-0 text-sm outline-none ring-0 focus:ring-0"
        @keydown.enter.exact.prevent="confirm"
        @keydown.escape.exact.prevent="cancel"
        :placeholder="placeholderValue ?? ''"
      />
    </div>
  </div>
</template>
