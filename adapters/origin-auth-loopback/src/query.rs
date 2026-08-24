/// Parse a URL query string into decoded key/value pairs.
///
/// Small and local on purpose: pulling in a URL crate for one query string would add a
/// dependency to every application that authenticates.
pub(crate) fn parse(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| match pair.split_once('=') {
            Some((key, value)) => (decode(key), decode(value)),
            None => (decode(pair), String::new()),
        })
        .collect()
}

/// Percent-decoding, with `+` meaning a space as in form encoding.
fn decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                match u8::from_str_radix(&value[index + 1..index + 3], 16) {
                    Ok(byte) => {
                        decoded.push(byte);
                        index += 3;
                    }
                    // A stray `%` is kept verbatim rather than dropped, so a malformed
                    // value stays recognisable in an error message.
                    Err(_) => {
                        decoded.push(b'%');
                        index += 1;
                    }
                }
            }
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }

    String::from_utf8_lossy(&decoded).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairs_are_decoded() {
        let parsed = parse("code=abc%2Fdef&state=xyz");
        assert_eq!(parsed[0], ("code".to_owned(), "abc/def".to_owned()));
        assert_eq!(parsed[1], ("state".to_owned(), "xyz".to_owned()));
    }

    #[test]
    fn plus_means_space() {
        assert_eq!(
            parse("error_description=access+denied")[0].1,
            "access denied"
        );
    }

    #[test]
    fn an_empty_query_yields_nothing() {
        assert!(parse("").is_empty());
    }

    #[test]
    fn a_malformed_escape_is_kept_verbatim() {
        assert_eq!(parse("x=100%")[0].1, "100%");
    }
}
