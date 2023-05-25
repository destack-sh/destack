<script lang="ts" setup>
import { HandThumbDownIcon } from "@heroicons/vue/24/outline";
import { HandThumbUpIcon } from "@heroicons/vue/24/solid";
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { ref } from "vue";

const props = defineProps<{
  modelValue: boolean;
  readonly: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: boolean): void;
}>();

const buttonRef = ref<HTMLButtonElement | null>(null);

defineExpose({
  click: () => {
    if (!props.readonly) {
      emit("update:modelValue", !props.modelValue);
    }
  },
  focus: () => buttonRef.value?.focus(),
  blur: () => buttonRef.value?.blur(),
});
</script>
<template>
  <button
    ref="buttonRef"
    class="p-0.5 hover:bg-orange-100"
    :disabled="readonly"
    @click.stop="emit('update:modelValue', !modelValue)"
  >
    <FadeTransition mode="out-in">
      <component
        :is="modelValue ? HandThumbUpIcon : HandThumbDownIcon"
        class="h-4 w-4"
        :class="[!modelValue ? 'text-gray-400' : 'text-orange-600']"
      />
    </FadeTransition>
  </button>
</template>
