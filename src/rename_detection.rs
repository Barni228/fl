/// Find the globally optimal pairing between `new_paths` and `deleted_paths`
/// using brute-force permutation search over the smaller set.
///
/// Returns a list of `(new_path, deleted_index)` pairs that minimize total
/// path distance. Unpaired items on either side are left out — the caller
/// handles the leftover deleted paths as true deletions and the leftover new
/// paths were already emitted as additions before this function is called.
///
/// # Complexity
/// O(min(n,d)!) in the size of the smaller set. This is only called for paths
/// that share an identical hash, so groups larger than ~8 are vanishingly rare
/// in practice.
pub fn optimal_pairings<'a>(
    new_paths: &[&'a str],
    deleted_paths: &[&'a str],
) -> Vec<(&'a str, usize)> {
    // Orient so we always permute the smaller set against the larger one.
    // `a` indexes into new_paths, `b` indexes into deleted_paths.
    let (shorter, longer, swapped) = if new_paths.len() <= deleted_paths.len() {
        (new_paths, deleted_paths, false)
    } else {
        (deleted_paths, new_paths, true)
    };

    let n = shorter.len();
    // Choose n indices from `longer` to pair with `shorter[0..n]`.
    // We try every permutation of every n-subset of `longer`.
    let longer_indices: Vec<usize> = (0..longer.len()).collect();

    let mut best_cost = usize::MAX;
    let mut best_assignment: Vec<usize> = Vec::new(); // longer index for each shorter index

    // Generate all n-subsets of longer_indices, then all permutations of each.
    for_each_permutation_of_subset(&longer_indices, n, &mut |perm: &[usize]| {
        let cost: usize = perm
            .iter()
            .enumerate()
            .map(|(i, &j)| {
                let (np, dp) = if swapped {
                    (longer[j], shorter[i])
                } else {
                    (shorter[i], longer[j])
                };
                path_distance(np, dp)
            })
            .sum();

        if cost < best_cost {
            best_cost = cost;
            best_assignment = perm.to_vec();
        }
    });

    if best_assignment.is_empty() {
        return Vec::new();
    }

    best_assignment
        .iter()
        .enumerate()
        .map(|(i, &j)| {
            if swapped {
                // shorter = deleted_paths, longer = new_paths
                (longer[j], i) // (new_path, deleted_index)
            } else {
                // shorter = new_paths, longer = deleted_paths
                (shorter[i], j) // (new_path, deleted_index)
            }
        })
        .collect()
}

/// Iterate over every permutation of every `k`-sized subset of `items`,
/// calling `f` for each permutation.
fn for_each_permutation_of_subset<T: Copy>(items: &[T], k: usize, f: &mut impl FnMut(&[T])) {
    let mut chosen = Vec::with_capacity(k);
    let mut used = vec![false; items.len()];
    permute(items, k, &mut chosen, &mut used, f);
}

fn permute<T: Copy>(
    items: &[T],
    remaining: usize,
    chosen: &mut Vec<T>,
    used: &mut Vec<bool>,
    f: &mut impl FnMut(&[T]),
) {
    if remaining == 0 {
        f(chosen);
        return;
    }
    for i in 0..items.len() {
        if !used[i] {
            used[i] = true;
            chosen.push(items[i]);
            permute(items, remaining - 1, chosen, used, f);
            chosen.pop();
            used[i] = false;
        }
    }
}

/// Score the dissimilarity between two paths. Lower is more similar.
///
/// The filename component is weighted more heavily than the directory component
/// because a file moved to a sibling directory is a more common and more
/// "obvious" rename than one that is also renamed at the same time.
fn path_distance(a: &str, b: &str) -> usize {
    let a_name = a.rsplit('/').next().unwrap_or(a);
    let b_name = b.rsplit('/').next().unwrap_or(b);
    let a_dir = a.rsplit_once('/').map_or("", |(dir, _)| dir);
    let b_dir = b.rsplit_once('/').map_or("", |(dir, _)| dir);

    let name_dist = strsim::damerau_levenshtein(a_name, b_name);
    let dir_dist = strsim::damerau_levenshtein(a_dir, b_dir);

    // Filename similarity matters more than directory similarity.
    name_dist * 3 + dir_dist
}
