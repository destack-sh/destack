<script lang="ts" setup>
import { useFragment, type FragmentType } from "@/gql";
import { ValueType } from "@/gql/graphql";
import { SchemaElementContentDeepType } from "@/utils/fragments";
import { computed } from "vue";

const props = defineProps<{
  element: FragmentType<typeof SchemaElementContentDeepType>;
}>();
const element = computed(() => useFragment(SchemaElementContentDeepType, props.element));
const children = computed(
  () => element.value?.elements?.map((e) => useFragment(SchemaElementContentDeepType, e)) ?? []
);
</script>
<template>
  <span class="text-xs text-gray-700">
    {{ element.name }}
    <span class="text-gray-500" v-if="element.type == ValueType.Object">
      { <SchemaElement v-for="el in children" :element="el" :key="el.name" /> }
    </span>
    <span class="text-gray-500" v-else-if="element.type == ValueType.Array">
      [ <SchemaElement v-for="el in children" :element="el" :key="el.name" /> ]
    </span>
    <span class="text-gray-500" v-else>
      {{ element.type.toLowerCase() }}
    </span>
  </span>
</template>
