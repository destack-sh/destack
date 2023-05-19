<script lang="ts" setup>
import { HandThumbDownIcon } from "@heroicons/vue/24/outline";
import { HandThumbUpIcon } from "@heroicons/vue/24/solid";
import FadeTransition from "@/components/basic/FadeTransition.vue";

const props = defineProps<{
  modelValue: boolean;
  readonly: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: boolean): void;
}>();

defineExpose({
  click: () => {
    if (!props.readonly) {
      emit("update:modelValue", !props.modelValue);
    }
  },
});
</script>
<template>
  <button class="p-0.5 hover:bg-orange-100" :disabled="readonly" @click.stop="emit('update:modelValue', !modelValue)">
    <FadeTransition mode="out-in">
      <component
        :is="modelValue ? HandThumbUpIcon : HandThumbDownIcon"
        class="h-4 w-4"
        :class="[!modelValue ? 'text-gray-400' : 'text-orange-600']"
      />
    </FadeTransition>
  </button>
</template>
