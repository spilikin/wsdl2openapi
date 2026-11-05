//! `xs:base64Binary` as the generated `Base64Binary` reads and writes it.

use serde::Deserialize as _;
use serde::de::value::{Error, StrDeserializer};
use testproj::conn::soap::Base64Binary;

fn read(text: &str) -> Result<Base64Binary, Error> {
    Base64Binary::deserialize(StrDeserializer::<Error>::new(text))
}

#[test]
fn whitespace_of_a_line_wrapping_encoder_is_ignored() {
    assert_eq!(read("aGVs\r\n bG8=\t").unwrap().0, b"hello");
}

#[test]
fn an_encoding_with_non_zero_padding_bits_is_refused() {
    assert_eq!(read("aGVsbG8=").unwrap().0, b"hello");
    assert!(read("aGVsbG9=").is_err());
}

#[test]
fn text_that_is_no_base64_is_refused() {
    for text in ["aGVsbG8", "aGVs*G8=", "===="] {
        assert!(read(text).is_err(), "{text:?}");
    }
}

#[test]
fn written_as_the_standard_alphabet_with_padding() {
    let written = quick_xml::se::to_string_with_root("v", &Base64Binary(b"hello?>".to_vec())).unwrap();
    assert_eq!(written, "<v>aGVsbG8/Pg==</v>");
}
