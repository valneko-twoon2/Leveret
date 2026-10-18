//! Lane assignment for the history graph.

use super::{Edge, EdgeKind, GraphRow, LogEntry};

/// Assigns each commit a column and the line segments needed to draw the graph.
///
/// `lanes[i]` holds the commit hash that lane `i` is waiting for (a parent of an
/// already placed commit) together with the lane's color. Lane slots are reused
/// once they end so the graph stays narrow.
pub(super) fn build_graph(entries: &mut [LogEntry]) {
    let mut lanes: Vec<Option<(String, u16)>> = Vec::new();
    let mut next_color: u16 = 0;
    let mut new_color = || {
        let c = next_color;
        next_color = next_color.wrapping_add(1);
        c
    };
    let free_slot = |lanes: &mut Vec<Option<(String, u16)>>| -> usize {
        match lanes.iter().position(Option::is_none) {
            Some(i) => i,
            None => {
                lanes.push(None);
                lanes.len() - 1
            }
        }
    };

    for entry in entries.iter_mut() {
        let mut edges = Vec::new();
        let waiting = |l: &Option<(String, u16)>| l.as_ref().is_some_and(|(h, _)| *h == entry.hash);

        let existing = lanes.iter().position(waiting);
        let col = match existing {
            Some(i) => i,
            None => {
                let i = free_slot(&mut lanes);
                lanes[i] = Some((entry.hash.clone(), new_color()));
                i
            }
        };
        let color = lanes[col].as_ref().map_or(0, |(_, c)| *c);

        // Lines entering from above: lanes merging into this commit and lanes passing through.
        let mut through = Vec::new();
        for (i, lane) in lanes.iter().enumerate() {
            let Some((hash, c)) = lane else { continue };
            if *hash == entry.hash {
                if existing.is_some() {
                    edges.push(Edge {
                        from: i as u16,
                        to: col as u16,
                        kind: EdgeKind::Up,
                        color: *c,
                    });
                }
            } else {
                through.push((i, *c));
                edges.push(Edge {
                    from: i as u16,
                    to: i as u16,
                    kind: EdgeKind::Up,
                    color: *c,
                });
            }
        }
        for lane in lanes.iter_mut() {
            if waiting(lane) {
                *lane = None;
            }
        }

        // Lines leaving downwards towards the parents. The first parent always
        // continues this commit's lane (even if another lane already waits for
        // it; the lanes converge at the parent), which keeps the mainline straight.
        for (k, parent) in entry.parents.iter().enumerate() {
            let target = if k == 0 {
                None
            } else {
                lanes.iter().position(|l| l.as_ref().is_some_and(|(h, _)| h == parent))
            };
            let (j, c) = match target {
                Some(j) => (j, lanes[j].as_ref().map_or(0, |(_, c)| *c)),
                None => {
                    let j = if k == 0 { col } else { free_slot(&mut lanes) };
                    let c = if k == 0 { color } else { new_color() };
                    lanes[j] = Some((parent.clone(), c));
                    (j, c)
                }
            };
            edges.push(Edge {
                from: col as u16,
                to: j as u16,
                kind: EdgeKind::Down,
                color: c,
            });
        }
        for (i, c) in through {
            edges.push(Edge {
                from: i as u16,
                to: i as u16,
                kind: EdgeKind::Down,
                color: c,
            });
        }

        let width = edges
            .iter()
            .map(|e| e.from.max(e.to))
            .chain([col as u16])
            .max()
            .unwrap_or(0)
            + 1;
        entry.graph = GraphRow {
            col: col as u16,
            color,
            width,
            edges,
        };

        while matches!(lanes.last(), Some(None)) {
            lanes.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(hash: &str, parents: &[&str]) -> LogEntry {
        LogEntry {
            hash: hash.into(),
            parents: parents.iter().map(|p| p.to_string()).collect(),
            author_name: String::new(),
            author_email: String::new(),
            author_time: 0,
            refs: Vec::new(),
            subject: String::new(),
            graph: GraphRow::default(),
            file_path: None,
        }
    }

    #[test]
    fn graph_for_merge() {
        // m merges a (first parent) and b; both come from root r.
        let mut e = vec![
            entry("m", &["a", "b"]),
            entry("b", &["r"]),
            entry("a", &["r"]),
            entry("r", &[]),
        ];
        build_graph(&mut e);
        assert_eq!(e[0].graph.col, 0);
        assert_eq!(e[1].graph.col, 1);
        assert_eq!(e[2].graph.col, 0);
        assert_eq!(e[3].graph.col, 0);
        // r receives lines from both lanes.
        let ups: Vec<_> = e[3]
            .graph
            .edges
            .iter()
            .filter(|x| x.kind == EdgeKind::Up)
            .map(|x| x.from)
            .collect();
        assert_eq!(ups, vec![0, 1]);
        assert_eq!(e[0].graph.width, 2);
    }
}
