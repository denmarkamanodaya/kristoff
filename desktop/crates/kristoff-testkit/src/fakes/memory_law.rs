//! `MemoryLawSearch`: an in-memory `LawSearch` over a handful of provisions. Ranking is crude
//! (exact citation first, then shared words); the contract suite pins only what the real index
//! promises too.

use kristoff_core::domain::law::reference::{LawRef, parse_references};
use kristoff_core::domain::law::{ArticleView, LawFilters, LawHit, LawSummary};
use kristoff_core::error::AppResult;
use kristoff_core::ports::LawSearch;

/// One stored provision.
#[derive(Debug, Clone)]
pub struct FakeProvision {
    /// The law it belongs to.
    pub law: LawSummary,
    /// The law as a citation target.
    pub law_ref: LawRef,
    /// Provision label, e.g. `SEC. 5`.
    pub label: String,
    /// Whole provision text.
    pub text: String,
}

/// In-memory law index; provisions keep their insertion order as the law's order.
#[derive(Debug, Clone, Default)]
pub struct MemoryLawSearch {
    provisions: Vec<FakeProvision>,
}

const VAWC: (&str, &str) = (
    "Republic Act No. 9262",
    "AN ACT DEFINING VIOLENCE AGAINST WOMEN AND THEIR CHILDREN",
);
const CHARTER: (&str, &str) = (
    "1987 Constitution",
    "The 1987 Constitution of the Republic of the Philippines",
);
const VAWC_SECTIONS: [(&str, &str); 3] = [
    (
        "SEC. 4",
        "SEC. 4. Construction.— This Act shall be liberally construed.",
    ),
    (
        "SEC. 5",
        "SEC. 5. Acts of Violence Against Women and Their Children.— The crime of violence \
         against women and their children is committed through any of the following acts:",
    ),
    (
        "SEC. 6",
        "SEC. 6. Penalties.— The crime of violence against women and their children shall be \
         punished.",
    ),
];
const CHARTER_SECTIONS: [(&str, &str); 2] = [
    (
        "ARTICLE III SECTION 1",
        "SECTION 1. No person shall be deprived of life, liberty, or property without due \
         process of law.",
    ),
    (
        "ARTICLE III SECTION 2",
        "SECTION 2. The right of the people to be secure in their persons, houses, papers, and \
         effects.",
    ),
];

fn law(identifier: &str, title: &str, number: Option<u32>, year: u32) -> LawSummary {
    LawSummary {
        identifier: identifier.to_owned(),
        title: title.to_owned(),
        number,
        year: Some(year),
    }
}

fn provision(law: &LawSummary, law_ref: LawRef, label: &str, text: &str) -> FakeProvision {
    FakeProvision {
        law: law.clone(),
        law_ref,
        label: label.to_owned(),
        text: text.to_owned(),
    }
}

impl MemoryLawSearch {
    /// An index holding the provisions the contract suite asks about (RA 9262 Secs. 4–6 and
    /// Constitution Art. III Secs. 1–2), with their real opening words.
    #[must_use]
    pub fn sample() -> Self {
        let vawc = law(VAWC.0, VAWC.1, Some(9262), 2004);
        let charter = law(CHARTER.0, CHARTER.1, None, 1987);
        let ra = LawRef::RepublicAct(9262);
        let provisions = VAWC_SECTIONS
            .iter()
            .map(|(label, text)| provision(&vawc, ra, label, text))
            .chain(
                CHARTER_SECTIONS
                    .iter()
                    .map(|(label, text)| provision(&charter, LawRef::Constitution, label, text)),
            )
            .collect();
        Self { provisions }
    }

    fn hit(&self, i: usize, score: f64) -> LawHit {
        let p = &self.provisions[i];
        LawHit {
            chunk_id: i64::try_from(i).unwrap(),
            law: p.law.clone(),
            label: p.label.clone(),
            text: p.text.clone(),
            source_url: format!("https://lawphil.net/fake/{}", p.law.number.unwrap_or(1987)),
            status: "current".to_owned(),
            truncated: false,
            part: 0,
            score,
        }
    }

    /// Index of the provision of `law` whose label carries the cited section/article.
    fn find(&self, law: LawRef, article: &str) -> Option<usize> {
        let want = parse_references(article);
        self.provisions.iter().position(|p| {
            let have = parse_references(&p.label);
            p.law_ref == law
                && want.section().is_none_or(|s| have.section() == Some(s))
                && want.article().is_none_or(|a| have.article() == Some(a))
                && (want.section().is_some() || want.article().is_some())
        })
    }

    fn word_score(&self, i: usize, query: &str) -> f64 {
        let text = self.provisions[i].text.to_lowercase();
        let shared = query
            .to_lowercase()
            .split_whitespace()
            .filter(|w| text.contains(w))
            .count();
        f64::from(u32::try_from(shared).unwrap_or(u32::MAX))
    }
}

impl LawSearch for MemoryLawSearch {
    fn warm(&self) -> AppResult<()> {
        Ok(())
    }

    fn law_count(&self) -> AppResult<Option<u64>> {
        let mut laws: Vec<&str> = self
            .provisions
            .iter()
            .map(|p| p.law.identifier.as_str())
            .collect();
        laws.dedup();
        Ok(Some(laws.len() as u64))
    }

    fn search(&self, query: &str, filters: &LawFilters, limit: usize) -> AppResult<Vec<LawHit>> {
        let refs = parse_references(query);
        let exact = refs.laws.iter().find_map(|l| self.find(*l, query));
        let mut scored: Vec<(usize, f64)> = (0..self.provisions.len())
            .map(|i| {
                (
                    i,
                    if Some(i) == exact {
                        1e9
                    } else {
                        self.word_score(i, query)
                    },
                )
            })
            .filter(|(i, s)| {
                *s > 0.0
                    && filters.admits(self.provisions[*i].law.number, self.provisions[*i].law.year)
            })
            .collect();
        scored.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        Ok(scored
            .into_iter()
            .take(limit)
            .map(|(i, s)| self.hit(i, s))
            .collect())
    }

    fn get_article(&self, law: LawRef, article: &str) -> AppResult<Option<ArticleView>> {
        let Some(i) = self.find(law, article) else {
            return Ok(None);
        };
        let same_law = |j: usize| {
            self.provisions
                .get(j)
                .filter(|p| p.law_ref == law)
                .map(|_| self.hit(j, 0.0))
        };
        Ok(Some(ArticleView {
            hit: self.hit(i, 0.0),
            previous: i.checked_sub(1).and_then(same_law),
            next: same_law(i + 1),
        }))
    }
}
