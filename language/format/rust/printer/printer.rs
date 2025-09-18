


#[derive(Debug, Clone)]
pub struct Printer {
    buffer: String,
    depth: usize,
    branch_stack: Vec<bool>,
    last_line_has_more: Option<bool>,
}
