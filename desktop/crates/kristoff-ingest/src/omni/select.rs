//! Which `OmniCorpus` rows Phase 7 ingests, and the law metadata for each statute row. Owner ruling
//! D-20 (2026-11-12): every legal statute and issuance (the codes, the current Rules of Court,
//! Acts, CA, BP, PD, LOI, GO, all executive issuances and the RAs the current index lacks);
//! decisions go to `phcases.db`.

use kristoff_law::schema::{
    AS_ENACTED, AS_ISSUED, AS_PROMULGATED, AS_SIGNED, CONSTITUTIONAL_RECORDS, COURT_REFERENCE, kind,
};

use super::read::Info;

/// Rules of Court documents kept, by `short_title`, with their display identifier. Older rules
/// (1900, 1918, 1964, 1985, 1988) are history, not current procedure, and are left out.
const RULES: [(&str, &str); 5] = [
    (
        "1997 Rules of Civil Procedure",
        "1997 Rules of Civil Procedure (Rules of Court)",
    ),
    (
        "Revised Rules of Criminal Procedure",
        "Revised Rules of Criminal Procedure (Rules of Court)",
    ),
    (
        "Revised Rules on Evidence (Rules 128-134",
        "Revised Rules on Evidence (Rules of Court)",
    ),
    (
        "A.M. No. 19-08-15-Sc",
        "A.M. No. 19-08-15-SC (2019 Amendments to the Revised Rules on Evidence)",
    ),
    (
        "A.M. No. 19-10-20-Sc",
        "A.M. No. 19-10-20-SC (2019 Amendments to the 1997 Rules of Civil Procedure)",
    ),
];

/// Codes known by name: (kind, number, date prefix, name appended to the identifier, so search
/// on "Revised Penal Code" or "Labor Code" finds the provisions).
const CODES: [(&str, u32, &str, &str); 4] = [
    (kind::ACT, 3815, "1930", "Revised Penal Code"),
    (kind::EO, 209, "1987", "Family Code of the Philippines"),
    (kind::EO, 227, "1987", "amending the Family Code"),
    (kind::PD, 442, "1974", "Labor Code of the Philippines"),
];

/// Metadata of a statute row to ingest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statute {
    /// `laws.kind`.
    pub kind: &'static str,
    /// Number within its kind, when it has one.
    pub number: Option<u32>,
    /// Display identifier, e.g. `Act No. 3815 (Revised Penal Code)`.
    pub identifier: String,
    /// `laws.edition`: `As enacted`, `As promulgated` or `As issued`.
    pub edition: &'static str,
}

/// True for a decision row.
#[must_use]
pub fn is_decision(info: &Info) -> bool {
    info.kind == "court_decision"
}

fn number_in(short_title: &str) -> Option<u32> {
    let digits: String = short_title
        .split(|c: char| !c.is_ascii_digit())
        .find(|t| !t.is_empty())?
        .to_owned();
    digits.parse().ok()
}

/// `(laws.kind, edition)` for an `OmniCorpus` type and `E-Library` shelf; `None` for rows not
/// ingested as statutes.
fn kind_of(info_kind: &str, shelf: &str) -> Option<(&'static str, &'static str)> {
    Some(match info_kind {
        "republic_act" => (kind::RA, AS_ENACTED),
        "act" => (kind::ACT, AS_ENACTED),
        "commonwealth_act" => (kind::CA, AS_ENACTED),
        "batas_pambansa" => (kind::BP, AS_ENACTED),
        "presidential_decree" => (kind::PD, AS_ENACTED),
        "letter_of_instruction" => (kind::LOI, AS_ISSUED),
        "letter_of_implementation" => (kind::LOIMP, AS_ISSUED),
        "general_order" => (kind::GO, AS_ISSUED),
        "treaty" => (kind::TREATY, AS_SIGNED),
        "constitution" => (kind::CONREC, CONSTITUTIONAL_RECORDS),
        "other_legal_document" => match shelf {
            "1986 ConCom" | "1934-35 ConCon" | "1986 Draft Constitution" => {
                (kind::CONREC, CONSTITUTIONAL_RECORDS)
            }
            "Benchbooks"
            | "Manuals of Clerks of Court"
            | "Manual of Judicial Writing"
            | "Fundamentals of Decision Writing" => (kind::COURTREF, COURT_REFERENCE),
            _ => return None,
        },
        "executive_issuance" => match shelf {
            "Executive Orders" => (kind::EO, AS_ISSUED),
            "Presidential Proclamations" => (kind::PROC, AS_ISSUED),
            "Administrative Orders" => (kind::AO, AS_ISSUED),
            "Memorandum Orders" => (kind::MO, AS_ISSUED),
            "Memorandum Circulars" => (kind::MC, AS_ISSUED),
            _ => (kind::NAR, AS_ISSUED),
        },
        _ => return None,
    })
}

/// Why a non-decision row is left out, for the ingest log (lead decision 2026-11-12); `None`
/// for rows that are ingested.
#[must_use]
pub fn exclusion(info: &Info, shelf: &str) -> Option<&'static str> {
    let short = info.short_title.as_deref().unwrap_or("");
    match (info.kind.as_str(), shelf) {
        ("court_decision", _) => None,
        (_, "Lawyers' List") => Some("Lawyers' List: a personal-data list of lawyers"),
        (_, "Official Gazette") => {
            Some("Official Gazette notices: noise (land, publication and court notices)")
        }
        (_, "Election Cases") => Some("Election Cases: one 1.4 MB blob of unstructured case lists"),
        ("constitution", _) if short.starts_with("1987") => {
            Some("1987 Constitution: already in the current dataset, structured")
        }
        ("republic_act", _) if !short.starts_with("Republic Act No.") => {
            Some("IRRs and circulars on the Republic Acts shelf")
        }
        ("rules_of_court", _) if rules(short).is_none() => {
            Some("superseded Rules of Court (1900, 1918, 1964, 1985, 1988)")
        }
        _ if statute(info, shelf).is_none() => Some("other: not a recognised legal type"),
        _ => None,
    }
}

/// The statute metadata for a row Phase 7 ingests, or `None` for rows it leaves out.
#[must_use]
pub fn statute(info: &Info, shelf: &str) -> Option<Statute> {
    let short = info.short_title.as_deref().unwrap_or("").trim();
    if info.kind == "rules_of_court" {
        return rules(short);
    }
    let (kind, mut edition) = kind_of(&info.kind, shelf)?;
    if kind == kind::RA && !short.starts_with("Republic Act No.") {
        return None; // IRRs and circulars filed under the Republic Acts shelf
    }
    if kind == kind::CONREC && short.starts_with("1987") {
        return None; // the current dataset holds the 1987 Constitution, structured
    }
    let number = number_in(short);
    let year = info.any_date().unwrap_or("");
    let code = CODES
        .iter()
        .find(|(k, n, y, _)| *k == kind && Some(*n) == number && year.starts_with(y));
    let identifier = match code {
        Some((_, _, _, name)) => {
            edition = AS_ENACTED;
            format!("{short} ({name})")
        }
        None if short.is_empty() => info.title.as_deref().unwrap_or("").trim().to_owned(),
        None => short.to_owned(),
    };
    (!identifier.is_empty()).then_some(Statute {
        kind,
        number,
        identifier,
        edition,
    })
}

fn rules(short_title: &str) -> Option<Statute> {
    RULES
        .iter()
        .find(|(s, _)| short_title.starts_with(s))
        .map(|(_, id)| Statute {
            kind: kind::RULES,
            number: None,
            identifier: (*id).to_owned(),
            edition: AS_PROMULGATED,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st(kind: &str, short: &str, date: &str, shelf: &str) -> Option<Statute> {
        let info = Info {
            kind: kind.into(),
            short_title: Some(short.into()),
            date_of_enactment: Some(date.into()),
            ..Info::default()
        };
        statute(&info, shelf)
    }

    #[test]
    fn names_the_codes_as_enacted() {
        let rpc = st("act", "Act No. 3815", "1930-12-08", "Acts").unwrap();
        let fc = st(
            "executive_issuance",
            "Executive Order No. 209",
            "1987-07-06",
            "Executive Orders",
        )
        .unwrap();

        assert_eq!(
            (rpc.identifier.as_str(), rpc.edition),
            ("Act No. 3815 (Revised Penal Code)", AS_ENACTED)
        );
        assert_eq!((rpc.kind, rpc.number), (kind::ACT, Some(3815)));
        assert_eq!(
            fc.identifier,
            "Executive Order No. 209 (Family Code of the Philippines)"
        );
        assert_eq!(fc.edition, AS_ENACTED);
    }

    #[test]
    fn labels_old_statutes_and_issuances_by_kind() {
        let pd = st(
            "presidential_decree",
            "Presidential Decree No. 1251",
            "1977-11-28",
            "Presidential Decree",
        )
        .unwrap();
        let eo = st(
            "executive_issuance",
            "Executive Order No. 209",
            "1949-03-30",
            "Executive Orders",
        )
        .unwrap();
        let nar = st(
            "executive_issuance",
            "BSP Circular No. 646",
            "2009-02-23",
            "National Administrative Register",
        )
        .unwrap();
        let treaty = st("treaty", "Agreement X", "1995-12-02", "Bilateral").unwrap();

        assert_eq!(
            (pd.kind, pd.number, pd.edition),
            (kind::PD, Some(1251), AS_ENACTED)
        );
        assert_eq!((eo.kind, eo.edition), (kind::EO, AS_ISSUED));
        assert_eq!((nar.kind, nar.edition), (kind::NAR, AS_ISSUED));
        assert_eq!((treaty.kind, treaty.edition), (kind::TREATY, AS_SIGNED));
    }

    #[test]
    fn keeps_only_the_current_rules_as_promulgated() {
        let civ = st(
            "rules_of_court",
            "1997 Rules of Civil Procedure",
            "1997-07-01",
            "Rules of Court",
        )
        .unwrap();

        assert_eq!((civ.kind, civ.edition), (kind::RULES, AS_PROMULGATED));
        assert_eq!(
            st(
                "rules_of_court",
                "Rules of Court in the Philippines",
                "1964-01-01",
                "Rules of Court"
            ),
            None
        );
    }

    #[test]
    fn leaves_out_decisions_irrs_the_1987_copy_and_noise_with_a_reason() {
        assert_eq!(
            st(
                "court_decision",
                "",
                "2000-01-01",
                "Decisions / Signed Resolutions"
            ),
            None
        );
        assert_eq!(
            st(
                "republic_act",
                "IRR of REPUBLIC ACT NO. 9255",
                "2023-01-01",
                "Republic Acts"
            ),
            None
        );
        assert_eq!(
            st(
                "constitution",
                "1987 Constitution",
                "1987-02-02",
                "Constitutions"
            ),
            None
        );
        assert_eq!(
            st("other_legal_document", "", "2012-04-16", "Official Gazette"),
            None
        );
        let lawyers = Info {
            kind: "other_legal_document".into(),
            ..Info::default()
        };
        assert!(exclusion(&lawyers, "Lawyers' List").is_some());
    }
}
