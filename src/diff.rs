//! Structural comparison of two statecharts.
//!
//! Produces a list of differences between two statecharts, useful for:
//! - Reviewing changes before saving an edited SCXML
//! - Validating that a roundtrip (parse → export → parse) is lossless
//! - Comparing compiled output against expected definitions

use crate::model::{State, Statechart};

/// A single difference between two statecharts.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Difference {
    /// Path to the differing element (e.g. `states.review.transitions\[1\].guard`).
    pub path: String,
    /// What changed.
    pub kind: DiffKind,
}

/// The type of difference.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiffKind {
    /// A value changed from `old` to `new`.
    Changed {
        /// The old value.
        old: String,
        /// The new value.
        new: String,
    },
    /// An element was added.
    Added {
        /// Description of the added element.
        value: String,
    },
    /// An element was removed.
    Removed {
        /// Description of the removed element.
        value: String,
    },
}

/// Compare two statecharts and return all structural differences.
///
/// States are matched by ID (semantic diff), not by array position.
/// Returns an empty vec if the charts are structurally equivalent.
///
/// ```rust
/// use scxml::{parse_xml};
/// use scxml::diff::diff;
///
/// let xml_a = r#"
///     <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
///         <state id="a"><transition event="go" target="b"/></state>
///         <final id="b"/>
///     </scxml>
/// "#;
/// let xml_b = r#"
///     <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
///         <state id="a"><transition event="go" target="b" cond="ready"/></state>
///         <final id="b"/>
///     </scxml>
/// "#;
/// let a = parse_xml(xml_a).unwrap();
/// let b = parse_xml(xml_b).unwrap();
/// let diffs = diff(&a, &b);
/// assert!(!diffs.is_empty());
/// assert!(diffs.iter().any(|d| d.path.contains("guard")));
/// ```
pub fn diff(a: &Statechart, b: &Statechart) -> Vec<Difference> {
    let mut diffs = Vec::new();

    if a.initial != b.initial {
        diffs.push(Difference {
            path: "initial".into(),
            kind: DiffKind::Changed {
                old: a.initial.to_string(),
                new: b.initial.to_string(),
            },
        });
    }

    if a.name != b.name {
        diffs.push(Difference {
            path: "name".into(),
            kind: DiffKind::Changed {
                old: format!("{:?}", a.name),
                new: format!("{:?}", b.name),
            },
        });
    }

    let limit = crate::max_depth();
    diff_states(&a.states, &b.states, "states", 0, &mut diffs, limit);

    diffs
}

fn diff_states(
    a: &[State],
    b: &[State],
    prefix: &str,
    depth: usize,
    diffs: &mut Vec<Difference>,
    limit: usize,
) {
    if depth > limit {
        return;
    }
    // Build ID-indexed maps for semantic matching.
    let a_map: std::collections::BTreeMap<&str, &State> =
        a.iter().map(|s| (s.id.as_str(), s)).collect();
    let b_map: std::collections::BTreeMap<&str, &State> =
        b.iter().map(|s| (s.id.as_str(), s)).collect();

    // States present in both: compare.
    for (id, sa) in &a_map {
        if let Some(sb) = b_map.get(id) {
            diff_state(sa, sb, &format!("{prefix}.{id}"), depth, diffs, limit);
        } else {
            diffs.push(Difference {
                path: format!("{prefix}.{id}"),
                kind: DiffKind::Removed {
                    value: id.to_string(),
                },
            });
        }
    }

    // States only in b: added.
    for id in b_map.keys() {
        if !a_map.contains_key(id) {
            diffs.push(Difference {
                path: format!("{prefix}.{id}"),
                kind: DiffKind::Added {
                    value: id.to_string(),
                },
            });
        }
    }
}

fn diff_state(
    a: &State,
    b: &State,
    prefix: &str,
    depth: usize,
    diffs: &mut Vec<Difference>,
    limit: usize,
) {
    // IDs match by construction (caller matched by ID).
    if a.kind != b.kind {
        diffs.push(Difference {
            path: format!("{prefix}.kind"),
            kind: DiffKind::Changed {
                old: format!("{:?}", a.kind),
                new: format!("{:?}", b.kind),
            },
        });
    }

    if a.transitions.len() != b.transitions.len() {
        diffs.push(Difference {
            path: format!("{prefix}.transitions.len"),
            kind: DiffKind::Changed {
                old: a.transitions.len().to_string(),
                new: b.transitions.len().to_string(),
            },
        });
    } else {
        for (j, (ta, tb)) in a.transitions.iter().zip(b.transitions.iter()).enumerate() {
            let tp = format!("{prefix}.transitions[{j}]");
            if ta.event != tb.event {
                diffs.push(Difference {
                    path: format!("{tp}.event"),
                    kind: DiffKind::Changed {
                        old: format!("{:?}", ta.event),
                        new: format!("{:?}", tb.event),
                    },
                });
            }
            if ta.guard != tb.guard {
                diffs.push(Difference {
                    path: format!("{tp}.guard"),
                    kind: DiffKind::Changed {
                        old: format!("{:?}", ta.guard),
                        new: format!("{:?}", tb.guard),
                    },
                });
            }
            if ta.targets != tb.targets {
                diffs.push(Difference {
                    path: format!("{tp}.targets"),
                    kind: DiffKind::Changed {
                        old: format!("{:?}", ta.targets),
                        new: format!("{:?}", tb.targets),
                    },
                });
            }
            if ta.delay != tb.delay {
                diffs.push(Difference {
                    path: format!("{tp}.delay"),
                    kind: DiffKind::Changed {
                        old: format!("{:?}", ta.delay),
                        new: format!("{:?}", tb.delay),
                    },
                });
            }
            if ta.quorum != tb.quorum {
                diffs.push(Difference {
                    path: format!("{tp}.quorum"),
                    kind: DiffKind::Changed {
                        old: format!("{:?}", ta.quorum),
                        new: format!("{:?}", tb.quorum),
                    },
                });
            }
        }
    }

    // Recurse into children.
    diff_states(
        &a.children,
        &b.children,
        &format!("{prefix}.children"),
        depth + 1,
        diffs,
        limit,
    );
}

/// Returns `true` if two statecharts are structurally equivalent.
pub fn is_equivalent(a: &Statechart, b: &Statechart) -> bool {
    diff(a, b).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Transition;

    fn simple_chart() -> Statechart {
        Statechart::new(
            "a",
            vec![
                {
                    let mut s = State::atomic("a");
                    s.transitions.push(Transition::new("go", "b"));
                    s
                },
                State::final_state("b"),
            ],
        )
    }

    #[test]
    fn identical_charts_no_diff() {
        let a = simple_chart();
        let b = simple_chart();
        assert!(is_equivalent(&a, &b));
    }

    #[test]
    fn different_initial() {
        let a = simple_chart();
        let mut b = simple_chart();
        b.initial = "b".into();
        let diffs = diff(&a, &b);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "initial");
    }

    #[test]
    fn added_state() {
        let a = simple_chart();
        let mut b = simple_chart();
        b.states.push(State::atomic("c"));
        let diffs = diff(&a, &b);
        assert!(diffs.iter().any(|d| d.path == "states.c"
            && matches!(&d.kind, DiffKind::Added { value } if value == "c")));
    }

    #[test]
    fn changed_guard() {
        let a = simple_chart();
        let mut b = simple_chart();
        b.states[0].transitions[0] = Transition::new("go", "b").with_guard("new_guard");
        let diffs = diff(&a, &b);
        assert!(diffs.iter().any(|d| d.path.contains("guard")));
    }

    #[test]
    fn roundtrip_is_equivalent() {
        let xml = r#"
            <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
                <state id="a"><transition event="go" target="b"/></state>
                <final id="b"/>
            </scxml>
        "#;
        let chart = crate::parse_xml(xml).unwrap();
        let exported = crate::export::xml::to_xml(&chart);
        let chart2 = crate::parse_xml(&exported).unwrap();
        assert!(is_equivalent(&chart, &chart2));
    }
}
