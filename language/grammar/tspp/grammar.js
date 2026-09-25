const JavaScript = require('tree-sitter-javascript/grammar');

module.exports = grammar(JavaScript, {
  name: 'tspp',

  reserved: {
    global: (_, previous) => previous.filter((word) => word.value !== 'throw'),
  },

  externals: ($, previous) => previous.concat([
    $._function_signature_automatic_semicolon,
    $.__error_recovery,
    $._lifetime,
    $._type_declaration_keyword,
    $._type_value_keyword,
  ]),

  supertypes: ($, previous) => previous.concat([
    $.type,
    $.primary_type,
  ]),

  precedences: ($, previous) => previous.concat([
    [
      'call',
      'instantiation',
      'unary',
      'cast_type',
      'as',
      'binary',
      $.await_expression,
      $.arrow_function,
    ],
    [
      'extends',
      'instantiation',
    ],
    [
      'if_let_right',
      'literal',
    ],
    [
      'if_let_pattern',
      'assign',
    ],
    [
      $.intersection_type,
      $.union_type,
      $.conditional_type,
      $.function_type,
      'binary',
      $.readonly_type,
    ],
    [$.mapped_type_clause, $.primary_expression],
    [$.accessibility_modifier, $.primary_expression],
    ['unary_void', $.expression],
    [$.extends_clause, $.primary_expression],
    ['unary', 'assign'],
    ['declaration', $.expression],
    [$.predefined_type, $.unary_expression],
    [$.tuple_type, $.array_type, $.pattern, $.type],
    [$.readonly_type, $.pattern],
    [$.readonly_type, $.primary_expression],
    [$.type_query, $.subscript_expression, $.expression],
    [$.type_query, $._type_query_subscript_expression],
    [$.nested_type_identifier, $.generic_type, $.primary_type, $.lookup_type, $.index_type_query, $.type],
    [$.as_expression, $.satisfies_expression, $.primary_type],
    [$._type_query_member_expression, $.member_expression],
    [$._type_query_member_expression, $.primary_expression],
    [$._type_query_subscript_expression, $.subscript_expression],
    [$._type_query_subscript_expression, $.primary_expression],
    [$._type_query_call_expression, $.primary_expression],
    [$._type_query_instantiation_expression, $.primary_expression],
    [$.type_query, $.primary_expression],
    [$.override_modifier, $.primary_expression],
    [$.decorator_call_expression, $.decorator],
    [$.literal_type, $.pattern],
    [$.predefined_type, $.pattern],
    [$.call_expression, $._type_query_call_expression],
    [$.new_expression, $.primary_expression],
    [$.meta_property, $.primary_expression],
    [$.construct_signature, $._property_name],
  ]),

  conflicts: ($, previous) => previous
    .filter((conflict) => !sameConflict(conflict, ['class_static_block', '_property_name']))
    .filter((conflict) => !sameConflict(conflict, ['primary_expression', 'method_definition']))
    .filter((conflict) => !sameConflict(conflict, ['_initializer', 'binary_expression']))
    .concat([
      [$.const_expression, $.primary_type],
      [$.statement_block, $.object, $.match_object_pattern],
      [$.object_assignment_pattern, $.assignment_expression, $._property_name],
      [$.assignment_expression, $._initializer],
      [$.object, $._property_name],
      [$.primary_expression, $.memory_pattern, $.memory_expression],
      [$.object_pattern, $.match_object_pattern_property],
      [$.object_pattern, $.match_object_pattern],
      [$.object, $.match_object_pattern],
      [$.object, $.object_pattern, $.match_object_pattern],
      [$.object_assignment_pattern, $._property_name],
      [$.assignment_expression, $.literal_type],
      [$.assignment_expression, $.match_pattern],
      [$.range_expression, $.range_pattern],
      [$.assignment_expression, $.pattern, $.literal_type],
      [$._range_expression_atom, $._range_pattern_bound],
      [$.assignment_expression, $.pattern, $.match_pattern],
      [$.assignment_expression, $.literal_binding_pattern],
      [$.primary_expression, $.literal_binding_pattern],
      [$.primary_expression, $.must_binding_pattern],
      [$.variable_declarator, $.primary_expression],
      [$.subscript_expression, $.match_arm_expression_statement],
      [$.primary_expression, $.await_expression, $.rest_pattern],
      [$.primary_expression, $._static_value_operand],
      [$.primary_expression, $.static_value_argument],
      [$.nested_identifier, $.nested_type_identifier, $.primary_expression],
      [$.nested_identifier, $.nested_type_identifier],
      [$.primary_expression, $.nested_identifier],
      [$._call_signature, $.constructor_type],
      [$._call_signature, $.function_type],

      [$.primary_expression, $._parameter_name],
      [$.primary_expression, $.receiver_parameter],
      [$.primary_expression, $.receiver_parameter, $.primary_type],
      [$.receiver_parameter, $.primary_type],
      [$.where_clause],
      [$.primary_expression, $._parameter_name, $.primary_type],
      [$.primary_expression, $.literal_type],
      [$.primary_expression, $.literal_type, $.rest_pattern],
      [$.literal_type, $._static_value_operand],
      [$.primary_expression, $.predefined_type, $.rest_pattern],
      [$.primary_expression, $.primary_type],
      [$.primary_expression, $.placement_type],
      [$.lexical_declaration, $.primary_type],
      [$.placed_type_satisfies_left, $.primary_type],
      [$.primary_type, $._static_value_operand],
      [$.primary_type, $._static_value_call_expression],
      [$.generic_type, $._static_value_call_expression],
      [$.static_value_argument, $._static_value_operand],
      [$.jsx_namespace_name, $.primary_type, $._static_value_operand],
      [$.primary_expression, $._struct_literal_generic_type],
      [$.primary_expression, $._struct_literal_generic_type, $.generic_type],
      [$.primary_expression, $._struct_literal_generic_type, $.generic_type, $._static_value_call_expression],
      [$.primary_expression, $.predefined_type],
      [$.primary_expression, $.pattern, $.primary_type],
      [$._augmented_assignment_lhs, $.dereference_assignment_statement],
      [$.rest_pattern, $.primary_type],
      [$.rest_pattern, $.literal_type],
      [$.rest_pattern, $.predefined_type],
      [$.rest_pattern, $._try_propagation_argument],
      [$._try_propagation_argument, $.primary_type],
      [$.pattern, $._try_propagation_argument, $.primary_type],
      [$._try_propagation_argument, $.type_query],
      [$.rest_pattern, $._try_propagation_argument, $.primary_type],
      [$._destructuring_pattern, $._let_else_pattern],
      [$.variable_declarator, $._let_else_pattern],
      [$._parameter_name, $.primary_type],
      [$.pattern, $.primary_type],

      [$.rest_pattern, $.primary_type, $.primary_expression],

      [$.object, $.object_type],
      [$.object, $.object_pattern, $.object_type],
      [$.object, $.object_pattern, $._property_name],
      [$.primary_expression, $._property_name, $.placement_type],
      [$.object_pattern, $.object_type],

      [$.tuple_expression, $.tuple_type],
      [$.template_literal_type, $.template_string],
      [$.primary_expression, $.struct_literal_expression],
      [$.struct_literal_expression, $._extends_clause_single],
      [$.primary_type, $.associated_type_projection],
      [$.expression, $.memory_expression],
      [$.type, $.memory_type],
      [$.type, $.placement_type],
      [$.type, $._conditional_union_right],
      [$.expression, $._satisfies_left_expression],
      [$.primary_expression, $._satisfies_left_expression],
      [$.primary_expression, $.associated_type_projection],
      [$.if_statement, $.primary_expression],
      [$.expression, $.placement_expression],
      [$.primary_expression, $.memory_expression],
      [$.primary_expression, $._range_expression_atom],
      [$.primary_expression, $._range_expression_atom, $._static_value_operand],
      [$.primary_expression, $._static_value_call_expression],
      [$._range_expression_atom, $.literal_type],
      [$.range_expression, $.interval_type],
      [$._range_expression_bound],
      [$._range_expression_atom, $._interval_type_bound],
    ]).concat([
      [$.const_block_statement, $.const_expression],
      [$.static_if_expression],
      [$.optional_type, $.index_type_query],
      [$.type, $.optional_type],
      [$.primary_type, $._interval_type_bound],
      [$.primary_type, $.predefined_type_parameters],
      [$.formal_parameters, $.tuple_type],
      [$.formal_parameters, $.tuple_expression],
      [$.pattern, $._try_propagation_argument],
      [$.call_expression, $.match_arm_expression_statement],
      [$.primary_expression, $.match_arm_expression_statement],
      [$.binary_expression, $.match_arm_expression_statement],
      [$.if_statement, $.primary_expression, $.if_let_condition],
      [$.primary_expression, $.if_let_condition],
      [$.expression, $._if_let_right_expression],
      [$.primary_expression, $._if_let_right_expression],
      [$.parenthesized_expression, $._if_condition],
    ]).concat([
      [$.jsx_opening_element, $.type_parameter],
    ]),

  inline: ($, previous) => previous
    .filter((rule) => ![
      '_formal_parameter',
      '_call_signature',
    ].includes(rule.name))
    .concat([
      $._type_identifier,
      $._jsx_start_opening_element,
      $._type_satisfies_left,
    ]),

  rules: {
    _property_name: $ => reserved('properties', choice(
      alias(
        choice($.identifier, $._reserved_identifier),
        $.property_identifier,
      ),
      $.string,
      alias(token(integerName()), $.number),
    )),

    pair_pattern: $ => seq(
      field('key', choice($._property_name, $.computed_property_name)),
      ':',
      field('value', choice($.pattern, $.assignment_pattern)),
    ),

    public_field_definition: $ => seq(
      repeat(field('decorator', $.decorator)),
      optional($.accessibility_modifier),
      optional('declare'),
      optional('static'),
      optional($.override_modifier),
      optional('readonly'),
      optional('abstract'),
      field('name', $._property_name),
      optional('?'),
      field('type', optional($.type_annotation)),
      optional($._initializer),
    ),

    try_statement: $ => seq(
      'try',
      field('body', $.statement_block),
      optional(field('handler', choice($.catch_match_clause, $.catch_clause))),
      optional(field('finalizer', $.finally_clause)),
    ),

    // override original catch_clause, add optional type annotation
    catch_clause: $ => seq(
      'catch',
      optional(
        choice(
          seq(
            '(',
            field(
              'parameter',
              choice($.identifier, $._destructuring_pattern),
            ),
            optional(field('type', $.type_annotation)),
            ')',
          ),
          field('parameter', choice($.identifier, $._destructuring_pattern)),
        ),
      ),
      field('body', $.statement_block),
    ),

    catch_match_clause: $ => seq(
      'catch',
      'match',
      '(',
      field('value', $.expression),
      ')',
      '{',
      repeat($.match_arm),
      '}',
    ),

    call_expression: $ => choice(
      prec('call', seq(
        field('function', $.expression),
        field('type_arguments', optional($.type_arguments)),
        field('arguments', $.arguments),
      )),
      prec('template_call', seq(
        field('function', choice($.primary_expression, $.new_expression)),
        field('arguments', $.template_string),
      )),
      prec('member', seq(
        field('function', $.primary_expression),
        '?.',
        field('type_arguments', optional($.type_arguments)),
        field('arguments', $.arguments),
      )),
    ),

    new_expression: $ => prec.right('new', seq(
      'new',
      field('constructor', $.primary_expression),
      field('type_arguments', optional($.type_arguments)),
      field('arguments', optional($.arguments)),
    )),

    assignment_expression: (_, previous) => previous,

    using_assignment_expression: $ => prec.right('assign', seq(
      optional('await'),
      'using',
      commaSep1($.using_binding),
    )),

    using_binding: $ => seq(
      field('left', choice(
        $.identifier,
        $._destructuring_pattern,
        $.tuple_pattern,
        $.struct_pattern,
      )),
      '=',
      field('right', $.expression),
    ),

    annotation_call_expression: $ => prec.right('unary', seq(
      '@',
      field('function', choice(
        $.identifier,
        alias($.decorator_member_expression, $.member_expression),
      )),
      field('arguments', $.arguments),
    )),

    _augmented_assignment_lhs: ($, previous) => choice(
      previous,
      $.non_null_expression,
    ),

    _lhs_expression: ($, previous) => choice(
      previous,
      $.non_null_expression,
    ),

    primary_expression: $ => choice(
      $.struct_literal_expression,
      $.fixed_array_expression,
      $.annotation_call_expression,
      $.type_value,
      $.try_propagation_expression,
      $.memory_expression,
      $.placement_expression,
      $.await_try_propagation_expression,
      $.const_expression,
      $.do_expression,
      $.match_expression,
      $.tuple_expression,
      $.subscript_expression,
      $.member_expression,
      $.parenthesized_expression,
      $._identifier,
      alias($._reserved_identifier, $.identifier),
      $.this,
      $.super,
      $.number,
      $.string,
      $.template_string,
      $.regex,
      $.true,
      $.false,
      $.null,
      $.object,
      $.array,
      $.function_expression,
      $.arrow_function,
      $.generator_function,
      $.meta_property,
      $.call_expression,
      $.non_null_expression,
    ),

    struct_literal_expression: $ => prec('literal', seq(
      field('type', choice(
        $.identifier,
        alias($._struct_literal_generic_type, $.generic_type),
      )),
      field('value', $.object),
    )),

    _struct_literal_generic_type: $ => prec('call', seq(
      field('name', $.identifier),
      field('type_arguments', $.type_arguments),
    )),

    fixed_array_expression: $ => seq(
      '[',
      field('value', $.expression),
      ';',
      field('length', choice(
        $.static_value_argument,
        $.number,
        $.identifier,
        $.associated_type_projection,
      )),
      ']',
    ),

    expression: ($, previous) => {
      const choices = [
        $.static_if_expression,
        $.as_expression,
        $.satisfies_expression,
        $.instantiation_expression,
        $.range_expression,
        ...previous.members,
      ];

      return choice(...choices);
    },

    ternary_expression: $ => prec.right('ternary', seq(
      field('condition', $.expression),
      alias($._question_token, '?'),
      field('consequence', $.expression),
      ':',
      field('alternative', $.expression),
    )),

    _question_token: _ => token('?'),

    if_expression: $ => prec.right(seq(
      'if',
      field('condition', choice(
        $.parenthesized_expression,
        seq('(', $.if_let_condition, ')'),
      )),
      field('consequence', $.statement_block),
      optional(seq(
        'else',
        field('alternative', choice($.statement_block, $.if_expression)),
      )),
    )),

    do_expression: $ => prec.right(seq(
      'do',
      field('body', $.statement_block),
    )),

    while_expression: $ => seq(
      'while',
      field('condition', $.parenthesized_expression),
      field('body', $.statement_block),
    ),

    for_expression: $ => prec(1, seq(
      'for',
      '(',
      field('initializer', optional(choice(
        seq(choice('let', 'const'), commaSep1($.variable_declarator)),
        $.expression,
      ))),
      ';',
      field('condition', optional($.expression)),
      ';',
      field('increment', optional($.expression)),
      ')',
      field('body', $.statement_block),
    )),

    tuple_expression: $ => choice(
      seq('(', ')'),
      seq(
        '(',
        $.expression,
        ',',
        commaSep($.expression),
        optional(','),
        ')',
      ),
    ),

    range_expression: $ => prec.left('binary_relation', seq(
      field('left', optional($._range_expression_bound)),
      field('operator', choice('..', '..=')),
      field('right', optional($._range_expression_bound)),
    )),

    _range_expression_bound: $ => choice(
      $._range_expression_atom,
      prec.left('unary', seq(
        '-',
        $._range_expression_atom,
      )),
      prec.left('binary_plus', seq(
        $._range_expression_atom,
        choice('+', '-'),
        $._range_expression_atom,
      )),
      prec.left('binary_times', seq(
        $._range_expression_atom,
        choice('*', '/', '%'),
        $._range_expression_atom,
      )),
    ),

    _range_expression_atom: $ => choice(
      $.number,
      $.identifier,
    ),

    statement: $ => choice(
      $.decorated_statement,
      $.static_if_statement,
      $.const_block_statement,
      $.type_satisfies_statement,
      $.value_satisfies_statement,
      $.let_else_statement,
      $.using_assignment_statement,
      $.dereference_assignment_statement,
      $.loop_expression,
      $.export_statement,
      $.import_statement,
      $.declaration,
      $.debugger_statement,
      $.expression_statement,
      $.statement_block,
      $.if_statement,
      $.switch_statement,
      $.for_statement,
      $.for_in_statement,
      $.while_statement,
      $.do_statement,
      $.try_statement,
      $.break_statement,
      $.continue_statement,
      $.return_statement,
      $.empty_statement,
      $.labeled_statement,
    ),

    let_else_statement: $ => prec.dynamic(1, prec.right('declaration', seq(
      field('kind', choice('let', 'const')),
      field('left', $._let_else_pattern),
      field('type', optional($.type_annotation)),
      '=',
      field('right', $.expression),
      'else',
      field('alternative', $.statement_block),
      $._semicolon,
    ))),

    _let_else_pattern: $ => choice(
      $.match_constructor_pattern,
      $.tuple_pattern,
      $.array_pattern,
      $.must_binding_pattern,
      $.literal_binding_pattern,
    ),

    must_binding_pattern: $ => prec.left('unary', seq(
      field('argument', choice(
        $.identifier,
        $.match_constructor_pattern,
        $.match_member_pattern,
        $.match_struct_pattern,
        $.object_pattern,
        $.tuple_pattern,
        $.array_pattern,
      )),
      '!',
    )),

    literal_binding_pattern: $ => choice(
      $.number,
      $.string,
      $.true,
      $.false,
      $.null,
      $.undefined,
    ),

    using_assignment_statement: $ => seq(
      field('expression', $.using_assignment_expression),
      $._semicolon,
    ),

    dereference_assignment_statement: $ => prec.right('assign', seq(
      '*',
      field('left', choice(
        $.identifier,
        $.member_expression,
        $.subscript_expression,
        $.parenthesized_expression,
      )),
      field('operator', choice(
        '=',
        '+=',
        '-=',
        '*=',
        '/=',
        '%=',
        '^=',
        '&=',
        '|=',
        '>>=',
        '>>>=',
        '<<=',
        '**=',
        '&&=',
        '||=',
        '??=',
      )),
      field('right', $.expression),
      $._semicolon,
    )),

    lexical_declaration: $ => prec.right('declaration', seq(
      field('kind', choice('let', 'const')),
      commaSep1($.variable_declarator),
      optional(seq(
        'else',
        field('alternative', $.statement_block),
      )),
      $._semicolon,
    )),

    if_statement: ($, previous) => choice(
      prec.right(seq(
        'if',
        'let',
        field('left', $.if_let_pattern),
        field('type', optional($.type_annotation)),
        '=',
        field('right', $._if_let_right_expression),
        field('consequence', $.statement_block),
        optional(field('alternative', $.else_clause)),
      )),
      prec.right(seq(
        'if',
        '(',
        field('condition', $._if_condition),
        ')',
        field('consequence', $.statement_block),
        optional(field('alternative', $.else_clause)),
      )),
      prec.right(seq(
        'if',
        field('condition', $._if_condition),
        field('consequence', $.statement_block),
        optional(field('alternative', $.else_clause)),
      )),
      previous,
    ),

    _for_header: $ => seq(
      '(',
      choice(
        field('left', choice(
          $._lhs_expression,
          $.parenthesized_expression,
        )),
        seq(
          field('kind', choice('let', 'const')),
          field('left', choice(
            $.identifier,
            $._destructuring_pattern,
          )),
          optional($._automatic_semicolon),
        ),
        seq(
          field('kind', seq(optional('await'), 'using')),
          field('left', choice(
            $.identifier,
            $._destructuring_pattern,
            $.tuple_pattern,
            $.struct_pattern,
          )),
        ),
      ),
      field('operator', 'of'),
      field('right', $._expressions),
      ')',
    ),

    switch_case: $ => seq(
      'case',
      field('value', $._expressions),
      optional(seq('if', field('guard', $.expression))),
      ':',
      field('body', repeat($.statement)),
    ),

    match_expression: $ => prec(1, seq(
      'match',
      '(',
      field('value', $.expression),
      ')',
      '{',
      repeat($.match_arm),
      '}',
    )),

    match_arm: $ => seq(
      repeat(field('decorator', $.decorator)),
      field('pattern', $.match_pattern),
      optional(seq('if', field('guard', $.expression))),
      '=>',
      field('body', choice($.match_arm_statement, alias($.match_arm_expression_statement, $.expression_statement))),
    ),

    match_arm_statement: $ => choice(
      $.statement_block,
      $.break_statement,
      $.continue_statement,
      $.return_statement,
      $.empty_statement,
    ),

    // NOTE #Robustness: a bare expression arm followed by an arm whose pattern
    // starts with '[' or '(' loses the GLR fork to subscript/call continuation;
    // fixing this needs newline awareness in the shared scanner
    match_arm_expression_statement: $ => choice(
      prec.dynamic(2, $.call_expression),
      prec.dynamic(1, seq(
        $.expression,
        optional(choice(',', $._semicolon, $._automatic_semicolon)),
      )),
    ),

    match_pattern: $ => prec(1, choice(
      $.union_pattern,
      $.must_pattern,
      $.memory_pattern,
      $.default_pattern,
      $.range_pattern,
      $.match_constructor_pattern,
      $.match_struct_pattern,
      $.match_member_pattern,
      $.tuple_pattern,
      $.rest_pattern,
      $.array,
      $.array_pattern,
      $.match_object_pattern,
      $.literal_type,
      $.identifier,
    )),

    match_object_pattern: $ => seq(
      '{',
      optional(seq(
        commaSep1($.match_object_pattern_property),
        optional(','),
      )),
      '}',
    ),

    match_object_pattern_property: $ => choice(
      $.rest_pattern,
      seq(
        field('name', choice($._property_name, $.computed_property_name)),
        optional(choice(
          seq(':', field('value', $.match_pattern)),
          seq('=', field('default', choice($.literal_type, $.identifier))),
        )),
      ),
    ),

    default_pattern: $ => prec.right('assign', seq(
      field('left', $.match_pattern),
      '=',
      field('right', choice($.literal_type, $.identifier)),
    )),

    union_pattern: $ => prec.left('binary_relation', seq(
      field('left', $.match_pattern),
      '|',
      field('right', $.match_pattern),
    )),

    must_pattern: $ => prec.left('unary', seq(
      field('argument', $.match_pattern),
      '!',
    )),

    memory_pattern: $ => prec.left('unary', choice(
      seq(
        '^',
        optional('readonly'),
        field('argument', $.match_pattern),
      ),
      seq(
        '&',
        optional(choice('readonly', 'exclusive')),
        field('argument', $.match_pattern),
      ),
      seq(
        '*',
        field('argument', $.match_pattern),
      ),
    )),

    range_pattern: $ => prec(2, seq(
      field('left', optional($._range_pattern_bound)),
      field('operator', choice('..', '..=')),
      field('right', optional($._range_pattern_bound)),
    )),

    _range_pattern_bound: $ => choice(
      $.literal_type,
      $.identifier,
    ),

    match_member_pattern: $ => prec(2, seq(
      field('object', $.identifier),
      '.',
      field('property', $.identifier),
    )),

    match_constructor_pattern: $ => prec(2, seq(
      field('name', choice($.identifier, $.nested_identifier)),
      '(',
      commaSep1($.match_pattern),
      optional(','),
      ')',
    )),

    match_struct_pattern: $ => prec(2, seq(
      field('name', choice($.identifier, $.nested_identifier)),
      field('pattern', $.match_object_pattern),
    )),

    tuple_pattern: $ => seq(
      '(',
      commaSep1($.match_pattern),
      optional(','),
      ')',
    ),

    // a bare constraint list eats commas greedily; the parenthesized form
    // bounds the list explicitly where members are comma-separated
    where_clause: $ => seq(
      'where',
      choice(
        commaSep1($.where_constraint),
        seq(
          '(',
          commaSep1($.where_constraint),
          optional(','),
          ')',
        ),
      ),
    ),

    where_constraint: $ => seq(
      field('left', $._where_constraint_left),
      field('operator', choice(':', '==')),
      field('right', $._where_constraint_right),
    ),

    _where_constraint_left: $ => choice(
      $.associated_type_projection,
      $.generic_type,
      $.nested_type_identifier,
      $._type_identifier,
    ),

    _where_constraint_right: $ => choice(
      $.type,
      $.static_value_argument,
    ),

    loop_expression: $ => seq(
      'loop',
      field('body', $.statement_block),
    ),

    try_expression: $ => prec.right(1, alias($.try_statement, $.try_expression)),

    try_propagation_expression: $ => prec.dynamic(1, prec.left('unary', seq(
      field('argument', $._try_propagation_argument),
      token.immediate('?'),
      optional(seq(
        'satisfies',
        field('target', choice(alias($._cast_intersection_type, $.intersection_type), $.type)),
      )),
    ))),

    // the scanner only lexes a ternary question mark when it is detached, so
    // an attached `?` after any of these operands is try-propagation
    _try_propagation_argument: $ => choice(
      $.identifier,
      $.parenthesized_expression,
      $.member_expression,
      $.subscript_expression,
      $.call_expression,
      $.annotation_call_expression,
    ),

    await_try_propagation_expression: $ => prec.right('unary', seq(
      field('operator', $._await_try_propagation_operator),
      field('argument', $.expression),
    )),

    _await_try_propagation_operator: _ => token(prec(2, seq('await', choice('?', '!')))),

    const_expression: $ => prec.dynamic(-1, prec.right('unary', seq(
      'const',
      field('value', choice(
        $.statement_block,
        $.if_expression,
        $.expression,
      )),
    ))),

    break_statement: $ => prec(1, seq(
      'break',
      optional(choice(
        seq(
          field('label', alias($.identifier, $.statement_identifier)),
          optional(seq(':', field('value', $.expression))),
        ),
        field('value', $.expression),
      )),
      $._semicolon,
    )),

    memory_expression: $ => prec.left('unary', choice(
      seq(
        '^',
        optional('readonly'),
        field('argument', $.primary_expression),
      ),
      seq(
        '&',
        optional(choice('readonly', 'exclusive')),
        field('argument', $.primary_expression),
      ),
      prec.right('unary', seq(
        '*',
        field('argument', $.primary_expression),
      )),
    )),

    placement_expression: $ => prec.left('unary', seq(
      choice(
        alias(token(prec(1, seq('local', /\s+/))), 'local'),
        alias(token(prec(1, seq('shared', /\s+/))), 'shared'),
      ),
      field('argument', $.primary_expression),
    )),

    const_block_statement: $ => seq(
      'const',
      field('body', $.statement_block),
    ),

    decorated_statement: $ => prec.right('declaration', seq(
      repeat1(field('decorator', $.decorator)),
      choice(
        $.lexical_declaration,
        $.variable_declaration,
        $._decorated_statement_target,
      ),
    )),

    static_if_guard: $ => prec.right(choice(
      seq(
        '@',
        'if',
        '(',
        commaSep($.expression),
        optional(','),
        ')',
      ),
      seq('@', 'if'),
    )),

    static_if_statement: $ => prec.right('declaration', seq(
      repeat1(field('guard', $.static_if_guard)),
      choice(
        $.declaration,
        $.import_statement,
        $.export_statement,
        $._statement_target,
      ),
    )),

    _statement_target: $ => choice(
      $._decorated_statement_target,
      $.empty_statement,
    ),

    _decorated_statement_target: $ => choice(
      $.expression_statement,
      $.statement_block,
      $.if_statement,
      $.switch_statement,
      $.for_statement,
      $.for_in_statement,
      $.while_statement,
      $.do_statement,
      $.try_statement,
      $.break_statement,
      $.continue_statement,
      $.return_statement,
      $.labeled_statement,
      $.const_block_statement,
    ),

    static_if_expression: $ => prec.right('declaration', seq(
      repeat1(field('guard', $.static_if_guard)),
      field('value', $.expression),
    )),

    if_let_pattern: $ => prec.right('if_let_pattern', $.match_pattern),

    _if_condition: $ => choice(
      $.if_let_chain,
      $.if_let_condition,
      $.expression,
    ),

    if_let_chain: $ => prec.left('binary', choice(
      seq(
        field('left', $.if_let_condition),
        field('operator', '&&'),
        field('right', choice($.if_let_condition, $.expression, $.if_let_chain)),
      ),
      seq(
        field('left', $.expression),
        field('operator', '&&'),
        field('right', choice($.if_let_condition, $.if_let_chain)),
      ),
    )),

    if_let_condition: $ => prec.right('if_let_pattern', seq(
      choice('let', 'const'),
      field('left', $.if_let_pattern),
      field('type', optional($.type_annotation)),
      '=',
      field('right', $._if_let_right_expression),
    )),

    _if_let_right_expression: $ => prec.right('if_let_right', choice(
      $.identifier,
      $.member_expression,
      $.call_expression,
      $.subscript_expression,
      $.parenthesized_expression,
      $.array,
      $.object,
      $.number,
      $.string,
      $.true,
      $.false,
      $.null,
      $.undefined,
      $.this,
      $.super,
      $.as_expression,
      $.satisfies_expression,
      $.instantiation_expression,
      $.try_propagation_expression,
    )),

    _jsx_start_opening_element: $ => seq(
      '<',
      optional(
        seq(
          choice(
            field('name', choice(
              $._jsx_identifier,
              $.jsx_namespace_name,
            )),
            seq(
              field('name', choice(
                $.identifier,
                alias($.nested_identifier, $.member_expression),
              )),
              field('type_arguments', optional($.type_arguments)),
            ),
          ),
          repeat(field('attribute', $._jsx_attribute)),
        ),
      ),
    ),

    jsx_opening_element: $ => prec.dynamic(-1, seq(
      $._jsx_start_opening_element,
      '>',
    )),

    // tsx only. See jsx_opening_element.
    jsx_self_closing_element: $ => prec.dynamic(-1, seq(
      $._jsx_start_opening_element,
      '/>',
    )),

    export_specifier: ($, previous) => seq(
      repeat(field('guard', $.static_if_guard)),
      optional('type'),
      choice(
        previous,
        // `type` itself is exportable as a plain name
        seq(
          field('name', alias('type', $.identifier)),
          optional(seq('as', field('alias', $._module_export_name))),
        ),
      ),
    ),

    _import_identifier: $ => choice(
      $.identifier,
      alias('type', $.identifier),
      alias('local', $.identifier),
      alias('shared', $.identifier),
      alias($.placement_modifier, $.identifier),
    ),

    import_specifier: $ => seq(
      repeat(field('guard', $.static_if_guard)),
      optional('type'),
      choice(
        field('name', $._import_identifier),
        seq(
          field('name', choice($._module_export_name, alias('type', $.identifier))),
          'as',
          field('alias', $._import_identifier),
        ),
      ),
    ),

    import_attribute: $ => seq('with', $.object),

    import_clause: $ => choice(
      $.namespace_import,
      $.named_imports,
      seq(
        $._import_identifier,
        optional(seq(
          ',',
          choice(
            $.namespace_import,
            $.named_imports,
          ),
        )),
      ),
    ),

    import_statement: $ => seq(
      'import',
      optional('type'),
      choice(
        seq($.import_clause, $._from_clause),
        field('source', $.string),
      ),
      optional($.import_attribute),
      $._semicolon,
    ),

    arguments: $ => seq(
      '(',
      optional(seq(
        optional(choice(
          $.expression,
          $.spread_element,
        )),
        repeat(seq(
          ',',
          optional(choice(
            $.expression,
            $.spread_element,
          )),
        )),
      )),
      ')',
    ),

    export_statement: ($, previous) => choice(
      previous,
      seq(
        'export',
        choice(
          seq('*', $._from_clause),
          seq($.namespace_export, $._from_clause),
          seq($.export_clause, $._from_clause),
        ),
        optional($.import_attribute),
        $._semicolon,
      ),
      seq(
        'export',
        'type',
        '*',
        $._from_clause,
        $._semicolon,
      ),
      seq(
        'export',
        'type',
        $.export_clause,
        optional($._from_clause),
        $._semicolon,
      ),
    ),

    non_null_expression: $ => prec.left('unary', seq(
      $.expression, '!',
    )),

    variable_declarator: $ => choice(
      seq(
        field('name', choice(
          $.identifier,
          $._destructuring_pattern,
          $.tuple_pattern,
          $.struct_pattern,
          $.match_constructor_pattern,
          $.literal_binding_pattern,
        )),
        field('type', optional($.type_annotation)),
        optional($._initializer),
      ),
      seq(
        field('name', $.must_binding_pattern),
        field('type', optional($.type_annotation)),
        $._initializer,
      ),
    ),

    struct_pattern: $ => seq(
      field('type', $.identifier),
      field('pattern', $.match_object_pattern),
    ),

    method_definition_member: $ => prec.left(seq(
      optional($.accessibility_modifier),
      optional('static'),
      optional($.override_modifier),
      // the modifier form requires whitespace so `virtual:` stays a field name
      optional(alias(token(seq('virtual', /[ \t]+/)), 'virtual')),
      optional('async'),
      optional(choice('get', 'set')),
      field('name', $._property_name),
      optional('?'),
      $._call_signature,
      optional($.where_clause),
      field('body', $.statement_block),
    )),

    method_signature: $ => seq(
      optional($.accessibility_modifier),
      optional('declare'),
      optional('static'),
      optional('readonly'),
      optional($.override_modifier),
      // the modifier form requires whitespace so `virtual:` stays a field name
      optional(alias(token(seq('virtual', /[ \t]+/)), 'virtual')),
      optional('async'),
      optional(choice('get', 'set')),
      field('name', $._property_name),
      optional('?'),
      $._call_signature,
      optional($.where_clause),
    ),

    abstract_method_signature: $ => seq(
      optional($.accessibility_modifier),
      optional('static'),
      'abstract',
      optional($.override_modifier),
      optional(choice('get', 'set', '*')),
      field('name', $._property_name),
      optional('?'),
      $._call_signature,
      optional($.where_clause),
    ),

    parenthesized_expression: $ => seq(
      '(',
      seq($.expression, field('type', optional($.type_annotation))),
      ')',
    ),

    _formal_parameter: $ => choice(
      $.optional_parameter,
      $.required_parameter,
    ),

    function_signature: $ => seq(
      repeat(field('decorator', $.decorator)),
      optional('const'),
      optional('async'),
      'function',
      field('name', $.identifier),
      $._call_signature,
      optional($.where_clause),
      choice($._semicolon, $._function_signature_automatic_semicolon),
    ),

    declare_function_signature: $ => seq(
      repeat(field('decorator', $.decorator)),
      'declare',
      optional('const'),
      optional('async'),
      'function',
      optional('*'),
      field('name', $.identifier),
      $._call_signature,
      optional($.where_clause),
      choice($._semicolon, $._function_signature_automatic_semicolon),
    ),

    function_declaration: $ => prec.right('declaration', seq(
      repeat(field('decorator', $.decorator)),
      optional('const'),
      optional('async'),
      'function',
      field('name', $.identifier),
      $._call_signature,
      optional($.where_clause),
      field('body', $.statement_block),
      optional($._automatic_semicolon),
    )),

    generator_function_declaration: $ => prec.right('declaration', seq(
      repeat(field('decorator', $.decorator)),
      optional('const'),
      optional('async'),
      'function',
      '*',
      field('name', $.identifier),
      $._call_signature,
      optional($.where_clause),
      field('body', $.statement_block),
      optional($._automatic_semicolon),
    )),

    decorator: $ => seq(
      '@',
      choice(
        $.identifier,
        alias($.decorator_member_expression, $.member_expression),
        alias($.decorator_call_expression, $.call_expression),
        alias($.decorator_instantiation_expression, $.instantiation_expression),
        alias($.decorator_parenthesized_expression, $.parenthesized_expression),
      ),
      optional($._automatic_semicolon),
    ),

    decorator_member_expression: $ => prec('member', seq(
      field('object', choice(
        $.identifier,
        alias($.super, $.identifier),
        alias($.decorator_member_expression, $.member_expression),
        alias($.decorator_call_expression, $.call_expression),
        alias($.decorator_instantiation_expression, $.instantiation_expression),
      )),
      '.',
      field('property', alias($.identifier, $.property_identifier)),
    )),

    decorator_call_expression: $ => prec('call', seq(
      field('function', choice(
        $.identifier,
        alias($.decorator_member_expression, $.member_expression),
        alias($.decorator_call_expression, $.call_expression),
        alias($.decorator_instantiation_expression, $.instantiation_expression),
      )),
      optional(field('type_arguments', $.type_arguments)),
      field('arguments', $.arguments),
    )),

    decorator_instantiation_expression: $ => prec('instantiation', seq(
      field('function', choice(
        $.identifier,
        alias($.decorator_member_expression, $.member_expression),
        alias($.decorator_call_expression, $.call_expression),
      )),
      field('type_arguments', $.type_arguments),
    )),

    decorator_parenthesized_expression: $ => seq(
      '(',
      choice(
        $.identifier,
        alias($.decorator_member_expression, $.member_expression),
        alias($.decorator_call_expression, $.call_expression),
        alias($.decorator_instantiation_expression, $.instantiation_expression),
      ),
      ')',
    ),

    class_body: $ => seq(
      '{',
      repeat(choice(
        seq(
          repeat(field('decorator', $.decorator)),
          alias($.method_definition_member, $.method_definition),
          optional($._semicolon),
        ),
        seq(
          repeat(field('decorator', $.decorator)),
          $.method_signature,
          choice($._function_signature_automatic_semicolon, $._semicolon, ','),
        ),
        seq(
          $.struct_embedding_member,
          optional(choice($._semicolon, ',')),
        ),
        seq(
          repeat(field('decorator', $.decorator)),
          choice(
            $.class_static_block,
            $.associated_type_declaration,
            $.associated_const_declaration,
            $.const_block_statement,
            $.index_signature,
            $.abstract_method_signature,
          ),
          choice($._semicolon, ','),
        ),
        seq(
          $.public_field_definition,
          choice($._semicolon, ','),
        ),
        ';',
      )),
      '}',
    ),

    method_definition: $ => prec.left(seq(
      optional('static'),
      optional('async'),
      optional(choice('get', 'set')),
      field('name', $._property_name),
      optional('?'),
      $._call_signature,
      optional($.where_clause),
      field('body', $.statement_block),
    )),

    class_static_block: $ => seq(
      'static',
      $.statement_block,
    ),

    declaration: ($, previous) => choice(
      $.function_declaration,
      $.generator_function_declaration,
      $.class_declaration,
      $.lexical_declaration,
      $.struct_declaration,
      $.shared_lexical_declaration,
      $.declare_function_signature,
      $.function_signature,
      $.abstract_class_declaration,
      $.extension_declaration,
      $.type_alias_declaration,
      $.enum_declaration,
      $.interface_declaration,
      $.module_declaration,
      $.global_declaration,
      $.ambient_declaration,
    ),

    shared_lexical_declaration: $ => seq(
      field('place', $.placement_modifier),
      $.lexical_declaration,
    ),

    as_expression: $ => prec.left('as', seq(
      $.expression,
      'as',
      choice(
        'const',
        alias($._cast_intersection_type, $.intersection_type),
        $.type,
      ),
    )),

    satisfies_expression: $ => prec.left('as', seq(
      $.expression,
      'satisfies',
      choice(
        alias($._cast_intersection_type, $.intersection_type),
        $.type,
      ),
    )),

    type_satisfies_statement: $ => prec.dynamic(1, prec.right('declaration', seq(
      field('left', $._type_satisfies_left),
      'satisfies',
      field('right', $.type),
      $._semicolon,
    ))),

    value_satisfies_statement: $ => prec.dynamic(10, prec.right('declaration', seq(
      field('left', $._satisfies_left_expression),
      'satisfies',
      field('right', $.type),
      $._semicolon,
    ))),

    _satisfies_left_expression: $ => choice(
      $.primary_expression,
      $.call_expression,
      $.new_expression,
      $.await_expression,
      $.unary_expression,
      $.binary_expression,
      $.ternary_expression,
      $.update_expression,
      $.as_expression,
      $.range_expression,
    ),

    _type_satisfies_left: $ => choice(
      $.generic_type,
      $.memory_type,
      $.placement_type,
      alias($.placed_type_satisfies_left, $.placement_type),
      $.readonly_type,
      $.parenthesized_type,
    ),

    placed_type_satisfies_left: $ => prec.right('unary', seq(
      $.placement_modifier,
      choice($.memory_type, $.primary_type),
    )),

    _cast_intersection_type: $ => prec.right(100, seq(
      $.primary_type,
      '&',
      choice(
        alias($._cast_intersection_type, $.intersection_type),
        $.primary_type,
      ),
    )),

    instantiation_expression: $ => prec('instantiation', seq(
      $.expression,
      field('type_arguments', $.type_arguments),
    )),

    binary_expression: ($, previous) => choice(
      previous,
      prec.left('binary_shift', seq(
        field('left', $.expression),
        field('operator', choice('<<|', '>>|', '>>>|')),
        field('right', $.expression),
      )),
      prec.left('binary_plus', seq(
        field('left', $.expression),
        field('operator', choice('+%', '+|', '-%', '-|')),
        field('right', $.expression),
      )),
      prec.left('binary_times', seq(
        field('left', $.expression),
        field('operator', choice('*%', '*|')),
        field('right', $.expression),
      )),
      prec.right('binary_exp', seq(
        field('left', $.expression),
        field('operator', choice('**%', '**|')),
        field('right', $.expression),
      )),
      prec.left('binary_relation', seq(
        field('left', $.expression),
        field('operator', 'extends'),
        field('right', $.expression),
      )),
      prec.left('binary_relation', seq(
        field('left', $.expression),
        field('operator', 'is'),
        field('right', $.expression),
      )),
    ),

    class_heritage: $ => choice(
      seq($.extends_clause, optional($.implements_clause)),
      $.implements_clause,
    ),

    extends_clause: $ => seq(
      'extends',
      commaSep1($._extends_clause_single),
    ),

    _extends_clause_single: $ => prec('extends', seq(
      field('value', choice(
        $.identifier,
        $.nested_identifier,
        $.member_expression,
        $.subscript_expression,
        $.call_expression,
        $.new_expression,
        $.instantiation_expression,
        $.parenthesized_expression,
        $.this,
        $.super,
      )),
    )),

    implements_clause: $ => seq(
      'implements',
      commaSep1($._implements_clause_single),
    ),

    _implements_clause_single: $ => choice(
      $.negated_implements_type,
      $._type_identifier,
      $.nested_type_identifier,
      $.associated_type_projection,
      $.generic_type,
      $.lookup_type,
      $.index_type_query,
      $.type_query,
      $.parenthesized_type,
      $.predefined_type,
      $.union_type,
      $.intersection_type,
    ),

    negated_implements_type: $ => seq(
      '!',
      choice(
        $._type_identifier,
        $.nested_type_identifier,
        $.associated_type_projection,
        $.generic_type,
        $.lookup_type,
        $.index_type_query,
        $.type_query,
        $.parenthesized_type,
        $.predefined_type,
      ),
    ),

    ambient_declaration: $ => prec('declaration', seq(
      'declare',
      choice(
        $.global_declaration,
        $.abstract_class_declaration,
        $.class_declaration,
        $.struct_declaration,
        $.variable_declaration,
        $.lexical_declaration,
        $.type_alias_declaration,
        $.enum_declaration,
        $.interface_declaration,
      ),
    )),

    global_declaration: $ => seq(
      'global',
      field('body', $.statement_block),
    ),

    module_declaration: $ => seq(
      repeat(field('decorator', $.decorator)),
      'module',
      field('body', $.statement_block),
    ),

    abstract_class_declaration: $ => prec('declaration', seq(
      repeat(field('decorator', $.decorator)),
      field('place', optional($.placement_modifier)),
      'abstract',
      'class',
      field('name', $._type_identifier),
      field('type_parameters', optional($.type_parameters)),
      optional($.class_heritage),
      optional($.where_clause),
      field('body', $.class_body),
    )),

    class_declaration: $ => prec.left('declaration', seq(
      repeat(field('decorator', $.decorator)),
      field('place', optional($.placement_modifier)),
      optional('final'),
      'class',
      field('name', $._type_identifier),
      field('type_parameters', optional($.type_parameters)),
      optional($.class_heritage),
      optional($.where_clause),
      field('body', $.class_body),
      optional($._automatic_semicolon),
    )),

    struct_declaration: $ => prec.left('declaration', seq(
      repeat(field('decorator', $.decorator)),
      field('place', optional($.placement_modifier)),
      'struct',
      field('name', $._type_identifier),
      field('type_parameters', optional($.type_parameters)),
      optional($.implements_clause),
      optional($.where_clause),
      field('body', $.class_body),
      optional($._automatic_semicolon),
    )),

    struct_embedding_member: $ => seq(
      '...',
      field('type', $.type),
    ),

    extension_declaration: $ => seq(
      repeat(field('decorator', $.decorator)),
      'extension',
      field('name', optional($._type_identifier)),
      field('type_parameters', optional($.type_parameters)),
      'of',
      field('target', $.type),
      optional($.implements_clause),
      optional($.where_clause),
      field('body', $.class_body),
    ),

    nested_type_identifier: $ => prec('member', seq(
      field('module', choice($.identifier, $.nested_identifier)),
      '.',
      field('name', $._type_identifier),
    )),

    interface_declaration: $ => prec.right('declaration', seq(
      repeat(field('decorator', $.decorator)),
      field('place', optional($.placement_modifier)),
      optional('newtype'),
      'interface',
      field('name', optional($._type_identifier)),
      field('type_parameters', optional($.type_parameters)),
      optional($.extends_type_clause),
      optional($.where_clause),
      field('body', $.interface_body),
    )),

    interface_body: $ => seq(
      '{',
      optional(seq(
        sepBy1(choice(',', $._semicolon), $._interface_member),
        optional(choice(',', $._semicolon)),
      )),
      '}',
    ),

    _interface_member: $ => seq(
      repeat(field('guard', $.static_if_guard)),
      repeat(field('decorator', $.decorator)),
      choice(
        $.associated_type_declaration,
        $.associated_const_declaration,
        prec(1, $.method_signature),
        alias($.method_definition_member, $.method_definition),
        $.property_signature,
        $.call_signature,
        $.construct_signature,
        $.index_signature,
      ),
    ),

    extends_type_clause: $ => prec.right(seq(
      'extends',
      commaSep1(field('type', choice(
        $._type_identifier,
        $.nested_type_identifier,
        $.generic_type,
        $.object_type,
      ))),
    )),

    enum_declaration: $ => seq(
      repeat(field('decorator', $.decorator)),
      field('place', optional($.placement_modifier)),
      'enum',
      field('name', optional($.identifier)),
      field('type_parameters', optional($.type_parameters)),
      optional($.extends_type_clause),
      optional($.implements_clause),
      optional($.where_clause),
      field('body', $.enum_body),
    ),

    enum_body: $ => seq(
      '{',
      optional(seq(
        sepBy1(
          choice(',', $._semicolon),
          $._enum_member,
        ),
        optional(choice(',', $._semicolon)),
      )),
      '}',
    ),

    _enum_member: $ => seq(
      repeat(field('guard', $.static_if_guard)),
      repeat(field('decorator', $.decorator)),
      choice(
        field('name', $._property_name),
        $.enum_assignment,
        alias($.method_definition_member, $.method_definition),
        $.method_signature,
        $.enum_static_field,
      ),
    ),

    enum_assignment: $ => seq(
      field('name', $._property_name),
      $._initializer,
    ),

    enum_static_field: $ => seq(
      'static',
      optional('readonly'),
      field('name', $._property_name),
      optional('?'),
      field('type', optional($.type_annotation)),
      optional($._initializer),
    ),

    type_alias_declaration: $ => prec('declaration', seq(
      repeat(field('decorator', $.decorator)),
      field('place', optional($.placement_modifier)),
      choice(alias($._type_declaration_keyword, 'type'), 'newtype'),
      field('name', $._type_identifier),
      field('type_parameters', optional($.type_parameters)),
      '=',
      field('value', choice($.implements_type, $.type)),
      $._semicolon,
    )),

    associated_type_declaration: $ => seq(
      'type',
      field('name', $._type_identifier),
      field('type_parameters', optional($.type_parameters)),
      field('constraint', optional(seq(':', $.type))),
      field('value', optional(seq('=', $.type))),
    ),

    associated_const_declaration: $ => seq(
      'const',
      field('name', $._type_identifier),
      field('type', optional($.type_annotation)),
      optional($._initializer),
    ),

    accessibility_modifier: _ => choice(
      'public',
      'private',
      'protected',
    ),

    override_modifier: _ => 'override',

    placement_modifier: _ => token(prec(2, seq(choice('local', 'shared'), /[ \t]+/))),

    required_parameter: $ => seq(
      $._parameter_name,
      field('type', optional($.type_annotation)),
      optional($._initializer),
    ),

    optional_parameter: $ => prec.right(1, seq(
      $._parameter_name,
      token.immediate('?'),
      field('type', optional($.type_annotation)),
      optional($._initializer),
    )),

    _parameter_name: $ => seq(
      repeat(field('guard', $.static_if_guard)),
      repeat(field('decorator', $.decorator)),
      optional($.accessibility_modifier),
      optional('readonly'),
      field('pattern', choice(
        $.pattern,
        $.this,
        $.receiver_parameter,
        alias('in', $.identifier),
        alias('out', $.identifier),
        alias('match', $.identifier),
      )),
    ),

    // methods bind their receiver through an explicit memory form
    receiver_parameter: $ => seq(
      choice('^', seq('&', optional(choice('readonly', 'exclusive')))),
      $.this,
    ),

    _initializer: $ => (
      seq('=', field('value', choice(
        $.literal_try_propagation_expression,
        $.expression,
        $.if_expression,
        $.loop_expression,
        $.for_expression,
        $.while_expression,
        $.try_expression,
        $.using_assignment_expression,
        $.switch_statement,
        $.labeled_loop_initializer,
      )))
    ),

    literal_try_propagation_expression: $ => prec.left('unary', seq(
      field('argument', choice(
        $.number,
        $.string,
        $.true,
        $.false,
        $.null,
        $.undefined,
      )),
      token.immediate('?'),
    )),

    labeled_loop_initializer: $ => seq(
      field('label', alias(choice($.identifier, $._reserved_identifier), $.statement_identifier)),
      ':',
      field('value', $.loop_expression),
    ),

    omitting_type_annotation: $ => seq('-?:', $.type),
    adding_type_annotation: $ => seq('+?:', $.type),
    opting_type_annotation: $ => seq('?:', $.type),
    type_annotation: $ => seq(
      ':',
      optional(repeat1(field('decorator', $.decorator))),
      $.type,
    ),

    static_return_type_annotation: $ => seq(
      ':',
      optional(repeat1(field('decorator', $.decorator))),
      $.static_value_argument,
    ),

    type: $ => choice(
      $.function_type,
      $.constructor_type,
      $.conditional_union_type,
      $.static_conditional_type,
      $.conditional_type,
      $.key_in_type,
      $.primary_type,
      $.lifetime,
      $.optional_type,
      $.interval_type,
      $.negated_type,
      $.readonly_type,
    ),

    optional_type: $ => prec.right(seq($.primary_type, token.immediate('?'))),
    rest_type: $ => prec(1, seq('...', $.type)),
    negated_type: $ => prec.right('unary', seq('!', field('type', $.type))),

    _tuple_type_member: $ => choice(
      $.type,
      $.rest_type,
    ),

    constructor_type: $ => prec.left(seq(
      optional('abstract'),
      'new',
      field('type_parameters', optional($.type_parameters)),
      field('parameters', $.formal_parameters),
      '=>',
      field('type', $.type),
    )),

    primary_type: $ => choice(
      $.memory_type,
      $.placement_type,
      $.parenthesized_type,
      $.predefined_type,
      $._type_identifier,
      $.associated_type_projection,
      $.nested_type_identifier,
      $.generic_type,
      $.object_type,
      $.array_type,
      $.slice_type,
      $.fixed_array_type,
      $.tuple_type,
      $.type_query,
      $.index_type_query,
      alias($.this, $.this_type),
      $.literal_type,
      $.lookup_type,
      $.template_literal_type,
      $.infer_type,
      $.intersection_type,
      $.union_type,
      'const',
    ),

    template_type: $ => seq(
      '${',
      choice(
        $.type,
      ),
      '}',
    ),

    template_literal_type: $ => seq(
      '`',
      repeat(choice(
        alias($._template_chars, $.string_fragment),
        $.template_type,
      )),
      '`',
    ),

    infer_type: $ => prec.right(seq(
      'infer',
      $._type_identifier,
      optional(seq(
        'extends',
        $.type,
      )),
    )),

    conditional_type: $ => prec.right(2, seq(
      field('left', $.type),
      'extends',
      field('right', $.type),
      optional(seq(
        '?',
        field('consequence', $.type),
        ':',
        field('alternative', $.type),
      )),
    )),

    static_conditional_type: $ => prec.right(2, seq(
      field('condition', $.static_comparison),
      '?',
      field('consequence', $.type),
      ':',
      field('alternative', $.type),
    )),

    conditional_union_type: $ => prec.right(3, seq(
      field('left', $.type),
      'extends',
      field('right', $._conditional_union_right),
      '?',
      field('consequence', $.type),
      ':',
      field('alternative', $.type),
    )),

    _conditional_union_right: $ => seq(
      $.primary_type,
      repeat1(seq('|', $.primary_type)),
    ),

    key_in_type: $ => prec.left(2, seq(
      field('key', $.literal_type),
      'in',
      field('target', choice(
        $.primary_type,
        $.union_type,
        $.intersection_type,
      )),
    )),

    implements_type: $ => prec.left('binary_relation', seq(
      field('left', $.type),
      'implements',
      field('right', $.type),
    )),

    generic_type: $ => prec('call', seq(
      field('name', choice(
        $.associated_type_projection,
        $._type_identifier,
        $.nested_type_identifier,
      )),
      field('type_arguments', $.type_arguments),
    )),

    associated_type_projection: $ => prec('member', seq(
      field('owner', choice(
        $.associated_type_projection,
        $.generic_type,
        alias($.this, $.this_type),
      )),
      '.',
      field('name', $._type_identifier),
    )),

    // Type query expressions are more restrictive than regular expressions
    _type_query_member_expression: $ => seq(
      field('object', choice(
        $.identifier,
        $.this,
        alias($._type_query_subscript_expression, $.subscript_expression),
        alias($._type_query_member_expression, $.member_expression),
        alias($._type_query_call_expression, $.call_expression),
      )),
      choice('.', '?.'),
      field('property', choice(
        $.private_property_identifier,
        alias($.identifier, $.property_identifier),
      )),
    ),
    _type_query_subscript_expression: $ => seq(
      field('object', choice(
        $.identifier,
        $.this,
        alias($._type_query_subscript_expression, $.subscript_expression),
        alias($._type_query_member_expression, $.member_expression),
        alias($._type_query_call_expression, $.call_expression),
      )),
      optional('?.'),
      '[', field('index', choice($.predefined_type, $.string, $.number)), ']',
    ),
    _type_query_call_expression: $ => seq(
      field('function', choice(
        $.identifier,
        alias($._type_query_member_expression, $.member_expression),
        alias($._type_query_subscript_expression, $.subscript_expression),
      )),
      field('arguments', $.arguments),
    ),
    _type_query_instantiation_expression: $ => seq(
      field('function', choice(
        $.identifier,
        alias($._type_query_member_expression, $.member_expression),
        alias($._type_query_subscript_expression, $.subscript_expression),
      )),
      field('type_arguments', $.type_arguments),
    ),
    type_query: $ => prec.right(seq(
      'typeof',
      choice(
        alias($._type_query_subscript_expression, $.subscript_expression),
        alias($._type_query_member_expression, $.member_expression),
        alias($._type_query_call_expression, $.call_expression),
        alias($._type_query_instantiation_expression, $.instantiation_expression),
        $.identifier,
        $.this,
      ),
    )),

    index_type_query: $ => seq(
      'keyof',
      $.primary_type,
    ),

    lookup_type: $ => seq(
      $.primary_type,
      '[',
      choice(
        $.type,
        $.static_value_argument,
        $.as_expression,
      ),
      ']',
    ),

    mapped_type_clause: $ => seq(
      field('name', $._type_identifier),
      'in',
      field('type', $.type),
      optional(seq('as', field('alias', $.type))),
    ),

    literal_type: $ => choice(
      alias($._number, $.unary_expression),
      $.number,
      $.string,
      $.true,
      $.false,
      $.null,
      $.undefined,
    ),

    _number: $ => prec.left(1, seq(
      field('operator', choice('-', '+')),
      field('argument', $.number),
    )),

    parenthesized_type: $ => prec(1, seq(
      '(',
      choice($.function_type, $.type),
      ')',
    )),

    number: _ => {
      const hexLiteral = seq(
        choice('0x', '0X'),
        /[\da-fA-F](_?[\da-fA-F])*/,
      );

      const decimalDigits = /\d(_?\d)*/;
      const signedInteger = seq(optional(choice('-', '+')), decimalDigits);
      const exponentPart = seq(choice('e', 'E'), signedInteger);

      const binaryLiteral = seq(choice('0b', '0B'), /[0-1](_?[0-1])*/);
      const octalLiteral = seq(choice('0o', '0O'), /[0-7](_?[0-7])*/);
      const bigintLiteral = seq(choice(hexLiteral, binaryLiteral, octalLiteral, decimalDigits), 'n');

      const decimalIntegerLiteral = choice(
        '0',
        seq(optional('0'), /[1-9]/, optional(seq(optional('_'), decimalDigits))),
      );

      const decimalLiteral = choice(
        seq(decimalIntegerLiteral, '.', decimalDigits, optional(exponentPart)),
        seq('.', decimalDigits, optional(exponentPart)),
        seq(decimalIntegerLiteral, exponentPart),
        decimalDigits,
      );

      return token(choice(
        hexLiteral,
        decimalLiteral,
        binaryLiteral,
        octalLiteral,
        bigintLiteral,
      ));
    },

    predefined_type: _ => choice(
      'number',
      'boolean',
      'string',
      'void',
      'unknown',
      'never',
      'object',
    ),

    type_arguments: $ => seq(
      token.immediate(prec(1, '<')),
      commaSep1(
        choice(
          $.rest_type,
          $.type,
          $.explicit_type_argument,
          $.static_value_argument,
          $.const_type_argument,
        ),
      ),
      optional(','),
      '>',
    ),

    explicit_type_argument: $ => prec(1, seq(
      'type',
      $.type,
      optional(seq('=', field('value', $.type))),
    )),

    object_type: $ => seq(
      '{',
      optional(seq(
        sepBy1(
          choice(',', $._semicolon),
          $._object_type_member,
        ),
        optional(choice(',', $._semicolon)),
      )),
      '}',
    ),

    _object_type_member: $ => seq(
      repeat(field('guard', $.static_if_guard)),
      choice(
        $.associated_type_declaration,
        $.associated_const_declaration,
        $.property_signature,
        $.call_signature,
        $.construct_signature,
        $.index_signature,
        $.method_signature,
      ),
    ),

    call_signature: $ => $._call_signature,

    property_signature: $ => seq(
      optional($.accessibility_modifier),
      optional('static'),
      optional($.override_modifier),
      optional('readonly'),
      field('name', $._property_name),
      optional('?'),
      field('type', optional($.type_annotation)),
    ),

    _call_signature: $ => seq(
      field('type_parameters', optional($.type_parameters)),
      field('parameters', $.formal_parameters),
      field('return_type', optional(
        choice(
          $.type_annotation,
          $.static_return_type_annotation,
        ),
      )),
    ),

    type_parameters: $ => seq(
      '<', commaSep1($.type_parameter), optional(','), '>',
    ),

    type_parameter: $ => seq(
      choice(
        seq(
          repeat(field('guard', $.static_if_guard)),
          optional('const'),
          optional('in'),
          optional('out'),
          optional('...'),
          field('name', choice($._type_identifier, $.lifetime)),
          field('constraint', optional($.constraint)),
          field('value', optional($.default_type)),
        ),
        choice('in', 'out'),
      ),
    ),

    default_type: $ => seq(
      '=',
      choice($.type_value, $.static_value_argument, $.type, $.const_default_type),
    ),

    type_value: $ => prec.right('unary', seq(
      alias($._type_value_keyword, 'type'),
      field('value', $.type),
    )),

    const_default_type: $ => seq(
      'const',
      choice(
        $.number,
        $.string,
        $.true,
        $.false,
        $.identifier,
        $.static_value_argument,
      ),
    ),

    const_type_argument: $ => seq(
      'const',
      choice(
        $.number,
        $.string,
        $.true,
        $.false,
        seq(
          $.identifier,
          optional(seq('=', choice($.number, $.string, $.true, $.false, $.identifier))),
        ),
        $.static_value_argument,
      ),
    ),

    static_value_argument: $ => choice(
      $.parenthesized_expression,
      alias($._static_value_call_expression, $.call_expression),
      prec.left(seq(
        field('left', $._static_value_operand),
        field('operator', choice('+', '-', '*', '/', '%')),
        field('right', $._static_value_operand),
      )),
    ),

    _static_value_operand: $ => choice(
      $.generic_type,
      $.associated_type_projection,
      $.number,
      $.string,
      $.true,
      $.false,
      $.null,
      $.undefined,
      $.identifier,
      alias($.this, $.this_type),
      alias($._static_value_call_expression, $.call_expression),
      seq('(', $.static_value_argument, ')'),
    ),

    _static_value_call_expression: $ => seq(
      field('function', $.identifier),
      field('type_arguments', optional($.type_arguments)),
      field('arguments', $.arguments),
    ),

    constraint: $ => seq(
      ':',
      $.type,
    ),

    construct_signature: $ => seq(
      optional('abstract'),
      'new',
      field('type_parameters', optional($.type_parameters)),
      field('parameters', $.formal_parameters),
      field('type', optional($.type_annotation)),
    ),

    index_signature: $ => seq(
      optional(
        seq(
          field('sign', optional(choice('-', '+'))),
          'readonly',
        ),
      ),
      '[',
      choice(
        seq(
          field('name', choice(
            $.identifier,
            alias($._reserved_identifier, $.identifier),
          )),
          ':',
          field('index_type', $.type),
        ),
        $.mapped_type_clause,
      ),
      ']',
      field('type', choice(
        $.type_annotation,
        $.omitting_type_annotation,
        $.adding_type_annotation,
        $.opting_type_annotation,
      )),
    ),

    memory_type: $ => prec.right('unary', choice(
      seq('^', optional('readonly'), $.primary_type),
      seq(
        '&',
        field('lifetime', optional($.lifetime)),
        optional(choice('readonly', 'exclusive')),
        $.primary_type,
      ),
      seq('*', $.primary_type),
    )),
    placement_type: $ => prec.right('unary', seq(choice('local', 'shared'), $.primary_type)),
    interval_type: $ => prec.left('binary_relation', seq(
      field('left', optional($._interval_type_bound)),
      field('operator', choice('..', '..=')),
      field('right', optional($._interval_type_bound)),
    )),

    _interval_type_bound: $ => choice($.literal_type, $._type_identifier),

    array_type: $ => seq($.primary_type, '[', ']'),
    slice_type: $ => seq('[', $.type, ']'),
    fixed_array_type: $ => seq('[', $.type, ';', $._fixed_array_length, ']'),
    tuple_type: $ => choice(
      seq('(', ')'),
      seq('(', $.rest_type, ')'),
      seq('(', $._tuple_type_member, ',', ')'),
      seq('(', $._tuple_type_member, ',', commaSep1($._tuple_type_member), optional(','), ')'),
    ),
    readonly_type: $ => seq(
      'readonly',
      $.type,
    ),

    union_type: $ => prec.left(choice(
      seq($.type, '|', $.type),
      seq('|', $.type),
    )),
    intersection_type: $ => prec.left(seq($.type, '&', $.type)),

    function_type: $ => prec.left(seq(
      field('type_parameters', optional($.type_parameters)),
      field('parameters', choice(
        $.formal_parameters,
        alias($.predefined_type_parameters, $.formal_parameters),
      )),
      '=>',
      field('return_type', $.type),
    )),

    _fixed_array_length: $ => choice(
      $.static_value_argument,
      $.number,
      $.identifier,
      $.associated_type_projection,
      $.static_conditional,
    ),

    static_conditional: $ => prec.right(seq(
      field('condition', $.static_comparison),
      '?',
      field('consequence', $._fixed_array_length),
      ':',
      field('alternative', $._fixed_array_length),
    )),

    static_comparison: $ => prec.left(seq(
      field('left', $._static_value_operand),
      field('operator', choice('<', '<=', '>', '>=', '==', '!=')),
      field('right', $._static_value_operand),
    )),

    predefined_type_parameters: $ => seq(
      '(',
      commaSep1($.predefined_type),
      optional(','),
      ')',
    ),

    _type_identifier: $ => alias($.identifier, $.type_identifier),

    lifetime: $ => $._lifetime,

    _reserved_identifier: (_, previous) => choice(
      'declare',
      'type',
      'public',
      'private',
      'protected',
      'override',
      'readonly',
      'module',
      'local',
      'shared',
      'loop',
      'number',
      'boolean',
      'string',
      'export',
      'object',
      'new',
      previous,
    ),

  },
});

/**
 * Creates a rule to match one or more of the rules separated by a comma
 *
 * @param {RuleOrLiteral} rule
 *
 * @returns {SeqRule}
 */
function commaSep1(rule) {
  return sepBy1(',', rule);
}

/**
 * Creates a rule to optionally match one or more of the rules separated by a comma
 *
 * @param {RuleOrLiteral} rule
 *
 * @returns {SeqRule}
 */
function commaSep(rule) {
  return sepBy(',', rule);
}

/**
 * Creates a rule to optionally match one or more of the rules separated by a separator
 *
 * @param {RuleOrLiteral} sep
 *
 * @param {RuleOrLiteral} rule
 *
 * @returns {ChoiceRule}
 */
function sepBy(sep, rule) {
  return optional(sepBy1(sep, rule));
}

/**
 * Creates a rule to match one or more of the rules separated by a separator
 *
 * @param {RuleOrLiteral} sep
 *
 * @param {RuleOrLiteral} rule
 *
 * @returns {SeqRule}
 */
function sepBy1(sep, rule) {
  return seq(rule, repeat(seq(sep, rule)));
}

/**
 * Creates a token for one non-negative integer property name.
 *
 * @returns {ChoiceRule}
 */
function integerName() {
  const hexadecimal = seq(choice('0x', '0X'), /[\da-fA-F](_?[\da-fA-F])*/);
  const binary = seq(choice('0b', '0B'), /[0-1](_?[0-1])*/);
  const octal = seq(choice('0o', '0O'), /[0-7](_?[0-7])*/);
  const decimal = choice('0', /[1-9](_?\d)*/);

  return choice(hexadecimal, binary, octal, decimal);
}

/**
 * Check whether a conflict contains the same rules.
 *
 * @param {RuleOrLiteral[]} conflict
 *
 * @param {string[]} names
 *
 * @returns {boolean}
 */
function sameConflict(conflict, names) {
  if (conflict.length !== names.length) {
    return false;
  }

  return names.every((name) => conflict.some((rule) => rule.name === name));
}
