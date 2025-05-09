use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use inquire::{Autocomplete, CustomUserError, autocompletion::Replacement};

use crate::git::get_branches;

#[derive(Clone, Default)]
pub struct LocalBranchCompleter {
    input: String,
    branches: Vec<String>,
}

#[derive(Clone, Default)]
pub struct GlobalBranchCompleter {
    input: String,
    branches: Vec<String>,
}

trait BranchCompleter {
    fn update_input(&mut self, input: &str);
    fn get_branches(&self) -> &[String];

    fn fuzzy_sort(&self, input: &str) -> Vec<(String, i64)> {
        let mut matches: Vec<(String, i64)> = self
            .get_branches()
            .iter()
            .filter_map(|branch| {
                SkimMatcherV2::default()
                    .smart_case()
                    .fuzzy_match(branch, input)
                    .map(|score| (branch.clone(), score))
            })
            .collect();

        matches.sort_by(|a, b| b.1.cmp(&a.1));
        matches
    }

    fn get_last_word(input: &str) -> &str {
        if input.chars().nth(input.len() - 1) == Some(' ') {
            return "";
        }
        input.split_whitespace().last().unwrap_or("")
    }

    fn get_selected_branches(input: &str) -> Vec<String> {
        input.split_whitespace().map(String::from).collect()
    }
}

impl BranchCompleter for LocalBranchCompleter {
    fn update_input(&mut self, input: &str) {
        if input == self.input && !self.branches.is_empty() {
            return;
        }

        input.clone_into(&mut self.input);
        self.branches.clear();

        if let Ok(branches) = get_branches() {
            self.branches = branches;
        } else {
            self.branches = vec!["main".to_owned(), "master".to_owned()];
        }
    }

    fn get_branches(&self) -> &[String] {
        &self.branches
    }
}

impl BranchCompleter for GlobalBranchCompleter {
    fn update_input(&mut self, input: &str) {
        if input == self.input && !self.branches.is_empty() {
            return;
        }

        input.clone_into(&mut self.input);
        self.branches.clear();

        self.branches = vec!["main".to_owned(), "master".to_owned()];
    }

    fn get_branches(&self) -> &[String] {
        &self.branches
    }
}

macro_rules! impl_autocomplete {
    ($type:ty) => {
        impl Autocomplete for $type {
            fn get_suggestions(
                &mut self,
                input: &str,
            ) -> std::result::Result<Vec<String>, CustomUserError> {
                self.update_input(input);

                let last_word = Self::get_last_word(input);
                let selected_branches = Self::get_selected_branches(input);

                let matches = self.fuzzy_sort(last_word);
                Ok(matches
                    .into_iter()
                    .map(|(branch, _)| branch)
                    .filter(|branch| !selected_branches.contains(branch))
                    .take(15)
                    .collect())
            }

            fn get_completion(
                &mut self,
                input: &str,
                highlighted_suggestion: Option<String>,
            ) -> std::result::Result<Replacement, CustomUserError> {
                self.update_input(input);

                let mut selected_branches = Self::get_selected_branches(input);

                Ok(if let Some(suggestion) = highlighted_suggestion {
                    selected_branches.pop();
                    Replacement::Some(
                        selected_branches
                            .into_iter()
                            .chain(std::iter::once(suggestion))
                            .collect::<Vec<_>>()
                            .join(" "),
                    )
                } else {
                    let last_word = Self::get_last_word(input);
                    let matches = self.fuzzy_sort(last_word);

                    if let Some((branch, _)) = matches.first() {
                        selected_branches.pop();
                        Replacement::Some(
                            selected_branches
                                .into_iter()
                                .chain(std::iter::once(branch.clone()))
                                .collect::<Vec<_>>()
                                .join(" "),
                        )
                    } else {
                        Replacement::None
                    }
                })
            }
        }
    };
}

impl_autocomplete!(LocalBranchCompleter);
impl_autocomplete!(GlobalBranchCompleter);
