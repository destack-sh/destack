use destack_language_ast::Parser;
use destack_language_token::{SourceFile, SourceId, tokenize_semantic};

use crate::cli::parse::read_parse_input;
use crate::console::console;
use crate::console::parse::CommandArguments;

/// Parse source into AST (stub).
pub(crate) fn parse_ast(ctx: CommandArguments) -> i32 {
    // input
    let input = match read_parse_input(&ctx) {
        Ok(s) => s,
        Err(e) => {
            console::error(&format!("Read input error: {e}"));
            return 1;
        }
    };
    let file = SourceFile::new(0 as SourceId, &input, input.len() as u32);

    // options
    // let use_color = !ctx.flag("no-color");
    // let use_pager = !ctx.flag("no-pager");
    let as_node = ctx.option("as").unwrap_or("expression");

    // parse
    let tokens = tokenize_semantic(&input);
    let mut parser = Parser::new(file, &tokens);

    match as_node {
        "module" => {
            let Ok(module_id) = parser.eat_module_body() else {
                console::error("Parse error");
                return 1;
            };
            let module = parser.tree.get(module_id);
            console::info(&format!("Module: {module:?}"));
        }
        "expression" => {
            let Ok(expression_id) = parser.eat_expression(None) else {
                console::error("Parse error");
                return 1;
            };
            let expression = parser.tree.get(expression_id);
            console::info(&format!("Expression: {expression:?}"));
        }
        "type" => {
            let Ok(type_id) = parser.eat_type() else {
                console::error("Parse error");
                return 1;
            };
            let ty = parser.tree.get(type_id);
            console::info(&format!("Type: {ty:?}"));
        }
        _ => {
            console::error(&format!("Bad node type: {as_node}"));
            return 1;
        }
    };

    0
}
