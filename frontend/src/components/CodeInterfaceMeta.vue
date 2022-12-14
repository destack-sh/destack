<script lang="ts" setup>
import { type FragmentType, useFragment } from "@/gql";
import { CodeContentType } from "@/utils/code";
import type { SymbolContentType } from "@/utils/fragments";
import { computed } from "vue";

const props = defineProps<{
  symbol: FragmentType<typeof SymbolContentType>;
  content: FragmentType<typeof CodeContentType>;
}>();

const content = computed(() => useFragment(CodeContentType, props.content));
</script>
<template>
  <div class="inline-flex flex-row items-baseline gap-2">
    <span class="text-xs font-bold">{{ content.builtinId || "python" }}</span>
  </div>
</template>
