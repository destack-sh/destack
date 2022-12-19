<script lang="ts" setup>
import SchemaElement from "@/components/SchemaElement.vue";
import { useFragment, type FragmentType } from "@/gql";
import { FileHeaderType, StatementContentType } from "@/utils/fragments";
import { useSchemadSymbolSchema } from "@/utils/intellisense";
import { computed } from "vue";

const props = defineProps<{
  file: FragmentType<typeof FileHeaderType>;
  statement: FragmentType<typeof StatementContentType>;
  content: unknown;
}>();

const file = computed(() => useFragment(FileHeaderType, props.file));
const statement = computed(() => useFragment(StatementContentType, props.statement));
const { schema } = useSchemadSymbolSchema(file, statement);
</script>
<template>
  <div class="inline-flex flex-row items-baseline gap-2 text-xs">
    <SchemaElement :element="schema.element" class="text-xs opacity-50 group-hover:opacity-100" v-if="schema" />
    <span v-else class="italic text-yellow-500">no schema</span>
  </div>
</template>
