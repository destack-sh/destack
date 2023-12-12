<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { humanizeNumber } from "@/composables/useNow";
import type { Conditional, PageInfo, Sort } from "@/gql/graphql";
import { ArrowLeftIcon, ArrowRightIcon } from "@heroicons/vue/24/solid";
import { computed, ref, watch, type Ref } from "vue";

const props = defineProps<{
  pageInfo?: PageInfo;
  totalCount?: number;
  modelValue?: string;
  query?: Conditional;
  sort?: Sort[];
}>();
const emit = defineEmits<{
  (e: "update:modelValue", value?: string): void;
}>();

const previousCursors: Ref<(string | undefined)[]> = ref([]);
const canPageBackward = computed(() => previousCursors.value.length > 0);
const canPageForward = computed(() => props.pageInfo?.hasNextPage ?? false);

function pageBackward() {
  const previousCursor = previousCursors.value.pop();
  emit("update:modelValue", previousCursor);
}

function pageForward() {
  if (!canPageForward.value) return;
  previousCursors.value.push(props.modelValue);
  emit("update:modelValue", props.pageInfo?.endCursor ?? undefined);
}

// reset cursors on query change
watch(
  () => [props.query, props.sort],
  () => {
    previousCursors.value = [];
  },
  { deep: true }
);
</script>
<template>
  <div class="flex flex-row items-center">
    <FadeTransition mode="out-in">
      <span v-if="totalCount != null" class="text-gray-400">{{ humanizeNumber(totalCount ?? 0) }}</span>
      <BusySpinnerIcon v-else class="h-4 w-4 animate-spin" />
    </FadeTransition>
    <button
      class="ml-1 rounded-sm px-1 py-1"
      :class="[
        canPageBackward
          ? 'text-gray-700 transition-colors duration-75 hover:bg-orange-100 hover:text-gray-900'
          : 'text-gray-300',
      ]"
      :disabled="!canPageBackward"
      @click="pageBackward()"
    >
      <ArrowLeftIcon class="h-4 w-4" />
    </button>
    <button
      class="rounded-sm px-1 py-1"
      :class="[
        canPageForward
          ? 'text-gray-700 transition-colors duration-75 hover:bg-orange-100 hover:text-gray-900'
          : 'text-gray-300',
      ]"
      :disabled="!canPageForward"
      @click="pageForward()"
    >
      <ArrowRightIcon class="h-4 w-4" />
    </button>
  </div>
</template>
