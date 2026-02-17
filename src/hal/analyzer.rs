use super::tokenizer::TokenCounts;

/// Halstead complexity metrics computed from token counts.
#[derive(Debug, Clone)]
pub struct HalsteadMetrics {
    pub distinct_operators: usize, // η₁
    pub distinct_operands: usize,  // η₂
    pub total_operators: usize,    // N₁
    pub total_operands: usize,     // N₂
    pub vocabulary: usize,         // η = η₁ + η₂
    pub length: usize,             // N = N₁ + N₂
    pub volume: f64,               // V = N × log₂(η)
    pub difficulty: f64,           // D = (η₁ / 2) × (N₂ / η₂)
    pub effort: f64,               // E = D × V
    pub bugs: f64,                 // B = V / 3000
    pub time: f64,                 // T = E / 18 (seconds)
}

/// Compute Halstead metrics from raw token counts.
/// Returns `None` if there are no tokens (vocabulary = 0).
pub fn compute(counts: &TokenCounts) -> Option<HalsteadMetrics> {
    let n1 = counts.distinct_operators.len();
    let n2 = counts.distinct_operands.len();
    let big_n1 = counts.total_operators;
    let big_n2 = counts.total_operands;

    let vocabulary = n1 + n2;
    // Both operators and operands are needed for meaningful metrics.
    if n1 == 0 || n2 == 0 {
        return None;
    }

    let length = big_n1 + big_n2;
    let volume = length as f64 * (vocabulary as f64).log2();

    let difficulty = (n1 as f64 / 2.0) * (big_n2 as f64 / n2 as f64);

    let effort = difficulty * volume;
    // Halstead's delivered bugs estimate (B = V / 3000).
    // See: Halstead, M. (1977) "Elements of Software Science".
    let bugs = volume / 3000.0;
    // Stroud number: 18 elementary mental discriminations per second.
    let time = effort / 18.0;

    Some(HalsteadMetrics {
        distinct_operators: n1,
        distinct_operands: n2,
        total_operators: big_n1,
        total_operands: big_n2,
        vocabulary,
        length,
        volume,
        difficulty,
        effort,
        bugs,
        time,
    })
}

#[cfg(test)]
#[path = "analyzer_test.rs"]
mod tests;
