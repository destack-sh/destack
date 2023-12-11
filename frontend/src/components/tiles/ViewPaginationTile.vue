<script lang="ts" setup>
import { humanizeNumber } from "@/composables/useNow";
import type { PageInfo } from "@/gql/graphql";
import { ArrowLeftIcon, ArrowRightIcon } from "@heroicons/vue/24/solid";
import { computed, ref, type Ref } from "vue";

const props = defineProps<{ pageInfo?: PageInfo; totalCount?: number; modelValue?: string }>();
const emit = defineEmits<{
  (e: "update:modelValue", value?: string): void;
}>();

const previousCursors: Ref<string[]> = ref([]);
const canPageBackward = computed(() => previousCursors.value.length > 0);
const canPageForward = computed(() => props.pageInfo?.hasNextPage ?? false);

function pageBackward() {
  const previousCursor = previousCursors.value.pop();
  emit("update:modelValue", previousCursor);
}

function pageForward() {
  if (!canPageForward.value) return;
  previousCursors.value.push(props.pageInfo?.startCursor ?? "");
  emit("update:modelValue", props.pageInfo?.endCursor ?? undefined);
}
</script>
<template>
  <div class="flex flex-row items-center" v-if="totalCount != null">
    <span class="text-gray-400">{{ humanizeNumber(totalCount) }}</span>
    <button
      class="ml-1 rounded-sm px-1 py-1"
      :class="[canPageBackward ? 'text-gray-700 hover:bg-orange-100 hover:text-gray-900' : 'text-gray-300']"
      :disabled="!canPageBackward"
      @click="pageBackward()"
    >
      <ArrowLeftIcon class="h-4 w-4" />
    </button>
    <button
      class="rounded-sm px-1 py-1"
      :class="[canPageForward ? 'text-gray-700 hover:bg-orange-100 hover:text-gray-900' : 'text-gray-300']"
      :disabled="!canPageForward"
      @click="pageForward()"
    >
      <ArrowRightIcon class="h-4 w-4" />
    </button>
  </div>
</template>
