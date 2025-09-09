use dyst_language_ast::{BlockFormat, DumperOptions, ExpressionParserOptions, Parser};
use dyst_language_source::{Source, SourceId};

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
    let file_id = SourceId::new(0);
    let file = Source::new(file_id, input);

    // options
    // let use_color = !ctx.flag("no-color");
    // let use_pager = !ctx.flag("no-pager");
    let as_node = ctx.option("as").unwrap_or("statement");

    // parse
    let mut parser = Parser::from_source(&file, file_id);
    let dump_options = DumperOptions::default();
    let node_str = match as_node {
        "module" => {
            let Ok(module_id) = parser.eat_module_body(None, BlockFormat::Implicit) else {
                console::error("Parse error");
                return 1;
            };
            let module = parser.tree.get(module_id);
            let mut dumper = parser.dumper(dump_options);
            dumper.dump_line(module, None);
            dumper.finish()
        }
        "statement" => {
            let Ok(statement_id) = parser.eat_statement() else {
                console::error("Parse error");
                return 1;
            };
            let statement = parser.tree.get(statement_id);
            let mut dumper = parser.dumper(dump_options);
            dumper.dump_line(statement, None);
            dumper.finish()
        }
        "expression" => {
            let Ok(expression_id) = parser.eat_expression(ExpressionParserOptions::default())
            else {
                console::error("Parse error");
                return 1;
            };
            let expression = parser.tree.get(expression_id);
            let mut dumper = parser.dumper(dump_options);
            dumper.dump_line(expression, None);
            dumper.finish()
        }
        "type" => {
            let Ok(type_id) = parser.eat_type() else {
                console::error("Parse error");
                return 1;
            };
            let ty = parser.tree.get(type_id);
            let mut dumper = parser.dumper(dump_options);
            dumper.dump_line(ty, None);
            dumper.finish()
        }
        _ => {
            console::error(&format!(
                "Bad node kind: {as_node} (must be 'module', 'expression', 'statement', or 'type')"
            ));
            return 1;
        }
    };

    console::info(&node_str);

    0
}
