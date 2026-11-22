const NAMED: &[(&str, char)] = &[
    ("amp", '&'),
    ("lt", '<'),
    ("gt", '>'),
    ("quot", '"'),
    ("apos", '\''),
    ("nbsp", '\u{a0}'),
    ("hellip", '…'),
    ("mdash", '—'),
    ("ndash", '–'),
    ("laquo", '«'),
    ("raquo", '»'),
    ("copy", '©'),
    ("reg", '®'),
    ("trade", '™'),
];

const MAX_ENTITY_LENGTH: usize = 10;

pub fn decode_entities(text: &str) -> String {
    if !text.contains('&') {
        return text.to_owned();
    }

    let mut output = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(start) = rest.find('&') {
        output.push_str(&rest[..start]);
        rest = &rest[start..];

        let decoded = rest[1..]
            .char_indices()
            .take(MAX_ENTITY_LENGTH + 1)
            .find(|&(_, c)| c == ';')
            .and_then(|(end, _)| decode_entity(&rest[1..1 + end]).map(|c| (c, end + 2)));

        match decoded {
            Some((c, length)) => {
                output.push(c);
                rest = &rest[length..];
            }
            None => {
                output.push('&');
                rest = &rest[1..];
            }
        }
    }

    output.push_str(rest);
    output
}

fn decode_entity(entity: &str) -> Option<char> {
    if let Some(number) = entity.strip_prefix('#') {
        let code = match number.strip_prefix(['x', 'X']) {
            Some(hex) => u32::from_str_radix(hex, 16).ok()?,
            None => number.parse().ok()?,
        };

        return char::from_u32(code).filter(|&c| c != '\0');
    }

    NAMED
        .iter()
        .find(|(name, _)| *name == entity)
        .map(|&(_, c)| c)
}

#[cfg(test)]
mod tests {
    use super::decode_entities;

    #[test]
    fn decodes_numeric_entities() {
        assert_eq!(
            decode_entities("&#1055;&#1088;&#1080;&#1074;&#1077;&#1090;"),
            "Привет"
        );
        assert_eq!(decode_entities("A &#8594; B"), "A → B");
        assert_eq!(decode_entities("&#x41;&#X42;"), "AB");
        assert_eq!(decode_entities("&#127468;&#127463;"), "🇬🇧");
    }

    #[test]
    fn decodes_named_entities() {
        assert_eq!(decode_entities("a=1&amp;b=2"), "a=1&b=2");
        assert_eq!(decode_entities("&lt;tag&gt; &quot;x&quot;"), "<tag> \"x\"");
    }

    #[test]
    fn keeps_text_that_is_not_an_entity() {
        assert_eq!(decode_entities("Salt & Pepper"), "Salt & Pepper");
        assert_eq!(
            decode_entities("&unknown; &#; &#xZZ; &#0;"),
            "&unknown; &#; &#xZZ; &#0;"
        );
        assert_eq!(decode_entities("trailing &"), "trailing &");
        assert_eq!(decode_entities("&amp;amp;"), "&amp;");
    }
}
