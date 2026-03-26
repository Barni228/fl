use pathfinding::prelude::*;

/// Find the globally optimal pairing between `old_paths` and `new_paths`,
/// minimizing total [`path_distance`].
///
/// Returns a list of `(old_path, new_path)` pairs
pub fn optimal_pairings<'a>(
    old_paths: &[&'a str],
    new_paths: &[&'a str],
) -> Vec<(&'a str, &'a str)> {
    // Matrix will panic if its empty
    if old_paths.is_empty() || new_paths.is_empty() {
        return Vec::new();
    }

    // if there are more rows than columns, flip the matrix since this is required by the hungarian algorithm
    let flip = old_paths.len() > new_paths.len();

    // build up the cost matrix
    // basically it is just a grid where rows are new paths and columns are deleted paths
    // and every cell in there is the distance between the two
    // so something like:
    // |         | new | the | poppy |
    // |---------|-----|-----|-------|
    // | old     |  3  |  3  |   4   |
    // | there   |  4  |  2  |   5   |

    let costs = if !flip {
        Matrix::from_rows(
            old_paths
                .iter()
                .map(|old| new_paths.iter().map(|new| path_distance(old, new) as i32)),
        )
        .unwrap()
    } else {
        Matrix::from_rows(
            new_paths
                .iter()
                .map(|new| old_paths.iter().map(|old| path_distance(new, old) as i32)),
        )
        .unwrap()
    };

    // hungarian algorithm will pair every row with every column once, in a way to minimize the total cost
    // so `old -> new` and `the -> there`, and `troll` is left out, making the total cost `5`
    let (_cost, assignments) = kuhn_munkres_min(&costs);
    assignments
        .into_iter()
        .enumerate()
        // convert the indexes (i32, i32) to (new_path, deleted_path)
        .map(|(r, c)| {
            if !flip {
                (old_paths[r], new_paths[c])
            } else {
                (old_paths[c], new_paths[r])
            }
        })
        .collect()
}

/// Returns how different the two paths are
///
/// Bigger number = more different
/// 0 = exactly the same
/// It values file name changes more than directory changes
/// So `foo -> src/foo` is less than `foo -> bob`
pub fn path_distance(a: &str, b: &str) -> usize {
    let (a_dir, a_name) = a.rsplit_once('/').unwrap_or(("", a));
    let (b_dir, b_name) = b.rsplit_once('/').unwrap_or(("", b));

    let name_dist = strsim::damerau_levenshtein(a_name, b_name);
    let dir_dist = strsim::damerau_levenshtein(a_dir, b_dir);

    name_dist * 3 + dir_dist
}

#[cfg(test)]
mod tests {
    use super::*;

    // this is mainly to make sure the comment in `path_distance` is correct
    #[test]
    fn test_optimal_pairings() {
        let old_paths = ["old", "there"];
        let new_paths = ["new", "the", "poppy"];
        assert_eq!(
            optimal_pairings(&old_paths, &new_paths),
            vec![("old", "new"), ("there", "the")]
        );
    }

    #[test]
    fn test_hungarian() {
        // I know that these "path_differences" have to be multiplied by 3, but for simplicity this is good enough
        #[rustfmt::skip]
        let costs = Matrix::from_rows([
            [3, 3, 4],
            [4, 2, 5],
        ]).unwrap();

        // match 0 (index) to Some(0) ()
        assert_eq!(
            kuhn_munkres_min(&costs),
            (
                5,
                vec![
                    0, // [0] in this vec, so column[0] is paired with row[0]
                    1, // [1], so column[1] is paired with row[1]
                ]
            )
        );
    }

    #[test]
    fn test_path_difference() {
        assert_eq!(path_distance("old", "new"), 3 * 3);
        assert_eq!(path_distance("old", "the"), 3 * 3);
        assert_eq!(path_distance("old", "poppy"), 4 * 3);

        assert_eq!(path_distance("there", "new"), 4 * 3);
        assert_eq!(path_distance("there", "the"), 2 * 3);
        assert_eq!(path_distance("there", "poppy"), 5 * 3);
    }

    #[test]
    fn test_optimal_pairings_empty() {
        assert_eq!(optimal_pairings(&[], &["dont", "panic"]), vec![]);
    }
}
