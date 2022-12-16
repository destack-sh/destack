<script lang="ts" setup>
import SchemaElement from "@/components/SchemaElement.vue";
import { useFragment, type FragmentType } from "@/gql";
import { CodeContentType } from "@/utils/code";
import { SymbolContentType } from "@/utils/fragments";
import { useSchemadSymbolSchema } from "@/utils/intellisense";
import { computed } from "vue";

const props = defineProps<{
  symbol: FragmentType<typeof SymbolContentType>;
  content: FragmentType<typeof CodeContentType>;
}>();

const symbol = computed(() => useFragment(SymbolContentType, props.symbol));
const content = computed(() => useFragment(CodeContentType, props.content));

const { schema } = useSchemadSymbolSchema(symbol);
</script>
<template>
  <div class="inline-flex flex-row items-baseline gap-2">
    <SchemaElement :element="schema.element" class="text-xs opacity-50 group-hover:opacity-100" v-if="schema" />
    <span v-else class="italic text-yellow-500">no schema</span>
    <span class="text-xs text-gray-500">{{ content.builtinId || "python" }}</span>
  </div>
</template>
