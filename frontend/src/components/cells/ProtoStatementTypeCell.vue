<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import EditableSpan from "@/components/basic/EditableSpan.vue";
import { useStatementContext } from "@/state/statement";
import { ExpectationModifier, StatementType, TypeTag } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import {
  MODIFIER_BY_KEYWORD,
  SUPPORTED_MODIFIERS,
  SUPPORTED_STATEMENT_TYPES,
  STATEMENT_TYPE_BY_KEYWORD,
} from "@/state/type";
import { TypeFlag } from "@/state/module";
import { Combobox, ComboboxOption, ComboboxInput, ComboboxOptions, ComboboxButton } from "@headlessui/vue";
import { useFocus } from "@vueuse/core";
import { computed, nextTick, ref, watch, type Ref } from "vue";

defineProps<{ showDots?: boolean }>();

const emit = defineEmits<{
  (e: "navigateUp", position?: number): void;
  (e: "navigateDown", position?: number): void;
  (e: "navigateLeft"): void;
  (e: "navigateRight"): void;
  (e: "enter"): void;
  (e: "escape"): void;
  (e: "deleteLeft"): void;
  (e: "morphed"): void;
}>();

const context = useStatementContext();
const content: Ref<string> = ref("");
const spanRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);

const MAX_KEYWORD_LENGTH = [...Object.keys(MODIFIER_BY_KEYWORD), ...Object.keys(STATEMENT_TYPE_BY_KEYWORD)].reduce(
  (max, keyword) => Math.max(max, keyword.length),
  0
);

// parse content changes :ParseStatementInput
watch(content, (newContent) => {
  if (newContent == "/") {
    openCommandSelection();
    return;
  }

  const endsInSep =
    newContent.endsWith(" ") || newContent.endsWith(" ") || newContent.endsWith(":") || newContent.endsWith(";");
  const includesNonalpha = !newContent.match(/^[a-zA-Z]*$/);
  const tooLong = newContent.length > MAX_KEYWORD_LENGTH;
  const newContentTrim = newContent.slice(0, -1);

  // if it matches an allowed keyword, apply the keyword
  let morphed = true;
  if (endsInSep) {
    morphed = handleKeyword(newContentTrim);
  } else if (includesNonalpha || tooLong) {
    // auto-convert to comment if it can't be parsed anymore (keep content)
    newContent = newContent.replace(" ", " "); // replace non-breaking spaces
    context.morphToComment(newContent);
  } else {
    morphed = false;
  }
  if (morphed) {
    content.value = "";
    emit("morphed");
  }
});

function handleKeyword(newContentTrim: string): boolean {
  if (
    MODIFIER_BY_KEYWORD[newContentTrim] != null &&
    SUPPORTED_MODIFIERS.includes(MODIFIER_BY_KEYWORD[newContentTrim])
  ) {
    context.setModifier(MODIFIER_BY_KEYWORD[newContentTrim]);
  } else if (SUPPORTED_STATEMENT_TYPES.includes(STATEMENT_TYPE_BY_KEYWORD[newContentTrim])) {
    context.morpthToSymbol({ type: STATEMENT_TYPE_BY_KEYWORD[newContentTrim] });
  } else if (newContentTrim == "enum" || newContentTrim == "choice") {
    context.morpthToSymbol({ type: StatementType.Type, rootTypeTag: TypeTag.Enum });
  } else if (newContentTrim == "struct") {
    context.morpthToSymbol({ type: StatementType.Type, rootTypeTag: TypeTag.Struct });
  } else if (newContentTrim == "record") {
    context.morpthToSymbol({ type: StatementType.Data, rootTypeFlags: 0 });
  } else if (newContentTrim == "table") {
    context.morpthToSymbol({ type: StatementType.Data, rootTypeFlags: TypeFlag.IsArray });
  } else {
    return false;
  }
  return true;
}

// command/type selection dropdown
// (not sure if this is the best place to put it)
const commanding: Ref<boolean> = ref(false);
const commandQuery: Ref<string> = ref("");
const commandInputRef: Ref<InstanceType<typeof ComboboxInput> | null> = ref(null);
const commandButtonRef: Ref<InstanceType<typeof ComboboxButton> | null> = ref(null);
const { focused: commandInputRefFocused } = useFocus(commandInputRef);

type Command = {
  label: string;
  description: string;
  action: () => void;
};

const commands = computed(() => {
  const commands: Command[] = [
    {
      label: "task",
      description: "Instruct AI to do something.",
      action: () => (context.morpthToSymbol({ type: StatementType.Task }), emit("morphed")),
    },
    {
      label: "expect",
      description: "Specify desired behaviour.",
      action: () => (context.morpthToSymbol({ type: StatementType.Expectation }), emit("morphed")),
    },
    {
      label: "struct",
      description: "Define a data structure.",
      action: () => (
        context.morpthToSymbol({ type: StatementType.Type, rootTypeTag: TypeTag.Struct }), emit("morphed")
      ),
    },
    {
      label: "choice",
      description: "Define a choice type.",
      action: () => (context.morpthToSymbol({ type: StatementType.Type, rootTypeTag: TypeTag.Enum }), emit("morphed")),
    },
    {
      label: "record",
      description: "Configure context and secrets.",
      action: () => (context.morpthToSymbol({ type: StatementType.Data, rootTypeFlags: 0 }), emit("morphed")),
    },
    {
      label: "table",
      description: "Define state or examples.",
      action: () => (
        context.morpthToSymbol({ type: StatementType.Data, rootTypeFlags: TypeFlag.IsArray }), emit("morphed")
      ),
    },
    {
      label: "code",
      description: "Implement logic in Python.",
      action: () => (context.morpthToSymbol({ type: StatementType.Code }), emit("morphed")),
    },
  ];

  if (context.statement.value.parent != null) {
    commands.push({
      label: "like",
      description: "Give positive behaviour examples.",
      action: () => (context.setModifier(ExpectationModifier.Like), emit("morphed")),
    });
    commands.push({
      label: "unlike",
      description: "Give negative behaviour examples.",
      action: () => (context.setModifier(ExpectationModifier.Unlike), emit("morphed")),
    });
  }

  return commands;
});
const filteredCommands = computed(() => {
  return commands.value.filter((command) => {
    return command.label.toLowerCase().includes(commandQuery.value.toLowerCase());
  });
});

function openCommandSelection() {
  commanding.value = true;
  nextTick(() => ((commandInputRefFocused.value = true), commandButtonRef.value?.$el.click()));
}

function stopCommanding() {
  commanding.value = false;
  content.value = "";
  nextTick(() => spanRef.value?.focus());
}

function selectCommand(command: Command) {
  commanding.value = false;
  content.value = "";
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

defineExpose({
  focus,
  blur,
  content,
});
</script>
<template>
  <EditableSpan
    ref="spanRef"
    v-if="!commanding"
    v-model="content"
    :readonly="context.readonly.value"
    @navigate-up="emit('navigateUp')"
    @navigate-down="emit('navigateDown')"
    @navigate-left="emit('navigateLeft')"
    @navigate-right="emit('navigateRight')"
    @enter="handleKeyword(content) || emit('enter')"
    @escape="emit('escape')"
    @delete-left="emit('deleteLeft')"
  />
  <!-- Command selection -->
  <Combobox
    v-else
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
        @keydown.escape.prevent="emit('escape')"
      />
    </span>
    <FadeTransition>
      <ComboboxOptions
        class="absolute top-7 z-20 flex max-h-64 w-80 flex-col gap-1 overflow-auto rounded-sm bg-white p-1 shadow-sm ring-1 ring-orange-900 ring-opacity-20 focus:outline-none"
      >
        <div v-if="filteredCommands.length == 0" class="w-full px-2 py-1">
          <span class="text-gray-700">No results</span>
        </div>
        <ComboboxOption v-for="command in filteredCommands" :key="command.label" :value="command" v-slot="{ active }">
          <li
            class="flex flex-col"
            :class="[
              'cursor-pointer select-none px-2 py-0.5',
              active ? 'bg-orange-100 text-gray-900' : 'text-gray-900',
            ]"
          >
            <span class="text-orange-600">
              {{ command.label }}
            </span>
            <span class="text-gray-700">
              {{ command.description }}
            </span>
          </li>
        </ComboboxOption>
      </ComboboxOptions>
    </FadeTransition>
  </Combobox>
</template>
