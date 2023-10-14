<script lang="ts" setup>
import { Switch } from "@headlessui/vue";

defineProps<{ modelValue: boolean; disabled?: boolean; xs?: boolean }>();
const emit = defineEmits<{ (e: "update:modelValue", value: boolean): void }>();
</script>
<template>
  <Switch
    :model-value="modelValue"
    @update:model-value="emit('update:modelValue', $event)"
    :class="[
      modelValue ? 'bg-orange-600' : 'bg-gray-200',
      'relative inline-flex  flex-shrink-0 cursor-pointer rounded-sm border-2 border-transparent ring-0 transition-colors duration-100 ease-in-out',
      disabled ? 'cursor-not-allowed opacity-50' : '',
      xs ? 'h-3 w-9' : 'h-4 w-9',
    ]"
    :disabled="disabled"
  >
    <span
      aria-hidden="true"
      :class="[
        modelValue ? (xs ? 'translate-x-6' : 'translate-x-5') : 'translate-x-0',
        'pointer-events-none inline-block  transform rounded-sm bg-white shadow ring-0 transition duration-100 ease-in-out',
        xs ? 'h-2 w-2' : 'h-3 w-3',
      ]"
    />
  </Switch>
</template>
