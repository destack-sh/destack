<script lang="ts" setup>
import SchemaElement from "@/components/SchemaElement.vue";
import { useFragment, type FragmentType } from "@/gql";
import { CodeContentType } from "@/utils/code";
import { FileHeaderType, StatementContentType } from "@/utils/fragments";
import { useSchemadSymbolSchema } from "@/utils/intellisense";
import { computed } from "vue";

const props = defineProps<{
  file: FragmentType<typeof FileHeaderType>;
  statement: FragmentType<typeof StatementContentType>;
  content: FragmentType<typeof CodeContentType>;
}>();

const file = computed(() => useFragment(FileHeaderType, props.file));
const statement = computed(() => useFragment(StatementContentType, props.statement));
const content = computed(() => useFragment(CodeContentType, props.content));

const { schema } = useSchemadSymbolSchema(file, statement);
</script>
<template>
  <div class="inline-flex flex-row items-baseline gap-2 text-xs">
    <SchemaElement :element="schema.element" class="opacity-50 group-hover:opacity-100" v-if="schema" />
    <span v-else class="italic text-yellow-500">no schema</span>
    <span class="text-gray-500">{{ content.builtinId || "python" }}</span>
  </div>
</template>
