<script lang="ts" setup>
import { useFragment, type FragmentType } from "@/gql";
import { SchemaElementContentDeepType } from "@/utils/fragments";
import { computed } from "vue";

const props = defineProps<{
  element: FragmentType<typeof SchemaElementContentDeepType>;
}>();
const element = computed(() => useFragment(SchemaElementContentDeepType, props.element));
</script>
<template>
  <span class="text-xs text-gray-700">
    {{ element.name }} <span class="text-gray-500">{{ element.type.toLowerCase() }}</span>
  </span>
</template>
