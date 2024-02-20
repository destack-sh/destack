<script lang="ts" setup>
import {
  parseTextHtml,
  renderTextHtml,
  type TextSpan,
  useTextMentions,
  type MentionableNode,
  type TextMention,
} from "@/state/text";
import { useElementRefs } from "@/composables/useGrid";
import { nextTick, ref, watch, type Ref, toRef, computed } from "vue";
import type { Field, ModuleObjectTypename } from "@/state/module";
import { VALID_TEXT_REGEXP } from "@/utils/validation";
import type { TextPlain } from "@/state/text";
import { v4 as uuidv4 } from "uuid";
import { TypeTag } from "@/gql/graphql";
import { getEnumColor } from "@/state/statement";

const props = defineProps<{
  modelValue: string;
  readonly?: boolean;
  supportedAnnotations?: ModuleObjectTypename[];
  suppressShortcuts?: boolean;
  minimalMentions?: boolean;
  file?: { ck: string };
  statement?: { ck: string };
  allowAllCharacters?: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", v: string): void;
  (e: "navigateUp", position?: number): void;
  (e: "navigateDown", position?: number): void;
  (e: "navigateLeft"): void;
  (e: "navigateRight"): void;
  (e: "enterLeft"): void;
  (e: "enter"): void;
  (e: "enterRight"): void;
  (e: "execute"): void;
  (e: "escape"): void;
  (e: "deleteLeft"): void;
  (e: "illegal", char: string): void;
  (e: "toggleLanguage"): void;
}>();

const spans = ref(parseTextHtml(props.modelValue));
const spanRefs = useElementRefs<HTMLSpanElement>(spans);
const insertingPopoverOptionRefs: Ref<Record<string, HTMLElement>> = ref({});
const insertingMentionAt = ref<{
  span: TextPlain;
  idx: number;
  startChar: number;
  endChar: number;
  above: boolean;
  pos: { left: number; top: number };
} | null>(null);
const mentionQuery = ref("");
const activeMentionId = ref<string | null>(null);
const {
  resolvedMentions,
  filteredMentions,
  focus: focusMention,
} = useTextMentions(spans, {
  query: mentionQuery,
  searching: computed(() => !props.readonly && insertingMentionAt.value != null),
  file: toRef(props, "file"),
  statement: toRef(props, "statement"),
});

// sync modelValue into spans
watch(
  () => props.modelValue,
  () => {
    if (renderTextHtml(spans.value) == props.modelValue) return; // no change
    // TODO @UX: retain cursor position in annotated text state history like in MonacoEditor
    spans.value = parseTextHtml(props.modelValue);
  }
);

function onLocalWrite() {
  emit("update:modelValue", renderTextHtml(spans.value));
}

function closeMentionPopup() {
  insertingMentionAt.value = null;
  mentionQuery.value = "";
  activeMentionId.value = null;
}

function openMentionPopup(span: TextPlain, index: number, char: number) {
  const pos = window.getSelection()?.getRangeAt(0).getBoundingClientRect();
  if (pos == null) throw new Error("cannot get cursor position?");
  const isNearBottomScreenEdge = pos.top > window.innerHeight - 350; // popover height = 300px
  insertingMentionAt.value = { span, idx: index, startChar: char, endChar: char, pos, above: isNearBottomScreenEdge };
  mentionQuery.value = "";
  activeMentionId.value = filteredMentions.value[0]?.node.id ?? null;
}

function navigateMention(dir: "up" | "down") {
  const activeMentionIdx = filteredMentions.value.findIndex((m) => m.node.id == activeMentionId.value);
  if (activeMentionIdx == -1) {
    activeMentionId.value = filteredMentions.value[0]?.node.id ?? null;
  } else {
    if (dir == "up" && activeMentionIdx > 0) {
      activeMentionId.value = filteredMentions.value[activeMentionIdx - 1].node.id;
    } else if (dir == "down" && activeMentionIdx < filteredMentions.value.length - 1) {
      activeMentionId.value = filteredMentions.value[activeMentionIdx + 1].node.id;
    }
  }
  // ensure active mention is visible
  const activeMentionRef = insertingPopoverOptionRefs.value[activeMentionId.value as string];
  if (activeMentionRef != null) {
    activeMentionRef.scrollIntoView({ block: "nearest" });
  }
}

function onNavigateUp(span: TextSpan, index: number) {
  if (insertingMentionAt.value != null) {
    navigateMention("up");
  } else {
    emit("navigateUp", index);
  }
}

function onNavigateDown(span: TextSpan, index: number) {
  if (insertingMentionAt.value != null) {
    navigateMention("down");
  } else {
    emit("navigateDown", index);
  }
}

function onNavigateLeft(span: TextSpan, index: number, e: KeyboardEvent) {
  if (span.type == "text") {
    // close popover if we're leaving the mention text
    if (
      insertingMentionAt.value != null &&
      (window.getSelection()?.anchorOffset ?? 0) <= insertingMentionAt.value.startChar
    ) {
      // update text to include the @query part (excluded in onInput while inserting mention)
      const newSpans = [...spans.value];
      newSpans[index] = { ...span, text: fromNbsp((e.target as HTMLSpanElement).innerText ?? "") };
      spans.value = newSpans;
      onLocalWrite();
      closeMentionPopup();
      return;
    }
    if (window.getSelection()?.anchorOffset != 0) return; // ignore if not at start of text
    if (index > 0) {
      focus(index - 1, "last");
    } else {
      emit("navigateLeft");
    }
  } else if (span.type == "mention") {
    focus(index - 1, "last"); // has to be a preceding text span
  }
  e.preventDefault();
  e.stopPropagation();
}

function onNavigateRight(span: TextSpan, index: number, e: KeyboardEvent) {
  if (span.type == "text") {
    if (window.getSelection()?.anchorOffset != span.text.length) return; // ignore if not at end of text
    if (index < spans.value.length - 1) {
      focus(index + 1);
    } else {
      emit("navigateRight");
    }
  } else if (span.type == "mention") {
    focus(index + 1); // has to be a following text span
  }
  e.preventDefault();
  e.stopPropagation();
}

function onInput(span: TextSpan, index: number, e: InputEvent) {
  const text = (e.target as HTMLSpanElement).innerText;
  if (text == span.text) return; // no change
  if (span.type != "text") throw new Error(`cannot edit span ${span} directly`);

  if (insertingMentionAt.value != null) {
    insertingMentionAt.value.endChar = window.getSelection()?.anchorOffset ?? 0;
    mentionQuery.value = text.substring(insertingMentionAt.value.startChar, insertingMentionAt.value.endChar);
    // update active mention idx (if we were filtered out)
    const activeMentionIdx = filteredMentions.value.findIndex((m) => m.node.id == activeMentionId.value);
    if (activeMentionIdx == -1) {
      activeMentionId.value = filteredMentions.value[0]?.node.id ?? null;
    }
    return; // ignore input while adding mention
  }

  // check if char at current position is an @ to initiate mention
  // (this is not necessarily the end of the span)
  const char = window.getSelection()?.anchorOffset ?? 0;
  if (text[char - 1] == "@") {
    openMentionPopup(span, index, char);
    return;
  }

  // check if the input is valid
  if (!VALID_TEXT_REGEXP.test(text) && !props.allowAllCharacters) {
    const currentPos = window.getSelection()?.anchorOffset ?? 0;
    // invalid input, revert
    (e.target as HTMLElement).innerText = span.text;
    // restore cursor position to where it was before
    const selection = window.getSelection();
    if (selection) {
      selection.collapse((e.target as HTMLElement).childNodes[0], currentPos - 1);
    }
    const illegalChar = text[currentPos - 1];
    emit("illegal", illegalChar);
    return;
  }

  const newSpans = [...spans.value];
  newSpans[index] = { ...span, text: fromNbsp(text) };
  spans.value = newSpans;
  onLocalWrite();
}

function onDelete(span: TextSpan, index: number, e: KeyboardEvent) {
  const selection = window.getSelection();
  if (insertingMentionAt.value != null && selection?.anchorOffset == insertingMentionAt.value.startChar) {
    closeMentionPopup();
    e.preventDefault();
    e.stopPropagation();
  } else if (span.type == "text" && selection?.anchorOffset == 0 && selection.focusOffset == 0) {
    if (index == 0) {
      emit("deleteLeft");
    } else {
      const prevSpan = spans.value[index - 1];
      if (prevSpan.type == "text") {
        // merge with previous span
        const newSpans = [...spans.value];
        newSpans.splice(index - 1, 2, { ...prevSpan, text: prevSpan.text + span.text });
        spans.value = newSpans;
        onLocalWrite();
        nextTick(() => focus(index - 1, "last"));
      } else if (prevSpan.type == "mention") {
        // delete mention
        spans.value = spans.value.filter((s) => s.id != prevSpan.id);
        onLocalWrite();
        nextTick(() => focus(index - 2, "last"));
      }
    }
    e.preventDefault();
    e.stopPropagation();
  } else if (span.type == "mention") {
    // delete mention
    const newSpans = [...spans.value];
    newSpans.splice(index, 1);
    spans.value = newSpans;
    onLocalWrite();
  }
}

function onEnter(span: TextSpan, index: number, e: KeyboardEvent) {
  if (insertingMentionAt.value != null) {
    const mention = filteredMentions.value.find((m) => m.node.id == activeMentionId.value);
    if (mention != null) {
      insertMention(mention.node);
    }
  } else {
    const selection = window.getSelection();
    if (selection && index == 0 && selection.anchorOffset == 0) {
      emit("enterLeft");
    } else if (selection && index == spans.value.length - 1 && selection.anchorOffset == props.modelValue.length) {
      emit("enterRight");
    } else {
      emit("enter");
    }
  }
}

function insertMention(node: MentionableNode) {
  /* Inserts mention at the current insert pos */
  if (insertingMentionAt.value == null) return;
  const { span, idx, startChar: char } = insertingMentionAt.value;
  const mentionSpan: TextMention = {
    type: "mention",
    id: uuidv4(),
    referenceCk: node.ck,
    referenceType: node.__typename as ModuleObjectTypename,
  };

  // insert mention span (split if not at end of span)
  // also replace id of span that hosted the mention query to force it to update
  //  (its text didn't actually change, but the contenteditable model did with the @query)
  const newSpans = [...spans.value];
  if (char < span.text.length) {
    newSpans.splice(idx, 1, { ...span, id: uuidv4(), text: span.text.substring(0, char - 1) }, mentionSpan, {
      ...span,
      text: span.text.substring(char - 1),
    });
  } else {
    newSpans.splice(idx, 1, { ...span, id: uuidv4(), text: span.text.substring(0, char - 1) }, mentionSpan);
  }
  // ensure there's a text span at end & start of entire text
  if (newSpans[0].type != "text") {
    newSpans.splice(0, 0, { id: uuidv4(), type: "text", text: "" });
  }
  if (newSpans[newSpans.length - 1].type != "text") {
    newSpans.push({ id: uuidv4(), type: "text", text: "" });
  }
  // add space to the start of the next span if it's a text span
  if (newSpans[idx + 2]?.type == "text") {
    newSpans[idx + 2] = { ...newSpans[idx + 2], text: " " + newSpans[idx + 2].text };
  }

  spans.value = newSpans;
  closeMentionPopup();
  onLocalWrite();
  nextTick(() => focus(idx + 2, 1));
}

function focus(index: number, pos: "first" | "last" | number = "first") {
  const wasFocused = isFocused();
  const span = spans.value[index];
  if (span == null) {
    console.warn("cannot focus span", index, pos, spans.value);
    return;
  }

  const spanRef = spanRefs.getRef(span.id);
  spanRef?.focus();
  // move caret to start/end if it's a text span
  const selection = window.getSelection();
  if (span.type == "text" && selection != null && spanRef != null) {
    if (typeof pos == "number") {
      // put caret at pos
      if (spanRef.childNodes.length > 0) {
        selection.selectAllChildren(spanRef.childNodes[0]);
        selection.setBaseAndExtent(spanRef.childNodes[0], pos, spanRef.childNodes[0], pos);
      }
    } else if (pos == "first") {
      // select start of text
      if (spanRef.childNodes.length > 0) {
        selection.selectAllChildren(spanRef.childNodes[0]);
        selection.setBaseAndExtent(spanRef.childNodes[0], 0, spanRef.childNodes[0], 0);
      } else {
        // span has no text content yet
        selection.selectAllChildren(spanRef);
        if (props.modelValue.length > 0) {
          selection.collapseToStart();
        }
      }
    } else {
      // select end of text
      if (spanRef.childNodes.length > 0) {
        selection.selectAllChildren(spanRef.childNodes[0]);
        selection.setBaseAndExtent(
          spanRef.childNodes[0],
          spanRef.childNodes[0].textContent?.length ?? 0,
          spanRef.childNodes[0],
          spanRef.childNodes[0].textContent?.length ?? 0
        );
      } else {
        // span has no text content yet
        selection.selectAllChildren(spanRef);
        selection.collapseToEnd();
      }
    }
  }

  // check if caret is at @, if so insta open mention popup
  if (!wasFocused && span.type == "text") {
    const selection = window.getSelection();
    if (selection?.anchorOffset == 1 && selection.anchorNode?.textContent?.[selection.anchorOffset - 1] == "@") {
      openMentionPopup(span, index, selection.anchorOffset);
    }
  }
}

function isFocused(): boolean {
  return spans.value.some((span) => spanRefs.getRef(span.id)?.contains(document.activeElement));
}

function focusIfUnfocused() {
  if (!isFocused()) {
    focus(spans.value.length - 1, "last");
  }
}

function blur() {
  spanRefs.refs.value.forEach((ref) => ref.blur());
  closeMentionPopup();
}

function toNbsp(s: string) {
  return s.replace(/ /g, "\u00a0");
}

function fromNbsp(s: string) {
  return s.replace(/\u00a0/g, " ");
}

defineExpose({
  focus: (f: "first" | "last" = "first") => {
    if (f == "first") {
      focus(0);
    } else {
      focus(spans.value.length - 1, "last");
    }
  },
  focusIfUnfocused,
  open: computed(() => insertingMentionAt.value != null),
  blur,
  spans,
});
</script>
<template>
  <!-- TODO @UX: make entire annotated text div contenteditable? -->
  <div tabindex="-1" spellcheck="false" class="inline outline-none">
    <!-- Actual spans -->
    <template v-for="(span, i) in spans" :key="span.id">
      <!-- Normal text -->
      <span
        v-if="span.type == 'text'"
        :ref="(ref: any) => spanRefs.registerRef(span.id, ref)"
        :contenteditable="((readonly ? 'false' : 'plaintext-only') as any)"
        tabindex="-1"
        spellcheck="false"
        class="outline-none"
        :class="suppressShortcuts ? '' : 'mousetrap'"
        @keydown.up.exact.prevent="onNavigateUp(span, i)"
        @keydown.down.exact.prevent="onNavigateDown(span, i)"
        @keydown.left="onNavigateLeft(span, i, $event)"
        @keydown.right="onNavigateRight(span, i, $event)"
        @keydown.escape.prevent="insertingMentionAt == null ? closeMentionPopup() : emit('escape')"
        @keydown.enter.exact.prevent="onEnter(span, i, $event as KeyboardEvent)"
        @keydown.backspace.exact="onDelete(span, i, $event as KeyboardEvent)"
        @input="onInput(span, i, $event as InputEvent)"
        @keydown.ctrl.enter.prevent="emit('execute')"
        @keydown.meta.enter.prevent="emit('execute')"
        @keydown.alt.enter.prevent="emit('toggleLanguage')"
      >
        {{ span.text }}
      </span>
      <!-- Mention -->
      <div
        v-else-if="span.type == 'mention'"
        :ref="(ref: any) => spanRefs.registerRef(span.id, ref)"
        tabindex="-1"
        spellcheck="false"
        :contenteditable="false"
        @keydown.up.exact.prevent="emit('navigateUp')"
        @keydown.down.exact.prevent="emit('navigateDown')"
        @keydown.left="onNavigateLeft(span, i, $event)"
        @keydown.right="onNavigateRight(span, i, $event)"
        @keydown.escape.prevent="emit('escape')"
        @keydown.backspace.prevent="onDelete(span, i, $event as KeyboardEvent)"
        @keydown.ctrl.enter.prevent="emit('execute')"
        @keydown.meta.enter.prevent="emit('execute')"
        @keydown.alt.enter.prevent="emit('toggleLanguage')"
        class="mousetrap-ignore relative inline rounded-sm underline decoration-gray-300 underline-offset-4 ring-inset transition-colors duration-150 focus:border-0 focus:outline-none focus:ring-1"
        :class="[
          minimalMentions ? '' : '-my-0.5 mx-[1px] py-0.5  ',
          minimalMentions ? '' : 'hover:cursor-pointer',
          !minimalMentions && resolvedMentions[i]?.node.__typename == 'Field'
            ? 'hover:bg-amber-100 hover:decoration-amber-600'
            : '',
          !minimalMentions && resolvedMentions[i]?.node.__typename != 'Field'
            ? 'hover:bg-orange-100 hover:decoration-orange-600'
            : '',
          resolvedMentions[i]?.node.__typename == 'Field'
            ? 'focus:bg-amber-100 focus:decoration-amber-600 focus:ring-amber-600/10'
            : 'focus:bg-orange-100 focus:decoration-orange-600 focus:ring-orange-600/10',
        ]"
        @click="() => resolvedMentions[i] == null || minimalMentions || focusMention(resolvedMentions[i].node)"
      >
        <!-- Icon (enum icon for enums, otherwise given mention icon) -->
        <template v-if="!minimalMentions && resolvedMentions[i] != null">
          <svg
            v-if="resolvedMentions[i].node.__typename == 'Field' && (resolvedMentions[i].node as Field).tag == TypeTag.Literal"
            class="absolute left-2 top-[7px] h-[8px] w-[8px]"
            :style="{ fill: getEnumColor(resolvedMentions[i].node) }"
            viewBox="0 0 6 6"
            aria-hidden="true"
          >
            <rect rx="2" ry="2" width="5" height="6" />
          </svg>
          <component
            v-else
            :is="resolvedMentions[i].icon"
            class="absolute left-[1px] top-0.5 h-4 w-4"
            :class="[resolvedMentions[i].node.__typename == 'Field' ? 'text-yellow-500' : 'text-orange-600']"
          />
        </template>
        <span class="max-w-full truncate" :class="[!minimalMentions && resolvedMentions[i] != null ? 'ml-[21px]' : '']">
          {{ resolvedMentions[i]?.name ?? "???" }}
        </span>
      </div>
    </template>
    <!-- Placeholder if empty -->
    <template v-if="spans.length == 1 && spans[0].type == 'text' && spans[0].text.length == 0">&nbsp;</template>

    <!-- Popover -->
    <!-- Prevent scroll and capture click outside -->
    <div
      v-if="insertingMentionAt != null"
      class="fixed left-0 top-0 z-40 h-full w-full overscroll-none"
      @click.stop="insertingMentionAt = null"
    />
    <!-- Adding mention background info
       (only if mention query is empty since this is 'above' the query due to different stacking contexts) -->
    <div
      v-if="insertingMentionAt != null && mentionQuery == ''"
      class="fixed -mx-0.5 rounded-sm bg-orange-100 px-0.5 font-normal text-gray-400"
      :style="{
        left: insertingMentionAt.pos.left + 2 + 'px',
        top: insertingMentionAt.pos.top - 2 + 'px',
      }"
    >
      Mention a statement, field, ...
    </div>
    <!-- Adding mention popover -->
    <div
      v-if="insertingMentionAt != null"
      class="scroll-hidden fixed z-50 max-h-[300px] w-80 overflow-y-scroll rounded-sm bg-white p-1 text-sm font-normal text-gray-900 ring-1 ring-orange-900 ring-opacity-40"
      :style="{
        left: insertingMentionAt.pos.left - 12 + 'px',
        top: insertingMentionAt.above ? 'auto' : insertingMentionAt.pos.top + 18 + 'px',
        bottom: insertingMentionAt.above ? 'calc(100vh - ' + insertingMentionAt.pos.top + 'px)' : 'auto',
      }"
    >
      <ul class="flex flex-col">
        <!-- Mention candidate -->
        <!-- Keyboard events are handled in active text span, not here (we don't actually focus these) -->
        <li
          v-for="mention in filteredMentions"
          :ref="(ref: any) => insertingPopoverOptionRefs[mention.node.id] = ref"
          :key="mention.node.id"
          @click.stop.prevent="insertMention(mention.node)"
          class="flex cursor-pointer flex-row items-center justify-between gap-2.5 rounded-sm px-2 py-0.5 hover:bg-orange-100"
          :class="{ 'bg-orange-100': mention.node.id == activeMentionId }"
        >
          <span class="flex flex-shrink-0 flex-row items-center">
            <!-- Icon (enum icon for enums, otherwise given mention icon) -->
            <svg
              v-if="mention.node.__typename == 'Field' && (mention.node as Field).tag == TypeTag.Literal"
              class="ml-1 mr-3 h-[8px] w-[8px]"
              :style="{ fill: getEnumColor(mention.node) }"
              viewBox="0 0 6 6"
              aria-hidden="true"
            >
              <rect rx="2" ry="2" width="5" height="6" />
            </svg>
            <component
              v-else
              :is="mention.icon"
              class="mr-2 h-4 w-4"
              :class="[mention.node.__typename == 'Field' ? 'text-yellow-500' : 'text-orange-600']"
            />
            <span class="truncate text-gray-900">{{ mention.name }}</span>
          </span>
          <span class="truncate text-gray-400">{{ mention.path ?? "(builtin)" }}</span>
        </li>
        <div v-if="filteredMentions.length == 0" class="text-center">
          <span class="text-gray-400"> No matching nodes </span>
        </div>
      </ul>
    </div>
  </div>
</template>
