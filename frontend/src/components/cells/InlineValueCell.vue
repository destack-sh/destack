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
  if (!props.editing && !value.value && props.placeholderValue) {
    return props.placeholderValue;
  } else {
    return value.value;
  }
});

function writeValue(val: any) {
  value.value = val;
  emit("update:modelValue", val);
}

onClickOutside(valueRef, () => {
  if (props.editing) {
    emit("escape");
  }
});

// focus when we start/stop editing
watch(
  () => props.editing,
  () =>
    nextTick(() => {
      if (props.editing) {
        valueRef.value?.focus();
      } else {
        buttonRef.value?.focus();
      }
    })
);

defineExpose({
  focus: () => {
    if (!props.editing) {
      buttonRef.value?.focus();
    } else {
      valueRef.value?.focus();
    }
  },
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
    <!-- Editable content (takes over) -->
    <div
      class="absolute -left-0.5 -top-0.5 z-20 w-80 rounded-sm border border-solid border-black bg-orange-50 p-1"
      v-if="editing"
      @click.prevent="emit('edit')"
    >
      <!-- TODO @Incomplete: support other types -->
      <input
        :value="readValue"
        @input="(e) => writeValue(e.target?.value)"
        ref="valueRef"
        type="text"
        class="w-full min-w-0 rounded-none border-none bg-transparent p-0 text-sm outline-none ring-0 focus:ring-0"
        @keydown.escape.exact.prevent="emit('escape')"
      />
    </div>
  </div>
</template>
