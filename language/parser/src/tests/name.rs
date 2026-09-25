use crate::tests::TestParser;
use tspp_dir::Name;

#[test]
fn test_report_property_name_legacy_octal_index() {
    let test = TestParser::new("021");
    let mut parser = test.prepare();
    let error = parser.eat_property_name_with_range().unwrap_err();

    assert_eq!(parser.range_str(error.range()), "021");
}

#[test]
fn test_parse_property_name_integer_index() {
    let test = TestParser::new("2");
    let mut parser = test.prepare();
    let (name, _) = parser.eat_property_name_with_range().unwrap();

    assert_eq!(name, Name::Index(2));
    test.assert_no_errors(&parser);
}
