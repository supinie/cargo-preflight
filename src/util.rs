pub fn display_vecs(vec: &[String]) -> String {
    vec.iter()
        .map(|item| format!("- {item}"))
        .collect::<Vec<String>>()
        .join("\n")
}

pub fn display_checks(vec: &[String]) -> String {
    vec.iter()
        .map(|item| format!("[ ] {item}"))
        .collect::<Vec<String>>()
        .join("\n")
}
