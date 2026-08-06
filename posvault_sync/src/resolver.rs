use posvault_handler::errors::Result;
use posvault_handler::traits::ConflictResolver;

#[derive(Debug)]
pub struct UnionCsvResolver;

impl ConflictResolver for UnionCsvResolver {
    fn resolve(&self, base: &[u8], ours: &[u8], theirs: &[u8]) -> Result<Vec<u8>> {
        let base_lines = lines(base);
        let our_lines = lines(ours);
        let their_lines = lines(theirs);

        if our_lines == base_lines {
            Ok(theirs.to_vec())
        } else if their_lines == base_lines {
            Ok(ours.to_vec())
        } else {
            let mut merged = our_lines.clone();
            for line in &their_lines {
                if !merged.contains(line) {
                    merged.push(line.clone());
                }
            }
            Ok(merged.join("\n").into_bytes())
        }
    }
}

fn lines(data: &[u8]) -> Vec<String> {
    std::str::from_utf8(data)
        .unwrap_or("")
        .lines()
        .map(|s| s.to_owned())
        .collect()
}
