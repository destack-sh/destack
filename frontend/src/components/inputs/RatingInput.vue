<script lang="ts" setup>
import { StarIcon as StarOutlineIcon } from "@heroicons/vue/24/outline";
import { StarIcon } from "@heroicons/vue/24/solid";
import { ref } from "vue";
import FadeTransition from "@/components/basic/FadeTransition.vue";

const MAX_STARS = 5; // sync with minWidth in interfaces

const props = defineProps<{
  modelValue: number | null;
  readonly: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: number): void;
}>();

const containerRef = ref<HTMLElement | null>(null);

function onKeydown(e: KeyboardEvent) {
  // if e is number between 0 and MAX_STARS (dynamically)
  if (e.key.match(/^[0-9]$/) && !props.readonly && Number.parseInt(e.key) <= MAX_STARS) {
    e.preventDefault();
    e.stopPropagation();
    emit("update:modelValue", Number.parseInt(e.key));
  }
}

defineExpose({
  click: () => containerRef.value?.focus(),
  focus: () => containerRef.value?.focus(),
  blur: () => containerRef.value?.blur(),
  onKeydown,
});
</script>
<template>
  <div ref="containerRef" tabindex="-1" class="flex items-center gap-x-0.5" @keydown="onKeydown">
    <button
      v-for="i in MAX_STARS"
      :key="i"
      class="p-0.5 transition duration-200 hover:bg-orange-100 focus:outline-none"
      @click.stop="readonly || emit('update:modelValue', i)"
    >
      <FadeTransition mode="out-in">
        <component
          :is="i > (modelValue ?? 0) ? StarOutlineIcon : StarIcon"
          class="h-4 w-4"
          :class="[i > (modelValue ?? 0) ? 'text-gray-400' : 'text-orange-600']"
        />
      </FadeTransition>
    </button>
  </div>
</template>
