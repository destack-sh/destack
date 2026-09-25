use crate::{TestParser, assert_path};
use tspp_dir::TokenType;

#[test]
fn test_parse_simple_path_single_segment() {
    let test = TestParser::new("tspp");
    let mut parser = test.prepare();
    let path = parser.parse_path().unwrap();
    assert_path!(parser, path, "tspp");
}

#[test]
fn test_parse_simple_path_multiple_segments() {
    let test = TestParser::new("tspp.geometry.math");
    let mut parser = test.prepare();
    let path = parser.parse_path().unwrap();
    assert_path!(parser, path, "tspp.geometry.math");
}

#[test]
fn test_parse_simple_path_multiple_segments_with_newline() {
    let test = TestParser::new("tspp\n.geometry\n.math\n");
    let mut parser = test.prepare();
    let path = parser.parse_path().unwrap();
    assert_path!(parser, path, "tspp.geometry.math");
}

#[test]
fn test_parse_path_stops_before_group_brace() {
    let test = TestParser::new("ds.geometry.{Vector2}");
    let mut parser = test.prepare();
    let path = parser.parse_path().unwrap();
    assert_path!(parser, path, "ds.geometry");
    // ensure next token is the `.` for the group
    let next = parser.peek_token_span();
    assert_eq!(next.token.ty(), TokenType::Dot);
}

#[test]
fn test_parse_path_stops_before_angle_bracket() {
    let test = TestParser::new("geom.Vector<Dims: 2, float32>");
    let mut parser = test.prepare();
    let path = parser.parse_path().unwrap();
    assert_path!(parser, path, "geom.Vector");
    // ensure next token is the `<` for the generic arguments
    let next = parser.peek_token_span();
    assert_eq!(next.token.ty(), TokenType::LessThan);
}

#[test]
fn test_parse_path_stops_before_dot_with_leading_comment() {
    let test = TestParser::new("source /* hop */ .first()");
    let mut parser = test.prepare();
    let path = parser.parse_path().unwrap();
    assert_path!(parser, path, "source");
    let next = parser.peek_token_span();
    assert_eq!(next.token.ty(), TokenType::Dot);
}

#[test]
fn test_parse_path_stops_before_identifier_with_leading_comment_after_dot() {
    let test = TestParser::new("source. /* hop */ first()");
    let mut parser = test.prepare();
    let path = parser.parse_path().unwrap();
    assert_path!(parser, path, "source");
    let next = parser.peek_token_span();
    assert_eq!(next.token.ty(), TokenType::Dot);
}
