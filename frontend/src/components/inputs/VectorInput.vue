<script lang="ts" setup>
const props = defineProps<{
  modelValue: number[];
  readonly: boolean;
  preview: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: number[]): void;
}>();

defineExpose({
  click: () => {
    /* noop */
  },
  focus: () => {
    /* noop */
  },
  blur: () => {
    /* noop */
  },
});
</script>
<template>
  <div class="h-full w-full" :class="modelValue.length > 0 ? 'bg-orange-100 blur-sm' : ''">
    <span v-if="modelValue?.length > 0">
      <!-- TODO @UX: render blurred colored blob with embedding as seed -->
      {{ modelValue.length }}
    </span>
    &nbsp;
  </div>
</template>
