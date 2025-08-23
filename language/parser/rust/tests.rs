use destack_language_lexer::tokenize_semantic;

#[test]
fn test_literals() {
    let input = r##"
1
17
0x32
1.0f64
"Hello, world!"
[1, false, "Hello"]
(true, (1, false))
}"##;
}

#[test]
fn test_using() {
    let input = r##"
using destack;
using destack.geometry;
using destack as ds;
using ds.geometry as geom;
using ds.geometry.{Vector2, Vector3};
"##;
    let tokens = tokenize_semantic(input);
    println!("{tokens:?}");
}

#[test]
fn test_smoke() {
    let input = r##"
implement MyCustomView {
    const Pi: float32 = 3.1415;

	function toView(self) -> Option<ButtonView> {
        let value: int32 = ---;
        @if self.isVisible {
            Some(ButtonView { label: @format("Hi {self.name}!") })
        } @else {
            None
        }
	}
}"##;
    let tokens = tokenize_semantic(input);
    println!("{tokens:?}");
}
