<template>
  <input
    type="text"
    :value="text"
    @input="(event) => $emit('update:text', event.target)"
    name="email"
    class="block w-full rounded-md border-gray-300 shadow-sm focus:border-orange-500 focus:ring-orange-500 sm:text-sm"
    :placeholder="placeholder"
  />
</template>
<script lang="ts" setup>
const props = defineProps<{
  text: string;
  placeholder?: string;
}>();

defineEmits(["update:text"]);
</script>
