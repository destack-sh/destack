<script lang="ts" setup>
import TypePreview from "@/components/interfaces/TypePreview.vue";
import { SortOp, type Sort } from "@/gql/graphql";
import type { Field } from "@/state/module";
import { ArrowUpIcon } from "@heroicons/vue/24/outline";

const props = defineProps<{
  field: Field;
  modelValue?: Sort;
}>();
const emit = defineEmits<{
  (e: "update:modelValue", value?: Sort): void;
}>();
</script>
<template>
  <div class="flex flex-row items-center rounded-sm border border-orange-900/[12%] px-1.5 py-0.5 hover:bg-amber-100">
    <!-- nocheckin sort -->
    <TypePreview :type="field" />
    <span class="ml-1 underline decoration-gray-300 underline-offset-4">{{ field.name }}</span>
    <ArrowUpIcon
      class="ml-0.5 h-4 w-4 transform transition-transform"
      :class="modelValue?.order == SortOp.Ascending ? 'rotate-180' : 'rotate-0'"
    />
  </div>
</template>
