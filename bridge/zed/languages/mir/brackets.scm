((punctuation) @open
  (#match? @open "^[({\\[]$"))

((punctuation) @close
  (#match? @close "^[)}\\]]$"))
