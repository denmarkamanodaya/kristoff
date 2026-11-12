//! Real PDFs made with lopdf, for the splitter's contract and integration tests (no binary
//! fixtures). Page `p` shows "PAGE p" and can carry incompressible filler to make it heavy.

use lopdf::content::{Content, Operation};
use lopdf::{Document, Object, ObjectId, Stream, dictionary};

/// Incompressible filler bytes from a fixed-seed LCG.
fn filler_bytes(seed: &mut u32, len: usize) -> Vec<u8> {
    (0..len)
        .map(|_| {
            *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            seed.to_be_bytes()[0]
        })
        .collect()
}

fn add_page(doc: &mut Document, pages_id: ObjectId, text: String, filler: Vec<u8>) -> Object {
    let content = Content {
        operations: vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 12.into()]),
            Operation::new("Td", vec![50.into(), 600.into()]),
            Operation::new("Tj", vec![Object::string_literal(text)]),
            Operation::new("ET", vec![]),
        ],
    };
    let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
    let filler_id = doc.add_object(Stream::new(dictionary! {}, filler));
    doc.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages_id, "Contents" => content_id,
        "PieceInfo" => dictionary! { "Filler" => filler_id },
    })
    .into()
}

/// A PDF with `pages` pages; page `p` shows "PAGE p" and carries `filler(p)` incompressible bytes.
#[must_use]
pub fn make_pdf(pages: u32, filler: impl Fn(u32) -> usize) -> Vec<u8> {
    make_pdf_with_text(pages, filler, |p| format!("PAGE {p}"))
}

/// A PDF with `pages` pages; page `p` shows `text(p)` and carries `filler(p)` filler bytes.
///
/// # Panics
/// When lopdf cannot write it (a test-setup failure).
#[must_use]
pub fn make_pdf_with_text(
    pages: u32,
    filler: impl Fn(u32) -> usize,
    text: impl Fn(u32) -> String,
) -> Vec<u8> {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let font = dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica" };
    let font_id = doc.add_object(font);
    let resources_id = doc.add_object(dictionary! { "Font" => dictionary! { "F1" => font_id } });
    let mut seed: u32 = 0x1234_5678;
    let kids: Vec<Object> = (1..=pages)
        .map(|p| {
            let bytes = filler_bytes(&mut seed, filler(p));
            add_page(&mut doc, pages_id, text(p), bytes)
        })
        .collect();
    let tree = dictionary! {
        "Type" => "Pages", "Kids" => kids, "Count" => i64::from(pages), "Resources" => resources_id,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    };
    doc.objects.insert(pages_id, Object::Dictionary(tree));
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);
    let mut out = Vec::new();
    doc.save_to(&mut out).unwrap();
    out
}

/// The text of page 1 of `pdf` (to check which pages a part kept).
///
/// # Panics
/// When `pdf` is not readable.
#[must_use]
pub fn first_page_text(pdf: &[u8]) -> String {
    Document::load_mem(pdf).unwrap().extract_text(&[1]).unwrap()
}
