use crate::snapd_client::{
    interfaces::home::HomeInterface, ConstraintsFilter, Prompt, PromptReply, SnapdInterface,
};
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PromptSequence {
    version: u8,
    prompts: Vec<TypedPromptCase>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum TypedPromptCase {
    Home(PromptCase<HomeInterface>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PromptCase<I: SnapdInterface> {
    prompt_filter: PromptFilter<I>,
    reply: PromptReply<I>,
}

impl<I: SnapdInterface> PromptCase<I> {
    pub fn into_reply_or_error(self, p: Prompt<I>) -> Result<PromptReply<I>, Vec<MatchFailure>> {
        match self.prompt_filter.matches(&p) {
            MatchAttempt::Success => Ok(self.reply),
            MatchAttempt::Failure(failures) => Err(failures),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchAttempt {
    Success,
    Failure(Vec<MatchFailure>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MatchFailure {
    pub(crate) field: &'static str,
    pub(crate) expected: String,
    pub(crate) seen: String,
}

#[macro_export]
macro_rules! field_matches {
    ($self:ident, $other:ident, $failures:ident, $field:ident) => {
        if let Some(field) = &$self.$field {
            if field != &$other.$field {
                $failures.push(MatchFailure {
                    field: stringify!($field),
                    expected: format!("{:?}", field),
                    seen: format!("{:?}", $other.$field),
                });
            }
        }
    };
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PromptFilter<I: SnapdInterface> {
    snap: Option<String>,
    interface: Option<String>, // should this be an enum?
    constraints: Option<I::ConstraintsFilter>,
}

impl<I: SnapdInterface> PromptFilter<I> {
    pub fn with_snap(&mut self, snap: impl Into<String>) -> &mut Self {
        self.snap = Some(snap.into());
        self
    }

    pub fn with_interface(&mut self, interface: impl Into<String>) -> &mut Self {
        self.interface = Some(interface.into());
        self
    }

    pub fn with_constraints(&mut self, constraints: I::ConstraintsFilter) -> &mut Self {
        self.constraints = Some(constraints);
        self
    }

    pub fn matches(&self, p: &Prompt<I>) -> MatchAttempt {
        let mut failures = Vec::new();
        field_matches!(self, p, failures, snap);
        field_matches!(self, p, failures, interface);

        match &self.constraints {
            None if failures.is_empty() => MatchAttempt::Success,
            None => MatchAttempt::Failure(failures),
            Some(c) => match c.matches(&p.constraints) {
                MatchAttempt::Success if failures.is_empty() => MatchAttempt::Success,
                MatchAttempt::Success => MatchAttempt::Failure(failures),
                MatchAttempt::Failure(c_failures) => {
                    failures.extend(c_failures);
                    MatchAttempt::Failure(failures)
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapd_client::{
        interfaces::home::{HomeConstraints, HomeConstraintsFilter},
        PromptId,
    };
    use simple_test_case::{dir_cases, test_case};

    #[dir_cases("resources/filter-serialize-tests")]
    #[test]
    fn simple_serialize_works(path: &str, data: &str) {
        let res = serde_json::from_str::<'_, PromptFilter<HomeInterface>>(data);

        assert!(res.is_ok(), "error parsing {path}: {:?}", res);
    }

    #[test]
    fn all_fields_deserializes_correctly() {
        let data = include_str!("../resources/filter-serialize-tests/all_fields_home.json");
        let res = serde_json::from_str::<'_, PromptFilter<HomeInterface>>(data);

        assert!(res.is_ok(), "error {:?}", res);

        match res.unwrap() {
            PromptFilter {
                snap,
                interface,
                constraints:
                    Some(HomeConstraintsFilter {
                        path,
                        permissions,
                        available_permissions,
                    }),
            } => {
                assert_eq!(snap.as_deref(), Some("snapName"));
                assert_eq!(interface.as_deref(), Some("home"));
                assert_eq!(path.as_deref(), Some("/home/foo/bar"));
                assert_eq!(permissions, Some(vec!["read".to_string()]));
                assert_eq!(
                    available_permissions,
                    Some(vec![
                        "read".to_string(),
                        "write".to_string(),
                        "execute".to_string()
                    ])
                );
            }
            f => panic!("invalid filter: {f:?}"),
        }
    }

    fn mf(field: &'static str, expected: &str, seen: &str) -> MatchFailure {
        MatchFailure {
            field,
            expected: expected.to_string(),
            seen: seen.to_string(),
        }
    }

    #[test_case(r#"{}"#, MatchAttempt::Success; "empty filter")]
    #[test_case(r#"{ "interface": "home" }"#, MatchAttempt::Success; "interface matching")]
    #[test_case(
        r#"{ "interface": "home", "constraints": { "path": "/home/foo/bar" } }"#,
        MatchAttempt::Success;
        "interface and path matching"
    )]
    #[test_case(
        r#"{ "interface": "camera" }"#,
        MatchAttempt::Failure(vec![mf("interface", "\"camera\"", "\"home\"")]);
        "interface non-matching"
    )]
    #[test_case(
        r#"{ "interface": "home", "constraints": { "path": "/home/wrong/path" } }"#,
        MatchAttempt::Failure(vec![mf("path", "\"/home/wrong/path\"", "\"/home/foo/bar\"")]);
        "interface matching path non-matching"
    )]
    #[test_case(
        r#"{ "interface": "camera", "constraints": { "path": "/home/wrong/path" } }"#,
        MatchAttempt::Failure(vec![
            mf("interface", "\"camera\"", "\"home\""),
            mf("path", "\"/home/wrong/path\"", "\"/home/foo/bar\""),
        ]);
        "interface and path non-matching"
    )]
    #[test]
    fn filter_matches(filter_str: &str, expected: MatchAttempt) {
        let filter: PromptFilter<HomeInterface> = serde_json::from_str(filter_str).unwrap();
        let p = Prompt {
            id: PromptId("id".to_string()),
            interface: "home".to_string(),
            timestamp: "".to_string(),
            snap: "test".to_string(),
            constraints: HomeConstraints {
                path: "/home/foo/bar".to_string(),
                permissions: vec!["read".to_string()],
                available_permissions: vec!["read".to_string(), "write".to_string()],
            },
        };

        assert_eq!(filter.matches(&p), expected);
    }

    #[dir_cases("resources/prompt-sequence-tests")]
    #[test]
    fn deserialize_prompt_sequence_works(path: &str, data: &str) {
        let res = serde_json::from_str::<'_, PromptSequence>(data);

        assert!(res.is_ok(), "error parsing {path}: {:?}", res);
    }
}
