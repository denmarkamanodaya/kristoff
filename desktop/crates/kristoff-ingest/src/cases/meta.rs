//! Decision metadata read from `OmniCorpus` decision rows (Phase 7): docket, division, ponente,
//! short title. Pure functions.

/// Longest party name kept in a short title.
const PARTY_MAX: usize = 48;

/// Docket and division from the raw head, e.g. `731 PHIL. 339FIRST DIVISION[ G.R. No. 206770,
/// April 02, 2014 ]` gives (`G.R. No. 206770`, `FIRST DIVISION`).
#[must_use]
pub fn docket_and_division(raw: &str) -> (Option<String>, Option<String>) {
    let head: String = raw.chars().take(800).collect();
    let Some(open) = head.find("[ ") else {
        return (None, None);
    };
    let Some(close) = head[open..].find(']').map(|c| c + open) else {
        return (None, None);
    };
    let inside = head[open + 2..close].trim();
    let docket = inside
        .rsplit_once(", ")
        .map(|(d, _)| d)
        .and_then(|d| d.rsplit_once(", ").map_or(Some(d), |(d2, _)| Some(d2)));
    let division = [
        "EN BANC",
        "FIRST DIVISION",
        "SECOND DIVISION",
        "THIRD DIVISION",
        "SPECIAL",
    ]
    .into_iter()
    .find(|d| head[..open].contains(d))
    .map(str::to_owned);
    (
        docket
            .map(|d| d.trim().to_owned())
            .filter(|d| !d.is_empty()),
        division,
    )
}

/// The ponente from the opening, e.g. `…D E C I S I O NREYES, J.:` gives `REYES`;
/// `PER CURIAM:` gives `PER CURIAM`.
#[must_use]
pub fn ponente(clean: &str) -> Option<String> {
    let head: String = clean.chars().take(5_000).collect();
    if head.contains("PER CURIAM") {
        return Some("PER CURIAM".to_owned());
    }
    let marker = [
        "D E C I S I O N",
        "R E S O L U T I O N",
        "DECISION",
        "RESOLUTION",
    ]
    .into_iter()
    .find_map(|m| head.find(m).map(|i| i + m.len()))?;
    let after = &head[marker..];
    let end = after
        .find(", J.:")
        .or_else(|| after.find(", C.J.:"))
        .or_else(|| after.find(", J:"))?;
    let name = after[..end].trim();
    let ok = !name.is_empty()
        && name.len() <= 40
        && name.chars().all(|c| c.is_uppercase() || " .-'".contains(c));
    ok.then(|| name.to_owned())
}

/// `People of the Philippines, Plaintiff-Appellee, vs. Noel Prajes…` gives
/// `People of the Philippines v. Noel Prajes`.
#[must_use]
pub fn short_title(case_name: &str) -> String {
    let party = |s: &str| {
        let s = s
            .split(',')
            .next()
            .unwrap_or(s)
            .trim()
            .trim_end_matches('.');
        let cut: String = s.chars().take(PARTY_MAX).collect();
        if cut.len() < s.len() {
            format!("{}…", cut.trim_end())
        } else {
            cut
        }
    };
    let lower = case_name.to_lowercase();
    match lower
        .find(" vs. ")
        .or_else(|| lower.find(" vs "))
        .or_else(|| lower.find(" v. "))
    {
        Some(at) => {
            // `[at..]` is " vs. Noel …": drop the separator word.
            let right = case_name[at..]
                .trim_start()
                .split_once(' ')
                .map_or("", |(_, r)| r);
            format!("{} v. {}", party(&case_name[..at]), party(right))
        }
        None => party(case_name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_docket_and_division_from_the_head() {
        let raw = "731 PHIL. 339FIRST DIVISION[ G.R. No. 206770, April 02, 2014 ]PEOPLE v. X";

        assert_eq!(
            docket_and_division(raw),
            (
                Some("G.R. No. 206770".into()),
                Some("FIRST DIVISION".into())
            )
        );
    }

    #[test]
    fn reads_the_ponente_after_the_decision_marker() {
        assert_eq!(
            ponente("PEOPLE VS. X. D E C I S I O NREYES, J.:Before the Court").as_deref(),
            Some("REYES")
        );
        assert_eq!(
            ponente("X R E S O L U T I O NPER CURIAM:This").as_deref(),
            Some("PER CURIAM")
        );
        assert_eq!(ponente("no marker here"), None);
    }

    #[test]
    fn shortens_the_case_name_to_first_parties() {
        let name = "People of the Philippines, Plaintiff-Appellee, vs. Noel Prajes and Alipa Mala, Accused-Appellants.";

        assert_eq!(
            short_title(name),
            "People of the Philippines v. Noel Prajes and Alipa Mala"
        );
    }
}
