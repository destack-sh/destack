<script lang="ts" setup>
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
</script>
<template>
  <div class="inline-flex flex-row items-baseline gap-2 text-xs"></div>
</template>
