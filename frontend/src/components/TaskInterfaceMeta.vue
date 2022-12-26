<script lang="ts" setup>
import SchemaElement from "@/components/SchemaElement.vue";
import { useFragment, type FragmentType } from "@/gql";
import { FileHeaderType, StatementContentType } from "@/state/fragments";
import { useSchemadSymbolSchema } from "@/state/intellisense";
import { computed } from "vue";

const props = defineProps<{
  file: FragmentType<typeof FileHeaderType>;
  statement: FragmentType<typeof StatementContentType>;
  content: unknown;
}>();

const file = computed(() => useFragment(FileHeaderType, props.file));
const statement = computed(() => useFragment(StatementContentType, props.statement));
const { schemaContent } = useSchemadSymbolSchema(file, statement);
</script>
<template>
  <div class="inline-flex flex-row items-baseline gap-2 text-xs">
    <SchemaElement
      :element="schemaContent?.element"
      class="text-xs opacity-50 group-hover:opacity-100"
      v-if="schemaContent"
    />
    <span v-else class="italic text-yellow-500">no schema</span>
  </div>
</template>
