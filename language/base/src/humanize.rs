/// Pluralize a word with a naive 's'.
pub fn pluralize<T: AsRef<str>>(count: usize, singular: T) -> String {
    if count == 1 {
        singular.as_ref().to_owned()
    } else {
        format!("{}s", singular.as_ref())
    }
}
