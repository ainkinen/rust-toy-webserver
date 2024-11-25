use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
enum PathSegment {
    Static(String),
    Dynamic(String),
    Wildcard,
}

pub type RouteParams = HashMap<String, String>;

#[derive(Debug, PartialEq)]
pub struct RoutePattern {
    segments: Vec<PathSegment>,
}

impl RoutePattern {
    pub fn _new() -> Self {
        RoutePattern { segments: vec![] }
    }

    pub fn matches(&self, path: &str) -> Option<RouteParams> {
        let path_segments: Vec<_> = path.trim_matches('/').split('/').collect();

        let mut params = HashMap::new();
        let mut path_idx = 0;

        for segment in &self.segments {
            match segment {
                PathSegment::Static(expected) => {
                    if path_idx >= path_segments.len() || path_segments[path_idx] != expected {
                        return None;
                    }
                    path_idx += 1;
                }
                PathSegment::Dynamic(name) => {
                    if path_idx >= path_segments.len() {
                        return None;
                    }
                    params.insert(name.to_string(), path_segments[path_idx].to_string());
                    path_idx += 1;
                }
                PathSegment::Wildcard => {
                    params.insert("*".to_string(), path_segments[path_idx..].join("/"));
                    return Some(params);
                }
            }
        }

        // Check if all path segments were matched
        if path_idx == path_segments.len() {
            Some(params)
        } else {
            None
        }
    }
}

impl FromStr for RoutePattern {
    type Err = String;
    fn from_str(path: &str) -> Result<Self, Self::Err> {
        let segments = path
            .trim_matches('/')
            .split('/')
            .map(|segment| {
                if let Some(end) = segment.strip_prefix(':') {
                    PathSegment::Dynamic(end.to_string())
                } else if segment.starts_with('*') {
                    PathSegment::Wildcard
                } else {
                    PathSegment::Static(segment.to_string())
                }
            })
            .collect();

        Ok(RoutePattern { segments })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_route_pattern() {
        let parsed: RoutePattern = "foo/bar/baz/".parse().unwrap();

        let expected = RoutePattern {
            segments: vec![
                PathSegment::Static("foo".to_string()),
                PathSegment::Static("bar".to_string()),
                PathSegment::Static("baz".to_string()),
            ],
        };

        assert_eq!(parsed.segments.len(), 3);
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_parse_route_pattern_with_params() {
        let parsed: RoutePattern = "foo/:param1".parse().unwrap();

        let expected = RoutePattern {
            segments: vec![
                PathSegment::Static("foo".to_string()),
                PathSegment::Dynamic("param1".to_string()),
            ],
        };

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_parse_route_pattern_with_wildcard() {
        let parsed: RoutePattern = "prefix/*".parse().unwrap();

        let expected = RoutePattern {
            segments: vec![
                PathSegment::Static("prefix".to_string()),
                PathSegment::Wildcard,
            ],
        };

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_static_route_pattern_matching() {
        let pattern: RoutePattern = "foo/bar/baz/".parse().unwrap();

        assert!(pattern.matches("/foo/bar/baz/").is_some());
        assert!(pattern.matches("/fizz/buzz/").is_none());
    }

    #[test]
    fn test_dynamic_matching_returns_variables() {
        let pattern: RoutePattern = "foo/:param1/baz/:param2".parse().unwrap();

        let params = pattern.matches("/foo/1234/baz/yeah").unwrap();
        let expected = HashMap::from([
            ("param1".to_string(), "1234".to_string()),
            ("param2".to_string(), "yeah".to_string()),
        ]);
        assert_eq!(params, expected);
    }

    #[test]
    fn test_wildcard_matching() {
        let pattern: RoutePattern = "foo/*".parse().unwrap();
        let params = pattern.matches("/foo/1234/baz/yeah").unwrap();

        let expected = HashMap::from([("*".to_string(), "1234/baz/yeah".to_string())]);

        assert_eq!(params, expected);
    }
}
