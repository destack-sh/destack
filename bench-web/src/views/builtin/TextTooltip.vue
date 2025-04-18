<script lang="ts" setup>
import { IconData } from "@/proto/wire";
import { Command, CommandBuiltinId, getCommand } from "@/ui/command";
import { IconInline, makeIcon } from "@/ui/icon";
import { hasTextMark, markFormatCommand, setTextMark } from "@/ui/prosemirror/editor";
import { TextMarkType } from "@/ui/prosemirror/schema";
import { type TooltipProps } from "@/ui/prosemirror/view";

const props = defineProps<TooltipProps>();

type FormatCommand = {
  id: string | number;
  icon: IconData;
  command: Command;
  isChecked: () => boolean;
  toggle: () => void;
};

function makeCommand(mark: TextMarkType, commandId: CommandBuiltinId, icon: string): FormatCommand {
  const command = getCommand(commandId);
  const pmCommand = markFormatCommand(mark, () => props.view, () => true);
  return {
    id: mark,
    icon: makeIcon(icon),
    command,
    isChecked: () => pmCommand.isChecked(),
    toggle: () => pmCommand.command() ,
  };
}

const formatCommands: FormatCommand[] = [
  makeCommand("bold", "text.format.bold", "fas fa-bold"),
  makeCommand("italic", "text.format.italic", "fas fa-italic"),
  makeCommand("underline", "text.format.underline", "fas fa-underline"),
  makeCommand("strikethrough", "text.format.strikethrough", "fas fa-strikethrough"),
  makeCommand("code", "text.format.code", "fas fa-code"),
  makeCommand("spoiler", "text.format.spoiler", "fas fa-eye"),
];
</script>
<template>
  <Transition
    enter-active-class="transition-all ease-in duration-200"
    enter-from-class="opacity-0 scale-95"
    enter-to-class="opacity-100 scale-100"
    leave-active-class="transition-all ease-out duration-75"
    leave-from-class="opacity-100 scale-100"
    leave-to-class="opacity-0 scale-95"
    appear
  >
    <div
      v-if="visible && tick >= 0 /* react to tick */"
      class="flex w-fit flex-row items-center rounded-sm border border-gray-200 bg-white px-1 py-1"
      :style="{ height: `${height}px` }"
      @click.stop.prevent
      @mousedown.stop.prevent
    >
      <!-- Mark-ish commands -->
      <button
        v-for="command in formatCommands"
        :key="command.id"
        v-tooltip="{
          title: command.command.title,
          shortcuts: command.command.shortcuts,
          group: 'text.format',
        }"
        class="rounded-sm px-1 py-0.5 hover:bg-gray-100"
        :class="[command.isChecked() ? 'text-yellow-700' : '']"
        @mousedown.stop.prevent="command.toggle()"
      >
        <IconInline class="w-5 text-center" v-bind="command.icon" />
      </button>
    </div>
  </Transition>
</template>
