<script lang="ts" setup>
import { useFragment, type FragmentType } from "@/gql";
import { ValueType } from "@/gql/graphql";
import { SchemaElementContentDeepType } from "@/utils/fragments";
import { ArrowRightIcon } from "@heroicons/vue/20/solid";
import { computed } from "vue";

const props = defineProps<{
  element: FragmentType<typeof SchemaElementContentDeepType>;
  omitName?: boolean;
}>();
const element = computed(() => useFragment(SchemaElementContentDeepType, props.element));
const children = computed(
  () => element.value?.elements?.map((e) => useFragment(SchemaElementContentDeepType, e)) ?? []
);

// IO schemas are objects with two children named 'input' and 'output'
const isIO = computed(
  () =>
    element.value?.type == ValueType.Object &&
    children.value.length == 2 &&
    children.value.every((e) => e.name == "input" || e.name == "output")
);
</script>
<template>
  <span class="text-gray-700">
    <template v-if="!omitName">{{ element.name }}<template v-if="element.name">: </template></template>
    <span v-if="isIO" class="inline-flex flex-row items-center gap-1">
      <SchemaElement :element="children[0]" omit-name />
      <ArrowRightIcon class="h-3 w-3 text-gray-500" />
      <SchemaElement :element="children[1]" omit-name />
    </span>
    <span class="inline-flex flex-row gap-1 text-gray-500" v-else-if="element.type == ValueType.Object">
      { <SchemaElement v-for="el in children" :element="el" :key="el.name" /> }
    </span>
    <span class="text-gray-500" v-else-if="element.type == ValueType.Array">
      [ <SchemaElement v-for="el in children" :element="el" :key="el.name" /> ]
    </span>
    <span class="text-gray-500" v-else>
      {{ element.type.toLowerCase() }}<template v-if="element.required">!</template>
    </span>
  </span>
</template>
