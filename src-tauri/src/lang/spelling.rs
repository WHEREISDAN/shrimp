//! Spelling that follows the chosen English or German variant (LANG-03).
//! Whole words only, keeping the writer's capitalization.

/// American spelling and its British form. Canadian English takes the
/// British form except for -ize/-yze verbs.
const US_UK: [(&str, &str, bool); 44] = [
    ("color", "colour", true),
    ("colors", "colours", true),
    ("colored", "coloured", true),
    ("favorite", "favourite", true),
    ("favorites", "favourites", true),
    ("honor", "honour", true),
    ("humor", "humour", true),
    ("labor", "labour", true),
    ("neighbor", "neighbour", true),
    ("neighbors", "neighbours", true),
    ("behavior", "behaviour", true),
    ("behaviors", "behaviours", true),
    ("flavor", "flavour", true),
    ("harbor", "harbour", true),
    ("rumor", "rumour", true),
    ("center", "centre", true),
    ("centers", "centres", true),
    ("theater", "theatre", true),
    ("liter", "litre", true),
    ("liters", "litres", true),
    ("fiber", "fibre", true),
    ("catalog", "catalogue", true),
    ("defense", "defence", true),
    ("offense", "offence", true),
    ("gray", "grey", true),
    ("traveled", "travelled", true),
    ("traveling", "travelling", true),
    ("traveler", "traveller", true),
    ("canceled", "cancelled", true),
    ("canceling", "cancelling", true),
    ("modeled", "modelled", true),
    ("organize", "organise", false),
    ("organized", "organised", false),
    ("organizing", "organising", false),
    ("organization", "organisation", false),
    ("organizations", "organisations", false),
    ("realize", "realise", false),
    ("realized", "realised", false),
    ("recognize", "recognise", false),
    ("recognized", "recognised", false),
    ("apologize", "apologise", false),
    ("analyze", "analyse", false),
    ("analyzed", "analysed", false),
    ("prioritize", "prioritise", false),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spelling {
    American,
    British,
    Canadian,
    SwissGerman,
    /// Chinese characters (OpenCC's dictionaries through hanconv).
    Simplified,
    Traditional,
    /// Traditional characters as written in Hong Kong, for Cantonese.
    HongKong,
}

impl Spelling {
    pub fn for_language(id: &str) -> Option<Self> {
        match id {
            "en-US" => Some(Self::American),
            "en-GB" => Some(Self::British),
            "en-CA" => Some(Self::Canadian),
            "de-CH" => Some(Self::SwissGerman),
            "zh-Hans" => Some(Self::Simplified),
            "zh-Hant" => Some(Self::Traditional),
            "yue" => Some(Self::HongKong),
            _ => None,
        }
    }

    /// `word` (lowercase) in this spelling, if it changes.
    fn respell(self, word: &str) -> Option<&'static str> {
        US_UK.iter().find_map(|(us, uk, canadian)| match self {
            Self::American => (word == *uk).then_some(*us),
            Self::British => (word == *us).then_some(*uk),
            Self::Canadian if *canadian => (word == *us).then_some(*uk),
            Self::Canadian => (word == *uk).then_some(*us),
            _ => None,
        })
    }
}

fn with_case_of(original: &str, replacement: &str) -> String {
    let mut chars = original.chars();
    let first_upper = chars.next().is_some_and(char::is_uppercase);
    let all_upper = original.chars().count() > 1 && original.chars().all(|c| !c.is_lowercase());
    if all_upper {
        replacement.to_uppercase()
    } else if first_upper {
        let mut rest = replacement.chars();
        rest.next()
            .map(|first| first.to_uppercase().chain(rest).collect())
            .unwrap_or_default()
    } else {
        replacement.to_owned()
    }
}

/// `text` in this spelling.
pub fn apply(text: &str, spelling: Spelling) -> String {
    match spelling {
        Spelling::SwissGerman => return text.replace('ß', "ss"),
        Spelling::Simplified => return hanconv::t2s(text),
        Spelling::Traditional => return hanconv::s2t(text),
        Spelling::HongKong => return hanconv::s2hk(text),
        _ => {}
    }
    let mut out = String::with_capacity(text.len());
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        if !word.is_empty() {
            match spelling.respell(&word.to_lowercase()) {
                Some(replacement) => out.push_str(&with_case_of(word, replacement)),
                None => out.push_str(word),
            }
            word.clear();
        }
    };
    for c in text.chars() {
        if c.is_alphabetic() {
            word.push(c);
        } else {
            flush(&mut word, &mut out);
            out.push(c);
        }
    }
    flush(&mut word, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn british_and_canadian_spelling() {
        let text = "My favorite color is gray. We should organize the Center.";
        assert_eq!(
            apply(text, Spelling::British),
            "My favourite colour is grey. We should organise the Centre."
        );
        assert_eq!(
            apply(text, Spelling::Canadian),
            "My favourite colour is grey. We should organize the Centre."
        );
        assert_eq!(
            apply("The colours were analysed.", Spelling::American),
            "The colors were analyzed."
        );
        assert_eq!(apply("COLOR", Spelling::British), "COLOUR");
        assert_eq!(
            apply("discolored", Spelling::British),
            "discolored",
            "whole words only"
        );
    }

    #[test]
    fn chinese_follows_the_chosen_script() {
        assert_eq!(
            apply("我们明天开会讨论这个问题。", Spelling::Traditional),
            "我們明天開會討論這個問題。"
        );
        assert_eq!(
            apply("我們明天開會討論這個問題。", Spelling::Simplified),
            "我们明天开会讨论这个问题。"
        );
        assert_eq!(apply("Meeting 开会", Spelling::Traditional), "Meeting 開會");
    }

    #[test]
    fn swiss_german_writes_ss() {
        assert_eq!(
            apply("Die Straße ist groß.", Spelling::SwissGerman),
            "Die Strasse ist gross."
        );
    }
}
