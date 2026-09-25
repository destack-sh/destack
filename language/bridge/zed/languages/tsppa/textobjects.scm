(line_comment)+ @comment.around

(function_declaration) @function.around

(function_definition
  "{"
  (_)* @function.inside
  "}") @function.around
