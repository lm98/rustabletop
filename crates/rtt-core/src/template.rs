use typst::syntax::ast;

pub fn defines(text: &str, name: &str) -> bool {
    let root = typst::syntax::parse(text);
    let Some(markup) = root.cast::<ast::Markup>() else {
        return false;
    };
    markup.exprs().any(|expr| match expr {
        ast::Expr::LetBinding(binding) => binding
            .kind()
            .bindings()
            .iter()
            .any(|ident| ident.as_str() == name),
        ast::Expr::ModuleImport(import) => match import.imports() {
            Some(ast::Imports::Wildcard) => true,
            Some(ast::Imports::Items(items)) => {
                items.iter().any(|item| item.bound_name().as_str() == name)
            }
            None => false,
        },
        _ => false,
    })
}

pub fn icon_tokens(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b':'
            && let Some(len) = token_len(&bytes[i + 1..])
        {
            tokens.push(&text[i + 1..i + 1 + len]);
            i += len + 2;
        } else {
            i += 1;
        }
    }
    tokens
}

fn token_len(rest: &[u8]) -> Option<usize> {
    if !rest.first()?.is_ascii_lowercase() {
        return None;
    }
    let len = rest
        .iter()
        .take_while(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || **b == b'-')
        .count();
    (rest.get(len) == Some(&b':')).then_some(len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_face_functions() {
        let text = "#import \"@rtt/std:0.1.0\": fit\n#let front(card, ctx) = []\n#let back = (card, ctx) => []";
        assert!(defines(text, "front"));
        assert!(defines(text, "back"));
        assert!(!defines(text, "side"));
        assert!(defines("#import \"other.typ\": side", "side"));
        assert!(!defines("#let x = [#let front(c) = 1]", "front"));
    }

    #[test]
    fn scans_icon_tokens_like_the_stdlib_regex() {
        assert_eq!(
            icon_tokens("Difensore. :terra::terra: e :fuco:"),
            ["terra", "terra", "fuco"]
        );
        assert!(icon_tokens("Quando entra: ore 10:30: fine:").is_empty());
        assert_eq!(icon_tokens("à :aria:"), ["aria"]);
    }
}
