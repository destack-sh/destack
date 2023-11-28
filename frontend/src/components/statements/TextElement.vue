<script lang="ts" setup>
import AnnotatedText from "@/components/interfaces/AnnotatedText.vue";
import type { StatementEmit, StatementProps } from "@/components/statements";
import { useElementRefs } from "@/composables/useGrid";
import { useNow } from "@/composables/useNow";
import { StatementType } from "@/gql/graphql";
import { useOperations } from "@/state/operations";
import { newDynamicNodeKey } from "@/state/operations/statement";
import { getStatementIconSolid } from "@/state/statement";
import { syncProperty } from "@/utils/sync";
import { SparklesIcon } from "@heroicons/vue/24/solid";
import { AtSymbolIcon } from "@heroicons/vue/24/solid";
import { DateTime } from "luxon";
import { computed, nextTick, ref, watch, type Ref, type Component } from "vue";

const props = defineProps<Pick<StatementProps, "statement" | "readonly" | "focused" | "editing">>();
const emit = defineEmits<StatementEmit>();

const ops = useOperations();
const textRef: Ref<InstanceType<typeof AnnotatedText> | null> = ref(null);
const text: Ref<string> = ref(props.statement.text ?? "");
const textSync = syncProperty({
  read: () => (text.value = props.statement.text ?? ""),
  write: () => ops.symbol.updateStatementText(null, props.statement.id, props.statement.text ?? "", text.value),
  debounceMs: 500,
  debounceMaxWait: 3000,
});
const headingLevel = computed(() => props.statement.headingLevel ?? 0);

function focus(position: "first" | "last" = "first") {
  textRef.value?.focus("last"); // we always want to focus the end of the text
}

function blur() {
  textRef.value?.blur();
  quickActionsRefs.refs.value.forEach((ref) => ref?.blur());
}

function onInput() {
  textSync.onLocalWrite();
  // if text statement: morph to blank if empty
  // (only if we are the ones who caused the change)
  if (
    props.focused &&
    !props.readonly &&
    props.statement.type == StatementType.Text &&
    headingLevel.value == 0 &&
    text.value == ""
  ) {
    // should ideally be done in one tx, but we don't have that yet
    textSync.flushNow();
    ops.statement.morph(null, props.statement.id, props.statement, {
      type: StatementType.Blank,
      headingLevel: null,
    });
  }
}

function onDeleteLeft() {
  if (headingLevel.value > 0) {
    // remove heading level
    ops.statement.morph(null, props.statement.id, props.statement, {
      type: (text.value ?? "").length > 0 ? StatementType.Text : StatementType.Blank,
      headingLevel: null,
    });
  } else {
    // actually delete
    emit("deleteLeft");
  }
}

// quick inline actions
type QuickAction = {
  id: string;
  label: string;
  icon: Component;
  action: () => void;
  description?: string;
  fat?: boolean;
  highlight?: boolean;
};
const now = useNow(10000);
const quickActions = computed(() => {
  if (
    !props.focused ||
    props.statement.type != StatementType.Text ||
    props.statement.name != null ||
    props.readonly ||
    textRef.value?.open
  )
    return [];
  const actions: QuickAction[] = [];
  actions.push({
    id: "implement",
    label: "Assist",
    description: "Draft this for me",
    icon: SparklesIcon,
    fat: false, // too distracting
    action: () => {
      emit("launchAssist", "Continue from here");
    },
    // highlight if statement was just created (<1min ago)
    highlight:
      props.statement.createdAt != null &&
      now.value.diff(DateTime.fromISO(props.statement.createdAt)).as("minutes") < 10,
  });
  if ((props.statement.headingLevel ?? 0) > 0) {
    return actions; // no other actions for headings
  }
  if (props.statement.name == null) {
    actions.push({
      id: "name",
      label: "Add name",
      icon: AtSymbolIcon,
      action: () => {
        ops.statement.rename(null, props.statement.id, null, "");
        nextTick(() => emit("focus", "declaration"));
      },
    });
  }
  actions.push({
    id: "task",
    label: "Turn into task",
    icon: getStatementIconSolid(StatementType.Task),
    action: () => ops.statement.morph(null, props.statement.id, props.statement, { type: StatementType.Task }),
  });
  actions.push({
    id: "code",
    label: "Turn into code",
    icon: getStatementIconSolid(StatementType.Code),
    action: () => {
      ops.statement.morph(null, props.statement.id, props.statement, { type: StatementType.Code });
      nextTick(() => emit("focus", "code"));
    },
  });
  actions.push({
    id: "data",
    label: "Turn into database",
    icon: getStatementIconSolid(StatementType.Database),
    action: () => {
      ops.statement.morph(null, props.statement.id, props.statement, {
        type: StatementType.Database,
        versioned: true,
        key: newDynamicNodeKey(props.statement.id),
      });
      nextTick(() => emit("focus", "database"));
    },
  });

  return actions;
});
const quickActionsRefs = useElementRefs<HTMLButtonElement>(quickActions, {
  navigateLeft: () => textRef.value?.focus("last"),
  navigateRight: () => emit("navigateRight"), // unsure whether we should loop or not
});

function focusAction(id: string | number) {
  quickActionsRefs.focus(id);
}

defineExpose({
  focus,
  blur,
  syncNow: () => textSync.flushNow(),
});
</script>
<template>
  <div
    class="relative w-full text-gray-900"
    :class="{
      'mt-2 text-2xl': headingLevel == 1,
      'mt-1 text-xl': headingLevel == 2,
      'mt-0.5 text-lg': headingLevel == 3,
      'font-semibold': headingLevel > 0,
    }"
    @click="textRef?.focusIfUnfocused"
  >
    <!-- Actual text -->
    <AnnotatedText
      ref="textRef"
      :model-value="text || ''"
      @update:model-value="(text = $event), onInput()"
      @navigate-up="emit('navigateUp')"
      @navigate-down="emit('navigateDown')"
      @navigate-left="emit('navigateLeft')"
      @navigate-right="quickActions.length > 0 ? quickActionsRefs.focus(quickActions[0].id) : emit('navigateRight')"
      @enter-left="emit('enterLeft')"
      @enter="emit('enter')"
      @enter-right="emit('enterRight')"
      @delete-left="onDeleteLeft"
      @delete-if-empty="emit('deleteSelf')"
      @paste="emit('paste')"
      :focused="focused"
      :readonly="readonly"
      :statement="statement"
      :minimal-mentions="headingLevel > 0"
    />
    <!-- Placeholder if empty -->
    <template v-if="text.length == 0">&nbsp;</template>
    <button
      v-if="text.length == 0"
      class="absolute bottom-0 left-0 -m-0.5 -mx-0.5 flex flex-row items-center rounded-sm p-0.5 transition-colors duration-75 hover:bg-orange-100"
      :class="[focused ? 'text-gray-400' : 'text-gray-300']"
      @click="textRef?.focus"
    >
      Text...
    </button>
    <!-- Quick inline actions (positioned as not to disturb the flow) -->
    <!-- TODO @UX: inline actions don't wrap properly when text overflows -->
    <div v-if="quickActions.length > 0 && focused && editing" class="relative inline-block">
      <div
        class="absolute -bottom-[7px] z-20 flex animate-fadein-1000 flex-row gap-1.5 whitespace-nowrap text-sm font-normal transition-opacity"
        :class="[text.length == 0 ? 'ml-24' : 'ml-3' /* for 'type for text...' */]"
      >
        <button
          v-for="action in quickActions"
          :key="action.id"
          :ref="(ref: any) => quickActionsRefs.registerRef(action.id, ref)"
          class="group relative flex h-fit max-h-fit flex-row rounded-sm px-1 py-0.5 transition-colors duration-150 focus:outline-none"
          :class="[
            action.fat
              ? ''
              : 'text-gray-300 hover:bg-orange-100 hover:text-gray-500 focus:bg-orange-100 focus:text-gray-500',
            action.fat && action.highlight
              ? 'bg-orange-600 text-white ring-orange-600 hover:bg-orange-500 focus:bg-orange-500 '
              : '',
            action.fat && !action.highlight ? 'bg-gray-100 text-gray-400 hover:bg-orange-100 focus:bg-orange-100' : '',
          ]"
          @click="action.action"
          @keydown.right.stop.prevent="quickActionsRefs.navigateRight(action.id)"
          @keydown.left.stop.prevent="quickActionsRefs.navigateLeft(action.id)"
        >
          <span class="mt-0.5">
            <component :is="action.icon" class="h-4 w-4 transition-colors duration-150" />
          </span>
          <span v-if="action.fat" class="ml-1">
            {{ action.label }}
          </span>
          <!-- Label popover -->
          <span
            class="pointer-events-none absolute -left-1/2 top-6 z-10 whitespace-nowrap rounded-sm border border-orange-900 border-opacity-[15%] bg-white px-2 py-0.5 text-center text-xs text-gray-700 opacity-0 transition duration-150 group-focus-within:opacity-100 group-hover:opacity-100"
          >
            {{ action.description ?? action.label }}
          </span>
        </button>
      </div>
    </div>
  </div>
</template>
