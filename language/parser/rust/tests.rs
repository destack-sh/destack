use destack_language_lexer::tokenize_semantic;

#[test]
fn test_smoke() {
    let input = r##"
#entity(View2D) struct MyCustomView {
	fn render(self) {
        let value: i32 = ---;
        #if target == 'macos' {
            ButtonView::new({ test: #format("Hi {self.name}!") })
        } #else {
            None
        }
	}
}"##;
    let tokens = tokenize_semantic(input);
    println!("{tokens:?}");
}
