<template>
  <div>
    <label
      v-if="label"
      :for="label"
      :class="{ 'block text-sm font-medium text-gray-700': true, 'sr-only': labelHidden }"
    >
      {{ label }} <span v-if="optional" class="font-normal text-gray-500">(optional)</span>
    </label>
    <div class="mt-1">
      <textarea
        :value="modelValue"
        :placeholder="placeholder"
        @input="$emit('update:modelValue', $event.target?.value)"
        :id="inputId"
        :name="inputId"
        :rows="rows || 1"
        :minlength="minlength"
        :maxlength="maxlength"
        :required="!optional"
        class="block w-full rounded-md border border-gray-300 shadow-sm focus:border-orange-500 focus:ring-orange-500 sm:text-sm"
      />
    </div>
  </div>
</template>
<script lang="ts" setup>
import { computed } from "vue";

const props = defineProps<{
  modelValue: string;
  placeholder?: string;
  label?: string;
  labelHidden?: boolean;
  id?: string;
  optional?: boolean;
  rows?: number;
  minlength?: number;
  maxlength?: number;
}>();

defineEmits<{ e: "update:modelValue"; value: string }>();

const inputId = computed(() => props.id || props.label?.toLocaleLowerCase().replace(" ", "-"));
</script>
