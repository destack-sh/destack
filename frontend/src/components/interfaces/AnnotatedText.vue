<script lang="ts" setup>
import { parseTextHtml, type TextSpan, useTextMentions, type MentionableNode } from "@/state/text";
import { useElementRefs } from "@/composables/useGrid";
import { ref } from "vue";
import { useCurrentModule } from "@/state/module";
import { VALID_TEXT_REGEXP } from "@/utils/validation";

const props = defineProps<{
  modelValue: string;
  readonly: boolean;
  suppressAllShortcuts?: boolean;
  supportedAnnotations?: string[];
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
  (e: "escape"): void;
  (e: "deleteLeft"): void;
  (e: "illegal", char: string): void;
}>();

const module = useCurrentModule();
const spans = ref(parseTextHtml(props.modelValue));
const spanRefs = useElementRefs<HTMLSpanElement>(spans);
const addingMentionAt = ref<{ span: TextSpan; char: number; pos: { left: number; top: number } } | null>(null);
const addingMentionQuery = ref("");
const { resolvedMentions, filteredMentions } = useTextMentions(spans, addingMentionQuery);
const activeMentionId = ref<string | null>(null);

function close() {
  addingMentionAt.value = null;
  addingMentionQuery.value = "";
  activeMentionId.value = null;
}

function onNavigateUp(span: TextSpan, index: number) {
  if (addingMentionAt.value != null) {
    navigateMention("up");
  } else {
    emit("navigateUp", index);
  }
}

function onNavigateDown(span: TextSpan, index: number) {
  if (addingMentionAt.value != null) {
    navigateMention("down");
  } else {
    emit("navigateDown", index);
  }
}

function navigateMention(dir: "up" | "down") {
  const activeMentionIdx = filteredMentions.value.findIndex((m) => m.id == activeMentionId.value);
  if (activeMentionIdx == -1) {
    activeMentionId.value = filteredMentions.value[0]?.id ?? null;
  } else {
    if (dir == "up" && activeMentionIdx > 0) {
      activeMentionId.value = filteredMentions.value[activeMentionIdx - 1].id;
    } else if (dir == "down" && activeMentionIdx < filteredMentions.value.length - 1) {
      activeMentionId.value = filteredMentions.value[activeMentionIdx + 1].id;
    }
  }
}

function selectMention(node: MentionableNode) {
  console.log("selectMention", node); // nocheckin
  close();
}

function onInput(span: TextSpan, index: number, e: InputEvent) {
  const text = (e.target as HTMLSpanElement).innerText;
  if (text == span.text) return; // no change

  if (addingMentionAt.value != null) {
    addingMentionQuery.value = text.substring(addingMentionAt.value.char);
    // update active mention idx (if we were filtered out)
    const activeMentionIdx = filteredMentions.value.findIndex((m) => m.id == activeMentionId.value);
    if (activeMentionIdx == -1) {
      activeMentionId.value = filteredMentions.value[0]?.id ?? null;
    }
    return; // ignore input while adding mention
  }

  // check if char at current position is an @ to initiate mention
  // (this is not necessarily the end of the span)
  const selection = window.getSelection();
  const char = selection?.anchorOffset ?? 0;
  if (char != null && text[char - 1] == "@") {
    const pos = selection?.getRangeAt(0).getBoundingClientRect();
    addingMentionAt.value = { span, char, pos: { left: pos?.left ?? 0, top: pos?.top ?? 0 } };
    addingMentionQuery.value = "";
    activeMentionId.value = filteredMentions.value[0]?.id ?? null;
    return;
  }

  // check if the input is valid
  if (!VALID_TEXT_REGEXP.test(text)) {
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
}

function onDelete(span: TextSpan, index: number, e: KeyboardEvent) {
  const selection = window.getSelection();
  if (addingMentionAt.value != null && selection?.anchorOffset == addingMentionAt.value.char) {
    addingMentionAt.value = null;
    e.preventDefault();
    e.stopPropagation();
  } else if (span.type == "text" && index == 0 && selection?.anchorOffset == 0) {
    emit("deleteLeft");
    e.preventDefault();
    e.stopPropagation();
  } else if (span.type == "mention") {
    // delete mention
    const newSpans = [...spans.value];
    newSpans.splice(index, 1);
    spans.value = newSpans;
  }
}

function onEnter(span: TextSpan, index: number, e: KeyboardEvent) {
  if (addingMentionAt.value != null) {
    const node = filteredMentions.value.find((m) => m.id == activeMentionId.value);
    if (node != null) {
      selectMention(node);
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

function focus(index: number, pos: "first" | "last" = "first") {
  const spanRef = spanRefs.refs.value[index];
  const selection = window.getSelection();
  console.log("focus", index, spanRef); // nocheckin
  spanRef?.focus();
  if (selection != null && spanRef != null) {
    if (pos == "first") {
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
          props.modelValue.length,
          spanRef.childNodes[0],
          props.modelValue.length
        );
      } else {
        // span has no text content yet
        selection.selectAllChildren(spanRef);
        selection.collapseToEnd();
      }
    }
  }
}

function blur() {
  spanRefs.refs.value.forEach((ref) => ref.blur());
  close();
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
  blur,
});
</script>
<template>
  <div tabindex="-1" spellcheck="false" class="inline outline-none">
    <!-- Actual spans -->
    <template v-for="(span, i) in spans" :key="i">
      <!-- Normal text -->
      <span
        v-if="span.type == 'text'"
        :ref="(ref: any) => spanRefs.registerRef(span.id, ref)"
        :contenteditable="((readonly ? 'false' : 'plaintext-only') as any)"
        tabindex="-1"
        spellcheck="false"
        class="mousetrap whitespace-pre-wrap outline-none"
        @keydown.up.exact.prevent="onNavigateUp(span, i)"
        @keydown.down.exact.prevent="onNavigateDown(span, i)"
        @keydown.escape.exact.prevent="addingMentionAt == null ? close() : emit('escape')"
        @keydown.enter.exact.prevent="onEnter(span, i, $event as KeyboardEvent)"
        @keydown.backspace.exact="onDelete(span, i, $event as KeyboardEvent)"
        @input="onInput(span, i, $event as InputEvent)"
      >
        {{ span.text }}
      </span>
      <!-- Mention -->
      <span
        v-else-if="span.type == 'mention'"
        :ref="(ref: any) => spanRefs.registerRef(span.id, ref)"
        tabindex="-1"
        spellcheck="false"
        :contenteditable="false"
        @keydown.up.exact.prevent="emit('navigateUp')"
        @keydown.down.exact.prevent="emit('navigateDown')"
        @keydown.escape.exact.prevent="emit('escape')"
        @keydown.backspace.exact.prevent="onDelete(span, i, $event as KeyboardEvent)"
      >
        <span>{{ module.nodeOf(span.referenceCk)?.name ?? "???" }}</span>
      </span>
    </template>
    <!-- Prevent scroll and capture click outside -->
    <div
      v-if="addingMentionAt != null"
      class="fixed left-0 top-0 z-40 h-full w-full overscroll-none"
      @click.stop="addingMentionAt = null"
    />
    <!-- Adding mention -->
    <div
      v-if="addingMentionAt != null"
      class="fixed z-50 max-h-[300px] w-80 overflow-y-scroll rounded-sm bg-white p-1 text-gray-900 ring-1 ring-orange-900 ring-opacity-40"
      :style="{ left: addingMentionAt.pos.left + 'px', top: addingMentionAt.pos.top + 18 + 'px' }"
    >
      <ul class="flex flex-col">
        <li
          v-for="mention in filteredMentions"
          :key="mention.id"
          class="cursor-pointer rounded-sm px-2 py-0.5 hover:bg-orange-100"
          :class="{ 'bg-orange-100': mention.id == activeMentionId }"
          @click.stop.prevent="selectMention(mention)"
          @keydown.enter.stop.prevent="selectMention(mention)"
          @keydown.up.stop.prevent="navigateMention('up')"
          @keydown.down.stop.prevent="navigateMention('down')"
        >
          {{ mention.name }}
        </li>
        <div v-if="filteredMentions.length == 0" class="text-center">
          <span class="text-gray-400"> No matching nodes </span>
        </div>
      </ul>
    </div>
  </div>
</template>
