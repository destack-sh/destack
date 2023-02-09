<script lang="ts" setup>
import EditableSpan from "@/components/EditableSpan.vue";
import { TypeTag, type TypeNode } from "@/gql/graphql";
import { whenever } from "@vueuse/shared";
import { ref, type Ref } from "vue";

const props = defineProps<{
  modelValue: any;
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

// focus value when we start editing
whenever(
  () => props.editing,
  () => valueRef.value?.focus()
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
    ref="containerRef"
    @click="emit('edit')"
    @keydown.enter.exact.prevent="emit('edit')"
    @keydown.left.exact.prevent="emit('navigateLeft')"
    @keydown.right.exact.prevent="emit('navigateRight')"
    @keydown.up.exact.prevent="emit('navigateUp')"
    @keydown.down.exact.prevent="emit('navigateDown')"
  >
    <!-- Actual content (may be editable if not readonly and editing) -->
    <EditableSpan
      ref="valueRef"
      v-if="type.tag == TypeTag.String"
      v-model="value"
      :readonly="!editing"
      @navigate-up="emit('navigateUp')"
      @navigate-down="emit('navigateDown')"
      @navigate-left="emit('navigateLeft')"
      @navigate-right="emit('navigateRight')"
      @delete-left="emit('deleteLeft')"
      @enter="emit('enter')"
      @escape="emit('escape')"
    />
    <div ref="valueRef" v-else class="text-red-500">barf</div>
  </component>
</template>
