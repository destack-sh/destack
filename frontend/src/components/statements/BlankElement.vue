<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import EditableSpan from "@/components/basic/EditableSpan.vue";
import { TypeTag } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import { Combobox, ComboboxOption, ComboboxInput, ComboboxOptions, ComboboxButton } from "@headlessui/vue";
import { useFocus, type MaybeElementRef } from "@vueuse/core";
import { computed, nextTick, ref, watch, type Ref } from "vue";
import { StatementType } from "@/gql/graphql";
import { getStatementLabel, getStatementDescription, getStatementIconSolid } from "@/state/statement";
import { EllipsisHorizontalIcon } from "@heroicons/vue/24/outline";
import { useActiveScroll } from "@/composables/useScroll";
import { usePanelContext } from "@/state/bench";
import { type StatementProps } from "@/components/statements";
import type { StatementEmit } from "@/components/statements";
import type { TypeFlag } from "@/state/module";
import { closeTransaction, openTransaction, useOperations } from "@/state/operations";

const props = defineProps<Pick<StatementProps, "statement" | "readonly" | "bounding" | "focused" | "editing">>();
const emit = defineEmits<StatementEmit>();

const query: Ref<string> = ref("");
const spanRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
const panel = usePanelContext();
const ops = useOperations();

// open/close commanding and auto-convert to text on anything else
watch(query, (query) => {
  if (query.trim() == "") {
    commanding.value = false;
  } else if (query == "/") {
    openCommandSelection();
  } else if (!commanding.value) {
    const tx = openTransaction();
    ops.statement.morph(tx, props.statement.id, props.statement, { type: StatementType.Text });
    ops.symbol.updateStatementText(tx, props.statement.id, "", query);
    closeTransaction(tx);
  }
});

// command selection dropdown
// (not sure if this is the best place to put it)
const commanding: Ref<boolean> = ref(false);
const commandQuery: Ref<string> = ref("");
const commandInputRef: Ref<InstanceType<typeof ComboboxInput> | null> = ref(null);
const commandButtonRef: Ref<InstanceType<typeof ComboboxButton> | null> = ref(null);
const commandOptionsRef: Ref<InstanceType<typeof ComboboxOptions> | null> = ref(null);
const { focused: commandInputRefFocused } = useFocus(commandInputRef as MaybeElementRef);

useActiveScroll(computed(() => commandOptionsRef.value?.$el));

type Group = {
  name: string;
};

const GROUPS = {
  BASIC: { name: "Basic statements" },
  LAYOUT: { name: "Layout statements" },
  ADVANCED: { name: "Advanced statements" },
};

type Command = {
  group: Group;
  label: string;
  icon: any;
  description: string;
  aliases?: string[];
  action: () => void;
};

function simpleStatementCommand(
  group: Group,
  type: StatementType,
  options?: {
    tag?: TypeTag;
    flags?: TypeFlag;
    headingLevel?: number;
    icon?: any;
    label?: string;
    description?: string;
    aliases?: string[];
  }
): Command {
  return {
    group,
    label: options?.label ?? getStatementLabel(type, options?.tag),
    icon: options?.icon ?? getStatementIconSolid(type, options?.tag),
    description: options?.description ?? getStatementDescription(type, options?.tag),
    aliases: options?.aliases,
    action: () =>
      ops.statement.morph(null, props.statement.id, props.statement, {
        type,
        tag: options?.tag,
        flags: options?.flags,
        headingLevel: options?.headingLevel,
      }),
  };
}

function morphToText() {
  query.value = "";
  nextTick(() => spanRef.value?.focus());
}

// TODO @UX: blank statement menu sucks
const commands = computed(() => {
  const commands: Command[] = [
    // basic statements
    {
      group: GROUPS.BASIC,
      label: "Text",
      aliases: ["comment", "markdown", "title", "header"],
      icon: getStatementIconSolid(StatementType.Text),
      description: "Just type for a plain comment",
      action: morphToText,
    },
    simpleStatementCommand(GROUPS.BASIC, StatementType.Type, {
      tag: TypeTag.Struct,
      aliases: ["type", "struct"],
    }),
    simpleStatementCommand(GROUPS.BASIC, StatementType.Type, { tag: TypeTag.Enum, aliases: ["type", "enum"] }),
    simpleStatementCommand(GROUPS.BASIC, StatementType.Dataset, { aliases: ["table", "retrieval", "rag", "samples"] }),
    simpleStatementCommand(GROUPS.BASIC, StatementType.Code),
    simpleStatementCommand(GROUPS.BASIC, StatementType.Task, { aliases: ["prompt", "AI", "model", "bot"] }),

    // layout statements
    ...[1, 2, 3].map((level) =>
      simpleStatementCommand(GROUPS.LAYOUT, StatementType.Text, {
        headingLevel: level,
        label: `Heading ${level}`,
        description: `Text with heading ${level}`,
        aliases: [`h${level}`],
      })
    ),

    // advanced statements
    simpleStatementCommand(GROUPS.ADVANCED, StatementType.Variable, { aliases: ["const", "config", "secret"] }),
    // singleStatementCommand(GROUPS.ADVANCED, StatementType.Flow), not fully implemented
    simpleStatementCommand(GROUPS.ADVANCED, StatementType.Tag),
    simpleStatementCommand(GROUPS.ADVANCED, StatementType.Reference),
  ];

  return commands;
});

const filteredCommands = computed(() => {
  return commands.value
    .map((command) => {
      const titleMatch = command.label.toLowerCase().includes(commandQuery.value.toLowerCase());
      const aliasMatch = command.aliases?.some((alias) =>
        alias.toLowerCase().includes(commandQuery.value.toLowerCase())
      );
      const descriptionMatch = command.description.toLowerCase().includes(commandQuery.value.toLowerCase());

      return {
        ...command,
        score: titleMatch ? 1 : aliasMatch ? 0.5 : descriptionMatch ? 0.25 : 0,
      };
    })
    .filter((c) => c.score > 0)
    .sort((a, b) => b.score - a.score);
});

function openCommandSelection() {
  commanding.value = true;
  nextTick(() => ((commandInputRefFocused.value = true), commandButtonRef.value?.$el.click()));
}

function stopCommanding() {
  commanding.value = false;
  query.value = "";
  nextTick(() => spanRef.value?.focus());
}

function selectCommand(command: Command) {
  commanding.value = false;
  query.value = "";
  command.action();
}

function focus() {
  spanRef.value?.focus();
  commanding.value = false;
}

function blur() {
  spanRef.value?.blur();
  commandInputRefFocused.value = false;
  commanding.value = false;
}

const appearance = useAppearance();
const isInTopHalfOfPanel = computed(() => props.bounding.y.value < panel.size.value.height / 2);

defineExpose({
  focus: (position: "first" | "last" = "first") => focus(),
  blur,
  loading: ref(false),
});
</script>
<template>
  <div class="flex w-full flex-row items-center outline-none" @click="spanRef?.focus()">
    <EditableSpan
      ref="spanRef"
      v-if="!commanding"
      v-model="query"
      :readonly="readonly"
      @navigate-up="emit('navigateUp')"
      @navigate-down="emit('navigateDown')"
      @navigate-left="emit('navigateLeft')"
      @navigate-right="emit('navigateRight')"
      @enter="emit('enter')"
      @escape="emit('escape')"
      @delete-left="emit('deleteLeft')"
      @paste.prevent="emit('paste')"
    />
    <!-- Empty dots / prompt -->
    <div v-if="focused && !commanding" class="h-full w-full select-none items-center group-hover:opacity-100">
      <span class="text-gray-400" v-if="!editing"><EllipsisHorizontalIcon class="h-4 w-4" /></span>
      <span class="text-gray-400" v-else>Press '/' for commands, type for text...</span>
    </div>
    <!-- Command selection -->
    <Combobox
      v-if="commanding"
      as="div"
      class="relative flex w-full flex-col"
      @update:model-value="selectCommand($event)"
      by="label"
    >
      <!-- Hidden button to manage focus programmatically -->
      <ComboboxButton class="hidden" ref="commandButtonRef" />
      <span class="flex flex-row items-baseline">
        /
        <ComboboxInput
          as="input"
          ref="commandInputRef"
          @change="commandQuery = $event.target.value"
          spellcheck="false"
          class="w-full min-w-0 border-0 bg-transparent p-0 outline-none ring-0 focus:ring-0"
          :class="[appearance.textSmall ? 'text-sm' : 'text-md']"
          @keydown.backspace.exact="commandQuery.length > 0 || stopCommanding()"
          @keydown.escape.prevent="morphToText(), emit('escape')"
        />
      </span>
      <!-- Prevent scroll and capture click outside -->
      <div
        v-if="commanding"
        class="fixed left-0 top-0 z-40 h-full w-full overscroll-none"
        @click.stop="commanding = false"
      />
      <!-- Command popup options -->
      <FadeTransition>
        <ComboboxOptions
          ref="commandOptionsRef"
          class="absolute z-50 flex h-fit max-h-[360px] w-[340px] flex-col gap-1 overflow-y-auto rounded-sm bg-white p-1 py-1 shadow-md ring-1 ring-orange-900 ring-opacity-20 focus:outline-none"
          :class="[isInTopHalfOfPanel ? 'top-7' : 'bottom-7']"
        >
          <div v-if="filteredCommands.length == 0" class="w-full px-2 py-1">
            <span class="text-gray-700">No results</span>
          </div>
          <ComboboxOption
            v-for="(command, i) in filteredCommands"
            :key="command.label"
            :value="command"
            v-slot="{ active }"
          >
            <div
              v-if="i == 0 || command.group != filteredCommands[i - 1]?.group"
              class="select-none px-2 py-1 text-xs font-semibold tracking-wide text-gray-500"
            >
              {{ command.group?.name }}
            </div>
            <li
              class="flex flex-row items-center justify-between gap-3"
              :class="[
                'cursor-pointer select-none px-2 py-0.5',
                active ? 'bg-orange-100 text-gray-900' : 'text-gray-900',
              ]"
            >
              <div class="py-1">
                <div class="relative h-8 w-8 rounded-md bg-orange-500">
                  <component :is="command.icon" class="absolute left-1.5 top-1.5 h-5 w-5 text-white" />
                </div>
              </div>
              <div class="flex flex-1 flex-col">
                <span class="font-semibold text-orange-600">
                  {{ command.label }}
                </span>
                <span class="text-xs text-gray-700"> {{ command.description }}. </span>
              </div>
            </li>
          </ComboboxOption>
        </ComboboxOptions>
      </FadeTransition>
    </Combobox>
  </div>
</template>
