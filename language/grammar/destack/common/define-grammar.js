const JavaScript = require('tree-sitter-javascript/grammar');

module.exports = function defineGrammar(dialect) {
  return grammar(JavaScript, {
    name: dialect,

    externals: ($, previous) => previous.concat([
      $._function_signature_automatic_semicolon,
      $.__error_recovery,
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
        $.type_predicate,
        $.readonly_type,
      ],
      [$.mapped_type_clause, $.primary_expression],
      [$.accessibility_modifier, $.primary_expression],
      ['unary_void', $.expression],
      [$.extends_clause, $.primary_expression],
      ['unary', 'assign'],
      ['declaration', $.expression],
      [$.predefined_type, $.unary_expression],
      [$.type, $.flow_maybe_type],
      [$.tuple_type, $.array_type, $.pattern, $.type],
      [$.readonly_type, $.pattern],
      [$.readonly_type, $.primary_expression],
      [$.type_query, $.subscript_expression, $.expression],
      [$.type_query, $._type_query_subscript_expression],
      [$.nested_type_identifier, $.generic_type, $.primary_type, $.lookup_type, $.index_type_query, $.type],
      [$.as_expression, $.satisfies_expression, $.primary_type],
      [$._type_query_member_expression, $.member_expression],
      [$.member_expression, $._type_query_member_expression_in_type_annotation],
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
      [$.call_expression, $._type_query_call_expression_in_type_annotation],
      [$.new_expression, $.primary_expression],
      [$.meta_property, $.primary_expression],
      [$.construct_signature, $._property_name],
    ]),

    conflicts: ($, previous) => previous.concat([
      [$.call_expression, $.instantiation_expression, $.binary_expression],
      [$.call_expression, $.instantiation_expression, $.binary_expression, $.unary_expression],
      [$.call_expression, $.instantiation_expression, $.binary_expression, $.update_expression],
      [$.call_expression, $.instantiation_expression, $.binary_expression, $.await_expression],

      // This appears to be necessary to parse a parenthesized class expression
      [$.class],

      [$.nested_identifier, $.nested_type_identifier, $.primary_expression],
      [$.nested_identifier, $.nested_type_identifier],
      [$.primary_expression, $.nested_identifier],

      [$._call_signature, $.function_type],
      [$._call_signature, $.constructor_type],

      [$.primary_expression, $._parameter_name],
      [$.primary_expression, $._parameter_name, $.primary_type],
      [$.primary_expression, $.literal_type],
      [$.primary_expression, $.literal_type, $.rest_pattern],
      [$.primary_expression, $.predefined_type, $.rest_pattern],
      [$.primary_expression, $.primary_type],
      [$.primary_expression, $.generic_type],
      [$.primary_expression, $._struct_literal_generic_type],
      [$.primary_expression, $._struct_literal_generic_type, $.generic_type],
      [$.primary_expression, $.predefined_type],
      [$.primary_expression, $.pattern, $.primary_type],
      [$._parameter_name, $.primary_type],
      [$.pattern, $.primary_type],

      [$.rest_pattern, $.primary_type, $.primary_expression],

      [$.object, $.object_type],
      [$.object, $.object_pattern, $.object_type],
      [$.object, $.object_pattern, $._property_name],
      [$.object_pattern, $.object_type],

      [$.array, $.tuple_type],
      [$.array, $.array_pattern, $.tuple_type],
      [$.array_pattern, $.tuple_type],
      [$.template_literal_type, $.template_string],
      [$.primary_expression, $.struct_literal_expression],
      [$.struct_literal_expression, $._extends_clause_single],
      [$.primary_type, $.associated_type_projection],
      [$.expression, $.dereference_expression],
      [$.lookup_type, $.pointer_type, $.array_type],
      [$.lookup_type, $.managed_type, $.array_type],
      [$.type, $.pointer_type],
      [$.type, $.borrow_type],
      [$.type, $.managed_type],
      [$.primary_expression, $.associated_type_projection],
      [$.if_statement, $.primary_expression],
      [$.if_statement, $.parenthesized_expression],
      [$.expression, $.borrow_expression],
      [$.expression, $.managed_expression],
      [$.primary_expression, $.borrow_expression],
      [$.primary_expression, $.managed_expression],
      [$.primary_expression, $._range_expression_atom],
      [$.primary_expression, $._range_expression_atom, $.static_value_argument],
      [$._range_expression_atom, $.literal_type],
      [$.primary_expression, $._range_expression_atom, $.literal_type],
      ...(dialect === 'destack' ? [[$.range_expression, $.interval_type]] : []),
      [$._range_expression_bound],
      [$.primary_expression, $.comptime_type_argument],
    ]).concat(
      dialect === 'destack' ? [
        [$.comptime_block_statement, $.comptime_expression],
        [$._destructuring_pattern, $.destack_if_let_pattern],
        [$.primary_expression, $.function_expression, $.generator_function],
        [$.primary_type, $.type_predicate],
        [$.optional_type, $.primary_type],
        [$.primary_type, $.destack_predefined_type_parameters],
        [$.formal_parameters, $.tuple_type],
        [$.asserts, $.type_predicate],
        [$.readonly_type, $.intersection_type],
        [$.readonly_type, $.union_type],
        [$.pattern, $.try_propagation_expression],
        [$.call_expression, $.match_arm_expression_statement],
        [$.subscript_expression, $.match_arm_expression_statement],
        [$.binary_expression, $.match_arm_expression_statement],
      ] : [],
    ).concat(
      dialect === 'typescript' ? [
        [$.primary_type, $.type_parameter],
      ] : [
        [$.jsx_opening_element, $.type_parameter],
        [$.jsx_namespace_name, $.primary_type],
      ],
    ),

    inline: ($, previous) => previous
      .filter((rule) => ![
        '_formal_parameter',
        '_call_signature',
      ].includes(rule.name))
      .concat([
        $._type_identifier,
        $._jsx_start_opening_element,
        ...(dialect === 'destack' ? [$.destack_member_name] : []),
      ]),

    rules: {
      public_field_definition: $ => {
        if (dialect === 'destack') {
          return seq(
            repeat(field('decorator', $.decorator)),
            optional('static'),
            optional('readonly'),
            field('name', $._property_name),
            optional(choice('?', '!')),
            field('type', optional($.type_annotation)),
            optional($._initializer),
          );
        }

        return seq(
          repeat(field('decorator', $.decorator)),
          optional(choice(
            seq('declare', optional($.accessibility_modifier)),
            seq($.accessibility_modifier, optional('declare')),
          )),
          choice(
            seq(optional('static'), optional($.override_modifier), optional('readonly')),
            seq(optional('abstract'), optional('readonly')),
            seq(optional('readonly'), optional('abstract')),
            optional('accessor'),
          ),
          field('name', $._property_name),
          optional(choice('?', '!')),
          field('type', optional($.type_annotation)),
          optional($._initializer),
        );
      },

      try_statement: ($, previous) => {
        if (dialect !== 'destack') {
          return previous;
        }

        return seq(
          'try',
          field('body', $.statement_block),
          optional(field('handler', choice($.catch_match_clause, $.catch_clause))),
          optional(field('finalizer', $.finally_clause)),
        );
      },

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
              optional(
                // only types that resolve to 'any' or 'unknown' are supported
                // by the language but it's simpler to accept any type here.
                field('type', $.type_annotation),
              ),
              ')',
            ),
            ...(dialect === 'destack' ?
              [field('parameter', choice($.identifier, $._destructuring_pattern))] :
              []),
          ),
        ),
        field('body', $.statement_block),
      ),

      catch_match_clause: $ => {
        if (dialect !== 'destack') {
          return $.catch_clause;
        }

        return seq(
          'catch',
          'match',
          '(',
          field('value', $.expression),
          ')',
          '{',
          repeat($.match_arm),
          '}',
        );
      },

      call_expression: $ => choice(
        prec('call', seq(
          field('function', choice($.expression, $.import)),
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

      assignment_expression: ($, previous) => {
        if (dialect !== 'destack') {
          return previous;
        }

        return previous;
      },

      using_assignment_expression: $ => prec.right('assign', seq(
        optional('await'),
        'using',
        field('left', choice(
          $.identifier,
          $._destructuring_pattern,
          $.tuple_pattern,
          $.struct_pattern,
        )),
        '=',
        field('right', $.expression),
      )),

      annotation_call_expression: $ => {
        if (dialect !== 'destack') {
          return $.call_expression;
        }

        return prec.right('unary', seq(
          '@',
          field('function', choice(
            $.identifier,
            alias($.decorator_member_expression, $.member_expression),
          )),
          field('arguments', $.arguments),
        ));
      },

      _augmented_assignment_lhs: ($, previous) => (
        dialect === 'destack' ? previous : choice(previous, $.non_null_expression)
      ),

      _lhs_expression: ($, previous) => (
        dialect === 'destack' ? previous : choice(previous, $.non_null_expression)
      ),

      primary_expression: ($, previous) => choice(
        ...(dialect === 'destack' ? [
          $.struct_literal_expression,
          $.annotation_call_expression,
          $.try_propagation_expression,
          $.managed_expression,
          $.borrow_expression,
          $.dereference_expression,
          $.comptime_expression,
          $.do_expression,
        ] : []),
        previous,
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

      // If the dialect is regular typescript, we exclude JSX expressions and
      // include type assertions. If the dialect is TSX, we do the opposite.
      expression: ($, previous) => {
        const choices = [
          $.as_expression,
          $.satisfies_expression,
          $.instantiation_expression,
        ];

        if (dialect === 'destack') {
          choices.push($.range_expression);
        }

        if (dialect !== 'destack') {
          choices.push($.internal_module);
        }

        if (dialect === 'typescript') {
          choices.push($.type_assertion);
          choices.push(...previous.members.filter((member) =>
            member.name !== '_jsx_element',
          ));
        } else if (dialect === 'tsx' || dialect === 'destack') {
          choices.push(...previous.members);
        } else {
          throw new Error(`Unknown dialect ${dialect}`);
        }

        return choice(...choices);
      },

      if_expression: $ => prec.right(seq(
        'if',
        choice(
          seq(
            '(',
            field('condition', choice(
              $.expression,
              $.destack_if_let_condition,
            )),
            ')',
            field('consequence', $.statement_block),
            field('alternative', $.else_clause),
          ),
          seq(
            'let',
            field('left', $.destack_if_let_pattern),
            field('type', optional($.type_annotation)),
            '=',
            field('right', $._destack_if_let_right_expression),
            field('consequence', $.statement_block),
            optional(field('alternative', $.else_clause)),
          ),
        ),
      )),

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

      statement: ($, previous) => {
        if (dialect !== 'destack') {
          return choice(previous);
        }

        return choice(
          $.destack_decorated_statement,
          $.destack_static_if_statement,
          $.comptime_block_statement,
          $.destack_for_in_statement,
          $.let_else_statement,
          $.using_assignment_statement,
          $.match_statement,
          previous,
        );
      },

      let_else_statement: $ => {
        if (dialect !== 'destack') {
          return $.identifier;
        }

        return prec.right('declaration', seq(
          field('kind', choice('let', 'const')),
          field('left', $.match_constructor_pattern),
          field('type', optional($.type_annotation)),
          '=',
          field('right', $.expression),
          'else',
          field('alternative', $.statement_block),
          $._semicolon,
        ));
      },

      using_assignment_statement: $ => seq(
        field('expression', $.using_assignment_expression),
        $._semicolon,
      ),

      if_statement: ($, previous) => {
        if (dialect !== 'destack') {
          return previous;
        }

        return choice(
          prec.right(seq(
            'if',
            'let',
            field('left', $.destack_if_let_pattern),
            field('type', optional($.type_annotation)),
            '=',
            field('right', $._destack_if_let_right_expression),
            field('consequence', $.statement_block),
            optional(field('alternative', $.else_clause)),
          )),
          prec.right(seq(
            'if',
            '(',
            field('condition', choice(
              $.expression,
              $.destack_if_let_condition,
            )),
            ')',
            field('consequence', $.statement_block),
            optional(field('alternative', $.else_clause)),
          )),
          prec.right(seq(
            'if',
            field('condition', choice(
              $.expression,
              $.destack_if_let_condition,
            )),
            field('consequence', $.statement_block),
            optional(field('alternative', $.else_clause)),
          )),
          previous,
        );
      },

      _for_header: ($, previous) => {
        if (dialect !== 'destack') {
          return previous;
        }

        return seq(
          '(',
          choice(
            field('left', choice(
              $._lhs_expression,
              $.parenthesized_expression,
            )),
            seq(
              field('kind', 'var'),
              field('left', choice(
                $.identifier,
                $._destructuring_pattern,
              )),
              optional($._initializer),
            ),
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
          field('operator', choice('in', 'of')),
          field('right', $._expressions),
          ')',
        );
      },

      switch_case: ($, previous) => {
        if (dialect !== 'destack') {
          return previous;
        }

        return seq(
          'case',
          field('value', $._expressions),
          optional(seq('if', field('guard', $.expression))),
          ':',
          field('body', repeat($.statement)),
        );
      },

      match_expression: $ => seq(
        'match',
        '(',
        field('value', $.expression),
        ')',
        '{',
        repeat($.match_arm),
        '}',
      ),

      match_statement: $ => seq(
        'match',
        '(',
        field('value', $.expression),
        ')',
        '{',
        repeat($.match_arm),
        '}',
      ),

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
        $.throw_statement,
        $.empty_statement,
      ),

      match_arm_expression_statement: $ => prec.dynamic(1, seq(
        $.expression,
        optional(choice(',', $._semicolon, $._automatic_semicolon)),
      )),

      match_pattern: $ => prec(1, choice(
        $.union_pattern,
        $.must_pattern,
        $.managed_pattern,
        $.borrow_pattern,
        $.dereference_pattern,
        $.default_pattern,
        $.range_pattern,
        $.match_constructor_pattern,
        $.match_struct_pattern,
        $.match_member_pattern,
        $.tuple_pattern,
        $.rest_pattern,
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
          field('name', $._property_name),
          optional(seq(':', field('value', $.match_pattern))),
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

      managed_pattern: $ => prec.left('unary', seq(
        '^',
        optional('readonly'),
        field('argument', $.match_pattern),
      )),

      borrow_pattern: $ => prec.left('unary', seq(
        '&',
        optional(choice('readonly', 'exclusive')),
        field('argument', $.match_pattern),
      )),

      dereference_pattern: $ => prec.left('unary', seq(
        '*',
        field('argument', $.match_pattern),
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
        field('name', $.identifier),
        '(',
        commaSep1($.match_pattern),
        optional(','),
        ')',
      )),

      match_struct_pattern: $ => prec(2, seq(
        field('name', $.identifier),
        field('pattern', $.match_object_pattern),
      )),

      tuple_pattern: $ => seq(
        '(',
        commaSep1($.match_pattern),
        optional(','),
        ')',
      ),

      where_clause: $ => seq(
        'where',
        choice(
          $.where_constraint,
          seq(
            '(',
            commaSep1($.where_constraint),
            optional(','),
            ')',
          ),
        ),
      ),

      where_constraint: $ => seq(
        field('name', $._type_identifier),
        ':',
        field('type', $.type),
      ),

      loop_expression: $ => seq(
        'loop',
        field('body', $.statement_block),
      ),

      do_expression: $ => prec.right(seq(
        'do',
        field('body', $.statement_block),
      )),

      while_expression: $ => prec.right(1, alias($.while_statement, $.while_expression)),

      for_expression: $ => prec.right(1, alias($.for_statement, $.for_expression)),

      try_expression: $ => prec.right(1, alias($.try_statement, $.try_expression)),

      try_propagation_expression: $ => prec.left('unary', seq(
        field('argument', choice(
          $.parenthesized_expression,
          $.member_expression,
          $.subscript_expression,
          $.call_expression,
          $.annotation_call_expression,
        )),
        token.immediate('?'),
      )),

      comptime_expression: $ => prec.right(seq(
        'comptime',
        field('value', choice(
          $.statement_block,
          $.expression,
        )),
      )),

      break_statement: ($, previous) => {
        if (dialect !== 'destack') {
          return previous;
        }

        return prec(1, seq(
          'break',
          optional(choice(
            seq(
              field('label', alias($.identifier, $.statement_identifier)),
              optional(seq(':', field('value', $.expression))),
            ),
            field('value', $.expression),
          )),
          $._semicolon,
        ));
      },

      managed_expression: $ => prec.left('unary', seq(
        '^',
        optional('readonly'),
        field('argument', choice(
          $.primary_expression,
          $.new_expression,
          $.call_expression,
        )),
      )),

      borrow_expression: $ => prec.left('unary', seq(
        '&',
        optional('readonly'),
        field('argument', choice(
          $.primary_expression,
          $.new_expression,
          $.call_expression,
        )),
      )),

      dereference_expression: $ => prec.right('unary', seq(
        '*',
        field('argument', $.primary_expression),
      )),

      comptime_block_statement: $ => seq(
        'comptime',
        field('body', $.statement_block),
      ),

      destack_for_in_statement: $ => prec.right('declaration', seq(
        'for',
        field('left', choice($.identifier, $._destructuring_pattern)),
        'in',
        field('right', $.expression),
        field('body', $.statement_block),
      )),

      destack_decorated_statement: $ => prec.right('declaration', seq(
        repeat1(field('decorator', $.decorator)),
        choice(
          $.lexical_declaration,
          $.variable_declaration,
          $._destack_statement_target,
        ),
      )),

      destack_static_if_guard: $ => prec.right(choice(
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

      destack_static_if_statement: $ => prec.right('declaration', seq(
        repeat1(field('guard', $.destack_static_if_guard)),
        choice(
          $.declaration,
          $._destack_statement_target,
        ),
      )),

      _destack_statement_target: $ => choice(
        $.expression_statement,
        $.statement_block,
        $.if_statement,
        $.switch_statement,
        $.for_statement,
        $.for_in_statement,
        $.while_statement,
        $.do_statement,
        $.try_statement,
        $.with_statement,
        $.break_statement,
        $.continue_statement,
        $.return_statement,
        $.throw_statement,
        $.empty_statement,
        $.labeled_statement,
        $.destack_for_in_statement,
        $.comptime_block_statement,
        $.match_statement,
      ),

      destack_static_if_expression: $ => prec.right('declaration', seq(
        repeat1(field('guard', $.destack_static_if_guard)),
        field('value', $.expression),
      )),

      destack_if_let_pattern: $ => prec.right('if_let_pattern', choice(
        $.identifier,
        $.number,
        $.string,
        $.true,
        $.false,
        $.null,
        $.undefined,
        $.this,
        $.super,
        $.array_pattern,
        $.object_pattern,
        $.rest_pattern,
        $.tuple_pattern,
        $.call_expression,
        $.struct_literal_expression,
        $.member_expression,
      )),

      destack_if_let_condition: $ => prec.right('if_let_pattern', seq(
        'const',
        field('left', $.destack_if_let_pattern),
        field('type', optional($.type_annotation)),
        '=',
        field('right', $._destack_if_let_right_expression),
      )),

      _destack_if_let_right_expression: $ => prec.right('if_let_right', choice(
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

      // This rule is only referenced by expression when the dialect is 'tsx'
      jsx_opening_element: $ => prec.dynamic(-1, seq(
        $._jsx_start_opening_element,
        '>',
      )),

      // tsx only. See jsx_opening_element.
      jsx_self_closing_element: $ => prec.dynamic(-1, seq(
        $._jsx_start_opening_element,
        '/>',
      )),

      export_specifier: (_, previous) => seq(
        optional(choice('type', 'typeof')),
        previous,
      ),

      _import_identifier: $ => choice($.identifier, alias('type', $.identifier)),

      import_specifier: $ => seq(
        optional(choice('type', 'typeof')),
        choice(
          field('name', $._import_identifier),
          seq(
            field('name', choice($._module_export_name, alias('type', $.identifier))),
            'as',
            field('alias', $._import_identifier),
          ),
        ),
      ),

      import_attribute: $ => seq(choice('with', 'assert'), $.object),

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
        optional(choice('type', 'typeof')),
        choice(
          seq($.import_clause, $._from_clause),
          $.import_require_clause,
          field('source', $.string),
        ),
        optional($.import_attribute),
        $._semicolon,
      ),

      arguments: ($, previous) => {
        if (dialect !== 'destack') {
          return previous;
        }

        return seq(
          '(',
          optional(seq(
            optional(choice(
              $.destack_static_if_expression,
              $.expression,
              $.spread_element,
            )),
            repeat(seq(
              ',',
              optional(choice(
                $.destack_static_if_expression,
                $.expression,
                $.spread_element,
              )),
            )),
          )),
          ')',
        );
      },

      export_statement: ($, previous) => choice(
        previous,
        seq(
          'export',
          'type',
          $.export_clause,
          optional($._from_clause),
          $._semicolon,
        ),
        seq('export', '=', $.expression, $._semicolon),
        seq('export', 'as', 'namespace', $.identifier, $._semicolon),
      ),

      non_null_expression: $ => prec.left('unary', seq(
        $.expression, '!',
      )),

      variable_declarator: $ => choice(
        seq(
          field('name', choice($.identifier, $._destructuring_pattern, $.tuple_pattern, $.struct_pattern)),
          field('type', optional($.type_annotation)),
          optional($._initializer),
        ),
        prec('declaration', seq(
          field('name', $.identifier),
          '!',
          field('type', $.type_annotation),
        )),
      ),

      struct_pattern: $ => seq(
        field('type', $.identifier),
        field('pattern', $.object_pattern),
      ),

      destack_member_name: $ => choice(
        alias($.identifier, $.property_identifier),
        $.computed_property_name,
      ),

      destack_method_signature_member: $ => choice(
        seq(
          optional('static'),
          optional('async'),
          field('name', $.destack_member_name),
          optional('?'),
          $._call_signature,
        ),
        seq(
          optional('static'),
          optional('async'),
          choice('get', 'set'),
          field('name', $.destack_member_name),
          optional('?'),
          $._call_signature,
        ),
      ),

      destack_method_definition_member: $ => choice(
        prec.left(seq(
          optional('static'),
          optional('async'),
          field('name', $.destack_member_name),
          optional('?'),
          $._call_signature,
          field('body', choice(
            $.statement_block,
            $.placeholder_block,
          )),
        )),
        prec.left(seq(
          optional('static'),
          optional('async'),
          choice('get', 'set'),
          field('name', $.destack_member_name),
          optional('?'),
          $._call_signature,
          field('body', choice(
            $.statement_block,
            $.placeholder_block,
          )),
        )),
      ),

      destack_field_member: $ => seq(
        optional('static'),
        optional('readonly'),
        field('name', $.destack_member_name),
        optional(choice('?', '!')),
        field('type', optional($.type_annotation)),
        optional($._initializer),
      ),

      destack_enum_assignment_member: $ => seq(
        field('name', $.destack_member_name),
        $._initializer,
      ),

      destack_enum_static_field_member: $ => seq(
        'static',
        optional('readonly'),
        field('name', $.destack_member_name),
        optional(choice('?', '!')),
        field('type', optional($.type_annotation)),
        optional($._initializer),
      ),

      method_signature: $ => seq(
        ...(dialect === 'destack' ?
          [
            optional('static'),
            optional('async'),
          ] :
          [
            optional($.accessibility_modifier),
            optional('static'),
            optional($.override_modifier),
            optional('readonly'),
            optional('async'),
          ]),
        optional(choice('get', 'set', ...(dialect === 'destack' ? [] : ['*']))),
        field('name', $._property_name),
        optional('?'),
        $._call_signature,
      ),

      abstract_method_signature: $ => seq(
        optional($.accessibility_modifier),
        'abstract',
        optional($.override_modifier),
        optional(choice('get', 'set', '*')),
        field('name', $._property_name),
        optional('?'),
        $._call_signature,
      ),

      parenthesized_expression: $ => seq(
        '(',
        choice(
          seq($.expression, field('type', optional($.type_annotation))),
          $.sequence_expression,
        ),
        ')',
      ),

      _formal_parameter: $ => choice(
        $.optional_parameter,
        $.required_parameter,
      ),

      function_signature: $ => seq(
        repeat(field('decorator', $.decorator)),
        ...(dialect === 'destack' ? [optional('comptime')] : []),
        optional('async'),
        'function',
        field('name', $.identifier),
        $._call_signature,
        ...(dialect === 'destack' ? [optional($.where_clause)] : []),
        choice($._semicolon, $._function_signature_automatic_semicolon),
      ),

      destack_declare_function_signature: $ => {
        if (dialect !== 'destack') {
          return $.function_signature;
        }

        return seq(
          repeat(field('decorator', $.decorator)),
          'declare',
          optional('comptime'),
          optional('async'),
          'function',
          field('name', $.identifier),
          $._call_signature,
          optional($.where_clause),
          choice($._semicolon, $._function_signature_automatic_semicolon),
        );
      },

      destack_declare_generator_function_signature: $ => {
        return seq(
          repeat(field('decorator', $.decorator)),
          'declare',
          optional('comptime'),
          optional('async'),
          'function',
          '*',
          field('name', $.identifier),
          $._call_signature,
          optional($.where_clause),
          choice($._semicolon, $._function_signature_automatic_semicolon),
        );
      },

      function_declaration: ($, previous) => {
        if (dialect !== 'destack') {
          return previous;
        }

        return prec.right('declaration', seq(
          repeat(field('decorator', $.decorator)),
          optional('comptime'),
          optional('async'),
          'function',
          field('name', $.identifier),
          $._call_signature,
          optional($.where_clause),
          field('body', $.statement_block),
          optional($._automatic_semicolon),
        ));
      },

      generator_function_declaration: ($, previous) => {
        if (dialect !== 'destack') {
          return previous;
        }

        return prec.right('declaration', seq(
          repeat(field('decorator', $.decorator)),
          optional('comptime'),
          optional('async'),
          'function',
          '*',
          field('name', $.identifier),
          $._call_signature,
          optional($.where_clause),
          field('body', $.statement_block),
          optional($._automatic_semicolon),
        ));
      },

      decorator: $ => seq(
        '@',
        choice(
          $.identifier,
          alias($.decorator_member_expression, $.member_expression),
          alias($.decorator_call_expression, $.call_expression),
          alias($.decorator_instantiation_expression, $.instantiation_expression),
          alias($.decorator_parenthesized_expression, $.parenthesized_expression),
        ),
      ),

      decorator_member_expression: $ => prec('member', seq(
        field('object', choice(
          $.identifier,
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

      class_body: $ => {
        if (dialect !== 'destack') {
          return seq(
            '{',
            repeat(choice(
              seq(
                repeat(field('decorator', $.decorator)),
                $.method_definition,
                optional($._semicolon),
              ),
              seq($.method_signature, choice($._function_signature_automatic_semicolon, ',')),
              seq(
                choice(
                  $.index_signature,
                  $.method_signature,
                  $.public_field_definition,
                ),
                choice($._semicolon, ','),
              ),
              ';',
            )),
            '}',
          );
        }

        return seq(
          '{',
          repeat(choice(
            seq(
              repeat(field('decorator', $.decorator)),
              alias($.destack_method_definition_member, $.method_definition),
              optional($._semicolon),
            ),
            seq(
              repeat(field('decorator', $.decorator)),
              alias($.destack_method_signature_member, $.method_signature),
              choice($._function_signature_automatic_semicolon, $._semicolon, ','),
            ),
            seq(
              $.struct_embedding_member,
              optional(choice($._semicolon, ',')),
            ),
            seq(
              repeat(field('decorator', $.decorator)),
              choice(
                $.associated_type_declaration,
                $.associated_const_declaration,
                $.comptime_block_statement,
                $.index_signature,
                alias($.destack_field_member, $.public_field_definition),
              ),
              choice($._semicolon, ','),
            ),
            ';',
          )),
          '}',
        );
      },

      method_definition: $ => prec.left(seq(
        ...(dialect === 'destack' ?
          [
            optional('static'),
            optional('async'),
          ] :
          [
            optional($.accessibility_modifier),
            optional('static'),
            optional($.override_modifier),
            optional('readonly'),
            optional('async'),
          ]),
        optional(choice('get', 'set', ...(dialect === 'destack' ? [] : ['*']))),
        field('name', $._property_name),
        optional('?'),
        $._call_signature,
        field('body', choice(
          $.statement_block,
          ...(dialect === 'destack' ? [$.placeholder_block] : []),
        )),
      )),

      class_static_block: ($, previous) => {
        if (dialect !== 'destack') {
          return previous;
        }

        return seq(
          'static',
          $.statement_block,
        );
      },

      declaration: ($, previous) => choice(
        previous,
        ...(dialect === 'destack' ? [$.struct_declaration] : []),
        ...(dialect === 'destack' ? [$.destack_declare_function_signature] : []),
        ...(dialect === 'destack' ? [$.destack_declare_generator_function_signature] : []),
        $.function_signature,
        ...(dialect === 'destack' ? [] : [$.abstract_class_declaration]),
        ...(dialect === 'destack' ? [] : [$.module]),
        ...(dialect === 'destack' ? [] : [prec('declaration', $.internal_module)]),
        ...(dialect === 'destack' ? [$.extension_declaration] : []),
        $.type_alias_declaration,
        $.enum_declaration,
        $.interface_declaration,
        $.import_alias,
        $.ambient_declaration,
      ),

      type_assertion: $ => prec.left('unary', seq(
        $.type_arguments,
        $.expression,
      )),

      as_expression: $ => prec.left('binary', seq(
        $.expression,
        'as',
        choice('const', $.type),
      )),

      satisfies_expression: $ => prec.left('binary', seq(
        $.expression,
        'satisfies',
        $.type,
      )),

      instantiation_expression: $ => prec('instantiation', seq(
        $.expression,
        field('type_arguments', $.type_arguments),
      )),

      binary_expression: ($, previous) => {
        if (dialect !== 'destack') {
          return previous;
        }

        return choice(
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
        );
      },

      class_heritage: $ => choice(
        seq($.extends_clause, optional($.implements_clause)),
        $.implements_clause,
      ),

      import_require_clause: $ => seq(
        $.identifier,
        '=',
        'require',
        '(',
        field('source', $.string),
        ')',
      ),

      extends_clause: $ => seq(
        'extends',
        commaSep1($._extends_clause_single),
      ),

      _extends_clause_single: $ => prec('extends', seq(
        field('value', dialect === 'destack' ? choice(
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
        ) : $.expression),
        ...(dialect === 'destack' ? [] : [field('type_arguments', optional($.type_arguments))]),
      )),

      implements_clause: $ => seq(
        'implements',
        commaSep1(dialect === 'destack' ? $._implements_clause_single : $.type),
      ),

      _implements_clause_single: $ => choice(
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

      ambient_declaration: $ => {
        if (dialect !== 'destack') {
          return seq(
            'declare',
            choice(
              $.declaration,
              seq('global', $.statement_block),
              seq('module', '.', alias($.identifier, $.property_identifier), ':', $.type, $._semicolon),
            ),
          );
        }

        return seq(
          'declare',
          choice(
            $.ambient_const_declaration,
            prec('declaration', $.internal_module),
          ),
        );
      },

      ambient_const_declaration: $ => seq(
        'const',
        commaSep1($.variable_declarator),
        $._semicolon,
      ),

      placeholder_block: $ => seq(
        '{',
        '...',
        '}',
      ),

      class: $ => prec('literal', seq(
        repeat(field('decorator', $.decorator)),
        'class',
        field('name', optional($._type_identifier)),
        field('type_parameters', optional($.type_parameters)),
        optional($.class_heritage),
        field('body', $.class_body),
      )),

      abstract_class_declaration: $ => prec('declaration', seq(
        repeat(field('decorator', $.decorator)),
        'abstract',
        'class',
        field('name', $._type_identifier),
        field('type_parameters', optional($.type_parameters)),
        optional($.class_heritage),
        field('body', $.class_body),
      )),

      class_declaration: $ => prec.left('declaration', seq(
        repeat(field('decorator', $.decorator)),
        'class',
        field('name', $._type_identifier),
        field('type_parameters', optional($.type_parameters)),
        optional($.class_heritage),
        field('body', $.class_body),
        optional($._automatic_semicolon),
      )),

      struct_declaration: $ => prec.left('declaration', seq(
        repeat(field('decorator', $.decorator)),
        'struct',
        field('name', $._type_identifier),
        field('type_parameters', optional($.type_parameters)),
        optional($.implements_clause),
        field('body', $.class_body),
        optional($._automatic_semicolon),
      )),

      struct_embedding_member: $ => seq(
        '...',
        field('type', $.type),
      ),

      extension_declaration: $ => seq(
        'extension',
        field('name', optional($._type_identifier)),
        field('type_parameters', optional($.type_parameters)),
        'for',
        field('target', $.type),
        optional($.implements_clause),
        field('body', $.class_body),
      ),

      module: $ => seq(
        'module',
        $._module,
      ),

      internal_module: $ => seq(
        'namespace',
        dialect === 'destack' ?
          seq(
            field('name', choice($.identifier, $.nested_identifier)),
            field('body', $.statement_block),
          ) :
          $._module,
      ),

      _module: $ => prec.right(seq(
        field('name', choice($.string, $.identifier, $.nested_identifier)),
        // On .d.ts files "declare module foo" desugars to "declare module foo {}",
        // hence why it is optional here
        field('body', optional($.statement_block)),
      )),

      import_alias: $ => seq(
        'import',
        $.identifier,
        '=',
        choice($.identifier, $.nested_identifier),
        $._semicolon,
      ),

      nested_type_identifier: $ => prec('member', seq(
        field('module', choice($.identifier, $.nested_identifier)),
        '.',
        field('name', $._type_identifier),
      )),

      interface_declaration: $ => prec.right('declaration', seq(
        ...(dialect === 'destack' ? [optional('newtype')] : []),
        'interface',
        field('name', $._type_identifier),
        field('type_parameters', optional($.type_parameters)),
        optional($.extends_type_clause),
        field(
          'body',
          (dialect === 'destack' ?
            $.interface_body :
            alias($.object_type, $.interface_body)),
        ),
      )),

      interface_body: $ => {
        if (dialect !== 'destack') {
          return alias($.object_type, $.interface_body);
        }

        return seq(
          '{',
          optional(seq(
            sepBy1(choice(',', $._semicolon), $._destack_interface_member),
            optional(choice(',', $._semicolon)),
          )),
          '}',
        );
      },

      _destack_interface_member: $ => seq(
        repeat(field('guard', $.destack_static_if_guard)),
        repeat(field('decorator', $.decorator)),
        choice(
          $.associated_type_declaration,
          $.associated_const_declaration,
          prec(1, alias($.destack_method_signature_member, $.method_signature)),
          alias($.destack_method_definition_member, $.method_definition),
          prec(1, alias($.destack_field_member, $.destack_property_signature)),
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
          ...(dialect === 'destack' ? [$.object_type] : []),
        ))),
      )),

      enum_declaration: $ => seq(
        optional('const'),
        'enum',
        field('name', $.identifier),
        field('body', $.enum_body),
      ),

      enum_body: $ => seq(
        '{',
        optional(seq(
          sepBy1(
            dialect === 'destack' ? choice(',', $._semicolon) : ',',
            dialect === 'destack' ?
              $._destack_enum_member :
              choice(
                field('name', $._property_name),
                $.enum_assignment,
              ),
          ),
          optional(dialect === 'destack' ? choice(',', $._semicolon) : ','),
        )),
        '}',
      ),

      _destack_enum_member: $ => seq(
        repeat(field('guard', $.destack_static_if_guard)),
        repeat(field('decorator', $.decorator)),
        choice(
          field('name', $.destack_member_name),
          alias($.destack_enum_assignment_member, $.enum_assignment),
          alias($.destack_method_definition_member, $.method_definition),
          alias($.destack_method_signature_member, $.method_signature),
          alias($.destack_enum_static_field_member, $.enum_static_field),
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
        optional(choice('?', '!')),
        field('type', optional($.type_annotation)),
        optional($._initializer),
      ),

      type_alias_declaration: $ => seq(
        dialect === 'destack' ? choice('type', 'newtype') : 'type',
        field('name', $._type_identifier),
        field('type_parameters', optional($.type_parameters)),
        '=',
        field('value', dialect === 'destack' ? choice($.implements_type, $.type) : $.type),
        $._semicolon,
      ),

      associated_type_declaration: $ => seq(
        'type',
        field('name', $._type_identifier),
        field('type_parameters', optional($.type_parameters)),
        field('constraint', optional(seq(':', $.type))),
        field('value', optional(seq('=', $.type))),
      ),

      associated_const_declaration: $ => seq(
        'comptime',
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

      _parameter_name: $ => {
        const commonPrefix = [
          repeat(field('decorator', $.decorator)),
          ...(dialect === 'destack' ? [optional($.accessibility_modifier), optional('readonly')] : []),
          ...(dialect === 'destack' ? [optional(choice('comptime', 'static'))] : []),
        ];

        if (dialect === 'destack') {
          return seq(
            ...commonPrefix,
            field('pattern', choice(
              $.pattern,
              $.this,
              alias('in', $.identifier),
              alias('out', $.identifier),
            )),
          );
        }

        return choice(
          seq(
            ...commonPrefix,
            field('pattern', choice(
              $.pattern,
              $.this,
              alias('in', $.identifier),
              alias('out', $.identifier),
            )),
          ),
          seq(
            ...commonPrefix,
            choice('in', 'out'),
            field('pattern', choice(
              $.identifier,
              alias($._reserved_identifier, $.identifier),
              $.this,
            )),
          ),
        );
      },

      _initializer: ($, previous) => (
        dialect === 'destack' ?
          seq('=', field('value', choice(
            $.expression,
            $.using_assignment_expression,
            $.switch_statement,
            $.labeled_loop_initializer,
          ))) :
          previous
      ),

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
        ...(dialect === 'destack' ? [optional(repeat1(field('decorator', $.decorator)))] : []),
        $.type,
      ),

      // Oh boy
      // The issue is these special type queries need a lower relative precedence than the normal ones,
      // since these are used in type annotations whereas the other ones are used where `typeof` is
      // required beforehand. This allows for parsing of annotations such as
      // foo: import('x').y.z;
      // but was a nightmare to get working.
      _type_query_member_expression_in_type_annotation: $ => seq(
        field('object', choice(
          $.import,
          alias($._type_query_member_expression_in_type_annotation, $.member_expression),
          alias($._type_query_call_expression_in_type_annotation, $.call_expression),
        )),
        '.',
        field('property', choice(
          $.private_property_identifier,
          alias($.identifier, $.property_identifier),
        )),
      ),
      _type_query_call_expression_in_type_annotation: $ => seq(
        field('function', choice(
          $.import,
          alias($._type_query_member_expression_in_type_annotation, $.member_expression),
        )),
        field('arguments', $.arguments),
      ),

      asserts: $ => seq(
        'asserts',
        choice($.type_predicate, $.identifier, $.this),
      ),

      asserts_annotation: $ => seq(
        seq(':', $.asserts),
      ),

      type: $ => choice(
        $.primary_type,
        ...(dialect === 'destack' ? [$.optional_type] : []),
        $.function_type,
        $.readonly_type,
        ...(dialect === 'destack' ? [] : [$.constructor_type]),
        ...(dialect === 'destack' ? [] : [$.infer_type]),
        prec(-1, alias($._type_query_member_expression_in_type_annotation, $.member_expression)),
        prec(-1, alias($._type_query_call_expression_in_type_annotation, $.call_expression)),
      ),

      tuple_parameter: $ => seq(
        field('name', choice($.identifier, $.rest_pattern)),
        field('type', $.type_annotation),
      ),

      optional_tuple_parameter: $ => seq(
        field('name', $.identifier),
        '?',
        field('type', $.type_annotation),
      ),

      optional_type: $ => seq(
        choice(
          ...(dialect === 'destack' ? [$.managed_type, $.borrow_type, $.pointer_type] : []),
          $.parenthesized_type,
          $.predefined_type,
          $._type_identifier,
          $.nested_type_identifier,
          $.generic_type,
          $.object_type,
          $.array_type,
          $.tuple_type,
          $.flow_maybe_type,
          $.type_query,
          $.index_type_query,
          alias($.this, $.this_type),
          $.literal_type,
          $.lookup_type,
          $.template_literal_type,
          $.intersection_type,
          $.union_type,
          'const',
        ),
        '?',
      ),
      rest_type: $ => seq('...', $.type),

      _tuple_type_member: $ => choice(
        alias($.tuple_parameter, $.required_parameter),
        alias($.optional_tuple_parameter, $.optional_parameter),
        $.type,
        ...(dialect === 'destack' ? [] : [$.optional_type]),
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
        ...(dialect === 'destack' ?
          [
            $.managed_type,
            $.borrow_type,
            $.pointer_type,
          ] :
          []),
        $.parenthesized_type,
        $.predefined_type,
        $._type_identifier,
        $.associated_type_projection,
        $.nested_type_identifier,
        $.generic_type,
        $.object_type,
        $.array_type,
        $.tuple_type,
        $.flow_maybe_type,
        $.type_query,
        $.index_type_query,
        alias($.this, $.this_type),
        ...(dialect === 'destack' ? [] : [$.existential_type]),
        $.literal_type,
        $.lookup_type,
        ...(dialect === 'destack' ? [] : [$.conditional_type]),
        $.template_literal_type,
        $.intersection_type,
        $.union_type,
        'const',
      ),

      template_type: $ => seq(
        '${',
        choice(
          $.primary_type,
          ...(dialect === 'destack' ? [] : [$.infer_type]),
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

      conditional_type: $ => prec.right(seq(
        field('left', $.type),
        'extends',
        field('right', $.type),
        '?',
        field('consequence', $.type),
        ':',
        field('alternative', $.type),
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
          $.generic_type,
          alias($.this, $.this_type),
        )),
        '.',
        field('name', $._type_identifier),
      )),

      type_predicate: $ => seq(
        field('name', choice(
          $.identifier,
          $.this,
          // Sometimes tree-sitter contextual lexing is not good enough to know
          // that 'object' in ':object is foo' is really an identifier and not
          // a predefined_type, so we must explicitely list all possibilities.
          // TODO: should we use '_reserved_identifier'? Should all the element in
          // 'predefined_type' be added to '_reserved_identifier'?
          alias($.predefined_type, $.identifier),
        )),
        'is',
        field('type', $.type),
      ),

      type_predicate_annotation: $ => seq(
        seq(':', $.type_predicate),
      ),

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
          $.import,
          $.identifier,
          alias($._type_query_member_expression, $.member_expression),
          alias($._type_query_subscript_expression, $.subscript_expression),
        )),
        field('arguments', $.arguments),
      ),
      _type_query_instantiation_expression: $ => seq(
        field('function', choice(
          $.import,
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
        dialect === 'destack' ?
          choice(
            $.type,
            $.static_value_argument,
            $.as_expression,
          ) :
          $.type,
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

      existential_type: _ => '*',

      flow_maybe_type: $ => prec.right(seq('?', $.primary_type)),

      parenthesized_type: $ => seq('(', $.type, ')'),

      number: (_, previous) => {
        if (dialect !== 'destack') {
          return previous;
        }

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
        'any',
        'number',
        'boolean',
        'string',
        'symbol',
        alias(seq('unique', 'symbol'), 'unique symbol'),
        'void',
        'unknown',
        'string',
        'never',
        'object',
      ),

      type_arguments: $ => seq(
        dialect === 'destack' ? token.immediate(prec(1, '<')) : '<',
        commaSep1(
          dialect === 'destack' ?
            choice(
              $.type,
              $.static_value_argument,
              $.comptime_type_argument,
            ) :
            $.type,
        ),
        optional(','),
        '>',
      ),

      object_type: $ => seq(
        choice('{', '{|'),
        optional(seq(
          sepBy1(
            choice(',', $._semicolon),
            dialect === 'destack' ?
              $._destack_object_type_member :
              choice(
                $.export_statement,
                $.property_signature,
                $.call_signature,
                $.construct_signature,
                $.index_signature,
                $.method_signature,
              ),
          ),
          optional(choice(',', $._semicolon)),
        )),
        choice('}', '|}'),
      ),

      _destack_object_type_member: $ => seq(
        repeat(field('guard', $.destack_static_if_guard)),
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

      destack_property_signature: $ => seq(
        optional('static'),
        optional('readonly'),
        field('name', $._property_name),
        optional('?'),
        field('type', $.type_annotation),
        optional($._initializer),
      ),

      _call_signature: $ => seq(
        field('type_parameters', optional($.type_parameters)),
        field('parameters', $.formal_parameters),
        field('return_type', optional(
          choice(
            $.type_annotation,
            ...(dialect === 'destack' ? [] : [$.asserts_annotation, $.type_predicate_annotation]),
          ),
        )),
      ),

      type_parameters: $ => seq(
        '<', commaSep1($.type_parameter), optional(','), '>',
      ),

      type_parameter: $ => seq(
        choice(
          seq(
            optional(choice('const', 'comptime')),
            optional(choice('in', 'out')),
            ...(dialect === 'destack' ? [optional('...')] : []),
            field('name', $._type_identifier),
            field('constraint', optional($.constraint)),
            field('value', optional($.default_type)),
          ),
          choice('in', 'out'),
        ),
      ),

      default_type: $ => seq(
        '=',
        dialect === 'destack' ?
          choice($.type, $.comptime_default_type) :
          $.type,
      ),

      comptime_default_type: $ => seq(
        'comptime',
        choice(
          $.number,
          $.string,
          $.true,
          $.false,
          $.identifier,
          $.static_value_argument,
        ),
      ),

      comptime_type_argument: $ => seq(
        'comptime',
        choice(
          $.number,
          $.string,
          $.true,
          $.false,
          $.identifier,
          $.static_value_argument,
        ),
      ),

      static_value_argument: $ => choice(
        prec.left(seq(
          field('left', choice(
            $.number,
            $.identifier,
            seq('(', $.static_value_argument, ')'),
          )),
          field('operator', choice('+', '-', '*', '/', '%')),
          field('right', choice(
            $.number,
            $.identifier,
            seq('(', $.static_value_argument, ')'),
          )),
        )),
      ),

      constraint: $ => seq(
        choice('extends', ':'),
        ...(dialect === 'destack' ? [optional('static')] : []),
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

      managed_type: $ => seq('^', optional('readonly'), $.primary_type),
      borrow_type: $ => seq('&', optional('readonly'), $.primary_type),
      pointer_type: $ => seq('*', $.primary_type),

      array_type: $ => seq($.primary_type, '[', ']'),
      tuple_type: $ => (
        dialect === 'destack' ?
          choice(
            seq('[', commaSep($._tuple_type_member), optional(','), ']'),
            seq('(', ')'),
            seq('(', $.type, ',', ')'),
            seq('(', $.type, ',', commaSep1($.type), optional(','), ')'),
          ) :
          seq('[', commaSep($._tuple_type_member), optional(','), ']')
      ),
      readonly_type: $ => seq(
        'readonly',
        $.type,
      ),

      union_type: $ => prec.left(seq(optional($.type), '|', $.type)),
      intersection_type: $ => prec.left(seq(optional($.type), '&', $.type)),

      function_type: $ => prec.left(seq(
        field('type_parameters', optional($.type_parameters)),
        field('parameters', choice(
          $.formal_parameters,
          ...(dialect === 'destack' ?
            [alias($.destack_predefined_type_parameters, $.formal_parameters)] :
            []),
        )),
        '=>',
        field('return_type', choice(
          $.type,
          ...(dialect === 'destack' ? [] : [$.asserts, $.type_predicate]),
        )),
      )),

      destack_predefined_type_parameters: $ => seq(
        '(',
        commaSep1($.predefined_type),
        optional(','),
        ')',
      ),

      _type_identifier: $ => alias($.identifier, $.type_identifier),

      _reserved_identifier: (_, previous) => choice(
        'declare',
        'namespace',
        'type',
        'public',
        'private',
        'protected',
        'override',
        'readonly',
        'module',
        'loop',
        'any',
        'number',
        'boolean',
        'string',
        'symbol',
        'export',
        'object',
        'new',
        'readonly',
        previous,
      ),

    },
  });
};

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
