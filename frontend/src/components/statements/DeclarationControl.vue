<script lang="ts" setup>
import EditableSpan from "@/components/basic/EditableSpan.vue";
import { getStatementDescription, getStatementIconSolid, getStatementLabel } from "@/state/statement";
import { computed, ref, type Ref } from "vue";
import { useKeyModifier } from "@vueuse/core";
import { usePanelContext, type StatementHeader, useBenchState } from "@/state/bench";
import {
  STANDALONE_ENABLED,
  STATEMENT_INTERFACES,
  STATEMENT_RUNNABLE_TYPES,
  type StatementEmit,
  type StatementProps,
} from "@/components/statements";
import { useOperations } from "@/state/operations";
import { syncProperty } from "@/utils/sync";
import { StatementType } from "@/gql/graphql";
import { PlayIcon } from "@heroicons/vue/24/outline";

const props = defineProps<Pick<StatementProps, "statement" | "readonly">>();
const emit = defineEmits<StatementEmit>();

const bench = useBenchState();
const ops = useOperations();
const nameRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
const name: Ref<string> = ref(props.statement.name ?? "");
const nameSync = syncProperty({
  read: () => (name.value = props.statement.name ?? ""),
  write: () => ops.statement.rename(null, props.statement.id, props.statement.name ?? "", name.value),
  debounceMs: 500,
  debounceMaxWait: 2000,
});
const hasName = computed(() => name.value.trim().length > 0);
const icon = computed(() => getStatementIconSolid(props.statement.type, props.statement.tag));
const headingLevel = computed(() =>
  props.statement.type != StatementType.Text ? 0 : props.statement.headingLevel ?? 0
);
const runButtonRef = ref<HTMLButtonElement | null>(null);

const canOpenInStandaloneEditor = computed(
  () => (STANDALONE_ENABLED && STATEMENT_INTERFACES[props.statement.type]?.foldable) ?? false
);
const altKey = useKeyModifier("Alt");
const panel = usePanelContext();

function openInEditor() {
  panel.panel.value.bench.openEditStatement(props.statement as StatementHeader, { focus: true });
}

function onDeleteLeft() {
  if ((props.statement.headingLevel ?? 0) > 0) {
    // remove heading level
    ops.statement.morph(null, props.statement.id, props.statement, {
      type: (props.statement.text ?? "").length > 0 ? StatementType.Text : StatementType.Blank,
      headingLevel: null,
    });
  } else if (props.statement.type == StatementType.Text) {
    // remove name from text
    ops.statement.rename(null, props.statement.id, props.statement.name ?? "", null);
  } else {
    // actually delete
    emit("deleteLeft");
  }
}

function focus(position: "first" | "last" = "first") {
  nameRef.value?.focus("last"); // we always want the cursor at the end
}

defineExpose({
  focus,
  blur: () => {
    nameRef.value?.blur();
  },
});
</script>
<template>
  <div
    class="relative flex w-fit flex-row whitespace-nowrap"
    :class="{
      'mt-2 text-2xl': headingLevel == 1,
      'mt-1 text-xl': headingLevel == 2,
      'mt-0.5 text-lg': headingLevel == 3,
      'text-orange-600': headingLevel == 0,
      'text-gray-900': headingLevel > 0,
    }"
  >
    <!-- Icon -->
    <span class="group/icon relative mr-[18px]" v-if="headingLevel == 0">
      <component :is="icon" class="absolute top-0.5 h-4 w-4" />
      <!-- Statement explanation on hover -->
      <span
        class="pointer-events-none absolute left-full top-6 z-30 rounded-sm bg-white px-1.5 py-0.5 text-xs text-gray-500 opacity-0 ring-1 ring-orange-900 ring-opacity-[25%] transition delay-500 duration-75 hover:delay-in-500 group-hover/icon:opacity-100"
      >
        <span class="font-semibold"> {{ getStatementLabel(statement.type, statement.tag) }}</span
        >:
        {{ getStatementDescription(statement.type, statement.tag) }}
      </span>
    </span>
    <!-- Alt click to open in full -->
    <EditableSpan
      ref="nameRef"
      class="text-md flex-shrink-0 whitespace-nowrap font-semibold"
      :class="[
        altKey && canOpenInStandaloneEditor
          ? 'cursor-pointer decoration-gray-600 underline-offset-4 hover:underline'
          : 'cursor-text',
        headingLevel == 0 ? 'px-0.5' : '',
      ]"
      regex="name"
      v-model="name"
      @update:model-value="nameSync.onLocalWrite"
      :readonly="readonly"
      @click="altKey && canOpenInStandaloneEditor && openInEditor()"
      @navigate-up="emit('navigateUp')"
      @navigate-down="emit('navigateDown')"
      @navigate-left="emit('navigateLeft')"
      @navigate-right="runButtonRef != null ? runButtonRef.focus() : emit('navigateRight')"
      @enter-left="emit('enterLeft')"
      @enter-right="emit('enterRight')"
      @enter="emit('enter')"
      @delete-left="onDeleteLeft"
      @illegal="emit('illegal', $event)"
    />
    <!-- Anonymous placeholder if unnamed (as a button) -->
    <button
      tabindex="-1"
      v-if="!hasName"
      @click="nameRef?.focus()"
      class="w-fit select-none rounded-sm text-gray-300 hover:bg-orange-100 hover:text-gray-700 group-focus-within/statement:text-gray-400"
      :class="[headingLevel == 0 ? '-ml-0.5' : '']"
    >
      unnamed
    </button>
    <!-- Instant run/launch button -->
    <!-- not totally happy with this position or styling but need to make it more obvious -->
    <button
      v-if="bench.canUse && STATEMENT_RUNNABLE_TYPES.includes(statement.type)"
      ref="runButtonRef"
      class="ml-0.5 rounded-sm p-[1px] text-orange-600 hover:bg-orange-100 focus:bg-orange-100 focus:outline-none"
      @click="emit('run')"
      @keydown.enter.prevent="emit('run')"
      @keydown.left.prevent="nameRef?.focus('last')"
      @keydown.right.prevent="emit('navigateRight')"
    >
      <component :is="PlayIcon" class="h-4 w-4" />
    </button>
  </div>
</template>
