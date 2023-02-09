<script lang="ts" setup>
import EditableSpan from "@/components/EditableSpan.vue";
import { TypeTag, type TypeNode } from "@/gql/graphql";
import { whenever } from "@vueuse/shared";
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
const containerRef: Ref<HTMLButtonElement | null> = ref(null);
const valueRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
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

// focus when we start/stop editing
watch(
  () => props.editing,
  () =>
    nextTick(() => {
      if (props.editing) {
        valueRef.value?.focus();
      } else {
        containerRef.value?.focus();
      }
    })
);

defineExpose({
  focus: () => {
    if (!props.editing) {
      containerRef.value?.focus();
    } else {
      valueRef.value?.focus();
    }
  },
  defocus: () => {
    containerRef.value?.blur();
    valueRef.value?.defocus();
  },
});
</script>
<template>
  <!-- Wrapper for selectable value container -->
  <component
    :is="editing ? 'div' : 'button'"
    class="z-10 text-left outline-transparent"
    :class="readValue == placeholderValue ? 'text-gray-300' : ''"
    ref="containerRef"
    @click.capture.prevent="
      emit('edit');
      valueRef?.focus();
    "
    @keydown.enter.exact="editing || emit('edit')"
    @keydown.left.exact="editing || emit('navigateLeft')"
    @keydown.right.exact="editing || emit('navigateRight')"
    @keydown.up.exact="editing || emit('navigateUp')"
    @keydown.down.exact="editing || emit('navigateDown')"
  >
    <!-- Actual content (may be editable if not readonly and editing) -->
    <EditableSpan
      suppress-shortcuts
      ref="valueRef"
      v-if="type.tag == TypeTag.String"
      :model-value="readValue"
      @update:model-value="writeValue"
      :readonly="!editing"
      @navigate-up="emit('navigateUp')"
      @navigate-down="emit('navigateDown')"
      @navigate-left="emit('navigateLeft')"
      @navigate-right="emit('navigateRight')"
      @enter="emit('enter')"
      @escape="emit('escape')"
    />
    <!-- TODO @Incomplete: support other data -->
    <!-- Can't render this type! -->
    <div ref="valueRef" v-else class="text-red-500">{{ value }}</div>
  </component>
</template>
