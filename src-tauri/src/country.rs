use std::collections::HashMap;

const KNOWN_ISO2: &[&str] = &[
    "HK", "JP", "SG", "TW", "US", "GB", "DE", "KR", "FR", "MY", "RU", "CA", "AU", "NL", "TR", "IN",
    "BR", "AR", "TH", "PH", "VN", "ID", "AE", "IL", "ZA", "IT", "ES", "CH", "SE", "NO", "FI", "DK",
    "PL", "UA", "MX",
];

fn is_known(code: &str) -> bool {
    KNOWN_ISO2.iter().any(|k| *k == code)
}

// Chinese region keywords. Longest match wins (the parser scans the whole list and picks
// the longest keyword found anywhere in the string). Single-char fallbacks are intentional
// but deliberately omitted where they would collide (e.g. `瑞` is left mapped to CH by the
// plan even though 瑞典 → SE; longest-match keeps that working).
const CN_KEYWORDS: &[(&str, &str)] = &[
    // 4-char
    ("阿根廷", "AR"),
    ("阿联酋", "AE"),
    ("澳大利亚", "AU"),
    ("俄罗斯", "RU"),
    ("加拿大", "CA"),
    ("菲律宾", "PH"),
    ("马来西亚", "MY"),
    ("土耳其", "TR"),
    ("乌克兰", "UA"),
    ("以色列", "IL"),
    ("意大利", "IT"),
    ("印度尼西亚", "ID"),
    ("墨西哥", "MX"),
    ("西班牙", "ES"),
    ("瑞典", "SE"),
    // 2-3 char
    ("香港", "HK"),
    ("日本", "JP"),
    ("新加坡", "SG"),
    ("狮城", "SG"),
    ("美国", "US"),
    ("台湾", "TW"),
    ("韩国", "KR"),
    ("英国", "GB"),
    ("德国", "DE"),
    ("法国", "FR"),
    ("荷兰", "NL"),
    ("印度", "IN"),
    ("巴西", "BR"),
    ("马来", "MY"),
    ("泰国", "TH"),
    ("越南", "VN"),
    ("印尼", "ID"),
    ("迪拜", "AE"),
    ("南非", "ZA"),
    ("瑞士", "CH"),
    ("挪威", "NO"),
    ("芬兰", "FI"),
    ("丹麦", "DK"),
    ("波兰", "PL"),
    // 1 char fallbacks
    ("港", "HK"),
    ("日", "JP"),
    ("新", "SG"),
    ("美", "US"),
    ("台", "TW"),
    ("韩", "KR"),
    ("英", "GB"),
    ("德", "DE"),
    ("法", "FR"),
    ("俄", "RU"),
    ("加", "CA"),
    ("澳", "AU"),
    ("荷", "NL"),
    ("土", "TR"),
    ("印", "IN"),
    ("巴", "BR"),
    ("阿", "AR"),
    ("泰", "TH"),
    ("菲", "PH"),
    ("越", "VN"),
    ("以", "IL"),
    ("意", "IT"),
    ("西", "ES"),
    ("瑞", "CH"),
    ("挪", "NO"),
    ("芬", "FI"),
    ("丹", "DK"),
    ("波", "PL"),
    ("乌", "UA"),
    ("墨", "MX"),
];

// English substrings, case-insensitive. Order: more specific cities first so `Los Angeles`
// resolves to US even though "Los" alone wouldn't.
const EN_KEYWORDS: &[(&str, &str)] = &[
    ("hong kong", "HK"),
    ("united kingdom", "GB"),
    ("united states", "US"),
    ("united arab", "AE"),
    ("los angeles", "US"),
    ("san jose", "US"),
    ("new york", "US"),
    ("netherlands", "NL"),
    ("amsterdam", "NL"),
    ("singapore", "SG"),
    ("frankfurt", "DE"),
    ("stockholm", "SE"),
    ("argentina", "AR"),
    ("indonesia", "ID"),
    ("jakarta", "ID"),
    ("australia", "AU"),
    ("istanbul", "TR"),
    ("malaysia", "MY"),
    ("philippines", "PH"),
    ("germany", "DE"),
    ("america", "US"),
    ("chicago", "US"),
    ("seattle", "US"),
    ("toronto", "CA"),
    ("thailand", "TH"),
    ("bangkok", "TH"),
    ("vietnam", "VN"),
    ("denmark", "DK"),
    ("finland", "FI"),
    ("ukraine", "UA"),
    ("israel", "IL"),
    ("mexico", "MX"),
    ("japan", "JP"),
    ("tokyo", "JP"),
    ("osaka", "JP"),
    ("taiwan", "TW"),
    ("korea", "KR"),
    ("seoul", "KR"),
    ("london", "GB"),
    ("france", "FR"),
    ("paris", "FR"),
    ("russia", "RU"),
    ("moscow", "RU"),
    ("canada", "CA"),
    ("sydney", "AU"),
    ("turkey", "TR"),
    ("india", "IN"),
    ("mumbai", "IN"),
    ("brazil", "BR"),
    ("dubai", "AE"),
    ("italy", "IT"),
    ("milan", "IT"),
    ("spain", "ES"),
    ("madrid", "ES"),
    ("switzerland", "CH"),
    ("sweden", "SE"),
    ("norway", "NO"),
    ("poland", "PL"),
];

/// Parse a country code from a node name.
///
/// Strategy, first hit wins:
///   1. exact match in `overrides`
///   2. flag emoji (pair of Regional Indicator Symbols U+1F1E6..U+1F1FF)
///   3. bracketed code: `[XX]`, `(XX)`, fullwidth `［XX］`, `【XX】`, `「XX」`
///   4. prefix: `[A-Za-z]{2}[-_ \t]?[0-9A-Za-z]`
///   5. Chinese region keyword (longest match wins)
///   6. English substring, case-insensitive
///   7. fallback "XX"
pub fn parse_country(name: &str, overrides: &HashMap<String, String>) -> String {
    if let Some(c) = overrides.get(name) {
        return c.to_uppercase();
    }
    if let Some(c) = decode_flag_emoji(name) {
        if is_known(&c) {
            return c;
        }
    }
    if let Some(c) = bracketed_code(name) {
        if is_known(&c) {
            return c;
        }
    }
    if let Some(c) = prefix_code(name) {
        if is_known(&c) {
            return c;
        }
    }
    if let Some(c) = chinese_keyword(name) {
        return c.to_string();
    }
    if let Some(c) = english_keyword(name) {
        return c.to_string();
    }
    "XX".to_string()
}

fn decode_flag_emoji(name: &str) -> Option<String> {
    let chars: Vec<char> = name.chars().collect();
    for i in 0..chars.len().saturating_sub(1) {
        let a = chars[i] as u32;
        let b = chars[i + 1] as u32;
        if (0x1F1E6..=0x1F1FF).contains(&a) && (0x1F1E6..=0x1F1FF).contains(&b) {
            let c1 = (a - 0x1F1E6) as u8 + b'A';
            let c2 = (b - 0x1F1E6) as u8 + b'A';
            return Some(format!("{}{}", c1 as char, c2 as char));
        }
    }
    None
}

fn bracketed_code(name: &str) -> Option<String> {
    const OPENERS: &[char] = &['[', '(', '［', '【', '「'];
    const CLOSERS: &[char] = &[']', ')', '］', '】', '」'];
    let chars: Vec<char> = name.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if OPENERS.contains(&chars[i]) {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j + 1 < chars.len() && chars[j].is_ascii_alphabetic() && chars[j + 1].is_ascii_alphabetic() {
                let code = format!(
                    "{}{}",
                    chars[j].to_ascii_uppercase(),
                    chars[j + 1].to_ascii_uppercase()
                );
                let mut k = j + 2;
                while k < chars.len() && chars[k].is_whitespace() {
                    k += 1;
                }
                if k < chars.len() && CLOSERS.contains(&chars[k]) {
                    return Some(code);
                }
            }
        }
        i += 1;
    }
    None
}

fn prefix_code(name: &str) -> Option<String> {
    // Plan deviation: plan regex is `[A-Za-z]{2}[-_ \t]?[0-9A-Za-z]`, which would also
    // match "Frankfurt" (Fr+a) and "Tokyo" (To+k) and grab the wrong code. We tighten to
    // require a digit OR (separator + alphanumeric) after the 2 letters. This still
    // matches the plan's examples (US01, HK-01, JP_Tokyo) but rejects run-on letter words.
    let trimmed = name.trim_start();
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() < 3 {
        return None;
    }
    if !chars[0].is_ascii_alphabetic() || !chars[1].is_ascii_alphabetic() {
        return None;
    }
    let c2 = chars[2];
    let ok = c2.is_ascii_digit()
        || (matches!(c2, '-' | '_' | ' ' | '\t')
            && chars.get(3).map_or(false, |c| c.is_ascii_alphanumeric()));
    if !ok {
        return None;
    }
    Some(format!(
        "{}{}",
        chars[0].to_ascii_uppercase(),
        chars[1].to_ascii_uppercase()
    ))
}

fn chinese_keyword(name: &str) -> Option<&'static str> {
    let mut best: Option<(usize, &'static str)> = None;
    for (kw, code) in CN_KEYWORDS {
        if name.contains(kw) {
            let len = kw.chars().count();
            if best.map_or(true, |(l, _)| len > l) {
                best = Some((len, code));
            }
        }
    }
    best.map(|(_, c)| c)
}

fn english_keyword(name: &str) -> Option<&'static str> {
    let lower = name.to_lowercase();
    let mut best: Option<(usize, &'static str)> = None;
    for (kw, code) in EN_KEYWORDS {
        if lower.contains(kw) {
            let len = kw.len();
            if best.map_or(true, |(l, _)| len > l) {
                best = Some((len, code));
            }
        }
    }
    best.map(|(_, c)| c)
}

pub fn iso2_to_flag(iso2: &str) -> String {
    if iso2.len() != 2 || !iso2.chars().all(|c| c.is_ascii_uppercase()) {
        return "🏴".to_string();
    }
    let mut out = String::with_capacity(8);
    for c in iso2.chars() {
        let cp = 0x1F1E6u32 + (c as u32 - 'A' as u32);
        if let Some(c) = char::from_u32(cp) {
            out.push(c);
        }
    }
    out
}

pub fn iso2_to_display(iso2: &str) -> &'static str {
    match iso2 {
        "HK" => "Hong Kong",
        "JP" => "Japan",
        "SG" => "Singapore",
        "TW" => "Taiwan",
        "US" => "United States",
        "GB" => "United Kingdom",
        "DE" => "Germany",
        "KR" => "South Korea",
        "FR" => "France",
        "MY" => "Malaysia",
        "RU" => "Russia",
        "CA" => "Canada",
        "AU" => "Australia",
        "NL" => "Netherlands",
        "TR" => "Turkey",
        "IN" => "India",
        "BR" => "Brazil",
        "AR" => "Argentina",
        "TH" => "Thailand",
        "PH" => "Philippines",
        "VN" => "Vietnam",
        "ID" => "Indonesia",
        "AE" => "UAE",
        "IL" => "Israel",
        "ZA" => "South Africa",
        "IT" => "Italy",
        "ES" => "Spain",
        "CH" => "Switzerland",
        "SE" => "Sweden",
        "NO" => "Norway",
        "FI" => "Finland",
        "DK" => "Denmark",
        "PL" => "Poland",
        "UA" => "Ukraine",
        "MX" => "Mexico",
        "XX" => "Unknown",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> HashMap<String, String> {
        HashMap::new()
    }

    fn p(name: &str) -> String {
        parse_country(name, &empty())
    }

    // --- overrides ---
    #[test]
    fn override_wins_over_everything() {
        let mut o = HashMap::new();
        o.insert("🇭🇰 Tokyo".to_string(), "JP".to_string());
        assert_eq!(parse_country("🇭🇰 Tokyo", &o), "JP");
    }

    #[test]
    fn override_is_uppercased() {
        let mut o = HashMap::new();
        o.insert("weird".to_string(), "hk".to_string());
        assert_eq!(parse_country("weird", &o), "HK");
    }

    // --- flag emoji ---
    #[test]
    fn flag_emoji_hk() {
        assert_eq!(p("🇭🇰 香港 01"), "HK");
    }
    #[test]
    fn flag_emoji_jp() {
        assert_eq!(p("🇯🇵 Tokyo"), "JP");
    }
    #[test]
    fn flag_emoji_us() {
        assert_eq!(p("🇺🇸 美国 4K"), "US");
    }
    #[test]
    fn flag_emoji_sg() {
        assert_eq!(p("🇸🇬 SG · Premium"), "SG");
    }

    // --- bracketed ---
    #[test]
    fn bracketed_jp() {
        assert_eq!(p("[JP]Tokyo-Premium-2"), "JP");
    }
    #[test]
    fn bracketed_hk_lowercase() {
        assert_eq!(p("[hk] node"), "HK");
    }
    #[test]
    fn bracketed_parens() {
        assert_eq!(p("(US) Netflix"), "US");
    }
    #[test]
    fn bracketed_fullwidth() {
        assert_eq!(p("【KR】 Seoul"), "KR");
    }
    #[test]
    fn bracketed_unknown_code_falls_through() {
        // [ZZ] is not in known set; should fall through and miss country
        assert_eq!(p("[ZZ] random"), "XX");
    }

    // --- prefix ---
    #[test]
    fn prefix_us_netflix() {
        assert_eq!(p("US_Netflix_03"), "US");
    }
    #[test]
    fn prefix_hk_iepl() {
        assert_eq!(p("HK-IEPL-01"), "HK");
    }
    #[test]
    fn prefix_jp_tokyo() {
        assert_eq!(p("JP_Tokyo"), "JP");
    }
    #[test]
    fn prefix_us01_no_sep() {
        assert_eq!(p("US01"), "US");
    }
    #[test]
    fn prefix_unknown_code_falls_through() {
        assert_eq!(p("ZZ-01"), "XX");
    }
    #[test]
    fn prefix_skipped_when_followed_by_non_ascii() {
        // LA香港01直 — "LA" is not followed by ASCII alphanumeric ('香'), so prefix doesn't match,
        // and Chinese matcher picks 香港 → HK.
        assert_eq!(p("LA香港01直"), "HK");
    }

    // --- Chinese keywords (from live corpus) ---
    #[test]
    fn cn_hongkong_corpus() {
        assert_eq!(p("level1-香港01"), "HK");
    }
    #[test]
    fn cn_hongkong_anytls() {
        assert_eq!(p("香港AnyTLS测试01"), "HK");
    }
    #[test]
    fn cn_hongkong_direct() {
        assert_eq!(p("香港直连1"), "HK");
    }
    #[test]
    fn cn_taiwan_corpus() {
        assert_eq!(p("level1-台湾01"), "TW");
    }
    #[test]
    fn cn_taiwan_v2() {
        assert_eq!(p("V2-台湾 03"), "TW");
    }
    #[test]
    fn cn_singapore_corpus() {
        assert_eq!(p("level1-新加坡01-NF"), "SG");
    }
    #[test]
    fn cn_japan_corpus() {
        assert_eq!(p("level1-日本01-NF"), "JP");
    }
    #[test]
    fn cn_japan_osaka() {
        assert_eq!(p("日本 大阪 BGP"), "JP");
    }
    #[test]
    fn cn_france() {
        assert_eq!(p("level1-法国01"), "FR");
    }
    #[test]
    fn cn_korea() {
        assert_eq!(p("level1-韩国01"), "KR");
    }
    #[test]
    fn cn_malay_two_char() {
        // Live corpus uses 马来 (not 马来西亚) — must still resolve to MY.
        assert_eq!(p("level1-马来01"), "MY");
    }
    #[test]
    fn cn_malaysia_four_char() {
        assert_eq!(p("马来西亚-Premium"), "MY");
    }
    #[test]
    fn cn_germany() {
        assert_eq!(p("德国-Frankfurt 1"), "DE");
    }
    #[test]
    fn cn_uae_via_dubai() {
        assert_eq!(p("迪拜 1"), "AE");
    }
    #[test]
    fn cn_uae_full() {
        assert_eq!(p("阿联酋 1"), "AE");
    }
    #[test]
    fn cn_argentina_full_over_single_char() {
        // 阿根廷 (3-char) must win over 阿 (single-char) which would also map to AR — same answer
        // but verifies longest-match path runs without bug.
        assert_eq!(p("阿根廷-1"), "AR");
    }
    #[test]
    fn cn_indonesia_over_india() {
        // 印尼 (2-char ID) must beat 印 (1-char IN).
        assert_eq!(p("印尼-Jakarta"), "ID");
    }
    #[test]
    fn cn_indonesia_long() {
        assert_eq!(p("印度尼西亚-1"), "ID");
    }
    #[test]
    fn cn_india_single_char_via_full() {
        assert_eq!(p("印度-Mumbai"), "IN");
    }
    #[test]
    fn cn_sweden_over_swiss() {
        // 瑞典 (SE) must win over 瑞士 (CH) when both could partial-match — only 瑞典 is present.
        assert_eq!(p("瑞典-Stockholm"), "SE");
    }

    // --- English keywords ---
    #[test]
    fn en_tokyo() {
        assert_eq!(p("Tokyo-Premium"), "JP");
    }
    #[test]
    fn en_los_angeles() {
        assert_eq!(p("Los Angeles 4K"), "US");
    }
    #[test]
    fn en_united_kingdom_over_kingdom() {
        assert_eq!(p("United Kingdom Premium"), "GB");
    }
    #[test]
    fn en_frankfurt() {
        assert_eq!(p("Frankfurt-DE-1"), "DE");
    }
    #[test]
    fn en_singapore_case_insensitive() {
        assert_eq!(p("singapore-edge"), "SG");
    }
    #[test]
    fn en_amsterdam() {
        assert_eq!(p("Amsterdam-NL"), "NL");
    }

    // --- specials & fallback ---
    #[test]
    fn special_direct() {
        assert_eq!(p("DIRECT"), "XX");
    }
    #[test]
    fn special_reject() {
        assert_eq!(p("REJECT"), "XX");
    }
    #[test]
    fn special_auto() {
        assert_eq!(p("Auto"), "XX");
    }
    #[test]
    fn special_node_picker() {
        assert_eq!(p("🚀 节点选择"), "XX");
    }

    // --- ordering checks ---
    #[test]
    fn flag_beats_prefix() {
        // "AU" prefix would say AU, but a 🇯🇵 flag earlier should win.
        assert_eq!(p("🇯🇵 AU-fake-prefix"), "JP");
    }
    #[test]
    fn bracket_beats_chinese() {
        assert_eq!(p("[US] 香港路由"), "US");
    }

    // --- iso2_to_flag / iso2_to_display sanity ---
    #[test]
    fn flag_hk() {
        assert_eq!(iso2_to_flag("HK"), "🇭🇰");
    }
    #[test]
    fn flag_xx_invalid() {
        // "XX" produces a valid emoji sequence even though it isn't a real country.
        // The point is just: not "🏴" (which is for bad input).
        assert_ne!(iso2_to_flag("XX"), "🏴");
    }
    #[test]
    fn flag_lowercase_is_invalid() {
        assert_eq!(iso2_to_flag("hk"), "🏴");
    }
    #[test]
    fn display_known() {
        assert_eq!(iso2_to_display("JP"), "Japan");
        assert_eq!(iso2_to_display("XX"), "Unknown");
    }
}
