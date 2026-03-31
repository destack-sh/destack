type T = test extends B
  ? /* comment
       comment
       comment
       comment
    */
    foo
  : test extends B
  ? /* comment
  comment
    comment */
    foo
  : bar;
