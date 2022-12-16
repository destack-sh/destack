<script lang="ts" setup>
import SchemaElement from "@/components/SchemaElement.vue";
import { useFragment, type FragmentType } from "@/gql";
import { SymbolContentType } from "@/utils/fragments";
import { useSchemadSymbolSchema } from "@/utils/intellisense";
import { computed } from "vue";

const props = defineProps<{
  symbol: FragmentType<typeof SymbolContentType>;
  content: unknown;
}>();

const symbol = computed(() => useFragment(SymbolContentType, props.symbol));
const { schema } = useSchemadSymbolSchema(symbol);
</script>
<template>
  <div class="inline-flex flex-row items-baseline gap-2 text-xs">
    <SchemaElement :element="schema.element" class="text-xs opacity-50 group-hover:opacity-100" v-if="schema" />
    <span v-else class="italic text-yellow-500">no schema</span>
  </div>
</template>
