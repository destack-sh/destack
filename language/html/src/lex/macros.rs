/// Return early when one option is missing.
macro_rules! unwrap_or_return {
    ($option:expr) => {{
        let Some(value) = $option else {
            return;
        };
        value
    }};
    ($option:expr, $return_value:expr) => {{
        let Some(value) = $option else {
            return $return_value;
        };
        value
    }};
}

pub(crate) use unwrap_or_return;

/// Measure one expression and return its result with elapsed nanoseconds.
macro_rules! time {
    ($expression:expr) => {{
        let start = ::std::time::Instant::now();
        let result = $expression;
        let elapsed_ns = start.elapsed().as_nanos() as u64;
        (result, elapsed_ns)
    }};
}

pub(crate) use time;

/// Build one small character set literal.
macro_rules! small_char_set {
    ($($element:expr)+) => {
        $crate::lex::SmallCharSet {
            bits: $((1 << ($element as usize)))|+
        }
    };
}

pub(crate) use small_char_set;

/// Apply one shorthand lexer state-machine action.
macro_rules! shorthand {
    ( $lexer:ident : create_tag $kind:ident $character:expr ) => {
        $lexer.create_tag($kind, $character)
    };
    ( $lexer:ident : push_tag $character:expr ) => {
        $lexer.push_tag_name($character)
    };
    ( $lexer:ident : discard_tag ) => {
        $lexer.discard_tag()
    };
    ( $lexer:ident : discard_char $input:expr ) => {
        $lexer.discard_char($input)
    };
    ( $lexer:ident : push_temp $character:expr ) => {
        $lexer.push_temp_char($character)
    };
    ( $lexer:ident : clear_temp ) => {
        $lexer.clear_temp_buf()
    };
    ( $lexer:ident : create_attr $character:expr ) => {
        $lexer.create_attribute($character)
    };
    ( $lexer:ident : push_name $character:expr ) => {
        $lexer.push_attribute_name($character)
    };
    ( $lexer:ident : push_value $character:expr ) => {
        $lexer.push_attribute_value($character)
    };
    ( $lexer:ident : append_value $value:expr ) => {
        $lexer.append_attribute_value($value)
    };
    ( $lexer:ident : push_comment $character:expr ) => {
        $lexer.push_comment_char($character)
    };
    ( $lexer:ident : append_comment $value:expr ) => {
        $lexer.append_comment_text($value)
    };
    ( $lexer:ident : emit_comment ) => {
        $lexer.emit_current_comment()
    };
    ( $lexer:ident : clear_comment ) => {
        $lexer.clear_comment()
    };
    ( $lexer:ident : create_doctype ) => {
        $lexer.initialize_doctype()
    };
    ( $lexer:ident : push_doctype_name $character:expr ) => {
        $lexer.push_doctype_name($character)
    };
    ( $lexer:ident : push_doctype_id $kind:ident $character:expr ) => {
        $lexer.push_doctype_identifier($kind, $character)
    };
    ( $lexer:ident : clear_doctype_id $kind:ident ) => {
        $lexer.clear_doctype_id($kind)
    };
    ( $lexer:ident : force_quirks ) => {
        $lexer.force_quirks()
    };
    ( $lexer:ident : emit_doctype ) => {
        $lexer.emit_current_doctype()
    };
}

pub(crate) use shorthand;

/// Expand one shorthand action.
macro_rules! sh_trace {
    ( $lexer:ident : $($commands:tt)* ) => {
        $crate::lex::macros::shorthand!($lexer: $($commands)*)
    };
}

pub(crate) use sh_trace;

/// Sequence one lexer state-machine action list.
macro_rules! go {
    ( $lexer:ident : $a:tt                   ; $($rest:tt)* ) => ({ $crate::lex::macros::sh_trace!($lexer: $a);          go!($lexer: $($rest)*); });
    ( $lexer:ident : $a:tt $b:tt             ; $($rest:tt)* ) => ({ $crate::lex::macros::sh_trace!($lexer: $a $b);       go!($lexer: $($rest)*); });
    ( $lexer:ident : $a:tt $b:tt $c:tt       ; $($rest:tt)* ) => ({ $crate::lex::macros::sh_trace!($lexer: $a $b $c);    go!($lexer: $($rest)*); });
    ( $lexer:ident : $a:tt $b:tt $c:tt $d:tt ; $($rest:tt)* ) => ({ $crate::lex::macros::sh_trace!($lexer: $a $b $c $d); go!($lexer: $($rest)*); });

    ( $lexer:ident : to $state:ident ) => ({ $lexer.transition_to($state); return $crate::lex::lexer::ProcessResult::Continue; });
    ( $lexer:ident : to $state:ident $kind:expr ) => ({ $lexer.transition_to($state($kind)); return $crate::lex::lexer::ProcessResult::Continue; });
    ( $lexer:ident : to $state:ident $wrapper:ident $kind:expr ) => ({ $lexer.transition_to($state($wrapper($kind))); return $crate::lex::lexer::ProcessResult::Continue; });

    ( $lexer:ident : reconsume $state:ident ) => ({ $lexer.reconsume_next(); go!($lexer: to $state); });
    ( $lexer:ident : reconsume $state:ident $kind:expr ) => ({ $lexer.reconsume_next(); go!($lexer: to $state $kind); });
    ( $lexer:ident : reconsume $state:ident $wrapper:ident $kind:expr ) => ({ $lexer.reconsume_next(); go!($lexer: to $state $wrapper $kind); });

    ( $lexer:ident : consume_char_ref ) => ({ $lexer.start_consuming_character_reference(); return $crate::lex::lexer::ProcessResult::Continue; });

    ( $lexer:ident : emit_tag $state:ident ) => ({
        $lexer.transition_to($state);
        return $lexer.emit_current_tag();
    });

    ( $lexer:ident : eof ) => ({ $lexer.emit_eof(); return $crate::lex::lexer::ProcessResult::Suspend; });

    ( $lexer:ident : $($command:tt)+ ) => ( $crate::lex::macros::sh_trace!($lexer: $($command)+) );
    ( $lexer:ident : ) => (());
}

pub(crate) use go;

/// Return early when one input character is unavailable.
macro_rules! get_char {
    ( $lexer:expr, $input:expr ) => {
        $crate::lex::macros::unwrap_or_return!(
            $lexer.get_char($input),
            $crate::lex::lexer::ProcessResult::Suspend
        )
    };
}

pub(crate) use get_char;

/// Return early when one peeked character is unavailable.
macro_rules! peek {
    ( $lexer:expr, $input:expr ) => {
        $crate::lex::macros::unwrap_or_return!(
            $lexer.peek($input),
            $crate::lex::lexer::ProcessResult::Suspend
        )
    };
}

pub(crate) use peek;

/// Return early when one set pop must suspend.
macro_rules! pop_except_from {
    ( $lexer:expr, $input:expr, $set:expr ) => {
        $crate::lex::macros::unwrap_or_return!(
            $lexer.pop_except_from($input, $set),
            $crate::lex::lexer::ProcessResult::Suspend
        )
    };
}

pub(crate) use pop_except_from;

/// Return early when one case-insensitive eat must suspend.
macro_rules! eat {
    ( $lexer:expr, $input:expr, $pattern:expr ) => {
        $crate::lex::macros::unwrap_or_return!(
            $lexer.eat($input, $pattern, u8::eq_ignore_ascii_case),
            $crate::lex::lexer::ProcessResult::Suspend
        )
    };
}

pub(crate) use eat;

/// Return early when one exact eat must suspend.
macro_rules! eat_exact {
    ( $lexer:expr, $input:expr, $pattern:expr ) => {
        $crate::lex::macros::unwrap_or_return!(
            $lexer.eat($input, $pattern, u8::eq),
            $crate::lex::lexer::ProcessResult::Suspend
        )
    };
}

pub(crate) use eat_exact;
