<script lang="ts" setup>
import { useFragment, type FragmentType } from "@/gql";
import { CodeContentType } from "@/state/code";
import { FileHeaderType, StatementContentType } from "@/state/fragments";
import { computed } from "vue";

const props = defineProps<{
  file: FragmentType<typeof FileHeaderType>;
  statement: FragmentType<typeof StatementContentType>;
  content: FragmentType<typeof CodeContentType>;
}>();

const file = computed(() => useFragment(FileHeaderType, props.file));
const statement = computed(() => useFragment(StatementContentType, props.statement));
const content = computed(() => useFragment(CodeContentType, props.content));
</script>
<template>
  <div class="inline-flex flex-row items-baseline gap-2 text-xs">
    <span class="text-gray-500">{{ content.builtinId || "python" }}</span>
  </div>
</template>
