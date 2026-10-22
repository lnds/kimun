use super::*;

fn score(value: f64, grade: Grade) -> ProjectScore {
    ProjectScore {
        score: value,
        grade,
        files_analyzed: 1,
        total_loc: 10,
        dimensions: Vec::new(),
        needs_attention: Vec::new(),
    }
}

#[test]
fn the_colour_follows_the_letter_of_the_grade() {
    assert_eq!(colour(Grade::APlusPlus).0, "brightgreen");
    assert_eq!(colour(Grade::BMinus).0, "green");
    assert_eq!(colour(Grade::C).0, "yellow");
    assert_eq!(colour(Grade::DPlus).0, "orange");
    assert_eq!(colour(Grade::FMinusMinus).0, "red");
}

#[test]
fn the_endpoint_json_is_what_shields_reads() {
    let json: serde_json::Value =
        serde_json::from_str(&endpoint(&score(91.44, Grade::A)).unwrap()).unwrap();
    assert_eq!(json["schemaVersion"], 1);
    assert_eq!(json["label"], "km score");
    assert_eq!(json["message"], "A 91.4");
    assert_eq!(json["color"], "brightgreen");
}

#[test]
fn the_svg_is_self_contained_and_widens_with_its_text() {
    let short = svg(&score(55.0, Grade::D));
    let long = svg(&score(91.4, Grade::APlusPlus));
    assert!(short.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(short.contains("aria-label=\"km score: D 55.0\""));
    assert!(short.contains("#fe7d37"));
    assert!(!short.contains("href"), "no external reference");
    let width = |s: &str| -> usize {
        let at = s.find("width=\"").unwrap() + 7;
        s[at..].split('"').next().unwrap().parse().unwrap()
    };
    assert!(width(&long) > width(&short));
}

#[test]
fn write_picks_the_form_by_extension() {
    let dir = tempfile::tempdir().unwrap();
    let svg_path = dir.path().join("badge.svg");
    let json_path = dir.path().join("badge.JSON");
    write(&svg_path, &score(80.0, Grade::BPlus)).unwrap();
    write(&json_path, &score(80.0, Grade::BPlus)).unwrap();
    assert!(
        std::fs::read_to_string(svg_path)
            .unwrap()
            .starts_with("<svg")
    );
    assert!(
        std::fs::read_to_string(json_path)
            .unwrap()
            .contains("\"schemaVersion\": 1")
    );
}
