<script lang="ts" setup>
import { Switch } from "@headlessui/vue";

defineProps<{ modelValue: boolean }>();
const emit = defineEmits<{ (e: "update:modelValue", value: boolean): void }>();
</script>
<template>
  <Switch
    :model-value="modelValue"
    @update:model-value="emit('update:modelValue', $event)"
    :class="[
      modelValue ? 'bg-orange-600' : 'bg-gray-200',
      'relative inline-flex h-4 w-9 flex-shrink-0 cursor-pointer rounded-sm border-2 border-transparent ring-0 transition-colors duration-100 ease-in-out',
    ]"
  >
    <span
      aria-hidden="true"
      :class="[
        modelValue ? 'translate-x-5' : 'translate-x-0',
        'pointer-events-none inline-block h-3 w-3 transform rounded-sm bg-white shadow ring-0 transition duration-100 ease-in-out',
      ]"
    />
  </Switch>
</template>
