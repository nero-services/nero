use std::borrow::Cow;
pub fn dv(input: &[u8]) -> Cow<str> {
    String::from_utf8_lossy(&input)
}
