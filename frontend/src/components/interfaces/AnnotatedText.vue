<script lang="ts" setup>
import EditableSpan from "@/components/basic/EditableSpan.vue";
import { computed, ref, type Ref } from "vue";

const props = defineProps<{
  modelValue: string;
  readonly: boolean;
  suppressAllShortcuts?: boolean;
  supportedAnnotations?: string[];
}>();

const emit = defineEmits<{
  (e: "update:modelValue", v: string): void;
  (e: "navigateUp", position?: number): void;
  (e: "navigateDown", position?: number): void;
  (e: "navigateLeft"): void;
  (e: "navigateRight"): void;
  (e: "enterLeft"): void;
  (e: "enter"): void;
  (e: "enterRight"): void;
  (e: "escape"): void;
  (e: "deleteLeft"): void;
  (e: "illegal", char: string): void;
}>();

const spanRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);

defineExpose({
  focus: (pos: "first" | "last" = "first") => spanRef.value?.focus(pos),
  blur: () => spanRef.value?.blur(),
  selectAll: () => spanRef.value?.selectAll(),
  focused: computed(() => spanRef.value?.focused),
});
</script>
<template>
  <!-- placeholder passthrough until we get actual annotated text, see :BE-301 -->
  <EditableSpan
    ref="spanRef"
    :model-value="modelValue"
    @update:model-value="emit('update:modelValue', $event)"
    :readonly="readonly"
    @navigate-left="emit('navigateLeft')"
    @navigate-right="emit('navigateRight')"
    @navigate-up="emit('navigateUp')"
    @navigate-down="emit('navigateDown')"
    @enter-left="emit('enterLeft')"
    @enter="emit('enter')"
    @enter-right="emit('enterRight')"
    @delete-left="emit('deleteLeft')"
    @escape="emit('escape')"
    @illegal="emit('illegal', $event)"
  />
</template>
